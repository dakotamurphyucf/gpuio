//! Additional semantic state for elements whose GPUI convenience API does not
//! expose disabled/read-only flags. Preserve the wrapped element's identity.
use gpui::{
    A11ySubtreeBuilder, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Window, accesskit,
};
use gpuio_protocol::accessibility::{Config, Current, Live, Role};

/// Retain exact layout and paint while shielding hitboxes registered by the
/// subtree. Focus/IME and active popup/timer policy is handled by focus::Manager.
/// This wrapper also covers specialized renderers that bypass finish_element.
pub(super) struct Inert<E>(pub E);
impl<E: Element> IntoElement for Inert<E> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl<E: Element> Element for Inert<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;
    fn id(&self) -> Option<ElementId> {
        self.0.id()
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.0.source_location()
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.0.request_layout(id, inspector, window, cx)
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
        let state = self.0.prepaint(id, inspector, bounds, layout, window, cx);
        window.insert_hitbox(bounds, gpui::HitboxBehavior::BlockMouse);
        state
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
        self.0
            .paint(id, inspector, bounds, layout, prepaint, window, cx);
    }
    fn a11y_role(&self) -> Option<accesskit::Role> {
        Some(accesskit::Role::Group)
    }
    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        node.set_hidden();
    }
}

fn role(role: Role) -> accesskit::Role {
    match role {
        Role::Navigation => accesskit::Role::Navigation,
        Role::Group => accesskit::Role::Group,
        Role::Label => accesskit::Role::Label,
        Role::Link => accesskit::Role::Link,
        Role::Separator => accesskit::Role::Splitter,
        Role::DescriptionList => accesskit::Role::DescriptionList,
        Role::Term => accesskit::Role::Term,
        Role::Definition => accesskit::Role::Definition,
        Role::Status => accesskit::Role::Status,
        Role::Alert => accesskit::Role::Alert,
        Role::Image => accesskit::Role::Image,
        Role::Heading(_) => accesskit::Role::Heading,
    }
}
fn metadata(config: &Config, node: &mut accesskit::Node) {
    if let Some(label) = &config.label {
        node.set_label(label.clone());
    }
    if let Some(description) = &config.description {
        node.set_description(description.clone());
    }
    if let Some(current) = config.current {
        node.set_aria_current(match current {
            Current::True => accesskit::AriaCurrent::True,
            Current::Page => accesskit::AriaCurrent::Page,
            Current::Step => accesskit::AriaCurrent::Step,
            Current::Location => accesskit::AriaCurrent::Location,
            Current::Date => accesskit::AriaCurrent::Date,
            Current::Time => accesskit::AriaCurrent::Time,
        });
    }
    node.set_live(match config.live {
        Live::Off => accesskit::Live::Off,
        Live::Polite => accesskit::Live::Polite,
        Live::Assertive => accesskit::Live::Assertive,
    });
    if let Some(Role::Heading(level)) = config.role {
        node.set_level(level as usize);
    }
    if let Some(field) = &config.field {
        node.set_label(field.label.clone());
        if field.required {
            node.set_required();
        }
        if field.error.is_some() {
            node.set_invalid(accesskit::Invalid::True);
        }
        let description = [field.help.as_deref(), field.error.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("\n");
        if !description.is_empty() {
            node.set_description(description);
        }
    }
}

fn field_relationships(config: &Config, builder: &mut A11ySubtreeBuilder) {
    let Some(field) = &config.field else {
        return;
    };
    let label = builder.synthetic_node_id("gpuio-field-label");
    let mut label_node = accesskit::Node::new(accesskit::Role::Label);
    label_node.set_label(field.label.clone());
    if builder.push_child(label, label_node) {
        builder.parent_node().set_labelled_by(vec![label]);
    }
    for (key, text, error) in [
        ("gpuio-field-help", field.help.as_deref(), false),
        ("gpuio-field-error", field.error.as_deref(), true),
    ] {
        if let Some(text) = text {
            let id = builder.synthetic_node_id(key);
            let mut node = accesskit::Node::new(accesskit::Role::Label);
            node.set_label(text.to_owned());
            if builder.push_child(id, node) {
                builder.parent_node().push_described_by(id);
                if error {
                    builder.parent_node().set_error_message(id);
                }
            }
        }
    }
}
pub struct State<E> {
    pub hidden: bool,
    pub metadata: Option<std::sync::Arc<gpuio_protocol::accessibility::Config>>,
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
        self.metadata
            .as_ref()
            .and_then(|c| c.role.map(role))
            .or_else(|| self.element.a11y_role())
            .or_else(|| (self.hidden || self.metadata.is_some()).then_some(accesskit::Role::Group))
    }
    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.element.write_a11y_info(node);
        if self.hidden {
            node.set_hidden();
        }
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
        if let Some(config) = &self.metadata {
            metadata(config, node);
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
        if let Some(config) = &self.metadata {
            field_relationships(config, builder);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::prelude::*;
    #[test]
    fn hidden_structural_roots_have_a_semantic_node_to_hide_their_children() {
        let element = State {
            hidden: true,
            metadata: None,
            element: gpui::div().id("hidden-parent"),
            disabled: false,
            read_only: false,
            modal: false,
            live: None,
        };
        assert_eq!(element.a11y_role(), Some(accesskit::Role::Group));
        let mut node = accesskit::Node::new(element.a11y_role().unwrap());
        element.write_a11y_info(&mut node);
        assert!(node.is_hidden());
        assert!(!node.is_disabled());
    }
    #[test]
    fn field_state_keeps_control_actions_and_sets_required_invalid_and_help() {
        use gpuio_protocol::accessibility::Field;
        let mut config = Config {
            role: None,
            label: None,
            description: None,
            live: Live::Off,
            current: None,
            field: Some(Field {
                label: "Name".into(),
                help: Some("Public name".into()),
                error: Some("Required".into()),
                required: true,
            }),
        };
        let mut node = accesskit::Node::new(accesskit::Role::TextInput);
        node.add_action(accesskit::Action::Focus);
        node.add_action(accesskit::Action::SetValue);
        node.set_read_only();
        metadata(&config, &mut node);
        assert_eq!(node.role(), accesskit::Role::TextInput);
        assert_eq!(node.label(), Some("Name"));
        assert_eq!(node.description(), Some("Public name\nRequired"));
        assert!(node.is_required());
        assert_eq!(node.invalid(), Some(accesskit::Invalid::True));
        assert!(node.supports_action(accesskit::Action::Focus));
        assert!(node.supports_action(accesskit::Action::SetValue));
        assert!(node.is_read_only());
        let field = config.field.as_mut().unwrap();
        field.required = false;
        field.error = None;
        let mut next = accesskit::Node::new(accesskit::Role::TextInput);
        metadata(&config, &mut next);
        assert!(!next.is_required());
        assert_eq!(next.invalid(), None);
        assert_eq!(next.description(), Some("Public name"));
    }
    #[test]
    fn current_item_keeps_button_actions_and_is_not_selection() {
        let config = Config {
            role: None,
            label: None,
            description: Some("Current page".into()),
            live: Live::Off,
            field: None,
            current: Some(Current::Page),
        };
        let mut node = accesskit::Node::new(accesskit::Role::Button);
        node.add_action(accesskit::Action::Click);
        node.add_action(accesskit::Action::Focus);
        metadata(&config, &mut node);
        assert_eq!(node.aria_current(), Some(accesskit::AriaCurrent::Page));
        assert_eq!(node.description(), Some("Current page"));
        assert!(node.supports_action(accesskit::Action::Click));
        assert!(node.supports_action(accesskit::Action::Focus));
        assert_eq!(node.is_selected(), None);
        assert_eq!(node.toggled(), None);
        assert_eq!(role(Role::Navigation), accesskit::Role::Navigation);
    }
    #[test]
    fn notifications_preserve_polite_and_assertive_live_semantics() {
        for (role, live) in [
            (accesskit::Role::Status, accesskit::Live::Polite),
            (accesskit::Role::Alert, accesskit::Live::Assertive),
        ] {
            let element = State {
                hidden: false,
                metadata: None,
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
