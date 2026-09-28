//! Bounded, read-only original-data companion to a chart. It does not depend on
//! successful mesh preparation and browsing never changes semantic selection.
use super::*;
use crate::{chart_store::Snapshot, chart_table as table};

#[derive(Clone, Copy)]
enum Action {
    Open,
    Close,
    Browse(usize),
}
#[derive(Clone)]
struct Route {
    state: Shared,
    token: Rc<()>,
    snapshot: std::sync::Weak<Snapshot>,
}
impl Route {
    fn invoke(&self, action: Action, pointer: bool, window: &mut Window, cx: &mut App) {
        let mut state = self.state.borrow_mut();
        let Some(snapshot) = self.snapshot.upgrade() else {
            return;
        };
        if !Rc::ptr_eq(&state.input.token, &self.token)
            || !state.base_input_allowed(window, pointer)
            || !state
                .lease
                .as_ref()
                .and_then(Lease::snapshot)
                .is_some_and(|live| Arc::ptr_eq(&live, &snapshot))
        {
            return;
        }
        state.cancel_input(window);
        state.input.data_cursor = match action {
            Action::Open => Some(0),
            Action::Close => None,
            Action::Browse(index) if state.input.data_cursor.is_some() => {
                Some(index.min(table::count(snapshot.data()).saturating_sub(1)))
            }
            Action::Browse(_) => return,
        };
        window.focus(&state.input.focus, cx);
        state.redraw(window, cx);
        window.prevent_default();
        cx.stop_propagation();
    }
    fn button(&self, label: &'static str, action: Action, disabled: bool) -> gpui::AnyElement {
        let colors = presentation::Controls::new(&self.state.borrow().config.style);
        let click = self.clone();
        let access = self.clone();
        let element = div()
            .id(label)
            .role(gpui::Role::Button)
            .aria_label(label)
            .px_2()
            .py_1()
            .rounded_md()
            .bg(gpui::rgba(colors.background))
            .border_1()
            .border_color(gpui::rgba(if disabled {
                colors.muted
            } else {
                colors.foreground
            }))
            .text_color(gpui::rgba(if disabled {
                colors.muted
            } else {
                colors.foreground
            }))
            .child(label)
            .on_click(move |_, window, cx| {
                if !disabled {
                    click.invoke(action, true, window, cx);
                }
            })
            .on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                if !disabled {
                    access.invoke(action, false, window, cx);
                }
            });
        crate::semantics::State {
            hidden: false,
            metadata: None,
            element,
            disabled,
            read_only: false,
            modal: false,
            live: None,
        }
        .into_any_element()
    }
}
impl State {
    /// Source-relative keyboard browsing is deliberately independent of the
    /// plotted-mark cursor/selection and is clamped when publications change.
    pub(super) fn data_key(&mut self, key: &str, window: &mut Window, cx: &mut App) -> bool {
        if !self.base_input_allowed(window, false) {
            return false;
        }
        let Some(snapshot) = self.lease.as_ref().and_then(Lease::snapshot) else {
            return false;
        };
        if key == "d" || (key == "escape" && self.input.data_cursor.is_some()) {
            let open = self.input.data_cursor.is_none();
            self.cancel_input(window);
            self.input.data_cursor = if open { Some(0) } else { None };
        } else if let Some(cursor) = self.input.data_cursor {
            let count = table::count(snapshot.data());
            let page = table::Page::new(
                cursor,
                count,
                f64::from(f32::from(self.input.bounds.size.height)),
            );
            if let Some(next) = page.navigate(key, count) {
                self.input.data_cursor = Some(next);
            } else {
                return false;
            }
        } else {
            return false;
        }
        self.redraw(window, cx);
        window.prevent_default();
        cx.stop_propagation();
        true
    }
}
pub(in crate::host::chart_view) fn element(state: Shared) -> Option<gpui::AnyElement> {
    let current = state.borrow();
    if current.closed {
        return None;
    }
    let snapshot = current.lease.as_ref().and_then(Lease::snapshot)?;
    let route = Route {
        state: state.clone(),
        token: current.input.token.clone(),
        snapshot: Arc::downgrade(&snapshot),
    };
    let disabled = current.config.disabled;
    let colors = presentation::Controls::new(&current.config.style);
    let Some(cursor) = current.input.data_cursor else {
        return Some(
            div()
                .absolute()
                .top_0()
                .left_0()
                .text_size(px(11.))
                .line_height(px(16.))
                .child(route.button("View data", Action::Open, disabled))
                .into_any_element(),
        );
    };
    let count = table::count(snapshot.data());
    let page = table::Page::new(
        cursor,
        count,
        f64::from(f32::from(current.input.bounds.size.height)),
    );
    let mut rows=div().id("original-data-table").role(gpui::Role::Table)
        .aria_label(format!("{} · original data",current.config.label))
        .aria_row_count(count).aria_column_count(3).min_h(px(24.))
        .aria_description("Read-only original values. Arrow keys browse rows; Home and End reach the first and last values. Page Up and Page Down change pages. D or Escape returns to the plot.")
        .flex().flex_col().w_full();
    for index in page.start..page.end {
        let row = table::row(snapshot.data(), index).expect("bounded source row");
        let active = index == page.cursor;
        let focus = route.clone();
        let click = route.clone();
        let mouse = route.clone();
        let mut element = div()
            .id(("chart-data-row", index as u64))
            .role(gpui::Role::Row)
            .aria_row_index(index)
            .aria_label(format!(
                "Row {}: {}. {}. {}",
                index + 1,
                row.name,
                row.value,
                row.detail
            ))
            .h(px(24.))
            .flex_shrink_0()
            .flex()
            .gap_2()
            .px_2()
            .items_center()
            .bg(gpui::rgba(if active {
                colors.accent
            } else {
                colors.background
            }))
            .text_color(gpui::rgba(if active {
                colors.active_foreground
            } else {
                colors.foreground
            }))
            .on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
                focus.invoke(Action::Browse(index), false, window, cx)
            })
            .on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                click.invoke(Action::Browse(index), false, window, cx)
            })
            .on_click(move |_, window, cx| mouse.invoke(Action::Browse(index), true, window, cx));
        if active {
            element = element.aria_active_descendant();
        }
        for (column, (label, value)) in [
            ("Item", row.name),
            ("Value", row.value),
            ("Details", row.detail),
        ]
        .into_iter()
        .enumerate()
        {
            element = element.child(
                div()
                    .id(column)
                    .role(gpui::Role::Cell)
                    .aria_row_index(index)
                    .aria_column_index(column)
                    .aria_label(label)
                    .aria_value(value.clone())
                    .flex_1()
                    .min_w_0()
                    .text_ellipsis()
                    .child(value),
            );
        }
        rows = rows.child(crate::semantics::State {
            hidden: false,
            metadata: None,
            element,
            disabled,
            read_only: true,
            modal: false,
            live: None,
        });
    }
    let status = if count == 0 {
        "No data values".into()
    } else {
        format!(
            "Rows {}–{} of {} · current row {}",
            page.start + 1,
            page.end,
            count,
            page.cursor + 1
        )
    };
    Some(
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .overflow_hidden()
            .flex()
            .flex_col()
            .gap_1()
            .p_1()
            .bg(gpui::rgba(colors.background))
            .text_color(gpui::rgba(colors.foreground))
            .text_size(px(11.))
            .line_height(px(16.))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .h(px(28.))
                    .flex_shrink_0()
                    .child(
                        div()
                            .id("chart-data-title")
                            .role(gpui::Role::Label)
                            .aria_label("Original data · all values")
                            .child("Original data · all values"),
                    )
                    .child(route.button("Back to chart", Action::Close, disabled)),
            )
            .child(
                div()
                    .flex()
                    .px_2()
                    .gap_2()
                    .h(px(20.))
                    .flex_shrink_0()
                    .children(
                        ["Item", "Value", "Details"]
                            .map(|label| div().flex_1().min_w_0().child(label)),
                    ),
            )
            .child(rows)
            .child(
                div()
                    .id("chart-data-position")
                    .role(gpui::Role::Label)
                    .aria_label(status.clone())
                    .h(px(16.))
                    .flex_shrink_0()
                    .text_ellipsis()
                    .child(status),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .h(px(28.))
                    .flex_shrink_0()
                    .child(route.button(
                        "Previous data page",
                        Action::Browse(page.cursor.saturating_sub(page.size)),
                        disabled || page.start == 0,
                    ))
                    .child(route.button(
                        "Next data page",
                        Action::Browse((page.start + page.size).min(count.saturating_sub(1))),
                        disabled || page.end >= count,
                    )),
            )
            .into_any_element(),
    )
}

#[cfg(feature = "native-canvas-tests")]
pub(in crate::host::chart_view) fn capture_browse(
    state: Shared,
    index: usize,
) -> impl FnOnce(&mut Window, &mut App) {
    let route = {
        let current = state.borrow();
        Route {
            state: state.clone(),
            token: current.input.token.clone(),
            snapshot: Arc::downgrade(&current.lease.as_ref().and_then(Lease::snapshot).unwrap()),
        }
    };
    move |window, cx| route.invoke(Action::Browse(index), false, window, cx)
}
