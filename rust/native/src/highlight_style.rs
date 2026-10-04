//! Measure simple controls and observe visibility without treating clipping as hidden text.
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
    binding: Option<Binding>,
}
struct Binding {
    node: NodeId,
    part: Option<(usize, NodeId)>,
    styles: Arc<[Style]>,
    focus: focus::Shared,
    visibility: bool,
    clip: bool,
    control_bounds: bool,
}
impl<E> Frame<E> {
    pub(super) fn new(element: E, node: &crate::tree::Node, focus: &focus::Shared) -> Self {
        let dynamic = node.style.iter().any(|style| {
            matches!(style,
            Style::State(_, fields) if fields.iter().any(|field|
                matches!(field, Field::Display(_) | Field::Visibility(_))))
        });
        let clip = node.style.iter().any(|style| match style {
            Style::Fields(fields) | Style::State(_, fields) => fields
                .iter()
                .any(|field| matches!(field, Field::OverflowX(_) | Field::OverflowY(_))),
            _ => false,
        });
        let control_bounds = matches!(
            node.kind,
            gpuio_protocol::v1::Kind::Button
                | gpuio_protocol::v1::Kind::CommandButton
                | gpuio_protocol::v1::Kind::Link
                | gpuio_protocol::v1::Kind::Checkbox
                | gpuio_protocol::v1::Kind::Switch
                | gpuio_protocol::v1::Kind::Radio
                | gpuio_protocol::v1::Kind::RadioGroup
                | gpuio_protocol::v1::Kind::TabBar
                | gpuio_protocol::v1::Kind::Select
                | gpuio_protocol::v1::Kind::ChoicePicker
        );
        Self {
            element,
            binding: (dynamic || clip || control_bounds).then(|| Binding {
                node: node.id,
                part: None,
                styles: node.style.clone(),
                focus: focus.clone(),
                visibility: dynamic,
                clip,
                control_bounds,
            }),
        }
    }
    pub(super) fn clip(element: E, node: NodeId, focus: &focus::Shared) -> Self {
        Self {
            element,
            binding: Some(Binding {
                node,
                part: None,
                styles: Arc::from([]),
                focus: focus.clone(),
                visibility: false,
                clip: true,
                control_bounds: false,
            }),
        }
    }
    pub(super) fn part(
        element: E,
        node: &crate::tree::Node,
        part: usize,
        focus: &focus::Shared,
    ) -> Self {
        let mut frame = Self::new(element, node, focus);
        if let Some(binding) = &mut frame.binding {
            binding.part = Some((part, node.children[part]));
        }
        frame
    }
}
impl<E: Element<PrepaintState = Option<Hitbox>> + InteractiveElement> Frame<E> {
    fn observe(
        &mut self,
        id: Option<&GlobalElementId>,
        hitbox: Option<&Hitbox>,
        bounds: Option<Bounds<Pixels>>,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<usize> {
        let Some(binding) = &self.binding else {
            return None;
        };
        if !binding.visibility && !binding.clip {
            return None;
        }
        let style = self
            .element
            .interactivity()
            .compute_style(id, hitbox, window, cx);
        let hidden =
            style.display == gpui::Display::None || style.visibility == gpui::Visibility::Hidden;
        let mut focus = binding.focus.borrow_mut();
        if binding.visibility {
            match binding.part {
                None => focus.highlight_style(binding.node, &binding.styles, hidden),
                Some(part) => {
                    focus.highlight_part_style(binding.node, Some(part), &binding.styles, hidden)
                }
            }
        }
        if binding.clip
            && let Some(bounds) = bounds
        {
            // The native overflow mask is rectangular, including the other axis.
            // Only an actual scroll axis can reveal content beyond that mask.
            let x = style.overflow.x != gpui::Overflow::Scroll;
            let y = style.overflow.y != gpui::Overflow::Scroll;
            if (x || y)
                && let Some(mask) = style.overflow_mask(bounds, window.rem_size())
            {
                return Some(focus.enter_clip(focus::Clip {
                    bounds: mask.bounds,
                    x,
                    y,
                }));
            }
        }
        None
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
        let _ = self.observe(id, None, None, window, cx);
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
        let boundary = self.observe(id, prepaint.as_ref(), Some(bounds), window, cx);
        let measured = self
            .binding
            .as_ref()
            .filter(|binding| binding.control_bounds);
        let previous = measured.map(|binding| {
            binding
                .focus
                .borrow_mut()
                .replace_control_bounds(Some((binding.node, bounds)))
        });
        self.element
            .paint(id, inspector, bounds, layout, prepaint, window, cx);
        if let Some(previous) = previous {
            measured
                .unwrap()
                .focus
                .borrow_mut()
                .replace_control_bounds(previous);
        }
        if let Some(depth) = boundary {
            self.binding
                .as_ref()
                .unwrap()
                .focus
                .borrow_mut()
                .leave_boundary(depth);
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
