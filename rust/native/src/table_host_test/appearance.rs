//! Native retained presentation on TestPlatform; physical GPU readback is separate.
use super::behavior::{apply, draw};
use super::*;
use crate::session::Session;
use gpui::TestAppContext;
use std::os::unix::net::UnixStream;

#[test]
fn colors_and_padding_keep_owners_selection_and_keyed_scroll() {
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
    cx.simulate_a11y_active(true);
    apply(&view, cx, initial());
    let native = view.read_with(cx, |v, _| v.tables[&node(0)].borrow().native.clone());
    let before = view.read_with(cx, |v, _| v.probes.borrow()[&node(3)].bounds);
    native.update(cx, |t, cx| {
        assert!(t.replace_selection(
            Selection::Cell {
                row: RowKey(1),
                column: "name".into()
            },
            cx
        ))
    });
    let paint = wire::Appearance {
        striped: true,
        colors: vec![
            (wire::Part::HeaderBackground, 0xff000080),
            (wire::Part::HeaderForeground, 0x00ff00ff),
            (wire::Part::StripeBackground, 0x0000ffff),
            (wire::Part::ColumnBorder, 0x00ffffff),
        ],
        ..wire::Appearance::default()
    };
    apply(
        &view,
        cx,
        vec![Op::SetTableAppearance(node(0), Some(paint.clone()))],
    );
    let after = view.read_with(cx, |v, _| v.probes.borrow()[&node(3)].bounds);
    assert_eq!(before, after, "paint-only update preserves geometry");
    native.read_with(cx, |t, _| {
        let a = t.delegate().appearance();
        assert_eq!(
            a.tokens.table_head,
            super::super::super::color(&Color::Rgba(0xff000080))
        );
        assert_eq!(
            a.table_head_foreground,
            super::super::super::color(&Color::Rgba(0x00ff00ff))
        );
        assert_ne!(
            a.border,
            t.delegate().base_appearance().border,
            "part border must not change outer frame border"
        );
        assert_eq!(
            selection(t.selection()),
            wire::Selection::Cell(1, "name".into())
        );
    });
    let shared = wire::Padding {
        top: 2.,
        right: 2.,
        bottom: 2.,
        left: 2.,
    };
    let zero = wire::Padding {
        top: 0.,
        right: 0.,
        bottom: 0.,
        left: 0.,
    };
    let padded = wire::Appearance {
        padding: Some(shared),
        column_padding: vec![("name".into(), zero)],
        ..paint
    };
    let mut next = config();
    next.schema_revision = 2;
    apply(
        &view,
        cx,
        vec![
            Op::SetTableAppearance(node(0), Some(padded)),
            Op::SetTable(node(0), next.clone()),
        ],
    );
    let padded_bounds = view.read_with(cx, |v, _| v.probes.borrow()[&node(3)].bounds);
    assert!(
        padded_bounds.left() < before.left(),
        "zero padding reaches the native cell"
    );
    assert!(padded_bounds.top() < before.top());
    assert_eq!(
        native.entity_id(),
        view.read_with(cx, |v, _| v.tables[&node(0)].borrow().native.entity_id())
    );
    native.read_with(cx, |t, cx| {
        assert_eq!(t.delegate().column(0, cx).paddings.unwrap().left, px(0.));
        assert_eq!(t.delegate().column(1, cx).paddings.unwrap().left, px(2.));
        assert_eq!(
            selection(t.selection()),
            wire::Selection::Cell(1, "name".into())
        );
    });
    native.update(cx, |t, _| {
        t.vertical_scroll_handle
            .0
            .borrow_mut()
            .base_handle
            .set_offset(gpui::point(px(0.), px(-96.)))
    });
    draw(cx);
    let offset = native.read_with(cx, |t, _| {
        t.vertical_scroll_handle.0.borrow().base_handle.offset()
    });
    next.schema_revision = 3;
    apply(
        &view,
        cx,
        vec![
            Op::SetTableAppearance(node(0), None),
            Op::SetTable(node(0), next),
        ],
    );
    native.read_with(cx, |t, cx| {
        assert!(t.delegate().column(0, cx).paddings.is_none());
        assert_eq!(
            t.vertical_scroll_handle.0.borrow().base_handle.offset(),
            offset,
            "reset retains keyed viewport"
        );
        assert_eq!(
            selection(t.selection()),
            wire::Selection::Cell(1, "name".into())
        );
    });
    // Native stripes fill short tables without pretending filler rows are data.
    let mut short = vec![
        Op::Splice(node(0), 1, 11, vec![]),
        Op::SetListRows(
            node(0),
            vec![Row {
                id: 1,
                node: node(1),
            }],
        ),
        Op::SetListOrder(node(0), order(2, 1)),
        Op::SetTableAppearance(
            node(0),
            Some(wire::Appearance {
                striped: true,
                ..wire::Appearance::default()
            }),
        ),
    ];
    short.extend((6..61).map(|slot| Op::Remove(node(slot))));
    apply(&view, cx, short);
    draw(cx);
    assert_eq!(session.borrow().tree(window_id()).unwrap().len(), 6);
    let semantic = cx.a11y_tree().unwrap();
    let rows: Vec<_> = semantic
        .nodes
        .iter()
        .filter(|(_, n)| n.role() == gpui::accesskit::Role::Row)
        .collect();
    assert_eq!(rows.len(), 1, "filler rows are absent from table semantics");
    assert_eq!(rows[0].1.row_index(), Some(0));

    native.read_with(cx, |t, _| {
        assert_eq!(t.delegate().index.len(), 1);
        assert_eq!(
            t.delegate().handles.len(),
            1,
            "decorative fillers never get row owners"
        );
    });
    let mut empty = vec![
        Op::Splice(node(0), 0, 1, vec![]),
        Op::SetListRows(node(0), vec![]),
        Op::SetListOrder(node(0), order(3, 0)),
    ];
    empty.extend((1..6).map(|slot| Op::Remove(node(slot))));
    apply(&view, cx, empty);
    assert_eq!(session.borrow().tree(window_id()).unwrap().len(), 1);
}
