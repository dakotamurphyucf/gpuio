//! Actual production layout on TestPlatform, including empty data.
use super::behavior::{apply, draw};
use super::*;
use crate::session::Session;
use gpui::{TestAppContext, VisualTestContext};
use std::os::unix::net::UnixStream;

fn snapshot(
    native: &Entity<TableState<Delegate>>,
    cx: &VisualTestContext,
) -> Vec<(String, bool, bool)> {
    native.read_with(cx, |t, _| {
        t.column_viewport()
            .expect("completed layout")
            .columns
            .iter()
            .map(|c| (c.column.to_string(), c.pinned, c.fully_visible))
            .collect()
    })
}

fn published(view: &Entity<View>, cx: &VisualTestContext) -> Vec<wire::ColumnViewport> {
    view.read_with(cx, |v, _| v.transport.mailbox.lock().unwrap().drain(128))
        .into_iter()
        .filter_map(|event| match event {
            Event::TableColumnsObserved(_, _, _, _, viewport) => Some(viewport),
            _ => None,
        })
        .collect()
}

#[test]
fn empty_tables_measure_pins_partial_columns_scroll_and_schema_replacement() {
    let mut app = TestAppContext::single();
    app.update(|cx| {
        gpui_base::init(cx);
        gpuio_table_adapter::init(cx);
    });
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id(), "Table", 520., 300.)
        .unwrap();
    let (view, cx) = app.add_window_view(|_, _| View::new(window_id(), session.clone(), transport));
    let mut next = config();
    next.selection_mode = wire::SelectionMode::Rows;
    next.max_active_cells = 64;
    let template = next.schema.columns[1].clone();
    next.schema.columns = ["pin", "a", "b", "c"]
        .into_iter()
        .enumerate()
        .map(|(i, id)| wire::Column {
            id: id.into(),
            label: id.into(),
            width: 100.,
            pin: if i == 0 {
                wire::Pin::Left
            } else {
                wire::Pin::Unpinned
            },
            ..template.clone()
        })
        .collect();
    apply(
        &view,
        cx,
        vec![
            Op::Create(
                node(0),
                Kind::VirtualList,
                String::new(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetTable(node(0), next.clone()),
            Op::SetListConfig(node(0), next.list_config()),
            Op::SetListOrder(node(0), order(1, 0)),
            Op::SetStyle(
                node(0),
                vec![
                    Style::Width(Length::Px(250.)),
                    Style::Height(Length::Px(300.)),
                ],
            ),
            Op::SetRoot(Some(node(0))),
        ],
    );
    let native = view.read_with(cx, |v, _| v.tables[&node(0)].borrow().native.clone());
    assert_eq!(
        snapshot(&native, cx),
        vec![
            ("pin".into(), true, true),
            ("a".into(), false, true),
            ("b".into(), false, false),
        ]
    );
    let first = published(&view, cx);
    assert_eq!(first.len(), 1, "initial layout publishes once");
    assert_eq!(first[0].columns.len(), 3);
    draw(cx);
    assert!(
        published(&view, cx).is_empty(),
        "unchanged redraw is silent"
    );
    native.update(cx, |t, cx| {
        t.horizontal_scroll_handle
            .set_offset(gpui::point(px(-100.), px(0.)));
        cx.notify();
    });
    draw(cx);
    assert_eq!(
        snapshot(&native, cx),
        vec![
            ("pin".into(), true, true),
            ("b".into(), false, true),
            ("c".into(), false, false),
        ]
    );
    assert_eq!(published(&view, cx).len(), 1);
    native.update(cx, |t, cx| {
        t.horizontal_scroll_handle
            .set_offset(gpui::point(px(-110.), px(0.)));
        cx.notify();
    });
    draw(cx);
    assert_eq!(
        published(&view, cx).len(),
        1,
        "full-to-partial transition publishes"
    );
    native.update(cx, |t, cx| {
        t.horizontal_scroll_handle
            .set_offset(gpui::point(px(-111.), px(0.)));
        cx.notify();
    });
    draw(cx);
    assert!(
        published(&view, cx).is_empty(),
        "pixel movement with identical column bands is silent"
    );
    // Reset to a single unpinned column. No stale buffered range or pinned pane.
    next.schema_revision += 1;
    next.schema.columns = vec![wire::Column {
        id: "only".into(),
        label: "only".into(),
        width: 300.,
        ..template.clone()
    }];
    apply(&view, cx, vec![Op::SetTable(node(0), next.clone())]);
    assert_eq!(snapshot(&native, cx), vec![("only".into(), false, false)]);
    // All-pinned tables use the same geometry, including a clipped trailing pin.
    next.schema_revision += 1;
    next.schema.columns = ["x", "y", "z"]
        .into_iter()
        .map(|id| wire::Column {
            id: id.into(),
            label: id.into(),
            width: 100.,
            pin: wire::Pin::Left,
            ..template.clone()
        })
        .collect();
    apply(&view, cx, vec![Op::SetTable(node(0), next.clone())]);
    assert_eq!(
        snapshot(&native, cx),
        vec![
            ("x".into(), true, true),
            ("y".into(), true, true),
            ("z".into(), true, false),
        ]
    );
    native.update(cx, |t, cx| {
        t.reset_columns(cx);
        assert!(
            t.column_viewport().is_none(),
            "schema reset retires old layout immediately"
        );
    });
    draw(cx);
    assert_eq!(snapshot(&native, cx).len(), 3);
    assert_eq!(
        native.entity_id(),
        view.read_with(cx, |v, _| v.tables[&node(0)].borrow().native.entity_id())
    );
    cx.simulate_resize(gpui::size(px(50.), px(300.)));
    draw(cx);
    assert_eq!(snapshot(&native, cx), vec![("x".into(), true, false)]);
    assert_eq!(
        published(&view, cx).last().unwrap().columns,
        vec![("x".into(), wire::Pin::Left, false)]
    );
}

#[test]
fn all_64_column_commands_reveal_complete_bands_at_reference_width() {
    let mut app = TestAppContext::single();
    app.update(|cx| {
        gpui_base::init(cx);
        gpuio_table_adapter::init(cx);
    });
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id(), "Wide table", 1200., 800.)
        .unwrap();
    let (view, cx) = app.add_window_view(|_, _| View::new(window_id(), session.clone(), transport));
    cx.simulate_resize(gpui::size(px(1200.), px(800.)));
    let mut config = config();
    let template = config.schema.columns[1].clone();
    config.schema.columns = (0..64)
        .map(|i| wire::Column {
            id: i.to_string(),
            label: i.to_string(),
            width: 128.,
            pin: if i == 0 {
                wire::Pin::Left
            } else {
                wire::Pin::Unpinned
            },
            ..template.clone()
        })
        .collect();
    config.max_active_rows = 32;
    config.max_active_cells = 2048;
    let mut operations = vec![
        Op::Create(
            node(0),
            Kind::VirtualList,
            String::new(),
            Some(HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetListConfig(node(0), config.list_config()),
        Op::SetTable(node(0), config),
        Op::SetListOrder(node(0), order(1, 100_000)),
        Op::SetStyle(
            node(0),
            vec![
                Style::Width(Length::Px(1168.)),
                Style::Height(Length::Px(720.)),
            ],
        ),
        Op::Create(node(1), Kind::Container, String::new(), None),
    ];
    for i in 0..64 {
        operations.push(Op::CreateTableText(
            node(i + 2),
            wire::Cell {
                column: i.to_string(),
                copy_text: format!("Row 0 column {i} 世界"),
            },
        ));
    }
    operations.extend([
        Op::Splice(node(1), 0, 0, (2..66).map(node).collect()),
        Op::Splice(node(0), 0, 0, vec![node(1)]),
        Op::SetListRows(
            node(0),
            vec![Row {
                id: 1,
                node: node(1),
            }],
        ),
        Op::SetRoot(Some(node(0))),
    ]);
    apply(&view, cx, operations);
    let native = view.read_with(cx, |v, _| v.tables[&node(0)].borrow().native.clone());
    for i in 0..64 {
        apply(
            &view,
            cx,
            vec![command(
                i + 1,
                0,
                wire::Target::ScrollToColumn(i.to_string()),
            )],
        );
        let columns = snapshot(&native, cx);
        let (bounds, offset) = native.read_with(cx, |t, _| {
            (
                t.horizontal_scroll_handle.bounds(),
                t.horizontal_scroll_handle.offset(),
            )
        });
        assert!(
            columns
                .iter()
                .any(|(id, _, full)| id == &i.to_string() && *full),
            "column {i}: {columns:?}; bounds={bounds:?}; offset={offset:?}"
        );
        // Model the real client's event drain; otherwise repeated observations
        // intentionally overload the bounded transport after roughly 40 steps.
        let observations = published(&view, cx);
        if let Some(observation) = observations.last() {
            assert!(
                observation
                    .columns
                    .iter()
                    .any(|(id, _, full)| id == &i.to_string() && *full)
            );
        }
    }
}
