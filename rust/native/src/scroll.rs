//! Native container wheel routing. GPUI owns layout and clamping; a first-child
//! viewport hitbox handles eligible deltas before the parent's default listener,
//! after nested content. Events at boundaries can continue toward ancestors.
use gpui::{
    A11ySubtreeBuilder, App, Bounds, Element, ElementId, GlobalElementId, Hitbox,
    InspectorElementId, InteractiveElement, IntoElement, LayoutId, Pixels, Styled, Window,
    accesskit, canvas, prelude::*, px,
};
use gpuio_protocol::v1::{Field, Style};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
#[derive(Clone, Copy, Default)]
struct Axes {
    x: bool,
    y: bool,
}
#[derive(Default)]
pub(super) struct State {
    pub(super) handle: gpui::ScrollHandle,
    axes: Cell<Axes>,
    ongoing: RefCell<gpui::OngoingScroll>,
}
pub(super) fn declared(styles: &[Style]) -> bool {
    styles.iter().any(|style| match style {
        Style::Fields(fields) | Style::State(_, fields) => fields
            .iter()
            .any(|field| matches!(field, Field::OverflowX(3) | Field::OverflowY(3))),
        _ => false,
    })
}
pub(super) fn attach(
    mut element: gpui::Stateful<gpui::Div>,
    state: &Rc<State>,
) -> gpui::Stateful<gpui::Div> {
    // A wheel axis must not be silently translated into a different axis by the
    // fallback listener when our handler lets a boundary event bubble.
    element.style().restrict_scroll_to_axis = Some(true);
    let measure = Rc::downgrade(state);
    let paint = measure.clone();
    element.track_scroll(&state.handle).child(
        canvas(
            move |_, window, _| {
                measure.upgrade().map(|state| {
                    window.insert_hitbox(state.handle.bounds(), gpui::HitboxBehavior::Normal)
                })
            },
            move |_, hitbox, window, _| {
                let Some(hitbox) = hitbox else {
                    return;
                };
                let line_height = window.line_height();
                window.on_mouse_event(move |event: &gpui::ScrollWheelEvent, phase, window, cx| {
                    if phase != gpui::DispatchPhase::Bubble || !hitbox.should_handle_scroll(window)
                    {
                        return;
                    }
                    let Some(state) = paint.upgrade() else {
                        return;
                    };
                    let axes = state.axes.get();
                    let mut delta = event.delta.pixel_delta(line_height);
                    if event.delta.precise() {
                        state
                            .ongoing
                            .borrow_mut()
                            .filter(&mut delta, event.touch_phase);
                    }
                    if !axes.x {
                        delta.x = px(0.);
                    }
                    if !axes.y {
                        delta.y = px(0.);
                    }
                    if delta.x != px(0.) && delta.y != px(0.) {
                        if delta.x.abs() > delta.y.abs() {
                            delta.y = px(0.);
                        } else {
                            delta.x = px(0.);
                        }
                    }
                    let max = state.handle.max_offset();
                    let clamp = |point: gpui::Point<Pixels>| {
                        gpui::point(point.x.clamp(-max.x, px(0.)), point.y.clamp(-max.y, px(0.)))
                    };
                    let old = clamp(state.handle.offset());
                    let next = clamp(old + delta);
                    if next != old {
                        state.handle.set_offset(next);
                        window.refresh();
                        cx.stop_propagation();
                    }
                });
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full(),
    )
}
pub(super) struct Frame<E> {
    element: E,
    state: Weak<State>,
}
impl<E> Frame<E> {
    pub(super) fn new(element: E, state: &Rc<State>) -> Self {
        Self {
            element,
            state: Rc::downgrade(state),
        }
    }
}
impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement + Styled> IntoElement
    for Frame<E>
{
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement + Styled> Element
    for Frame<E>
{
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;
    fn id(&self) -> Option<ElementId> {
        Element::id(&self.element)
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.element.source_location()
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.element.request_layout(id, inspector, window, cx)
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.element
            .prepaint(id, inspector, bounds, layout, window, cx)
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(state) = self.state.upgrade() {
            let style =
                self.element
                    .interactivity()
                    .compute_style(id, prepaint.as_ref(), window, cx);
            state.axes.set(Axes {
                x: style.overflow.x == gpui::Overflow::Scroll,
                y: style.overflow.y == gpui::Overflow::Scroll,
            });
        }
        self.element
            .paint(id, inspector, bounds, layout, prepaint, window, cx);
    }
    fn a11y_role(&self) -> Option<accesskit::Role> {
        self.element.a11y_role()
    }
    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.element.write_a11y_info(node);
    }

    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut A11ySubtreeBuilder,
    ) {
        self.element.a11y_synthetic_children(prepaint, builder);
    }
}
