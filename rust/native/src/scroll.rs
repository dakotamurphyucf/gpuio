//! Native container wheel routing. GPUI owns layout and clamping; a first-child
//! viewport hitbox handles eligible deltas before the parent's default listener,
//! after nested content. Events at boundaries can continue toward ancestors.
use gpui::{
    A11ySubtreeBuilder, App, Bounds, Element, ElementId, GlobalElementId, Hitbox,
    InspectorElementId, InteractiveElement, IntoElement, LayoutId, Pixels, Styled, Window,
    accesskit, canvas, prelude::*, px,
};
use gpuio_protocol::{
    NodeId,
    v1::{Field, Style},
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Axes {
    x: bool,
    y: bool,
}
#[derive(Default)]
pub(super) struct State {
    pub(super) handle: gpui::ScrollHandle,
    pub(super) mask: Cell<Option<Bounds<Pixels>>>,
    axes: Cell<Axes>,
    ongoing: RefCell<gpui::OngoingScroll>,
}
impl gpui_base::ScrollbarHandle for State {
    fn viewport_bounds(&self) -> Bounds<Pixels> {
        self.handle.bounds()
    }
    fn offset(&self) -> gpui::Point<Pixels> {
        self.handle.offset()
    }
    fn set_offset(&self, offset: gpui::Point<Pixels>) {
        let axes = self.axes.get();
        let old = self.handle.offset();
        self.handle.set_offset(gpui::point(
            if axes.x { offset.x } else { old.x },
            if axes.y { offset.y } else { old.y },
        ));
    }
    fn content_size(&self) -> gpui::Size<Pixels> {
        let bounds = self.handle.bounds();
        let max = self.handle.max_offset();
        let axes = self.axes.get();
        gpui::size(
            bounds.size.width + if axes.x { max.x } else { px(0.) },
            bounds.size.height + if axes.y { max.y } else { px(0.) },
        )
    }
}
fn reveal_axis(start: Pixels, end: Pixels, near: Pixels, far: Pixels) -> Pixels {
    if end - start > far - near || start < near {
        near - start
    } else if end > far {
        far - end
    } else {
        px(0.)
    }
}
impl State {
    /// Compute reveal geometry without mutating scroll state (focus admission).
    pub(super) fn project(
        &self,
        mut target: Bounds<Pixels>,
    ) -> (Bounds<Pixels>, gpui::Point<Pixels>) {
        let viewport = self.mask.get().unwrap_or_else(|| self.handle.bounds());
        let axes = self.axes.get();
        let old = self.handle.offset();
        let maximum = self.handle.max_offset();
        let delta = gpui::point(
            if axes.x {
                reveal_axis(
                    target.left(),
                    target.right(),
                    viewport.left(),
                    viewport.right(),
                )
            } else {
                px(0.)
            },
            if axes.y {
                reveal_axis(
                    target.top(),
                    target.bottom(),
                    viewport.top(),
                    viewport.bottom(),
                )
            } else {
                px(0.)
            },
        );
        let next = gpui::point(
            if axes.x {
                (old.x + delta.x).clamp(-maximum.x, px(0.))
            } else {
                old.x
            },
            if axes.y {
                (old.y + delta.y).clamp(-maximum.y, px(0.))
            } else {
                old.y
            },
        );
        target.origin += next - old;
        (target, next)
    }
    /// Reveal with the least clamped movement on actual scroll axes and return
    /// translated target bounds for the next outer owner.
    pub(super) fn reveal(&self, target: Bounds<Pixels>) -> (Bounds<Pixels>, bool) {
        let (target, next) = self.project(target);
        let changed = next != self.handle.offset();
        if changed {
            self.handle.set_offset(next);
        }
        (target, changed)
    }
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
    focus: super::focus::Shared,
    node: NodeId,
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
                    // A two-axis viewport supports diagonal panning. Keep GPUI's
                    // gesture filtering only for single-axis containers, where
                    // it prevents cross-axis drift into nested scroll regions.
                    if event.delta.precise() && !(axes.x && axes.y) {
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
                    let max = state.handle.max_offset();
                    let clamp = |point: gpui::Point<Pixels>| {
                        gpui::point(point.x.clamp(-max.x, px(0.)), point.y.clamp(-max.y, px(0.)))
                    };
                    let old = clamp(state.handle.offset());
                    let next = clamp(old + delta);
                    if next != old {
                        state.handle.set_offset(next);
                        focus.borrow_mut().record_scroll_focus(node, window, cx);
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
    focus: super::focus::Shared,
    node: NodeId,
    tab_offset: Option<Weak<RefCell<super::tab_viewport::State>>>,
    scrollbar: Option<crate::scrollbar_widget::Shared>,
    scrollbar_element: Option<gpui::AnyElement>,
}
impl<E: InteractiveElement> InteractiveElement for Frame<E> {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.element.interactivity()
    }
}
impl<E> Frame<E> {
    pub(super) fn with_scrollbar(
        mut self,
        scrollbar: Option<crate::scrollbar_widget::Shared>,
    ) -> Self {
        self.scrollbar = scrollbar;
        self
    }
    pub(super) fn new(
        element: E,
        state: &Rc<State>,
        focus: super::focus::Shared,
        node: NodeId,
    ) -> Self {
        Self {
            element,
            state: Rc::downgrade(state),
            focus,
            node,
            tab_offset: None,
            scrollbar: None,
            scrollbar_element: None,
        }
    }
    pub(super) fn with_tab_offset(
        element: E,
        state: &Rc<State>,
        focus: super::focus::Shared,
        node: NodeId,
        tab: Option<&Rc<RefCell<super::tab_viewport::State>>>,
    ) -> Self {
        // Avoid a second bulky Frame temporary in the recursive host renderer.
        Self {
            element,
            state: Rc::downgrade(state),
            focus,
            node,
            tab_offset: tab.map(Rc::downgrade),
            scrollbar: None,
            scrollbar_element: None,
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
        if let (Some(tab), Some(scroll)) = (
            self.tab_offset.as_ref().and_then(Weak::upgrade),
            self.state.upgrade(),
        ) {
            tab.borrow_mut().prepare_layout(&scroll);
        }
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
        let prepaint = self
            .element
            .prepaint(id, inspector, bounds, layout, window, cx);
        let style = self
            .element
            .interactivity()
            .compute_style(id, prepaint.as_ref(), window, cx);
        if let Some(state) = self.state.upgrade() {
            let axes = Axes {
                x: style.overflow.x == gpui::Overflow::Scroll,
                y: style.overflow.y == gpui::Overflow::Scroll,
            };
            if state.axes.replace(axes) != axes {
                *state.ongoing.borrow_mut() = gpui::OngoingScroll::default();
            }
        }
        self.scrollbar_element = self.scrollbar.as_ref().map(|owner| {
            window.with_text_style(style.text_style().cloned(), |window| {
                let foreground = u32::from(window.text_style().color.to_rgb());
                let mut bar = crate::scrollbar_widget::overlay(
                    owner,
                    foreground,
                    style.opacity.unwrap_or(1.),
                );
                bar.layout_as_root(bounds.size.map(gpui::AvailableSpace::Definite), window, cx);
                bar.prepaint_at(bounds.origin, window, cx);
                bar
            })
        });
        prepaint
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
        let boundary = if let Some(state) = self.state.upgrade() {
            let style =
                self.element
                    .interactivity()
                    .compute_style(id, prepaint.as_ref(), window, cx);
            let axes = Axes {
                x: style.overflow.x == gpui::Overflow::Scroll,
                y: style.overflow.y == gpui::Overflow::Scroll,
            };
            state.mask.set(
                style
                    .overflow_mask(bounds, window.rem_size())
                    .map(|mask| mask.bounds),
            );
            if state.axes.replace(axes) != axes {
                // Hover/focus refinements can change axes during a gesture.
                // An earlier one-axis lock must not survive a policy change.
                *state.ongoing.borrow_mut() = gpui::OngoingScroll::default();
            }
            Some(self.focus.borrow_mut().enter_scroll(self.node, &state))
        } else {
            None
        };
        self.element
            .paint(id, inspector, bounds, layout, prepaint, window, cx);
        if let Some(bar) = &mut self.scrollbar_element {
            let style =
                self.element
                    .interactivity()
                    .compute_style(id, prepaint.as_ref(), window, cx);
            window.with_text_style(style.text_style().cloned(), |window| bar.paint(window, cx));
        }
        if let Some(depth) = boundary {
            self.focus.borrow_mut().leave_boundary(depth);
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reveal_uses_nearest_edge_or_start_for_oversized_targets() {
        for (start, end, expected) in [
            (20., 40., 0.),
            (10., 100., 0.),
            (0., 20., 10.),
            (90., 110., -10.),
            (110., 120., -20.),
            (-20., 0., 30.),
            (20., 120., -10.),
            (-20., 120., 30.),
        ] {
            assert_eq!(
                reveal_axis(px(start), px(end), px(10.), px(100.)),
                px(expected)
            );
        }
    }
}
