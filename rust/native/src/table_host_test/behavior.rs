//! Production Host on TestPlatform. No physical keyboard/AX claim.
use super::*;
use crate::session::Session;
use gpui::{TestAppContext, VisualTestContext};
use std::os::unix::net::UnixStream;

pub(super) fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
}
pub(super) fn apply(view: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|w, cx| {
        view.update(cx, |v, cx| {
            let base = v.session.borrow().tree(v.id).unwrap().revision();
            let transaction = Transaction {
                window: v.id,
                base,
                revision: base + 1,
                operations,
            };
            let applied = v
                .session
                .borrow_mut()
                .apply(&transaction)
                .unwrap_or_else(|error| {
                    panic!("Host transaction rejected: {error:?}: {transaction:?}")
                });
            v.update_editors(&applied.dirty, w, cx);
            v.table_actions(&applied.tables, w, cx);
            cx.notify();
        })
    });
    draw(cx);
}

#[test]
fn behavior_updates_keep_native_owner_repair_headers_and_drive_real_navigation() {
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
    cx.update(|w, _| w.activate_window());
    apply(&view, cx, initial());
    let native = view.read_with(cx, |v, _| v.tables[&node(0)].borrow().native.clone());
    native.update(cx, |t, cx| {
        assert!(t.row_header && t.loop_selection);
        assert!(t.replace_selection(Selection::Column("value".into()), cx));
    });
    let cell_left = view.read_with(cx, |v, _| v.probes.borrow()[&node(3)].bounds.left());
    let mut next = config();
    next.schema_revision = 2;
    apply(
        &view,
        cx,
        vec![
            Op::SetTable(node(0), next.clone()),
            Op::SetTableBehavior(
                node(0),
                Some(wire::Behavior {
                    row_header: false,
                    boundary: wire::Boundary::Stop,
                    selectable_headers: Some(vec!["name".into()]),
                }),
            ),
        ],
    );
    assert!(
        view.read_with(cx, |v, _| v.probes.borrow()[&node(3)].bounds.left()) < cell_left,
        "hidden row header releases actual cell space"
    );
    assert_eq!(
        native.entity_id(),
        view.read_with(cx, |v, _| v.tables[&node(0)].borrow().native.entity_id())
    );
    native.read_with(cx, |t, cx| {
        assert!(!t.row_header && !t.loop_selection);
        assert_eq!(t.selection(), &Selection::Empty);
        assert!(t.delegate().column(0, cx).selectable);
        assert!(!t.delegate().column(1, cx).selectable);
    });
    // Accessibility exposes Select only on eligible headers, while Sort remains
    // available independently. A manually queued forbidden Select is inert too.
    let header = |cx: &mut VisualTestContext, name: &str| {
        cx.a11y_tree()
            .unwrap()
            .nodes
            .into_iter()
            .find(|(_, n)| {
                n.role() == gpui::accesskit::Role::ColumnHeader && n.label() == Some(name)
            })
            .unwrap()
    };
    let (value_id, value) = header(cx, "value");
    assert!(!value.custom_actions().iter().any(|a| a.id == 0x4750_0011));
    assert!(value.custom_actions().iter().any(|a| a.id == 0x4750_0013));
    let (name_id, name) = header(cx, "name");
    assert!(name.custom_actions().iter().any(|a| a.id == 0x4750_0011));
    for (id, expected) in [
        (value_id, wire::Selection::Empty),
        (name_id, wire::Selection::Column("name".into())),
    ] {
        cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
            action: gpui::accesskit::Action::CustomAction,
            target_node: id,
            target_tree: gpui::accesskit::TreeId::ROOT,
            data: Some(gpui::accesskit::ActionData::CustomAction(0x4750_0011)),
        });
        draw(cx);
        native.read_with(cx, |t, _| assert_eq!(selection(t.selection()), expected));
    }
    cx.simulate_keystrokes("right");
    draw(cx);
    native.read_with(cx, |t, _| {
        assert_eq!(
            selection(t.selection()),
            wire::Selection::Column("name".into()),
            "header keyboard navigation cannot choose an ineligible header"
        )
    });
    // Home/End skip the forbidden first header; left with Stop stays put.
    next.schema_revision = 3;
    apply(
        &view,
        cx,
        vec![
            Op::SetTable(node(0), next.clone()),
            Op::SetTableBehavior(
                node(0),
                Some(wire::Behavior {
                    row_header: false,
                    boundary: wire::Boundary::Stop,
                    selectable_headers: Some(vec!["value".into()]),
                }),
            ),
        ],
    );
    for key in ["home", "left", "end"] {
        cx.simulate_keystrokes(key);
        draw(cx);
        native.read_with(cx, |t, _| {
            assert_eq!(
                selection(t.selection()),
                wire::Selection::Column("value".into())
            )
        });
    }
    // With Wrap the only eligible header is found again in either direction.
    next.schema_revision = 4;
    apply(
        &view,
        cx,
        vec![
            Op::SetTable(node(0), next.clone()),
            Op::SetTableBehavior(
                node(0),
                Some(wire::Behavior {
                    row_header: false,
                    boundary: wire::Boundary::Wrap,
                    selectable_headers: Some(vec!["value".into()]),
                }),
            ),
        ],
    );
    for key in ["left", "right"] {
        cx.simulate_keystrokes(key);
        draw(cx);
        native.read_with(cx, |t, _| {
            assert_eq!(
                selection(t.selection()),
                wire::Selection::Column("value".into())
            )
        });
    }
    // An empty eligibility set leaves header navigation inert.
    next.schema_revision = 5;
    apply(
        &view,
        cx,
        vec![
            Op::SetTable(node(0), next.clone()),
            Op::SetTableBehavior(
                node(0),
                Some(wire::Behavior {
                    row_header: false,
                    boundary: wire::Boundary::Stop,
                    selectable_headers: Some(vec![]),
                }),
            ),
        ],
    );
    for key in ["left", "right", "home", "end"] {
        cx.simulate_keystrokes(key);
        draw(cx);
        native.read_with(cx, |t, _| {
            assert_eq!(selection(t.selection()), wire::Selection::Empty)
        });
    }
    cx.update(|w, cx| native.focus_handle(cx).focus(w, cx));
    native.update(cx, |t, cx| {
        assert!(t.replace_selection(
            Selection::Cell {
                row: RowKey(1),
                column: "value".into()
            },
            cx
        ));
    });
    draw(cx);
    cx.simulate_keystrokes("up");
    draw(cx);
    native.read_with(cx, |t, _| {
        assert_eq!(
            selection(t.selection()),
            wire::Selection::Cell(1, "value".into())
        )
    });
    // Restore defaults and the same input now wraps to the final logical row,
    // even though that row has not been materialized by the OCaml producer.
    next.schema_revision = 6;
    apply(
        &view,
        cx,
        vec![
            Op::SetTable(node(0), next),
            Op::SetTableBehavior(node(0), None),
        ],
    );
    cx.simulate_keystrokes("up");
    draw(cx);
    native.read_with(cx, |t, _| {
        assert!(t.row_header && t.loop_selection);
        assert_eq!(
            selection(t.selection()),
            wire::Selection::Cell(100_000, "value".into())
        );
    });
}
