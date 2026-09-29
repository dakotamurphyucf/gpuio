//! Observe resolved native visibility without treating clipping as hidden text.
use super::focus;
use gpui::{
    A11ySubtreeBuilder, App, Bounds, Element, ElementId, GlobalElementId, Hitbox,
    InspectorElementId, InteractiveElement, IntoElement, LayoutId, Pixels, Window, accesskit,
};
use gpuio_protocol::{
    NodeId,
    v1::{Field, Style},
};
use std::sync::Arc;

pub(super) struct Frame<E> {
    element: E,
    binding: Option<(NodeId, Arc<[Style]>, focus::Shared)>,
}
impl<E> Frame<E> {
    pub(super) fn new(element: E, node: &crate::tree::Node, focus: &focus::Shared) -> Self {
        let dynamic = node.style.iter().any(|style| {
            matches!(style,
            Style::State(_, fields) if fields.iter().any(|field|
                matches!(field, Field::Display(_) | Field::Visibility(_))))
        });
        Self {
            element,
            binding: dynamic.then(|| (node.id, node.style.clone(), focus.clone())),
        }
    }
}
impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement> Frame<E> {
    fn observe(
        &mut self,
        id: Option<&GlobalElementId>,
        hitbox: Option<&Hitbox>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some((node, styles, focus)) = &self.binding else {
            return;
        };
        let style = self
            .element
            .interactivity()
            .compute_style(id, hitbox, window, cx);
        let hidden =
            style.display == gpui::Display::None || style.visibility == gpui::Visibility::Hidden;
        focus.borrow_mut().highlight_style(*node, styles, hidden);
    }
}
impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement> IntoElement for Frame<E> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement> Element for Frame<E> {
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
        let result = self.element.request_layout(id, inspector, window, cx);
        self.observe(id, None, window, cx);
        result
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
        self.observe(id, prepaint.as_ref(), window, cx);
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
