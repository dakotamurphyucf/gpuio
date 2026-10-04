//! Production table on TestPlatform; no physical desktop claim.
use super::*;
use crate::{host::scrollbar_host::Owner, scrollbar_geometry::Axis, session::Session};
use gpui::{TestAppContext, VisualTestContext};
use gpuio_protocol::scrollbar::{Appearance, Axis as Axes, Config, Mode, Motion};
use std::os::unix::net::UnixStream;

fn presentation() -> Config {
    Config {
        label: "Table viewport".into(),
        axis: Axes::Both,
        mode: Mode::Always,
        appearance: Appearance::default(),
        motion: Motion::default(),
    }
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn apply(view: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|w, cx| {
        view.update(cx, |v, cx| {
            let base = v.session.borrow().tree(v.id).unwrap().revision();
            let applied = v
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: v.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            v.update_editors(&applied.dirty, w, cx);
            v.table_actions(&applied.tables, w, cx);
            cx.notify();
        })
    });
    draw(cx);
}

#[test]
fn scoped_ranges_keep_table_handles_selection_and_offsets() {
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
    let (view, cx) =
        app.add_window_view(|_, _| View::new(window_id(), session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|w, _| w.activate_window());
    let mut operations = initial();
    operations.push(Op::SetScrollbar(node(0), Some(Box::new(presentation()))));
    apply(&view, cx, operations);
    let native = view.read_with(cx, |v, _| v.tables[&node(0)].borrow().native.clone());
    let bars: Vec<_> = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .filter(|(_, n)| n.role() == gpui::accesskit::Role::ScrollBar)
        .collect();
    assert_eq!(bars.len(), 2, "both actual overflowing axes");
    let range = |name| {
        bars.iter()
            .find(|(_, b)| b.label() == Some(name))
            .unwrap()
            .1
            .clone()
    };
    let h = range("Table viewport — horizontal");
    let v = range("Table viewport — vertical");
    let scale = cx.update(|w, _| f64::from(w.scale_factor()));
    let hb = h.bounds().unwrap();
    let vb = v.bounds().unwrap();
    assert_eq!(
        hb.x0 / scale,
        161.,
        "pinned column plus outer border excluded"
    );
    assert_eq!(vb.y0 / scale, 33., "header plus outer border excluded");
    assert!(
        hb.x1 <= vb.x0 && vb.y1 <= hb.y0,
        "corner ranges do not overlap"
    );
    assert_eq!(v.max_numeric_value(), Some(3_200_000. - 266.));
    native.update(cx, |t, cx| {
        assert!(t.replace_selection(
            Selection::Cell {
                row: RowKey(1),
                column: "name".into()
            },
            cx
        ));
    });
    draw(cx);
    let horizontal = view.read_with(cx, |v, _| {
        v.scrollbars[&(node(0), Owner::TableHorizontal)].clone()
    });
    let vertical = view.read_with(cx, |v, _| {
        v.scrollbars[&(node(0), Owner::TableVertical)].clone()
    });
    for (state, axis) in [(&horizontal, Axis::Horizontal), (&vertical, Axis::Vertical)] {
        let focus = state.borrow().focus(axis).clone();
        cx.update(|w, cx| w.focus(&focus, cx));
        draw(cx);
        assert!(cx.update(|w, _| focus.is_focused(w)));
        cx.simulate_keystrokes("end");
        draw(cx);
        assert!(native.read_with(cx, |t, _| matches!(t.selection(), Selection::Cell { row: RowKey(1), column } if column.as_ref() == "name")), "range keys must not select table rows");
    }
    // AX writes the existing handle, and dragging keeps table focus. Escape
    // must therefore be cancelled at Host capture, outside range key handlers.
    let vertical_id = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Table viewport — vertical"))
        .unwrap()
        .0;
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::SetValue,
        target_node: vertical_id,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: Some(gpui::accesskit::ActionData::NumericValue(640.)),
    });
    draw(cx);
    assert_eq!(
        native.read_with(cx, |t, _| t
            .vertical_scroll_handle
            .0
            .borrow()
            .base_handle
            .offset()
            .y),
        px(-640.)
    );
    cx.simulate_keystrokes("end");
    draw(cx);
    let table_focus = native.read_with(cx, |t, cx| t.focus_handle(cx));
    cx.update(|w, cx| w.focus(&table_focus, cx));
    draw(cx);
    let thumb = gpui::point(
        px((vb.x1 / scale - 7.) as f32),
        px((vb.y1 / scale - 28.) as f32),
    );
    cx.simulate_mouse_move(thumb, None, Default::default());
    draw(cx);
    cx.simulate_mouse_down(thumb, gpui::MouseButton::Left, Default::default());
    assert!(vertical.borrow().is_dragging());
    assert!(cx.update(|w, _| table_focus.is_focused(w)));
    cx.simulate_keystrokes("escape");
    assert!(!vertical.borrow().is_dragging());
    assert!(cx.update(|w, _| w.captured_hitbox().is_none()));
    assert!(
        native.read_with(cx, |t, _| matches!(t.selection(), Selection::Cell { .. })),
        "Escape cancels drag before table selection"
    );
    cx.simulate_mouse_up(thumb, gpui::MouseButton::Left, Default::default());
    let offsets = native.read_with(cx, |t, _| {
        (
            t.horizontal_scroll_handle.offset(),
            t.vertical_scroll_handle.0.borrow().base_handle.offset(),
        )
    });
    assert!(offsets.0.x < px(0.));
    assert!(offsets.1.y < px(-1000.));
    let mut custom = presentation();
    custom.appearance.track.width = Some(28.);
    apply(
        &view,
        cx,
        vec![Op::SetScrollbar(node(0), Some(Box::new(custom.clone())))],
    );
    assert!(view.read_with(cx, |v, _| Rc::ptr_eq(
        &v.scrollbars[&(node(0), Owner::TableVertical)],
        &vertical
    )));
    assert_eq!(
        native.read_with(cx, |t, _| (
            t.horizontal_scroll_handle.offset(),
            t.vertical_scroll_handle.0.borrow().base_handle.offset()
        )),
        offsets
    );
    custom.axis = Axes::Vertical;
    apply(
        &view,
        cx,
        vec![Op::SetScrollbar(node(0), Some(Box::new(custom)))],
    );
    let bars: Vec<_> = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .filter(|(_, n)| n.role() == gpui::accesskit::Role::ScrollBar)
        .collect();
    assert_eq!(bars.len(), 1);
    assert_eq!(
        bars[0].1.bounds().unwrap().y1 / scale,
        299.,
        "single axis reclaims corner"
    );
    let mut hidden = config();
    hidden.scrollbar = false;
    apply(
        &view,
        cx,
        vec![
            Op::SetListConfig(node(0), hidden.list_config()),
            Op::SetTable(node(0), hidden),
        ],
    );
    assert!(view.read_with(cx, |v, _| v.scrollbars.is_empty()));
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.role() == gpui::accesskit::Role::ScrollBar)
    );
    assert!(!cx.update(|w, _| vertical.borrow().focused(w)));
    apply(
        &view,
        cx,
        vec![
            Op::SetListConfig(node(0), config().list_config()),
            Op::SetTable(node(0), config()),
            Op::SetScrollbar(node(0), None),
        ],
    );
    assert!(view.read_with(cx, |v, _| v.scrollbars.is_empty()));
    assert!(
        view.read_with(cx, |v, _| v.tables[&node(0)].borrow().native.entity_id()
            == native.entity_id())
    );
    assert_eq!(
        native.read_with(cx, |t, _| (
            t.horizontal_scroll_handle.offset(),
            t.vertical_scroll_handle.0.borrow().base_handle.offset()
        )),
        offsets
    );
    let empty = vec![
        Op::SetScrollbar(node(0), Some(Box::new(presentation()))),
        Op::SetListOrder(node(0), order(2, 0)),
        Op::SetListRows(node(0), vec![]),
        Op::Splice(node(0), 0, 12, vec![]),
    ]
    .into_iter()
    .chain((1..=60).rev().map(|slot| Op::Remove(node(slot))))
    .collect();
    apply(&view, cx, empty);
    let bars: Vec<_> = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .filter(|(_, n)| n.role() == gpui::accesskit::Role::ScrollBar)
        .collect();
    assert_eq!(
        bars.len(),
        1,
        "empty body retains only overflowing header range"
    );
    assert_eq!(bars[0].1.label(), Some("Table viewport — horizontal"));
    assert_eq!(
        bars[0].1.bounds().unwrap().x1 / scale,
        519.,
        "stale empty-body handle cannot reserve a corner"
    );
    let mut pinned = config();
    pinned.schema_revision = 2;
    pinned.schema.columns[1].pin = wire::Pin::Left;
    apply(
        &view,
        cx,
        vec![
            Op::SetTable(node(0), pinned),
            Op::SetListOrder(node(0), order(3, 100_000)),
        ],
    );
    let bars: Vec<_> = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .filter(|(_, n)| n.role() == gpui::accesskit::Role::ScrollBar)
        .collect();
    assert_eq!(
        bars.len(),
        1,
        "all-pinned columns leave only the body range"
    );
    assert_eq!(bars[0].1.label(), Some("Table viewport — vertical"));
    assert_eq!(
        bars[0].1.bounds().unwrap().y1 / scale,
        299.,
        "all-pinned columns cannot reserve a horizontal corner"
    );
    cx.update(|w, cx| {
        view.update(cx, |v, cx| v.close_scrollbars(w, cx));
        w.remove_window();
    });
}
