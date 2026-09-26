//! Semantic nodes share the native table's revision fence and optimistic selection.
use super::*;
use gpui::{A11ySubtreeBuilder, Element, GlobalElementId, InspectorElementId, LayoutId, accesskit};

// Private opt-in contract with vendor/accesskit-macos/table-state.patch.
const SELECT: i32 = 0x4750_0011;
const DESELECT: i32 = 0x4750_0012;
const SORT: i32 = 0x4750_0013;

#[derive(Clone, Copy)]
enum Target {
    Row(usize),
    Cell(usize, usize),
    Column(usize),
}

pub(super) struct Semantic<E> {
    element: E,
    selectable: bool,
    sort: Option<ColumnSort>,
}
impl<E: Element> IntoElement for Semantic<E> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl<E: Element> Element for Semantic<E> {
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
        let mut actions = Vec::with_capacity(3);
        if self.selectable {
            actions.extend([
                accesskit::CustomAction {
                    id: SELECT,
                    description: "Select".into(),
                },
                accesskit::CustomAction {
                    id: DESELECT,
                    description: "Deselect".into(),
                },
            ]);
        }
        if let Some(sort) = self.sort {
            match sort {
                ColumnSort::Default => (),
                ColumnSort::Ascending => {
                    node.set_sort_direction(accesskit::SortDirection::Ascending)
                }
                ColumnSort::Descending => {
                    node.set_sort_direction(accesskit::SortDirection::Descending)
                }
            }
            actions.push(accesskit::CustomAction {
                id: SORT,
                description: "Change sort order".into(),
            });
        }
        node.set_custom_actions(actions);
    }
    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut A11ySubtreeBuilder,
    ) {
        self.element.a11y_synthetic_children(prepaint, builder);
    }
}

impl<D: TableDelegate> TableState<D> {
    fn accessible_target(&self, target: Target, cx: &App) -> Option<Selection> {
        match target {
            Target::Row(row) if self.row_selectable => {
                self.delegate.row_key(row, cx).map(Selection::Row)
            }
            Target::Cell(row, col) if self.cell_selectable => Some(Selection::Cell {
                row: self.delegate.row_key(row, cx)?,
                column: self.col_groups.get(col)?.column.key.clone(),
            }),
            Target::Column(col) if self.col_selectable => self
                .col_groups
                .get(col)
                .filter(|c| c.column.selectable)
                .map(|c| Selection::Column(c.column.key.clone())),
            _ => None,
        }
    }

    fn accessibility_listener(
        &self,
        target: Target,
        action: accesskit::Action,
        cx: &Context<Self>,
    ) -> impl FnMut(Option<&accesskit::ActionData>, &mut Window, &mut App) + 'static {
        let epoch = self.layout_epoch.clone();
        let entity = cx.weak_entity();
        move |data, window, cx| {
            let _ = entity.update(cx, |table, cx| {
                if !epoch.matches(&table.layout_epoch) || !table.delegate.input_enabled(cx) {
                    return;
                }
                if let Some(accesskit::ActionData::CustomAction(SORT)) = data {
                    if let Target::Column(col) = target {
                        table.perform_sort(col, window, cx);
                    }
                    return;
                }
                let Some(selection) = table.accessible_target(target, cx) else {
                    return;
                };
                let desired = match (action, data) {
                    (accesskit::Action::Click | accesskit::Action::Focus, _) => true,
                    (
                        accesskit::Action::CustomAction,
                        Some(accesskit::ActionData::CustomAction(SELECT)),
                    ) => true,
                    (
                        accesskit::Action::CustomAction,
                        Some(accesskit::ActionData::CustomAction(DESELECT)),
                    ) => false,
                    _ => return,
                };
                if !desired {
                    if table.selection == selection {
                        table.clear_selection(cx);
                    }
                    return;
                }
                // Press selects the addressed item; it never escalates a second
                // cell press to whole-row selection or activates application data.
                window.focus(&table.focus_handle, cx);
                match target {
                    Target::Row(row) => table.set_selected_row(row, cx),
                    Target::Cell(row, col) => table.set_selected_cell(row, col, cx),
                    Target::Column(col) => table.set_selected_col(col, cx),
                }
            });
        }
    }

    fn accessible_element<E: Element + gpui::StatefulInteractiveElement>(
        &self,
        mut element: E,
        target: Target,
        sort: Option<ColumnSort>,
        cx: &Context<Self>,
    ) -> Semantic<E> {
        let selection = self.accessible_target(target, cx);
        let selectable = selection.is_some();
        if let Some(selection) = selection {
            element = element
                .aria_selected(self.selection == selection)
                .when(self.selection == selection, |element| {
                    element.aria_active_descendant()
                })
                .on_a11y_action(
                    accesskit::Action::Click,
                    self.accessibility_listener(target, accesskit::Action::Click, cx),
                )
                .on_a11y_action(
                    accesskit::Action::Focus,
                    self.accessibility_listener(target, accesskit::Action::Focus, cx),
                );
        }
        if selectable || sort.is_some() {
            element = element.on_a11y_action(
                accesskit::Action::CustomAction,
                self.accessibility_listener(target, accesskit::Action::CustomAction, cx),
            );
        }
        Semantic {
            element,
            selectable,
            sort,
        }
    }

    pub(super) fn accessible_sort(
        &self,
        button: Stateful<Div>,
        col: usize,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let mut listener =
            self.accessibility_listener(Target::Column(col), accesskit::Action::CustomAction, cx);
        button
            .role(gpui::Role::Button)
            .aria_label(format!("Sort {}", self.col_groups[col].column.name))
            .on_a11y_action(accesskit::Action::Click, move |_, window, cx| {
                listener(Some(&accesskit::ActionData::CustomAction(SORT)), window, cx);
            })
    }

    pub(super) fn accessible_row<E: Element + gpui::StatefulInteractiveElement>(
        &self,
        row: E,
        index: usize,
        cx: &Context<Self>,
    ) -> Semantic<E> {
        self.accessible_element(
            row.role(gpui::Role::Row)
                .aria_row_index(index)
                .aria_label(format!("Row {}", index + 1)),
            Target::Row(index),
            None,
            cx,
        )
    }
    pub(super) fn accessible_cell(
        &self,
        cell: Stateful<Div>,
        row: usize,
        col: usize,
        cx: &Context<Self>,
    ) -> Semantic<Stateful<Div>> {
        self.accessible_element(
            cell.role(gpui::Role::Cell)
                .aria_row_index(row)
                .aria_column_index(col)
                .aria_label(self.col_groups[col].column.name.clone())
                .when_some(
                    self.delegate.cell_accessibility_value(row, col, cx),
                    |cell, value| cell.aria_value(value),
                ),
            Target::Cell(row, col),
            None,
            cx,
        )
    }
    pub(super) fn accessible_header<E: Element + gpui::StatefulInteractiveElement>(
        &self,
        header: E,
        col: usize,
        cx: &Context<Self>,
    ) -> Semantic<E> {
        let column = &self.col_groups[col].column;
        self.accessible_element(
            header
                .role(gpui::Role::ColumnHeader)
                .aria_column_index(col)
                .aria_label(column.name.clone()),
            Target::Column(col),
            self.sortable.then_some(column.sort).flatten(),
            cx,
        )
    }
}
