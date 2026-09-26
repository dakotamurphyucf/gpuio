//! Production adapter native measurements; not full OCH-39 acceptance.
use gpui::*;
use gpuio_table_adapter::table::{
    Column, DataTable, RowKey, Selection, TableDelegate, TableEvent, TableState,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

#[derive(Default)]
struct Metrics {
    touched: BTreeSet<(u64, usize)>,
    retained: BTreeMap<(u64, SharedString), SharedString>,
}
struct Delegate {
    keys: Vec<u64>,
    positions: BTreeMap<u64, usize>,
    columns: Vec<Column>,
    metrics: Rc<RefCell<Metrics>>,
}
impl TableDelegate for Delegate {
    fn rows_count(&self, _: &App) -> usize {
        self.keys.len()
    }
    fn row_key(&self, index: usize, _: &App) -> Option<RowKey> {
        self.keys.get(index).copied().map(RowKey)
    }
    fn row_index(&self, key: RowKey, _: &App) -> Option<usize> {
        self.positions.get(&key.0).copied()
    }
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }
    fn column(&self, col: usize, _: &App) -> Column {
        self.columns[col].clone()
    }
    fn move_column(
        &mut self,
        from: usize,
        to: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> bool {
        if from >= self.columns.len() || to >= self.columns.len() || from < 2 || to < 2 {
            return false;
        }
        let column = self.columns.remove(from);
        self.columns.insert(to, column);
        true
    }
    fn render_td(
        &mut self,
        row: usize,
        col: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let key = (self.keys[row], self.columns[col].key.clone());
        let mut metrics = self.metrics.borrow_mut();
        metrics.touched.insert((self.keys[row], col));
        // No host-language call, formatting, I/O or producer execution here.
        metrics
            .retained
            .get(&key)
            .cloned()
            .unwrap_or_else(|| SharedString::new("…"))
    }
    fn cell_text(&self, row: usize, col: usize, _: &App) -> String {
        self.metrics
            .borrow()
            .retained
            .get(&(self.keys[row], self.columns[col].key.clone()))
            .map(|text| text.to_string())
            .unwrap_or_default()
    }
}
struct Probe {
    table: Entity<TableState<Delegate>>,
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(DataTable::new(&self.table))
    }
}

fn exercise_keyed_selection(
    cx: &mut AsyncApp,
    handle: WindowHandle<Probe>,
    metrics: &Rc<RefCell<Metrics>>,
) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let _subscription = handle
        .update(cx, |view, _, cx| {
            cx.subscribe(&view.table, move |_, _, event, _| {
                let selection = match event {
                    TableEvent::SelectRow(row) => Some(Selection::Row(*row)),
                    TableEvent::SelectColumn(column) => Some(Selection::Column(column.clone())),
                    TableEvent::SelectCell(row, column) => Some(Selection::Cell {
                        row: *row,
                        column: column.clone(),
                    }),
                    TableEvent::ClearSelection => Some(Selection::Empty),
                    _ => None,
                };
                if let Some(selection) = selection {
                    observed.borrow_mut().push(selection);
                }
            })
        })
        .unwrap();
    metrics
        .borrow_mut()
        .retained
        .insert((50_001, "col-32".into()), "日本語👨‍👩‍👧‍👦".into());
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                table.set_selected_cell(50_000, 32, cx);
                assert_eq!(
                    table.selection(),
                    &Selection::Cell {
                        row: RowKey(50_001),
                        column: "col-32".into()
                    }
                );
            })
        })
        .unwrap();
    let count = events.borrow().len();
    assert_eq!(count, 1, "one user selection event");
    assert_eq!(
        events.borrow()[0],
        Selection::Cell {
            row: RowKey(50_001),
            column: "col-32".into()
        }
    );
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                let delegate = table.delegate_mut();
                delegate.keys.reverse();
                delegate.positions = delegate
                    .keys
                    .iter()
                    .enumerate()
                    .map(|(index, key)| (*key, index))
                    .collect();
                delegate.columns[2..].reverse();
                table.refresh(cx);
                assert_eq!(table.selected_cell(), Some((49_999, 33)));
                assert_eq!(table.delegate().cell_text(49_999, 33, cx), "日本語👨‍👩‍👧‍👦");
                assert!(table.replace_selection(Selection::Row(RowKey(50_001)), cx));
                assert_eq!(table.selected_row(), Some(49_999));
                assert_eq!(table.selected_cell(), None);
                assert_eq!(table.selected_col(), None);
                assert!(!table.replace_selection(Selection::Row(RowKey(0)), cx));
                assert_eq!(table.selected_row(), Some(49_999));
                let delegate = table.delegate_mut();
                delegate.keys.remove(49_999);
                delegate.positions = delegate
                    .keys
                    .iter()
                    .enumerate()
                    .map(|(index, key)| (*key, index))
                    .collect();
                table.refresh(cx);
                assert_eq!(table.selection(), &Selection::Empty);
                assert_eq!(table.selected_row(), None);
                assert!(table.replace_selection(Selection::Column("col-32".into()), cx));
                table
                    .delegate_mut()
                    .columns
                    .retain(|column| column.key != "col-32");
                table.refresh(cx);
                assert_eq!(table.selection(), &Selection::Empty);
                table.set_selected_row(usize::MAX, cx);
                table.set_selected_col(usize::MAX, cx);
                table.set_selected_cell(usize::MAX, usize::MAX, cx);
                assert_eq!(table.selection(), &Selection::Empty);
            })
        })
        .unwrap();
    assert_eq!(
        events.borrow().len(),
        count,
        "source reconciliation must not echo user selection"
    );
    eprintln!(
        "TABLE_KEYED_SELECTION_OK: 100k reorder, column reorder, Unicode retained text, removal, invalid commands, no reconciliation echo"
    );
}

#[path = "support/anchors.rs"]
mod anchors;
#[path = "support/events.rs"]
mod events;

fn main() {
    let verify_failure_exit = std::env::args().any(|arg| arg == "--verify-failure-exit");
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(QuitMode::Explicit);
        gpui_base::init(cx);
        gpuio_table_adapter::init(cx);
        let metrics = Rc::new(RefCell::new(Metrics::default()));
        let handle = cx.open_window(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(point(px(80.), px(80.)), size(px(720.), px(360.))))),
            focus: false,
            ..Default::default()
        }, |window, cx| {
            let table = cx.new(|cx| TableState::new(Delegate {
                keys: (1..=100_000).collect(),
                positions: (1..=100_000).enumerate().map(|(index, key)| (key, index)).collect(),
                columns: (0..64).map(|i| {
                    let col = Column::new(format!("col-{i}"), format!("Column {i}")).width(px(160.));
                    if i < 2 { col.fixed_left() } else { col }
                }).collect(),
                metrics: metrics.clone(),
            }, window, cx).cell_selectable(true).row_header(false));
            cx.new(|_| Probe { table })
        }).unwrap();
        cx.spawn(async move |cx| {
            let weak = handle.update(cx, |view, _, _| view.table.downgrade()).unwrap();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                assert!(!verify_failure_exit, "intentional native harness failure-exit check");
                let mut peak = 0;
                for (row, col) in [(0, 0), (50_000, 32), (99_999, 63), (0, 2)] {
                    metrics.borrow_mut().touched.clear();
                    handle.update(cx, |view, _, cx| view.table.update(cx, |table, cx| {
                        table.scroll_to_row(row, cx);
                        table.scroll_to_col(col, cx);
                    })).unwrap();
                    for _ in 0..3 {
                        let arena = cx.update_window(handle.into(), |_, window, cx| {
                            window.refresh();
                            window.draw(cx)
                        }).unwrap();
                        cx.update(|cx| arena.clear(cx));
                    }
                    let mut metrics = metrics.borrow_mut();
                    let touched = metrics.touched.clone();
                    assert!(!touched.is_empty(), "no candidate cells painted");
                    assert!(touched.len() <= 4096, "unbounded candidate render working set");
                    assert!(touched.contains(&(row as u64 + 1, col)), "target cell missing: {row}, {col}");
                    peak = peak.max(touched.len());
                    let rows: BTreeSet<_> = touched.iter().map(|(r, _)| *r).collect();
                    let cols: BTreeSet<_> = touched.iter().map(|(_, c)| *c).collect();
                    eprintln!("TABLE_CANDIDATE target=({row},{col}) unique_cells={} rows={} columns={cols:?}", touched.len(), rows.len());
                    // Stand-in asynchronous producer delivery outside all rendering callbacks.
                    metrics.retained = touched.into_iter().map(|key| ((key.0, format!("col-{}", key.1).into()), format!("{}:{} 日本語👨‍👩‍👧‍👦", key.0, key.1).into())).collect();
                }
                eprintln!("TABLE_CANDIDATE_OK logical_rows=100000 columns=64 sampled_positions=4 peak_sample_cells={peak}");
                exercise_keyed_selection(cx, handle, &metrics);
                events::exercise(cx, handle);
                anchors::exercise(cx, handle);
                handle.update(cx, |_, window, _| window.remove_window()).unwrap();
                cx.update(|_| ());
                assert!(weak.upgrade().is_none(), "closed window retained native table state");
                eprintln!("TABLE_ENTITY_RELEASE_OK");
            }));
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            if result.is_err() {
                cx.update(|_| ());
                if weak.upgrade().is_none() {
                    eprintln!("TABLE_FAILURE_CLEANUP_OK");
                }
                // GPUI's platform quit can terminate before Application::run
                // returns. Fail here after cleanup, never after the run loop.
                eprintln!("TABLE_NATIVE_FAILED");
                std::process::exit(1);
            }
            cx.update(|cx| cx.quit());
        }).detach();
    });
}
