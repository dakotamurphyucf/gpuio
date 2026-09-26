//! Isolated candidate measurement, not GPUIO production acceptance.
use gpui::*;
use gpui_component::table::{Column, DataTable, TableDelegate, TableState};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

#[derive(Default)]
struct Metrics {
    touched: BTreeSet<(u64, usize)>,
    retained: BTreeMap<(u64, usize), SharedString>,
}
struct Delegate {
    keys: Vec<u64>,
    columns: Vec<Column>,
    metrics: Rc<RefCell<Metrics>>,
}
impl TableDelegate for Delegate {
    fn rows_count(&self, _: &App) -> usize {
        self.keys.len()
    }
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }
    fn column(&self, col: usize, _: &App) -> Column {
        self.columns[col].clone()
    }
    fn render_td(
        &mut self,
        row: usize,
        col: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let key = (self.keys[row], col);
        let mut metrics = self.metrics.borrow_mut();
        metrics.touched.insert(key);
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
            .get(&(self.keys[row], col))
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

fn main() {
    let failure = Rc::new(RefCell::new(None));
    let outcome = failure.clone();
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(QuitMode::Explicit);
        gpui_component::init(cx);
        let metrics = Rc::new(RefCell::new(Metrics::default()));
        let handle = cx.open_window(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::new(point(px(80.), px(80.)), size(px(720.), px(360.))))),
            focus: false,
            ..Default::default()
        }, |window, cx| {
            let table = cx.new(|cx| TableState::new(Delegate {
                keys: (1..=100_000).collect(),
                columns: (0..64).map(|i| {
                    let col = Column::new(format!("col-{i}"), format!("Column {i}")).width(px(160.));
                    if i < 2 { col.fixed_left() } else { col }
                }).collect(),
                metrics: metrics.clone(),
            }, window, cx).cell_selectable(true).row_header(false));
            cx.new(|_| Probe { table })
        }).unwrap();
        cx.spawn(async move |cx| {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
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
                    metrics.retained = touched.into_iter().map(|key| (key, format!("{}:{} 日本語👨‍👩‍👧‍👦", key.0, key.1).into())).collect();
                }
                eprintln!("TABLE_CANDIDATE_OK logical_rows=100000 columns=64 sampled_positions=4 peak_sample_cells={peak}");
            }));
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            *outcome.borrow_mut() = result.err();
            cx.update(|cx| cx.quit());
        }).detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
