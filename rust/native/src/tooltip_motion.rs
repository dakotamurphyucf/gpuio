//! Fixed-size presentation history and frame-scoped native tooltip motion.
//! History owns only values, never content, handlers, entities or a timer.
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Point, Window, div, prelude::*, px,
};
use gpuio_protocol::NodeId;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
pub(super) struct Previous {
    pub owner: NodeId,
    pub anchor: Bounds<Pixels>,
    pub closed: Instant,
}
pub(super) type Painted = Rc<Cell<Option<Bounds<Pixels>>>>;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Transition {
    Enter,
    Slide(f32),
    Immediate,
}
#[derive(Clone, Copy, Debug)]
struct Sample {
    offset: Point<Pixels>,
    alpha: f32,
    running: bool,
}
impl Sample {
    fn settled() -> Self {
        Self {
            offset: Point::default(),
            alpha: 1.,
            running: false,
        }
    }
}
struct State {
    transition: Transition,
    start: Option<Instant>,
    settled: bool,
}
impl State {
    fn new(previous: Option<Bounds<Pixels>>, current: Bounds<Pixels>) -> Self {
        let transition = match previous {
            None => Transition::Enter,
            Some(previous) if (current.origin.y - previous.origin.y).abs() < px(10.) => {
                Transition::Slide(f32::from(previous.center().x - current.center().x))
            }
            Some(_) => Transition::Immediate,
        };
        Self {
            transition,
            start: None,
            settled: false,
        }
    }
    fn sample(&mut self, enabled: bool, reduced: bool, now: Instant) -> Sample {
        if !enabled || reduced || self.transition == Transition::Immediate {
            self.settled = true;
        }
        if self.settled {
            return Sample::settled();
        }
        let duration = match self.transition {
            Transition::Enter => Duration::from_millis(150),
            Transition::Slide(_) => Duration::from_millis(200),
            Transition::Immediate => unreachable!(),
        };
        let t = self
            .start
            .map_or(0., |start| {
                now.saturating_duration_since(start).as_secs_f32() / duration.as_secs_f32()
            })
            .min(1.);
        if t == 1. {
            self.settled = true;
            return Sample::settled();
        }
        match self.transition {
            Transition::Enter => {
                let eased = 1. - (1. - t).powi(3);
                Sample {
                    offset: gpui::point(px(0.), px(4. * (1. - eased))),
                    alpha: eased,
                    running: true,
                }
            }
            Transition::Slide(distance) => {
                let eased = if t < 0.5 {
                    4. * t.powi(3)
                } else {
                    1. - (-2. * t + 2.).powi(3) / 2.
                };
                Sample {
                    offset: gpui::point(px(distance * (1. - eased)), px(0.)),
                    alpha: 1.,
                    running: true,
                }
            }
            Transition::Immediate => unreachable!(),
        }
    }
}

pub(super) struct Entry {
    pub id: ElementId,
    pub enabled: bool,
    pub anchor: Rc<Cell<Bounds<Pixels>>>,
    pub previous: Option<Bounds<Pixels>>,
    pub content: Option<AnyElement>,
}
pub(super) struct Frame {
    content: AnyElement,
    state: Rc<RefCell<State>>,
    sample: Sample,
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
        // Deferred layout happens after ordinary anchors have their current bounds.
        let anchor = self.anchor.get();
        window.with_element_state(id.expect("tooltip motion identity"), |state, window| {
            let state: Rc<RefCell<State>> =
                state.unwrap_or_else(|| Rc::new(RefCell::new(State::new(self.previous, anchor))));
            let sample = state.borrow_mut().sample(
                self.enabled,
                cx.reduce_motion(),
                cx.background_executor().now(),
            );
            let mut content = div()
                .opacity(sample.alpha)
                .child(self.content.take().expect("one tooltip layout"))
                .into_any_element();
            let layout = content.request_layout(window, cx);
            (
                (
                    layout,
                    Frame {
                        content,
                        state: state.clone(),
                        sample,
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
        window.with_element_offset(frame.sample.offset, |window| {
            frame.content.prepaint(window, cx)
        });
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
        if frame.sample.running {
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
    fn anchor(x: f32, y: f32, width: f32) -> Bounds<Pixels> {
        Bounds::new(gpui::point(px(x), px(y)), gpui::size(px(width), px(30.)))
    }
    #[test]
    fn timing_uses_first_paint_and_trigger_rows_and_centers() {
        let now = Instant::now();
        let mut enter = State::new(None, anchor(100., 100., 100.));
        assert_eq!(enter.sample(true, false, now).alpha, 0.);
        assert_eq!(
            enter
                .sample(true, false, now + Duration::from_secs(1))
                .alpha,
            0.
        );
        enter.start = Some(now);
        let mid = enter.sample(true, false, now + Duration::from_millis(75));
        assert_eq!(mid.alpha, 0.875);
        assert_eq!(mid.offset.y, px(0.5));
        assert!(
            !enter
                .sample(true, false, now + Duration::from_millis(150))
                .running
        );
        let mut slide = State::new(Some(anchor(100., 100., 100.)), anchor(300., 109., 200.));
        assert_eq!(slide.transition, Transition::Slide(-250.));
        slide.start = Some(now);
        let mid = slide.sample(true, false, now + Duration::from_millis(100));
        assert_eq!(mid.alpha, 1.);
        assert_eq!(mid.offset.x, px(-125.));
        assert!(
            !slide
                .sample(true, false, now + Duration::from_millis(200))
                .running
        );
        let mut cross = State::new(Some(anchor(100., 100., 100.)), anchor(300., 110., 100.));
        assert!(!cross.sample(true, false, now).running);
    }
    #[test]
    fn reducing_or_disabling_settles_without_replay() {
        let now = Instant::now();
        for (enabled, reduced) in [(false, false), (true, true)] {
            let mut state = State::new(None, anchor(100., 100., 100.));
            assert!(!state.sample(enabled, reduced, now).running);
            assert!(!state.sample(true, false, now).running);
        }
    }
}
