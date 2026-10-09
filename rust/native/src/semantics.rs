//! Additional semantic state for elements whose GPUI convenience API does not
//! expose disabled/read-only flags. Preserve the wrapped element's identity.
use gpui::{
    A11ySubtreeBuilder, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Window, accesskit,
};
use gpuio_protocol::accessibility::{Config, Current, Live, Role};

// Paired with the role/action-guarded adaptation in accesskit-macos/tree-actions.patch.
pub(super) const TREE_SELECT: i32 = 0x4750_0001;
pub(super) const TREE_DESELECT: i32 = 0x4750_0002;
pub(super) const LIST_SELECT: i32 = 0x4753_0001;
pub(super) const LIST_DESELECT: i32 = 0x4753_0002;
pub(super) const LIST_CONFIRM: i32 = 0x4753_0003;
pub(super) const LIST_CONFIRM_SECONDARY: i32 = 0x4753_0004;
pub(super) const LIST_CONTEXT: i32 = 0x4753_0005;

/// Retain exact layout and paint while shielding hitboxes registered by the
/// subtree. Focus/IME and active popup/timer policy is handled by focus::Manager.
/// This wrapper also covers specialized renderers that bypass finish_element.
pub(super) struct InteractionShield<E> {
    element: E,
    expose_disabled: bool,
    identity: Option<ElementId>,
}
impl<E> InteractionShield<E> {
    /// An inert subtree needs an identified hidden ancestor even when its body
    /// is type-erased. Disabled bodies instead retain their own semantic nodes
    /// and receive disabled state through the prepaint scope.
    pub(super) fn inert(element: E, identity: ElementId) -> Self {
        Self {
            element,
            expose_disabled: false,
            identity: Some(identity),
        }
    }
    pub(super) fn disabled(element: E) -> Self {
        Self {
            element,
            expose_disabled: true,
            identity: None,
        }
    }
}
impl<E: Element> IntoElement for InteractionShield<E> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl<E: Element> Element for InteractionShield<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = E::PrepaintState;
    fn id(&self) -> Option<ElementId> {
        self.identity.clone().or_else(|| self.element.id())
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
        let state = if self.expose_disabled {
            window.with_a11y_disabled(|window| {
                self.element
                    .prepaint(id, inspector, bounds, layout, window, cx)
            })
        } else {
            self.element
                .prepaint(id, inspector, bounds, layout, window, cx)
        };
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
        self.element
            .paint(id, inspector, bounds, layout, prepaint, window, cx);
    }
    fn a11y_role(&self) -> Option<accesskit::Role> {
        if self.expose_disabled {
            self.element.a11y_role().or(Some(accesskit::Role::Group))
        } else {
            Some(accesskit::Role::Group)
        }
    }
    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        if self.expose_disabled {
            self.element.write_a11y_info(node);
            node.set_disabled();
            node.clear_actions();
        } else {
            node.set_hidden();
        }
    }
    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut A11ySubtreeBuilder,
    ) {
        if self.expose_disabled {
            self.element.a11y_synthetic_children(prepaint, builder);
        }
    }
}

fn role(role: Role) -> accesskit::Role {
    match role {
        Role::Tree(_) => accesskit::Role::Tree,
        Role::TreeItem(_) => accesskit::Role::TreeItem,
        Role::ListBox(_) => accesskit::Role::ListBox,
        Role::OptionItem(_) => accesskit::Role::ListBoxOption,
        Role::Table(_) => accesskit::Role::Table,
        Role::RowGroup => accesskit::Role::RowGroup,
        Role::TableRow(_) => accesskit::Role::Row,
        Role::TableCell(_) => accesskit::Role::Cell,
        Role::ColumnHeader(_) => accesskit::Role::ColumnHeader,
        Role::RowHeader(_) => accesskit::Role::RowHeader,
        Role::Caption => accesskit::Role::Caption,
        Role::Toolbar(_) => accesskit::Role::Toolbar,
        Role::RadioGroup(_) => accesskit::Role::RadioGroup,
        Role::Navigation => accesskit::Role::Navigation,
        Role::Group => accesskit::Role::Group,
        Role::Label => accesskit::Role::Label,
        Role::Link => accesskit::Role::Link,
        Role::Separator => accesskit::Role::Splitter,
        Role::DescriptionList => accesskit::Role::DescriptionList,
        Role::Term => accesskit::Role::Term,
        Role::Definition => accesskit::Role::Definition,
        Role::Status => accesskit::Role::Status,
        Role::Log => accesskit::Role::Log,
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
    match config.role {
        Some(Role::Table(info)) => {
            if let Some(rows) = info.rows {
                node.set_row_count(rows as usize);
            }
            if let Some(columns) = info.columns {
                node.set_column_count(columns as usize);
            }
        }
        Some(Role::TableRow(index)) => node.set_row_index(index as usize),
        Some(Role::TableCell(cell) | Role::ColumnHeader(cell) | Role::RowHeader(cell)) => {
            node.set_row_index(cell.row as usize);
            node.set_column_index(cell.column as usize);
            node.set_column_span(cell.column_span as usize);
        }
        Some(Role::Toolbar(orientation) | Role::RadioGroup(orientation)) => {
            node.set_orientation(match orientation {
                gpuio_protocol::accessibility::Orientation::Horizontal => {
                    accesskit::Orientation::Horizontal
                }
                gpuio_protocol::accessibility::Orientation::Vertical => {
                    accesskit::Orientation::Vertical
                }
            })
        }
        Some(Role::Tree(true) | Role::ListBox(true)) => node.set_multiselectable(),
        Some(Role::OptionItem(item)) => {
            node.set_position_in_set(item.index as usize + 1);
            if let Some(count) = item.count {
                node.set_size_of_set(count as usize);
            }
            node.set_selected(item.selected);
            if item.disabled {
                node.set_disabled();
                node.clear_actions();
            } else if node.supports_action(accesskit::Action::CustomAction) {
                node.set_custom_actions(
                    [
                        (LIST_SELECT, "Select item"),
                        (LIST_DESELECT, "Deselect item"),
                        (LIST_CONFIRM, "Activate item"),
                        (LIST_CONFIRM_SECONDARY, "Activate item in secondary mode"),
                        (LIST_CONTEXT, "Show item actions"),
                    ]
                    .map(|(id, description)| accesskit::CustomAction {
                        id,
                        description: description.into(),
                    }),
                );
            }
        }
        Some(Role::TreeItem(item)) => {
            node.set_level(item.level as usize);
            node.set_position_in_set(item.index as usize + 1);
            if let Some(count) = item.count {
                node.set_size_of_set(count as usize);
            }
            if let Some(expanded) = item.expanded {
                node.set_expanded(expanded);
            }
            node.set_selected(item.selected);
            if !item.disabled && node.supports_action(accesskit::Action::CustomAction) {
                node.set_custom_actions([
                    accesskit::CustomAction {
                        id: TREE_SELECT,
                        description: "Select item".into(),
                    },
                    accesskit::CustomAction {
                        id: TREE_DESELECT,
                        description: "Deselect item".into(),
                    },
                ]);
            }
            if item.disabled {
                node.set_disabled();
            }
            if item.busy {
                node.set_busy();
            }
        }
        _ => (),
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
    pub identity: Option<ElementId>,
    pub busy: bool,
    pub hidden: bool,
    pub metadata: Option<std::sync::Arc<gpuio_protocol::accessibility::Config>>,
    pub element: E,
    pub disabled: bool,
    pub read_only: bool,
    pub modal: bool,
    pub live: Option<accesskit::Live>,
}
impl<E: Element> State<E> {
    /// A type-erased child has no outer identity. Supply one so GPUI can emit
    /// the hidden ancestor before prepainting that child's real semantic nodes.
    pub(super) fn decorative(element: E, identity: ElementId) -> Self {
        Self {
            identity: Some(identity),
            element,
            hidden: true,
            metadata: None,
            busy: false,
            disabled: false,
            read_only: false,
            modal: false,
            live: None,
        }
    }
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
        self.identity.clone().or_else(|| self.element.id())
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
        if self.busy {
            node.set_busy();
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
    fn list_option_metadata_keeps_cursor_focus_independent_from_selection() {
        use gpuio_protocol::accessibility::OptionItem;
        let root = Config {
            role: Some(Role::ListBox(true)),
            label: Some("Choices".into()),
            description: None,
            live: Live::Off,
            field: None,
            current: None,
        };
        let mut node = accesskit::Node::new(role(root.role.unwrap()));
        metadata(&root, &mut node);
        assert_eq!(node.role(), accesskit::Role::ListBox);
        assert!(node.is_multiselectable());
        assert!(!node.supports_action(accesskit::Action::Focus));
        for (selected, disabled, count) in [
            (false, false, None),
            (true, false, Some(100_000)),
            (true, true, None),
        ] {
            let config = Config {
                role: Some(Role::OptionItem(OptionItem {
                    index: 99_999,
                    count,
                    selected,
                    disabled,
                })),
                ..root.clone()
            };
            let mut node = accesskit::Node::new(role(config.role.unwrap()));
            node.add_action(accesskit::Action::Focus);
            node.add_action(accesskit::Action::Click);
            metadata(&config, &mut node);
            assert_eq!(node.role(), accesskit::Role::ListBoxOption);
            assert_eq!(node.position_in_set(), Some(100_000));
            assert_eq!(node.size_of_set(), count.map(|count| count as usize));
            assert_eq!(node.is_selected(), Some(selected));
            assert_eq!(node.is_disabled(), disabled);
            assert_eq!(node.supports_action(accesskit::Action::Focus), !disabled);
            assert_eq!(node.supports_action(accesskit::Action::Click), !disabled);
            assert_eq!(node.level(), None);
            assert_eq!(node.is_expanded(), None);
            assert!(node.custom_actions().is_empty());
        }
    }

    #[test]
    fn log_wrapper_preserves_child_policy_and_explicit_live_priority() {
        for (live, expected) in [
            (Live::Off, accesskit::Live::Off),
            (Live::Polite, accesskit::Live::Polite),
            (Live::Assertive, accesskit::Live::Assertive),
        ] {
            let config = Config {
                role: Some(Role::Log),
                label: Some("Transcript".into()),
                description: None,
                live,
                field: None,
                current: None,
            };
            let element = State {
                identity: Some("transcript".into()),
                metadata: Some(std::sync::Arc::new(config)),
                element: gpui::div(),
                busy: false,
                hidden: false,
                disabled: false,
                read_only: false,
                modal: false,
                live: None,
            };
            let mut node = accesskit::Node::new(element.a11y_role().unwrap());
            element.write_a11y_info(&mut node);
            assert_eq!(node.role(), accesskit::Role::Log);
            assert_eq!(node.label(), Some("Transcript"));
            assert_eq!(node.live(), Some(expected));
            assert!(!node.supports_action(accesskit::Action::Focus));
            assert!(!node.supports_action(accesskit::Action::Click));
            let hidden = State {
                hidden: true,
                ..element
            };
            let mut node = accesskit::Node::new(hidden.a11y_role().unwrap());
            hidden.write_a11y_info(&mut node);
            assert!(node.is_hidden());
        }
    }
    #[test]
    fn toolbar_metadata_keeps_orientation_without_inventing_actions() {
        use gpuio_protocol::accessibility::Orientation;
        for (orientation, expected) in [
            (Orientation::Horizontal, accesskit::Orientation::Horizontal),
            (Orientation::Vertical, accesskit::Orientation::Vertical),
        ] {
            let config = Config {
                role: Some(Role::Toolbar(orientation)),
                label: Some("Formatting".into()),
                description: None,
                live: Live::Off,
                field: None,
                current: None,
            };
            let mut node = accesskit::Node::new(role(config.role.unwrap()));
            metadata(&config, &mut node);
            assert_eq!(node.role(), accesskit::Role::Toolbar);
            assert_eq!(node.orientation(), Some(expected));
            assert_eq!(node.label(), Some("Formatting"));
            assert_eq!(node.live(), Some(accesskit::Live::Off));
            assert!(!node.supports_action(accesskit::Action::Focus));
            assert!(!node.supports_action(accesskit::Action::Click));
            assert_eq!(node.toggled(), None);
        }
    }

    #[test]
    fn radio_group_metadata_keeps_orientation_without_inventing_actions() {
        use gpuio_protocol::accessibility::Orientation;
        for (orientation, expected) in [
            (Orientation::Horizontal, accesskit::Orientation::Horizontal),
            (Orientation::Vertical, accesskit::Orientation::Vertical),
        ] {
            let config = Config {
                role: Some(Role::RadioGroup(orientation)),
                label: Some("Modes".into()),
                description: None,
                live: Live::Off,
                field: None,
                current: None,
            };
            let mut node = accesskit::Node::new(role(config.role.unwrap()));
            metadata(&config, &mut node);
            assert_eq!(node.role(), accesskit::Role::RadioGroup);
            assert_eq!(node.orientation(), Some(expected));
            assert_eq!(node.label(), Some("Modes"));
            assert_eq!(node.live(), Some(accesskit::Live::Off));
            assert!(!node.supports_action(accesskit::Action::Focus));
            assert!(!node.supports_action(accesskit::Action::Click));
            assert_eq!(node.toggled(), None);
        }
    }

    #[test]
    fn tree_metadata_preserves_unknown_totals_leaf_state_and_native_focus() {
        use gpuio_protocol::accessibility::TreeItem;
        let root = Config {
            role: Some(Role::Tree(true)),
            label: Some("Project".into()),
            description: None,
            live: Live::Off,
            field: None,
            current: None,
        };
        assert_eq!(role(root.role.unwrap()), accesskit::Role::Tree);
        let mut node = accesskit::Node::new(accesskit::Role::Tree);
        metadata(&root, &mut node);
        assert!(node.is_multiselectable());
        let branch = Config {
            role: Some(Role::TreeItem(TreeItem {
                level: 2,
                index: 4,
                count: None,
                expanded: Some(false),
                selected: true,
                disabled: true,
                busy: true,
            })),
            label: Some("Branch".into()),
            ..root.clone()
        };
        let mut node = accesskit::Node::new(role(branch.role.unwrap()));
        node.add_action(accesskit::Action::Focus);
        metadata(&branch, &mut node);
        assert_eq!(node.role(), accesskit::Role::TreeItem);
        assert_eq!(node.level(), Some(2));
        assert_eq!(node.position_in_set(), Some(5));
        assert_eq!(node.size_of_set(), None);
        assert_eq!(node.is_expanded(), Some(false));
        assert_eq!(node.is_selected(), Some(true));
        assert!(node.is_disabled() && node.is_busy());
        assert!(node.supports_action(accesskit::Action::Focus));
        let leaf = Config {
            role: Some(Role::TreeItem(TreeItem {
                level: 3,
                index: 1,
                count: Some(2),
                expanded: None,
                selected: false,
                disabled: false,
                busy: false,
            })),
            ..branch.clone()
        };
        let mut next = accesskit::Node::new(role(leaf.role.unwrap()));
        metadata(&leaf, &mut next);
        assert_eq!(next.size_of_set(), Some(2));
        assert_eq!(next.is_expanded(), None);
        assert_eq!(next.is_selected(), Some(false));
        assert!(next.custom_actions().is_empty());
        next.add_action(accesskit::Action::CustomAction);
        metadata(&leaf, &mut next);
        assert_eq!(
            next.custom_actions()
                .iter()
                .map(|a| a.id)
                .collect::<Vec<_>>(),
            vec![TREE_SELECT, TREE_DESELECT]
        );
        let mut disabled = accesskit::Node::new(role(branch.role.unwrap()));
        disabled.add_action(accesskit::Action::CustomAction);
        metadata(&branch, &mut disabled);
        assert!(disabled.custom_actions().is_empty());
        assert!(!next.is_disabled() && !next.is_busy());
    }
    #[test]
    fn hidden_structural_roots_have_a_semantic_node_to_hide_their_children() {
        let element = State {
            identity: None,
            busy: false,
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
    fn decorative_type_erased_content_gets_a_real_hidden_ancestor_identity() {
        let child = gpui::div()
            .id("spinner")
            .role(accesskit::Role::ProgressIndicator)
            .aria_label("Decorative activity")
            .into_any_element();
        assert_eq!(
            Element::id(&child),
            None,
            "State<AnyElement> cannot inherit a semantic identity from its child"
        );
        let element = State::decorative(child, ("control-label", 42_u64).into());
        assert_eq!(element.id(), Some(("control-label", 42_u64).into()));
        assert_eq!(element.a11y_role(), Some(accesskit::Role::Group));
        let mut node = accesskit::Node::new(element.a11y_role().unwrap());
        element.write_a11y_info(&mut node);
        assert!(node.is_hidden());
        assert!(!node.is_disabled());
        assert!(!node.supports_action(accesskit::Action::Focus));
        assert!(!node.supports_action(accesskit::Action::Click));
    }
    #[test]
    fn inert_type_erased_content_is_hidden_but_disabled_content_remains_exposed() {
        fn body() -> gpui::AnyElement {
            gpui::div()
                .id("table")
                .role(accesskit::Role::Table)
                .into_any_element()
        }
        let inert = InteractionShield::inert(body(), ("inactive-page", 9_u64).into());
        assert_eq!(inert.id(), Some(("inactive-page", 9_u64).into()));
        let mut hidden = accesskit::Node::new(inert.a11y_role().unwrap());
        inert.write_a11y_info(&mut hidden);
        assert!(hidden.is_hidden());
        assert!(!hidden.supports_action(accesskit::Action::Focus));
        assert!(!hidden.supports_action(accesskit::Action::Click));

        let disabled = InteractionShield::disabled(body());
        let mut exposed = accesskit::Node::new(disabled.a11y_role().unwrap());
        disabled.write_a11y_info(&mut exposed);
        assert!(exposed.is_disabled());
        assert!(!exposed.is_hidden());
        assert!(!exposed.supports_action(accesskit::Action::Focus));
        assert!(!exposed.supports_action(accesskit::Action::Click));
    }
    #[test]
    fn button_busy_state_preserves_name_role_toggle_and_focus_without_disabling() {
        for busy in [false, true, false] {
            let element = State {
                identity: None,
                busy,
                hidden: false,
                metadata: None,
                live: None,
                disabled: false,
                read_only: false,
                modal: false,
                element: gpui::div()
                    .id("publish")
                    .role(accesskit::Role::Button)
                    .aria_label("Publish draft")
                    .aria_toggled(accesskit::Toggled::True),
            };
            assert_eq!(element.id(), Some("publish".into()));
            assert_eq!(element.a11y_role(), Some(accesskit::Role::Button));
            let mut node = accesskit::Node::new(element.a11y_role().unwrap());
            node.add_action(accesskit::Action::Focus);
            element.write_a11y_info(&mut node);
            assert_eq!(node.is_busy(), busy);
            assert_eq!(node.label(), Some("Publish draft"));
            assert_eq!(node.toggled(), Some(accesskit::Toggled::True));
            assert!(node.supports_action(accesskit::Action::Focus));
            assert!(!node.is_disabled() && !node.is_hidden() && !node.is_read_only());
        }
    }
    #[test]
    fn notifications_preserve_polite_and_assertive_live_semantics() {
        for (role, live) in [
            (accesskit::Role::Status, accesskit::Live::Polite),
            (accesskit::Role::Alert, accesskit::Live::Assertive),
        ] {
            let element = State {
                identity: None,
                busy: false,
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
