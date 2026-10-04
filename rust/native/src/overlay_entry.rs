//! Entry is scoped to the actually rendered deferred surface, with no timer,
//! retained content or callback into OCaml. The stable frame ID survives changes
//! to paint/configuration; absent surfaces retire GPUI's element state.
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Window,
};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Default)]
struct State {
    start: Option<Instant>,
    settled: bool,
}
impl State {
    fn sample(&mut self, enabled: bool, reduced: bool, now: Instant, duration: Duration) -> f32 {
        if !enabled || reduced {
            self.settled = true;
        }
        if self.settled {
            return 1.;
        }
        let progress = self
            .start
            .map_or(0., |start| {
                now.saturating_duration_since(start).as_secs_f32() / duration.as_secs_f32()
            })
            .min(1.);
        if progress == 1. {
            self.settled = true;
        }
        progress
    }
}

pub(super) struct Entry {
    pub id: ElementId,
    pub enabled: bool,
    pub node: gpuio_protocol::NodeId,
    pub focus: super::focus::Shared,
    pub duration: Duration,
    pub content: Option<Box<dyn FnOnce(f32) -> AnyElement>>,
}
pub(super) struct Frame {
    content: AnyElement,
    state: Rc<RefCell<State>>,
    progress: f32,
}
impl IntoElement for Entry {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Entry {
    type RequestLayoutState = Frame;
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Frame) {
        window.with_element_state(id.expect("entry identity"), |state, window| {
            let state: Rc<RefCell<State>> = state.unwrap_or_default();
            let progress = state.borrow_mut().sample(
                self.enabled,
                cx.reduce_motion(),
                cx.background_executor().now(),
                self.duration,
            );
            let mut content = self.content.take().expect("one entry layout")(progress);
            let layout = content.request_layout(window, cx);
            (
                (
                    layout,
                    Frame {
                        content,
                        state: state.clone(),
                        progress,
                    },
                ),
                state,
            )
        })
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        frame: &mut Frame,
        window: &mut Window,
        cx: &mut App,
    ) {
        frame.content.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        frame: &mut Frame,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        frame.content.paint(window, cx);
        if frame.progress < 1. {
            self.focus.borrow_mut().defer_overlay_entry(self.node);
            frame
                .state
                .borrow_mut()
                .start
                .get_or_insert_with(|| cx.background_executor().now());
            window.request_animation_frame();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_paint_starts_entry_and_settlement_never_replays() {
        let now = Instant::now();
        let duration = Duration::from_millis(250);
        let mut state = State::default();
        assert_eq!(state.sample(true, false, now, duration), 0.);
        assert_eq!(state.sample(true, false, now + duration, duration), 0.);
        state.start = Some(now + duration);
        assert_eq!(
            state.sample(true, false, now + duration + duration / 2, duration),
            0.5
        );
        assert_eq!(state.sample(true, false, now + duration * 2, duration), 1.);
        assert_eq!(state.sample(true, false, now + duration * 3, duration), 1.);
        for reduced in [false, true] {
            let mut state = State {
                start: Some(now),
                settled: false,
            };
            assert_eq!(state.sample(reduced, reduced, now, duration), 1.);
            assert_eq!(state.sample(true, false, now, duration), 1.);
        }
    }
}
