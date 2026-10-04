//! Native wheel routing and one coalesced quiet deadline. Timers hold a weak View
//! and weak mounted source; neither callbacks nor per-frame offsets enter OCaml.
use super::*;
use crate::carousel_track_wheel::{Intent, Output, Phase};
use std::time::Duration;
impl State {
    #[cfg(test)]
    pub(in super::super) fn wheel_snapshot(&self) -> (bool, bool, bool) {
        (
            self.wheel_task.is_some(),
            self.wheel.active(),
            self.motion.previewing(),
        )
    }
    pub(super) fn track_now(&mut self, cx: &App) -> Duration {
        let now = cx.background_executor().now();
        now.saturating_duration_since(*self.origin.get_or_insert(now))
    }
    pub(super) fn interrupt_track_wheel(&mut self, now: Duration, settle: bool) {
        if self.wheel.interrupt(now) && settle {
            self.motion.finish_preview();
        }
        self.wheel_task = None;
        self.wheel_axis = Default::default();
    }
}
impl View {
    fn wheel_available(&self, id: NodeId, window: &Window, cx: &App) -> bool {
        self.track_pointer_available(id, window, cx)
            && window.captured_hitbox().is_none()
            && self
                .carousel_tracks
                .get(&id)
                .is_some_and(|state| state.borrow().drag.is_none())
    }
    fn wheel_output(&mut self, id: NodeId, output: Output, window: &mut Window) {
        let Some(state) = self.carousel_tracks.get(&id) else {
            return;
        };
        let requests = {
            let mut state = state.borrow_mut();
            if output.finished {
                state.motion.finish_preview();
                state.wheel_axis = Default::default();
                window.refresh();
            }
            if let Some(offset) = output.preview {
                state.motion.preview(f64::from(offset));
                window.refresh();
            }
            output
                .intents
                .into_iter()
                .filter_map(|intent| match intent {
                    Intent::Step(crate::carousel_gesture::Step::Next) => {
                        state.model.manual(Request::Next)
                    }
                    Intent::Step(crate::carousel_gesture::Step::Previous) => {
                        state.model.manual(Request::Previous)
                    }
                    Intent::Select(index) => state
                        .model
                        .config()
                        .carousel
                        .ids
                        .get(index)
                        .map(|id| Request::Select(id.clone())),
                })
                .collect::<Vec<_>>()
        };
        for request in requests {
            self.carousel_track_request(id, request);
        }
    }
    pub(in super::super) fn track_wheel(
        &mut self,
        id: NodeId,
        expected: &Rc<RefCell<State>>,
        config: &Arc<gpuio_protocol::carousel_track::Config>,
        event: &gpui::ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.modifiers.modified()
            || !self.wheel_available(id, window, cx)
            || !self.carousel_tracks.get(&id).is_some_and(|state| {
                Rc::ptr_eq(state, expected) && state.borrow().model.config() == config.as_ref()
            })
        {
            return;
        }
        let painted_before = expected.borrow().motion.painted_offset();
        let expired = {
            let mut state = expected.borrow_mut();
            let now = state.track_now(cx);
            state.wheel.expire(now)
        };
        self.wheel_output(id, expired, window);
        let mut delta = event.delta.pixel_delta(window.line_height());
        let precise = event.delta.precise();
        let output = {
            let mut state = expected.borrow_mut();
            if precise {
                state.wheel_axis.filter(&mut delta, event.touch_phase);
            }
            if delta.x.abs() > delta.y.abs() {
                delta.y = px(0.);
            } else {
                delta.x = px(0.);
            }
            let primary = match config.carousel.axis {
                Axis::Horizontal => delta.x,
                Axis::Vertical => delta.y,
            };
            let now = state.track_now(cx);
            let phase = match event.touch_phase {
                gpui::TouchPhase::Started => Phase::Started,
                gpui::TouchPhase::Moved => Phase::Moved,
                gpui::TouchPhase::Ended => Phase::Ended,
                gpui::TouchPhase::Cancelled => Phase::Cancelled,
            };
            let Some(current) = state.model.config().carousel.selected else {
                return;
            };
            // An active preview has a canonical offset in its own wheel state;
            // this fallback is used only to start a new gesture.
            let painted = painted_before.or_else(|| state.model.geometry()?.snap(current as usize));
            let Some(painted) = painted else {
                return;
            };
            let State { model, wheel, .. } = &mut *state;
            let Some(geometry) = model.geometry() else {
                return;
            };
            let mut output = wheel.push(
                geometry,
                current as usize,
                painted,
                crate::carousel_track_wheel::Input {
                    delta: f64::from(primary),
                    precise,
                    phase,
                    now,
                },
            );
            // Pinned horizontal tracks retain their axis at the edge; vertical
            // tracks hand edge-started gestures to enclosing document scrollers.
            output.consumed |= config.carousel.axis == Axis::Horizontal && primary != px(0.);
            output
        };
        if output.consumed {
            cx.stop_propagation();
        }
        self.wheel_output(id, output, window);
        self.schedule_track_wheel(id, window, cx);
        self.schedule_carousel_track(id, window, cx);
    }
    pub(in super::super) fn schedule_track_wheel(
        &mut self,
        id: NodeId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.carousel_tracks.get(&id).cloned() else {
            return;
        };
        let available = self.wheel_available(id, window, cx);
        let mut state_ref = state.borrow_mut();
        let now = state_ref.track_now(cx);
        if !available {
            state_ref.interrupt_track_wheel(now, true);
            return;
        }
        let Some(deadline) = state_ref.wheel.deadline() else {
            state_ref.wheel_task = None;
            return;
        };
        if state_ref
            .wheel_task
            .as_ref()
            .is_some_and(|(epoch, _)| state_ref.wheel.matches(epoch))
        {
            return;
        }
        state_ref.wheel_task = None;
        let epoch = state_ref.wheel.epoch();
        let expected = Rc::downgrade(&state);
        state_ref.wheel_task = Some((
            epoch.clone(),
            cx.spawn_in(window, async move |owner, cx| {
                cx.background_executor()
                    .timer(deadline.saturating_sub(now))
                    .await;
                let _ = owner.update_in(cx, |view, window, cx| {
                    let Some(expected) = expected.upgrade() else {
                        return;
                    };
                    if !view
                        .carousel_tracks
                        .get(&id)
                        .is_some_and(|state| Rc::ptr_eq(state, &expected))
                        || !expected.borrow().wheel.matches(&epoch)
                    {
                        return;
                    }
                    let available = view.wheel_available(id, window, cx);
                    let output = {
                        let mut state = expected.borrow_mut();
                        state.wheel_task = None;
                        let now = state.track_now(cx);
                        if !available {
                            state.interrupt_track_wheel(now, true);
                            return;
                        }
                        let output = state.wheel.expire(now);
                        if state.wheel.deadline().is_none() {
                            state.wheel_axis = Default::default();
                        }
                        output
                    };
                    view.wheel_output(id, output, window);
                    view.schedule_track_wheel(id, window, cx);
                    view.schedule_carousel_track(id, window, cx);
                });
            }),
        ));
    }
}
