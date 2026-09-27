//! Full production table traversal; no viewport mocks or out-of-contract row sizes.
use super::*;
const COUNT: i64 = 100_000;
const ACTIVE: usize = 128;
const COLUMNS: usize = 4;
const CELLS: usize = ACTIVE * COLUMNS;
const STRIDE: usize = 1 + 2 * COLUMNS;
const ROW_HEIGHT: f64 = 20.;

// Keep this explicit user-requested workload runnable while its window is hidden.
// The activity is scoped to the test; no global App Nap or sleep settings change.
#[cfg(target_os = "macos")]
struct WorkloadActivity(
    objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2::runtime::NSObjectProtocol>>,
);
#[cfg(target_os = "macos")]
impl WorkloadActivity {
    fn new() -> Self {
        use objc2_foundation::{NSActivityOptions, NSProcessInfo, NSString};
        Self(
            NSProcessInfo::processInfo().beginActivityWithOptions_reason(
                NSActivityOptions::UserInitiatedAllowingIdleSystemSleep,
                &NSString::from_str("GPUIO hidden native table validation"),
            ),
        )
    }
}
#[cfg(target_os = "macos")]
impl Drop for WorkloadActivity {
    fn drop(&mut self) {
        // SAFETY: this token came from beginActivity on this process and ends once.
        unsafe { objc2_foundation::NSProcessInfo::processInfo().endActivity(&self.0) };
    }
}

fn dimensions(rows: usize) -> Vec<Style> {
    vec![
        Style::Width(Length::Px(520.)),
        Style::Height(Length::Px(ROW_HEIGHT * (rows + 1) as f64 + 2.)),
    ]
}
fn configuration() -> wire::Config {
    let mut config = config();
    config.row_height = ROW_HEIGHT;
    config.overscan = 0.;
    config.max_active_rows = ACTIVE as i64;
    config.max_active_cells = CELLS as i64;
    config.scrollbar = false;
    config.schema.columns = (0..COLUMNS)
        .map(|col| wire::Column {
            id: format!("column-{col}"),
            label: format!("Column {col}"),
            width: if col == 0 { 120. } else { 240. },
            min_width: 40.,
            max_width: 900.,
            pin: if col == 0 {
                wire::Pin::Left
            } else {
                wire::Pin::Unpinned
            },
            alignment: wire::Alignment::Left,
            resizable: true,
            movable: true,
            sortable: true,
        })
        .collect();
    config
}
fn id(slot: usize, generation: i64) -> NodeId {
    NodeId::from_parts(slot as i64, generation).unwrap()
}

fn peak_rss_bytes() -> u64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    assert_eq!(
        unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) },
        0
    );
    let usage = unsafe { usage.assume_init() };
    let units = if cfg!(target_os = "linux") { 1024 } else { 1 };
    usage.ru_maxrss as u64 * units
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: gpui::WindowHandle<View>,
) -> Vec<std::sync::Weak<str>> {
    #[cfg(target_os = "macos")]
    let _activity = WorkloadActivity::new();
    let fail = std::env::args().any(|arg| arg == "--verify-failure-exit");
    let probe = std::env::args().any(|arg| arg == "--probe-window");
    let cache_probe = std::env::args().any(|arg| arg == "--probe-cache");
    let config = configuration();
    apply(
        cx,
        window,
        vec![
            Op::Create(
                node(0),
                Kind::VirtualList,
                String::new(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetTable(node(0), config.clone()),
            Op::SetListConfig(node(0), config.list_config()),
            Op::SetListOrder(node(0), order(1, COUNT)),
            Op::SetStyle(node(0), dimensions(ACTIVE)),
            Op::SetRoot(Some(node(0))),
        ],
    );
    frame(cx, window).await;
    let (native, baseline_bytes) = window
        .update(cx, |view, window, _| {
            eprintln!(
                "TABLE_HISTORY_VIEWPORT size={:?} visible={:?}",
                window.viewport_size(),
                window.fully_visible_bounds()
            );
            assert!(
                window.viewport_size().height >= px(2582.),
                "platform constrained hidden tall window"
            );
            (
                view.tables[&node(0)].borrow().native.downgrade(),
                view.session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .retained_bytes(),
            )
        })
        .unwrap();
    let initial_rss = peak_rss_bytes();
    let mut previous: Vec<NodeId> = Vec::new();
    let mut text: Vec<std::sync::Weak<str>> = Vec::new();
    let mut metadata: Vec<std::sync::Weak<wire::Cell>> = Vec::new();
    let mut selections: Vec<std::rc::Weak<RefCell<crate::selection::State>>> = Vec::new();
    let mut generations = [0_i64; ACTIVE];
    let mut serial = 0;
    let mut visits = 0;
    let mut peak_cached = 0;
    let mut peak_bytes = 0;
    let start = std::time::Instant::now();
    let mut report = start;
    let mut apply_time = std::time::Duration::ZERO;
    let mut paint_time = std::time::Duration::ZERO;
    let mut check_time = std::time::Duration::ZERO;
    let batches = if cache_probe {
        vec![1, COUNT - 31]
    } else {
        (1..=COUNT).step_by(ACTIVE).collect()
    };
    'passes: for pass in 0..if cache_probe { 8 } else { 2 } {
        for &first in &batches {
            let count = ((COUNT - first + 1) as usize).min(ACTIVE);
            let mut operations: Vec<_> = previous.iter().copied().map(Op::Remove).collect();
            let mut current = Vec::with_capacity(count * STRIDE);
            let mut rows = Vec::with_capacity(count);
            let mut texts = Vec::with_capacity(count * COLUMNS);
            let mut cells = Vec::with_capacity(count * COLUMNS);
            for (offset, generation) in generations.iter_mut().enumerate().take(count) {
                *generation += 1;
                let generation = *generation;
                let row_key = first + offset as i64;
                let slot = 1 + offset * STRIDE;
                let row = id(slot, generation);
                rows.push(Row {
                    id: row_key,
                    node: row,
                });
                current.push(row);
                operations.push(Op::Create(row, Kind::Container, String::new(), None));
                let mut row_cells = Vec::with_capacity(COLUMNS);
                for col in 0..COLUMNS {
                    let cell = id(slot + 1 + 2 * col, generation);
                    let content = id(slot + 2 + 2 * col, generation);
                    let value = format!(
                        "日本語 👨‍👩‍👧‍👦 row {row_key} column {col} pass {pass}: {}",
                        "payload ".repeat(24)
                    );
                    row_cells.push(cell);
                    cells.push(cell);
                    texts.push(content);
                    current.extend([cell, content]);
                    operations.extend([
                        Op::Create(cell, Kind::Container, String::new(), None),
                        Op::SetTableCell(
                            cell,
                            wire::Cell {
                                column: format!("column-{col}"),
                                copy_text: value.clone(),
                            },
                        ),
                        Op::Create(content, Kind::Text, value, None),
                        Op::SetStyle(
                            content,
                            vec![Style::Fields(vec![
                                Field::UserSelect(true),
                                Field::Width(Length::Px(80.)),
                                Field::Height(Length::Px(18.)),
                                Field::OverflowY(1),
                            ])],
                        ),
                        Op::Splice(cell, 0, 0, vec![content]),
                    ]);
                }
                operations.push(Op::Splice(row, 0, 0, row_cells));
            }
            serial += 1;
            operations.extend([
                Op::Splice(
                    node(0),
                    0,
                    (previous.len() / STRIDE) as i64,
                    rows.iter().map(|row| row.node).collect(),
                ),
                Op::SetListRows(node(0), rows),
                Op::SetStyle(node(0), dimensions(count)),
                command(serial, 0, wire::Target::ScrollTo(first, 0.)),
            ]);
            window
                .update(cx, |view, _, cx| {
                    view.probes.borrow_mut().clear();
                    let native = view.tables[&node(0)].borrow().native.clone();
                    native.update(cx, |state, _| state.delegate_mut().rendered.clear());
                })
                .unwrap();
            assert!(operations.len() <= MAX_OPERATIONS);
            let phase = std::time::Instant::now();
            apply(cx, window, operations);
            apply_time += phase.elapsed();
            let phase = std::time::Instant::now();
            let mut horizontal = Vec::with_capacity(2);
            for column in [1, COLUMNS - 1] {
                serial += 1;
                apply(
                    cx,
                    window,
                    vec![command(
                        serial,
                        0,
                        wire::Target::ScrollToColumn(format!("column-{column}")),
                    )],
                );
                frame(cx, window).await;
                horizontal.push(
                    window
                        .update(cx, |view, _, cx| {
                            let native = view.tables[&node(0)].borrow();
                            (
                                native.native.read(cx).horizontal_scroll_handle.offset().x,
                                view.probes.borrow()[&texts[0]].bounds.origin.x,
                            )
                        })
                        .unwrap(),
                );
            }
            assert_eq!(horizontal[0].0, px(0.));
            assert!(
                horizontal[1].0 < px(-100.),
                "real horizontal movement: {horizontal:?}"
            );
            assert!(
                (horizontal[0].1 - horizontal[1].1).abs() < px(0.1),
                "pinned column moved horizontally"
            );
            paint_time += phase.elapsed();
            let phase = std::time::Instant::now();
            window
                .update(cx, |view, _, _| {
                    view.transport.mailbox.lock().unwrap().drain(128);
                })
                .unwrap();
            futures_lite::future::yield_now().await;
            text.retain(|weak| weak.strong_count() != 0);
            peak_cached = peak_cached.max(text.len());
            if text.len() > 2 * CELLS {
                for weak in text.iter().step_by((text.len() / 10).max(1)) {
                    eprintln!(
                        "TABLE_HISTORY_RETAINED strong={} text={:?}",
                        weak.strong_count(),
                        weak.upgrade()
                            .map(|value| value.chars().take(65).collect::<String>())
                    );
                }
            }
            assert!(
                text.len() <= 2 * CELLS,
                "text cache grows with history: {}",
                text.len()
            );
            assert!(
                metadata.iter().all(|weak| weak.strong_count() == 0),
                "retired cell metadata retained"
            );
            assert!(
                selections.iter().all(|weak| weak.upgrade().is_none()),
                "evicted text selections retained"
            );
            let (payloads, cell_metadata, current_selections) = window
                .update(cx, |view, _, cx| {
                    let state = view.tables[&node(0)].borrow();
                    let table = state.native.read(cx);
                    let delegate = table.delegate();
                    assert_eq!(delegate.index.len(), COUNT as usize);
                    assert_eq!(delegate.rows.len(), count);
                    assert_eq!(delegate.handles.len(), count);
                    assert_eq!(
                        delegate
                            .rows
                            .values()
                            .map(|cells| cells.len())
                            .sum::<usize>(),
                        count * COLUMNS
                    );
                    assert_eq!(
                        delegate.rendered.len(),
                        count * COLUMNS,
                        "every logical cell must render over the horizontal sweep"
                    );
                    let viewport = state.observed.as_ref().unwrap();
                    assert_eq!(viewport.visible_first, first - 1);
                    assert_eq!(viewport.visible_last, first - 1 + count as i64);
                    assert!(!viewport.budget_exhausted && viewport.pinned.is_empty());
                    assert!(viewport.requested.len() <= ACTIVE);
                    assert_eq!(view.selections.len(), count * COLUMNS);
                    let session = view.session.borrow();
                    let tree = session.tree(view.id).unwrap();
                    assert_eq!(tree.len(), 1 + count * STRIDE);
                    peak_bytes = peak_bytes.max(tree.retained_bytes());
                    assert!(
                        tree.retained_bytes().saturating_sub(baseline_bytes) < 1_048_576,
                        "mounted payload accounting grows beyond 1MiB: {}",
                        tree.retained_bytes().saturating_sub(baseline_bytes)
                    );
                    assert!(
                        view.lists.is_empty() && view.editors.is_empty() && view.images.is_empty()
                    );
                    (
                        texts
                            .iter()
                            .map(|id| Arc::downgrade(&tree.get(*id).unwrap().text))
                            .collect::<Vec<_>>(),
                        cells
                            .iter()
                            .map(|id| {
                                Arc::downgrade(tree.get(*id).unwrap().table_cell.as_ref().unwrap())
                            })
                            .collect::<Vec<_>>(),
                        view.selections
                            .values()
                            .map(Rc::downgrade)
                            .collect::<Vec<_>>(),
                    )
                })
                .unwrap();
            text.extend(payloads);
            metadata = cell_metadata;
            selections = current_selections;
            previous = current;
            visits += count;
            check_time += phase.elapsed();
            if first == 1 || report.elapsed().as_secs() >= 10 {
                eprintln!(
                    "TABLE_HISTORY pass={} visited={visits} active_rows={ACTIVE} active_cells={CELLS} peak_cached={peak_cached} peak_rss_bytes={} elapsed={:.1}s apply={:.1}s paint={:.1}s checks_and_yield={:.1}s",
                    pass + 1,
                    peak_rss_bytes(),
                    start.elapsed().as_secs_f64(),
                    apply_time.as_secs_f64(),
                    paint_time.as_secs_f64(),
                    check_time.as_secs_f64(),
                );
                report = std::time::Instant::now();
            }
            assert!(!fail, "TABLE_HISTORY_EXPECTED_FAILURE");
            if probe {
                break 'passes;
            }
        }
    }
    let mut operations = vec![Op::SetRoot(None), Op::Remove(node(0))];
    operations.extend(previous.into_iter().map(Op::Remove));
    apply(cx, window, operations);
    frame(cx, window).await;
    assert!(
        native.upgrade().is_none(),
        "unmounted native table entity retained"
    );
    assert!(metadata.iter().all(|weak| weak.strong_count() == 0));
    assert!(selections.iter().all(|weak| weak.upgrade().is_none()));
    text.retain(|weak| weak.strong_count() != 0);
    assert!(text.len() <= 2 * CELLS);
    window
        .update(cx, |view, _, _| {
            assert!(view.tables.is_empty() && view.selections.is_empty());
            assert_eq!(
                view.session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .retained_bytes(),
                0
            );
        })
        .unwrap();
    if cache_probe {
        assert_eq!(visits, 8 * (ACTIVE + 32));
        eprintln!("GPUIO_TABLE_HISTORY_CACHE_PROBE_OK: {visits} rows across shrinking batches");
    } else if probe {
        eprintln!(
            "GPUIO_TABLE_HISTORY_PROBE_OK: {visits} rows fully rendered in a hidden native window"
        );
    } else {
        assert_eq!(visits, 2 * COUNT as usize);
        eprintln!(
            "GPUIO_TABLE_HISTORY_OK: visits={visits} logical_rows={COUNT} columns={COLUMNS} active_rows={ACTIVE} active_cells={CELLS} peak_cached_payloads={peak_cached} peak_accounted_bytes={peak_bytes} baseline_accounted_bytes={baseline_bytes} after_unmount_cached={} initial_peak_rss_bytes={initial_rss} final_peak_rss_bytes={} elapsed={:.1}s",
            text.len(),
            peak_rss_bytes(),
            start.elapsed().as_secs_f64()
        );
    }
    text
}
