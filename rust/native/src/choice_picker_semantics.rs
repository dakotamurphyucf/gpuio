//! Native picker metadata absent from GPUI's Div convenience methods.
//! Preserve element identity/layout and action dispatch; no synthetic option tree.
use crate::choice_picker_rows::{Projection, Row};
use gpui::{
    A11ySubtreeBuilder, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Window, accesskit,
};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc, sync::Arc};

/// Per-render capture, populated only by accessibility prepaint. A list retry
/// may leave records for discarded rows; only current direct children are used.
/// Indices borrow group metadata from the shared immutable projection: no group
/// ID or label is copied per option.
#[derive(Clone)]
pub(super) struct Groups {
    projection: Arc<Projection>,
    rows: Rc<RefCell<BTreeMap<accesskit::NodeId, usize>>>,
}

impl Groups {
    pub(super) fn new(projection: Arc<Projection>) -> Self {
        Self {
            projection,
            rows: Rc::default(),
        }
    }

    fn apply(&self, builder: &mut A11ySubtreeBuilder) {
        let rows = self.rows.borrow();
        let mut groups: BTreeMap<&str, (&str, Vec<accesskit::NodeId>)> = BTreeMap::new();
        for child in builder.parent_node().children() {
            let group = rows
                .get(child)
                .and_then(|&index| match self.projection.row(index)? {
                    Row::Header(group) => Some(group),
                    Row::Item(_) => self.projection.membership(index)?.group,
                });
            if let Some(group) = group {
                groups
                    .entry(&group.id)
                    .or_insert_with(|| (&group.label, Vec::new()))
                    .1
                    .push(*child);
            }
        }
        for (id, (label, members)) in groups {
            let mut node = accesskit::Node::new(accesskit::Role::Group);
            node.set_label(label);
            builder.group_children(("picker-group", id), node, &members);
        }
    }
}

/// Full opaque IDs stay intact; group and item namespaces cannot alias.
pub(super) fn row_id(key: crate::choice_picker_rows::Key<'_>) -> ElementId {
    use crate::choice_picker_rows::Key;
    let (role, id) = match key {
        Key::Group(id) => (0, id),
        Key::Item(id) => (1, id),
    };
    ElementId::NamedInteger(gpui::SharedString::from(id.to_owned()), role)
}

pub(super) fn membership(
    element: gpui::Stateful<gpui::Div>,
    membership: crate::choice_picker_rows::Membership<'_>,
) -> gpui::Stateful<gpui::Div> {
    use gpui::StatefulInteractiveElement;
    element
        .aria_position_in_set(membership.index + 1)
        .aria_size_of_set(membership.count)
}

enum Kind {
    ListBox { multiple: bool },
    Option { selected: bool, disabled: bool },
    Clear { disabled: bool },
    Header,
}
pub(super) struct Semantics<E> {
    element: E,
    kind: Kind,
    groups: Option<Groups>,
    row: Option<usize>,
}
pub(super) fn listbox<E: Element>(element: E, multiple: bool) -> Semantics<E> {
    Semantics {
        element,
        kind: Kind::ListBox { multiple },
        groups: None,
        row: None,
    }
}
pub(super) fn option<E: Element>(element: E, selected: bool, disabled: bool) -> Semantics<E> {
    Semantics {
        element,
        kind: Kind::Option { selected, disabled },
        groups: None,
        row: None,
    }
}
pub(super) fn clear<E: Element>(element: E, disabled: bool) -> Semantics<E> {
    Semantics {
        element,
        kind: Kind::Clear { disabled },
        groups: None,
        row: None,
    }
}

pub(super) fn header<E: Element>(element: E) -> Semantics<E> {
    Semantics {
        element,
        kind: Kind::Header,
        groups: None,
        row: None,
    }
}

impl<E> Semantics<E> {
    pub(super) fn groups(mut self, groups: Groups) -> Self {
        self.groups = Some(groups);
        self
    }

    pub(super) fn row(mut self, index: usize) -> Self {
        self.row = Some(index);
        self
    }
}
impl<E: Element> IntoElement for Semantics<E> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl<E: Element> Element for Semantics<E> {
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
        if matches!(self.kind, Kind::ListBox { .. })
            && let Some(groups) = &self.groups
        {
            groups.rows.borrow_mut().clear();
        }
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
        Some(match self.kind {
            Kind::ListBox { .. } => accesskit::Role::ListBox,
            Kind::Option { .. } => accesskit::Role::ListBoxOption,
            Kind::Clear { .. } => accesskit::Role::Button,
            Kind::Header => accesskit::Role::GenericContainer,
        })
    }
    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.element.write_a11y_info(node);
        match self.kind {
            Kind::Header => node.set_hidden(),
            Kind::ListBox { multiple } => {
                if multiple {
                    node.set_multiselectable();
                }
            }
            Kind::Clear { disabled } => {
                if disabled {
                    node.set_disabled();
                    node.clear_actions();
                }
            }
            Kind::Option { selected, disabled } => {
                node.set_selected(selected);
                if disabled {
                    node.set_disabled();
                    node.clear_actions();
                }
            }
        }
    }
    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut A11ySubtreeBuilder,
    ) {
        self.element.a11y_synthetic_children(prepaint, builder);
        if let Some(groups) = &self.groups {
            if matches!(self.kind, Kind::ListBox { .. }) {
                groups.apply(builder);
            } else if let Some(index) = self.row {
                groups.rows.borrow_mut().insert(builder.parent_id(), index);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{StatefulInteractiveElement, div, prelude::*};

    #[test]
    fn keyed_option_metadata_keeps_logical_positions_without_repeating_group_labels() {
        use crate::choice_picker_rows::{Key, Membership};
        let id = "x".repeat(256);
        let group = gpuio_protocol::choice_picker::Group {
            id: id.clone(),
            label: "Research tools".into(),
            items: vec![],
        };
        let expected = row_id(Key::Item(&id));
        assert_eq!(
            expected,
            ElementId::NamedInteger(gpui::SharedString::from(id.clone()), 1)
        );
        assert_ne!(expected, row_id(Key::Group(&id)));
        for (index, count) in [(99, 4096), (0, 1)] {
            let element = option(
                membership(
                    div().id(row_id(Key::Item(&id))).aria_label("Search"),
                    Membership {
                        index,
                        count,
                        group: Some(&group),
                    },
                ),
                false,
                false,
            );
            assert_eq!(element.id(), Some(expected.clone()));
            let mut node = accesskit::Node::new(element.a11y_role().unwrap());
            element.write_a11y_info(&mut node);
            assert_eq!(node.label(), Some("Search"));
            assert_eq!(node.description(), None);
            assert_eq!(node.position_in_set(), Some(index + 1));
            assert_eq!(node.size_of_set(), Some(count));
        }
        let flat = option(
            membership(
                div().id("flat").aria_label("Flat"),
                Membership {
                    index: 1,
                    count: 3,
                    group: None,
                },
            ),
            true,
            false,
        );
        let mut node = accesskit::Node::new(flat.a11y_role().unwrap());
        flat.write_a11y_info(&mut node);
        assert_eq!(node.description(), None);
        assert_eq!(node.position_in_set(), Some(2));
        assert_eq!(node.size_of_set(), Some(3));
    }
    #[test]
    fn native_options_preserve_identity_labels_and_disabled_selection_without_actions() {
        for selected in [false, true] {
            for disabled in [false, true] {
                let element = option(
                    div()
                        .id("option")
                        .aria_label("Research")
                        .on_a11y_action(gpui::AccessibleAction::Click, |_, _, _| {}),
                    selected,
                    disabled,
                );
                assert_eq!(element.id(), Some("option".into()));
                let mut node = accesskit::Node::new(element.a11y_role().unwrap());
                element.write_a11y_info(&mut node);
                assert_eq!(node.role(), accesskit::Role::ListBoxOption);
                assert_eq!(node.label(), Some("Research"));
                assert_eq!(node.is_selected(), Some(selected));
                assert_eq!(node.is_disabled(), disabled);
                assert_eq!(node.supports_action(accesskit::Action::Click), !disabled);
                assert!(!node.supports_action(accesskit::Action::Focus));
            }
        }
    }
    #[test]
    fn clear_button_has_an_independent_label_and_no_disabled_actions() {
        for disabled in [false, true] {
            let element = clear(
                div()
                    .id("clear")
                    .aria_label("Clear selection")
                    .on_a11y_action(gpui::AccessibleAction::Click, |_, _, _| {})
                    .on_a11y_action(gpui::AccessibleAction::Focus, |_, _, _| {}),
                disabled,
            );
            assert_eq!(element.id(), Some("clear".into()));
            let mut node = accesskit::Node::new(element.a11y_role().unwrap());
            element.write_a11y_info(&mut node);
            assert_eq!(node.role(), accesskit::Role::Button);
            assert_eq!(node.label(), Some("Clear selection"));
            assert_eq!(node.is_disabled(), disabled);
            assert_eq!(node.supports_action(accesskit::Action::Click), !disabled);
            assert_eq!(node.supports_action(accesskit::Action::Focus), !disabled);
            assert_eq!(node.is_selected(), None);
        }
    }
    #[test]
    fn native_listbox_exposes_multiple_selection_without_inventing_a_selection_or_action() {
        for multiple in [false, true] {
            let element = listbox(div().id("list").aria_label("Capabilities"), multiple);
            assert_eq!(element.id(), Some("list".into()));
            let mut node = accesskit::Node::new(element.a11y_role().unwrap());
            element.write_a11y_info(&mut node);
            assert_eq!(node.role(), accesskit::Role::ListBox);
            assert_eq!(node.label(), Some("Capabilities"));
            assert_eq!(node.is_multiselectable(), multiple);
            assert_eq!(node.is_selected(), None);
            assert!(!node.supports_action(accesskit::Action::Click));
        }
    }
}
