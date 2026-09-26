//! Real retained host table rendering and asynchronous demand, not an isolated delegate.
use super::*;
#[path = "table_host_test/input.rs"]
mod input;
#[cfg(feature = "native-image-tests")]
#[path = "table_host_test/style.rs"]
mod style;
use gpuio_protocol::{
    list::{IdRun, Order, Row},
    v1::*,
};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn window_id() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn config() -> wire::Config {
    wire::Config {
        schema_revision: 1,
        query_generation: 0,
        schema: wire::Schema {
            columns: ["name", "value"]
                .into_iter()
                .enumerate()
                .map(|(i, id)| wire::Column {
                    id: id.into(),
                    label: id.into(),
                    width: if i == 0 { 160. } else { 480. },
                    min_width: 40.,
                    max_width: 900.,
                    pin: if i == 0 {
                        wire::Pin::Left
                    } else {
                        wire::Pin::Unpinned
                    },
                    alignment: wire::Alignment::Left,
                    resizable: true,
                    movable: true,
                    sortable: true,
                })
                .collect(),
            headers: vec![],
        },
        sort: None,
        row_height: 32.,
        overscan: 64.,
        max_active_rows: 16,
        max_active_cells: 32,
        selection_mode: wire::SelectionMode::RowsAndCells,
        column_selection: true,
        disabled: false,
        scrollbar: true,
        label: "Native table host".into(),
    }
}
fn order(revision: i64, count: i64) -> Order {
    Order {
        revision,
        runs: if count == 0 {
            vec![]
        } else {
            vec![IdRun { first: 1, count }]
        },
    }
}
fn initial() -> Vec<Op> {
    let mut ops = vec![
        Op::Create(
            node(0),
            Kind::VirtualList,
            String::new(),
            Some(HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetTable(node(0), config()),
        Op::SetListConfig(node(0), config().list_config()),
        Op::SetListOrder(node(0), order(1, 100_000)),
        Op::SetStyle(
            node(0),
            vec![
                Style::Width(Length::Px(520.)),
                Style::Height(Length::Px(300.)),
            ],
        ),
    ];
    for row in 0..12 {
        let base = 1 + row * 5;
        ops.push(Op::Create(node(base), Kind::Container, String::new(), None));
        for col in 0..2 {
            let cell = base + 1 + col * 2;
            ops.extend([
                Op::Create(node(cell), Kind::Container, String::new(), None),
                Op::SetTableCell(
                    node(cell),
                    wire::Cell {
                        column: if col == 0 { "name" } else { "value" }.into(),
                        copy_text: format!("日本語 row {}", row + 1),
                    },
                ),
                Op::Create(
                    node(cell + 1),
                    Kind::Text,
                    format!("日本語 row {} column {col}", row + 1),
                    None,
                ),
                Op::Splice(node(cell), 0, 0, vec![node(cell + 1)]),
            ]);
        }
        ops.push(Op::Splice(
            node(base),
            0,
            0,
            vec![node(base + 1), node(base + 3)],
        ));
    }
    ops.extend([
        Op::SetListRows(
            node(0),
            (0..12)
                .map(|row| Row {
                    id: row + 1,
                    node: node(1 + row * 5),
                })
                .collect(),
        ),
        Op::Splice(
            node(0),
            0,
            0,
            (0..12).map(|row| node(1 + row * 5)).collect(),
        ),
        Op::SetRoot(Some(node(0))),
    ]);
    ops
}
fn apply(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>, operations: Vec<Op>) {
    window
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let transaction = Transaction {
                window: view.id,
                base,
                revision: base + 1,
                operations,
            };
            let applied = view
                .session
                .borrow_mut()
                .apply(&transaction)
                .unwrap_or_else(|error| {
                    panic!("table test transaction rejected: {error:?}: {transaction:?}")
                });
            view.update_editors(&applied.dirty, window, cx);
            view.table_actions(&applied.tables, window, cx);
            cx.notify();
        })
        .unwrap();
}
async fn frame(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) {
    // Actual layout/paint without relying on an occluded window's display link.
    for _ in 0..3 {
        let arena = cx
            .update_window(window.into(), |_, window, cx| {
                window.refresh();
                window.draw(cx)
            })
            .unwrap();
        cx.update(|cx| arena.clear(cx));
    }
}
fn viewport(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) -> Viewport {
    window
        .update(cx, |view, _, _| {
            view.tables[&node(0)]
                .borrow()
                .observed
                .clone()
                .expect("painted table viewport")
        })
        .unwrap()
}
fn command(serial: i64, query_generation: i64, target: wire::Target) -> Op {
    Op::TableCommand(
        node(0),
        wire::Command {
            serial,
            query_generation,
            target,
        },
    )
}
async fn exercise(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) {
    apply(cx, window, initial());
    frame(cx, window).await;
    let first = viewport(cx, window);
    assert_eq!(first.visible_first, 0);
    assert!(first.visible_last > 1 && first.visible_last < 16);
    assert!(first.requested.contains(&1) && first.requested.len() <= 16);
    let weak = window
        .update(cx, |view, _, cx| {
            assert!(
                view.lists.is_empty(),
                "table must not allocate ordinary list state"
            );
            let state = view.tables[&node(0)].borrow();
            let native = state.native.read(cx);
            assert!(native.delegate().rendered.contains(&(1, "name".into())));
            assert!(native.delegate().rendered.len() <= 32);
            assert!(
                view.probes.borrow().contains_key(&node(3)),
                "retained cell View must paint through host"
            );
            assert!(
                view.focus
                    .borrow()
                    .can_restore(&state.native.focus_handle(cx)),
                "table focus registered after begin_frame"
            );
            state.native.downgrade()
        })
        .unwrap();
    input::exercise(cx, window).await;
    #[cfg(feature = "native-image-tests")]
    style::exercise(cx, window).await;
    for enabled in [false, true] {
        apply(
            cx,
            window,
            vec![Op::SetStyle(
                node(0),
                vec![
                    Style::Width(Length::Px(520.)),
                    Style::Height(Length::Px(300.)),
                    Style::Fields(vec![Field::PointerEvents(enabled)]),
                ],
            )],
        );
        window
            .update(cx, |view, _, cx| {
                let table = view.tables[&node(0)].borrow();
                assert_eq!(
                    table.native.read(cx).delegate().pointer_enabled(cx),
                    enabled
                );
            })
            .unwrap();
    }
    for disabled in [true, false] {
        let mut next = config();
        next.disabled = disabled;
        apply(cx, window, vec![Op::SetTable(node(0), next)]);
        window
            .update(cx, |view, _, cx| {
                let table = view.tables[&node(0)].borrow();
                assert_eq!(
                    table.native.read(cx).delegate().input_enabled(cx),
                    !disabled
                );
            })
            .unwrap();
    }
    apply(
        cx,
        window,
        vec![command(1, 0, wire::Target::ScrollTo(50_001, 7.))],
    );
    frame(cx, window).await;
    let middle = viewport(cx, window);
    assert_eq!(middle.visible_first, 50_000);
    assert_eq!(middle.anchor, Some((50_001, 7.)));
    assert!(middle.requested.contains(&50_001));
    // Materialize the demanded rows using the same bounded node pool.
    apply(
        cx,
        window,
        vec![Op::SetListRows(
            node(0),
            (0..12)
                .map(|row| Row {
                    id: 50_001 + row,
                    node: node(1 + row * 5),
                })
                .collect(),
        )],
    );
    frame(cx, window).await;
    window
        .update(cx, |view, _, cx| {
            assert!(
                view.tables[&node(0)]
                    .borrow()
                    .native
                    .read(cx)
                    .delegate()
                    .rendered
                    .contains(&(50_001, "name".into()))
            );
        })
        .unwrap();
    apply(
        cx,
        window,
        vec![
            command(
                2,
                0,
                wire::Target::SetSelection(wire::Selection::Cell(50_001, "value".into())),
            ),
            command(3, 0, wire::Target::Reveal(50_001, Some("value".into()))),
        ],
    );
    frame(cx, window).await;
    window
        .update(cx, |view, _, cx| {
            assert_eq!(
                selection(view.tables[&node(0)].borrow().native.read(cx).selection()),
                wire::Selection::Cell(50_001, "value".into())
            );
        })
        .unwrap();
    apply(
        cx,
        window,
        vec![command(4, 0, wire::Target::ScrollTo(50_001, 7.))],
    );
    frame(cx, window).await;
    // Capture native input immediately, before a later accepted query can rebind it.
    let old_route=window.update(cx,|view,window,cx|{
        let table=view.tables[&node(0)].borrow().native.clone();
        view.transport.mailbox.lock().unwrap().drain(128);
        table.update(cx,|state,cx|state.set_selected_cell(50_000,1,cx));
        let events=view.transport.mailbox.lock().unwrap().drain(128);
        assert!(events.iter().any(|event|matches!(event, Event::TableInput(_,_,_,_,input) if input.query_generation==0 && input.request==wire::Request::Select(wire::Selection::Cell(50_001,"value".into())))));
        table.focus_handle(cx).focus(window,cx);
        assert!(view.list_pins(window,cx).iter().any(|pin|pin.node==node(0) && pin.rows.contains(&50_001)));
        table.read(cx).delegate().route.clone()
    }).unwrap();
    apply(
        cx,
        window,
        vec![command(5, 0, wire::Target::ScrollTo(50_001, 7.))],
    );
    frame(cx, window).await;
    let mut resized = config();
    resized.row_height = 48.;
    apply(
        cx,
        window,
        vec![
            Op::SetTable(node(0), resized.clone()),
            Op::SetListConfig(node(0), resized.list_config()),
        ],
    );
    frame(cx, window).await;
    assert_eq!(viewport(cx, window).anchor, Some((50_001, 7.)));
    // A source reset retires old routing even if the native entity is preserved.
    let mut reset = resized;
    reset.query_generation = 1;
    apply(
        cx,
        window,
        vec![
            Op::SetTable(node(0), reset),
            Op::Bind(node(0), Some(HandlerId::from_parts(0, 2).unwrap())),
            Op::SetListOrder(node(0), order(4, 1)),
            Op::SetListRows(node(0), vec![]),
            Op::Splice(node(0), 0, 12, vec![]),
        ]
        .into_iter()
        .chain((1..=60).rev().map(|slot| Op::Remove(node(slot))))
        .collect(),
    );
    frame(cx, window).await;
    window
        .update(cx, |view, _, _| {
            view.transport.mailbox.lock().unwrap().drain(128);
            old_route.emit(wire::Request::Select(wire::Selection::Row(1)));
            assert!(view.transport.mailbox.lock().unwrap().drain(128).is_empty());
        })
        .unwrap();
    let single = viewport(cx, window);
    assert_eq!((single.visible_first, single.visible_last), (0, 1));
    assert_eq!(single.requested, vec![1]);
    apply(cx, window, vec![Op::SetListOrder(node(0), order(5, 0))]);
    frame(cx, window).await;
    let empty = viewport(cx, window);
    assert!(empty.requested.is_empty() && empty.anchor.is_none());
    assert_eq!((empty.visible_first, empty.visible_last), (0, 0));
    apply(cx, window, vec![Op::SetRoot(None), Op::Remove(node(0))]);
    frame(cx, window).await;
    window
        .update(cx, |view, _, cx| {
            assert!(view.tables.is_empty());
            assert!(weak.upgrade().is_none(), "removed table entity retained");
            assert_eq!(
                view.session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .retained_bytes(),
                0
            );
            let _ = cx;
        })
        .unwrap();
    eprintln!(
        "GPUIO_NATIVE_TABLE_HOST_OK: retained Views, 100k sparse rows, actual viewport, keyed commands, row-height anchor, single/empty sources and release"
    );
}
pub(super) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _read = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let write = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(write.as_raw_fd()).unwrap());
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let clipboard = cx.read_from_clipboard();
        gpui_base::init(cx);
        gpuio_table_adapter::init(cx);
        let session = Rc::new(RefCell::new(crate::session::Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, window_id(), "GPUIO Table", 540., 340.)
            .unwrap();
        let window = cx
            .open_window(
                gpui::WindowOptions {
                    window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds::centered(
                        None,
                        gpui::size(px(540.), px(340.)),
                        cx,
                    ))),
                    focus: false,
                    show: true,
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(window_id(), session.clone(), transport.clone())),
            )
            .unwrap();
        cx.spawn(async move |cx| {
            let result = super::super::native_test::protect(exercise(cx, window)).await;
            let _ = window.update(cx, |_, window, _| window.remove_window());
            *task_failure.borrow_mut() = result.err();
            cx.update(|cx| {
                cx.write_to_clipboard(
                    clipboard.unwrap_or_else(|| gpui::ClipboardItem::new_string(String::new())),
                );
                super::super::stop_application(cx);
            });
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
