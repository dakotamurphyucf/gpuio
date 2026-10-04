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
