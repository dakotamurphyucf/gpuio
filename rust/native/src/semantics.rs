//! Additional semantic state for elements whose GPUI convenience API does not
//! expose disabled/read-only flags. Preserve the wrapped element's identity.
use gpui::{
    A11ySubtreeBuilder, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Window, accesskit,
};
pub struct State<E> {
    pub element: E,
    pub disabled: bool,
    pub read_only: bool,
    pub modal: bool,
    pub live: Option<accesskit::Live>,
}
impl<E: Element> IntoElement for State<E> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl<E: Element> Element for State<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;
    fn id(&self) -> Option<ElementId> {
        self.element.id()
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
        self.element
            .paint(id, inspector, bounds, layout, prepaint, window, cx);
    }
    fn a11y_role(&self) -> Option<accesskit::Role> {
        self.element.a11y_role()
    }
    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.element.write_a11y_info(node);
        if let Some(live) = self.live {
            node.set_live(live);
        }
        if self.modal {
            node.set_modal();
        }
        if self.disabled {
            node.set_disabled();
        }
        if self.read_only {
            node.set_read_only();
        }
        // accesskit_macos 0.26.3 maps any non-False Toggled to NSNumber(true),
        // losing Mixed. Cocoa checkboxes require NSNumber(2) for mixed state.
        // Normalize only at the platform boundary; other backends retain the
        // native AccessKit tri-state. Remove when the pinned adapter preserves it.
        #[cfg(target_os = "macos")]
        if node.role() == accesskit::Role::CheckBox
            && node.toggled() == Some(accesskit::Toggled::Mixed)
        {
            node.clear_toggled();
            node.set_numeric_value(2.);
        }
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
    use gpui::prelude::*;
    #[test]
    fn notifications_preserve_polite_and_assertive_live_semantics() {
        for (role, live) in [
            (accesskit::Role::Status, accesskit::Live::Polite),
            (accesskit::Role::Alert, accesskit::Live::Assertive),
        ] {
            let element = State {
                element: gpui::div()
                    .id("notification")
                    .role(role)
                    .aria_label("Draft saved"),
                disabled: false,
                read_only: false,
                modal: false,
                live: Some(live),
            };
            assert_eq!(element.a11y_role(), Some(role));
            let mut node = accesskit::Node::new(role);
            element.write_a11y_info(&mut node);
            assert_eq!(node.live(), Some(live));
            assert_eq!(node.label(), Some("Draft saved"));
            assert!(!node.is_modal());
        }
    }
}
