//! Production ordinary-View layout/semantics on TestPlatform, not OS acceptance.
use super::*;
use crate::session::Session;
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpuio_protocol::{HandlerId, accessibility as a, grid_location as grid};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn metadata(role: a::Role) -> a::Config {
    a::Config {
        role: Some(role),
        label: None,
        description: None,
        live: a::Live::Off,
        field: None,
        current: None,
    }
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn apply(view: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    view.update(cx, |v, cx| {
        let base = v.session.borrow().tree(v.id).unwrap().revision();
        v.session
            .borrow_mut()
            .apply(&Transaction {
                window: v.id,
                base,
                revision: base + 1,
                operations,
            })
            .unwrap_or_else(|error| panic!("structural transaction base {base}: {error:?}"));
        cx.notify();
    });
    draw(cx);
}
fn cell(row: i64, column: i64, column_span: i64) -> a::TableCell {
    a::TableCell {
        row,
        column,
        column_span,
    }
}

#[test]
fn structural_table_layout_semantics_controls_and_reset_share_the_real_host() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window, "Table", 400., 300.)
        .unwrap();
    let (view, cx) =
        app.add_window_view(|_, _| View::new(window, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    let mut operations = (0..=16)
        .map(|n| {
            Op::Create(
                id(n),
                match n {
                    4 | 8 | 14 | 16 => Kind::Text,
                    10 => Kind::Button,
                    _ => Kind::Container,
                },
                if n == 10 { "Run".into() } else { String::new() },
                (n == 10).then(|| HandlerId::from_parts(0, 1).unwrap()),
            )
        })
        .collect::<Vec<_>>();
    for (parent, children) in [
        (0, vec![1, 5, 11, 15]),
        (1, vec![2]),
        (2, vec![3]),
        (3, vec![4]),
        (5, vec![6]),
        (6, vec![7, 9]),
        (7, vec![8]),
        (9, vec![10]),
        (11, vec![12]),
        (12, vec![13]),
        (13, vec![14]),
        (15, vec![16]),
    ] {
        operations.push(Op::Splice(
            id(parent),
            0,
            0,
            children.into_iter().map(id).collect(),
        ));
    }
    for (n, role) in [
        (
            0,
            a::Role::Table(a::TableInfo {
                rows: Some(3),
                columns: Some(3),
            }),
        ),
        (1, a::Role::RowGroup),
        (5, a::Role::RowGroup),
        (11, a::Role::RowGroup),
        (2, a::Role::TableRow(0)),
        (6, a::Role::TableRow(1)),
        (12, a::Role::TableRow(2)),
        (3, a::Role::ColumnHeader(cell(0, 0, 3))),
        (7, a::Role::RowHeader(cell(1, 0, 2))),
        (9, a::Role::TableCell(cell(1, 2, 1))),
        (13, a::Role::TableCell(cell(2, 0, 3))),
        (15, a::Role::Caption),
    ] {
        operations.push(Op::SetAccessibility(id(n), Some(metadata(role))));
    }
    operations.push(Op::SetStyle(
        id(0),
        vec![Style::Fields(vec![
            Field::Width(Length::Px(300.)),
            Field::Direction(1),
        ])],
    ));
    for n in [2, 6, 12] {
        operations.push(Op::SetStyle(
            id(n),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(300.)),
                Field::Height(Length::Px(40.)),
                Field::Display(2),
                Field::GridColumns(3),
            ])],
        ));
    }
    for (n, column, span) in [(3, 0, 3), (7, 0, 2), (9, 2, 1), (13, 0, 3)] {
        operations.push(Op::SetStyle(
            id(n),
            vec![Style::Fields(vec![Field::GridLocation(grid::Location {
                column: grid::Axis {
                    start: grid::Edge::Line(column + 1),
                    end: grid::Edge::Span(span),
                },
                row: grid::Axis {
                    start: grid::Edge::Auto,
                    end: grid::Edge::Auto,
                },
            })])],
        ));
    }
    operations.push(Op::SetRoot(Some(id(0))));
    apply(&view, cx, operations);
    let tree = cx.a11y_tree().unwrap();
    let table = tree
        .nodes
        .iter()
        .find(|(_, n)| n.role() == gpui::accesskit::Role::Table)
        .unwrap()
        .1
        .clone();
    assert_eq!(table.row_count(), Some(3));
    assert_eq!(table.column_count(), Some(3));
    let cells = tree
        .nodes
        .iter()
        .filter(|(_, n)| {
            matches!(
                n.role(),
                gpui::accesskit::Role::Cell
                    | gpui::accesskit::Role::ColumnHeader
                    | gpui::accesskit::Role::RowHeader
            )
        })
        .map(|(_, n)| (n.row_index(), n.column_index(), n.column_span()))
        .collect::<Vec<_>>();
    assert_eq!(cells.len(), 4);
    assert!(cells.contains(&(Some(1), Some(2), Some(1))));
    assert!(cells.contains(&(Some(1), Some(0), Some(2))));
    view.read_with(cx, |v, _| {
        let probes = v.probes.borrow();
        assert!((f32::from(probes[&id(7)].bounds.size.width) - 200.).abs() < 0.1);
        assert!(
            (f32::from(probes[&id(9)].bounds.left() - probes[&id(7)].bounds.left()) - 200.).abs()
                < 0.1
        );
    });
    let (button_id, button) = tree
        .nodes
        .iter()
        .find(|(_, n)| n.role() == gpui::accesskit::Role::Button)
        .unwrap();
    assert!(button.supports_action(gpui::accesskit::Action::Click));
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: *button_id,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    draw(cx);
    assert!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|e| matches!(e,Event::Press(_,node,_,_) if *node==id(10)))
    );
    apply(&view, cx, vec![Op::SetAccessibility(id(7), None)]);
    let tree = cx.a11y_tree().unwrap();
    assert!(
        !tree
            .nodes
            .iter()
            .any(|(_, n)| n.role() == gpui::accesskit::Role::RowHeader)
    );
    assert!(
        tree.nodes
            .iter()
            .any(|(id, n)| *id == *button_id && n.role() == gpui::accesskit::Role::Button)
    );
    let mut remove = vec![Op::SetRoot(None)];
    remove.extend((0..=16).rev().map(|n| Op::Remove(id(n))));
    apply(&view, cx, remove);
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.role() == gpui::accesskit::Role::Table)
    );
}
