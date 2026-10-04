//! A retained header slot may be mounted but outside its horizontal pane.
//! Measure before descendant semantics, then retire input before descendant paint.
use super::*;
use gpui::{AnyElement, Element, ElementId, GlobalElementId, InspectorElementId};
use std::cell::Cell;

pub(super) fn content(
    slot: NodeId,
    element: AnyElement,
    gate: &super::super::focus::Shared,
) -> AnyElement {
    let root = div()
        .id(("table-header-content", slot.slot()))
        .size_full()
        .min_w_0()
        .relative()
        .overflow_hidden()
        .child(clip(slot, element, gate));
    super::super::highlight_style::Frame::clip(root, slot, gate).into_any_element()
}

/// Used for each interactive descendant as well as the complete header root.
/// A partly visible composite must not keep a fully clipped child actionable.
pub(in crate::host) fn clip(
    node: NodeId,
    element: AnyElement,
    gate: &super::super::focus::Shared,
) -> AnyElement {
    let clipped = Rc::new(Cell::new(true));
    let mask = Mask {
        slot: node,
        element,
        clipped: clipped.clone(),
    }
    .into_any_element();
    Frame {
        slot: node,
        element: mask,
        clipped,
        gate: gate.clone(),
    }
    .into_any_element()
}

struct Mask {
    slot: NodeId,
    element: AnyElement,
    clipped: Rc<Cell<bool>>,
}
impl IntoElement for Mask {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Mask {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        Some(
            (
                "table-header-mask",
                ((self.slot.generation() as u64) << 32) | self.slot.slot() as u64,
            )
                .into(),
        )
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, ()) {
        (self.element.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.paint(window, cx);
    }
    fn a11y_role(&self) -> Option<gpui::accesskit::Role> {
        Some(gpui::accesskit::Role::GenericContainer)
    }
    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        if self.clipped.get() {
            node.set_hidden();
        }
    }
}

/// Measure the content root before its identified semantic mask is prepainted.
struct Frame {
    slot: NodeId,
    element: AnyElement,
    clipped: Rc<Cell<bool>>,
    gate: super::super::focus::Shared,
}
impl IntoElement for Frame {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Frame {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (gpui::LayoutId, ()) {
        (self.element.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: gpui::Bounds<gpui::Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let visible = bounds
            .intersect(&window.content_mask().bounds)
            .intersect(&window.fully_visible_bounds());
        self.clipped
            .set(visible.size.width <= px(0.) || visible.size.height <= px(0.));
        self.element.prepaint(window, cx);
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui::Bounds<gpui::Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.gate
            .borrow_mut()
            .track_card_clipped(self.slot, self.clipped.get());
        self.element.paint(window, cx);
    }
}
