use gpuio_native::{session::Session, tree::Tree};
use gpuio_protocol::{HandlerId, NodeId, WindowId, list, table::*, v1::*};

fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn handler(generation: i64) -> HandlerId {
    HandlerId::from_parts(0, generation).unwrap()
}
fn config() -> Config {
    Config {
        schema_revision: 1,
        query_generation: 0,
        schema: Schema {
            columns: vec![Column {
                id: "value".into(),
                label: "Value".into(),
                width: 160.,
                min_width: 40.,
                max_width: 400.,
                pin: Pin::Unpinned,
                alignment: Alignment::Left,
                resizable: true,
                movable: true,
                sortable: true,
            }],
            headers: vec![],
        },
        sort: None,
        row_height: 32.,
        overscan: 64.,
        max_active_rows: 4,
        max_active_cells: 4,
        selection_mode: SelectionMode::RowsAndCells,
        column_selection: true,
        disabled: false,
        scrollbar: true,
        label: "Results".into(),
    }
}
fn order(revision: i64, count: i64) -> list::Order {
    list::Order {
        revision,
        runs: vec![list::IdRun { first: 1, count }],
    }
}
fn tx(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: window(),
        base,
        revision: base + 1,
        operations,
    }
}
fn initial() -> Vec<Op> {
    vec![
        Op::Create(node(0), Kind::VirtualList, String::new(), Some(handler(1))),
        Op::SetTable(node(0), config()),
        Op::SetListConfig(node(0), config().list_config()),
        Op::SetListOrder(node(0), order(1, 100_000)),
        Op::Create(node(1), Kind::Container, String::new(), None),
        Op::Create(node(2), Kind::Container, String::new(), None),
        Op::SetTableCell(
            node(2),
            Cell {
                column: "value".into(),
                copy_text: "日本語👨‍👩‍👧‍👦".into(),
            },
        ),
        Op::Create(node(3), Kind::Text, "display".into(), None),
        Op::Splice(node(2), 0, 0, vec![node(3)]),
        Op::Splice(node(1), 0, 0, vec![node(2)]),
        Op::Splice(node(0), 0, 0, vec![node(1)]),
        Op::SetListRows(
            node(0),
            vec![list::Row {
                id: 1,
                node: node(1),
            }],
        ),
        Op::SetRoot(Some(node(0))),
    ]
}
fn command(serial: i64, target: Target) -> Op {
    Op::TableCommand(
        node(0),
        Command {
            serial,
            query_generation: 0,
            target,
        },
    )
}
#[test]
fn retained_cells_are_bounded_charged_and_structurally_owned() {
    let mut tree = Tree::new(window());
    tree.apply(&tx(0, initial())).unwrap();
    assert_eq!(tree.len(), 4);
    assert_eq!(
        tree.get(node(0))
            .unwrap()
            .list_index
            .as_ref()
            .unwrap()
            .len(),
        100_000
    );
    let baseline = tree.retained_bytes();
    assert!(baseline >= 100_000 * 192 + config().retained_bytes());
    let copy = "x".repeat(MAX_COPY_BYTES);
    let update = tx(
        1,
        vec![Op::SetTableCell(
            node(2),
            Cell {
                column: "value".into(),
                copy_text: copy.clone(),
            },
        )],
    );
    assert_eq!(
        tree.apply_with_budget(&update, baseline),
        Err(ErrorCode::LimitExceeded)
    );
    assert_eq!(tree.revision(), 1);
    assert_eq!(tree.retained_bytes(), baseline);
    assert_eq!(
        tree.get(node(2))
            .unwrap()
            .table_cell
            .as_ref()
            .unwrap()
            .copy_text,
        "日本語👨‍👩‍👧‍👦"
    );
    tree.apply(&update).unwrap();
    assert_eq!(
        tree.retained_bytes() - baseline,
        copy.len() - "日本語👨‍👩‍👧‍👦".len()
    );
    // Touching a cell or row must revalidate its unchanged table ancestor.
    for invalid in [
        Op::SetTableCell(
            node(2),
            Cell {
                column: "other".into(),
                copy_text: String::new(),
            },
        ),
        Op::SetText(node(1), "not an inert row".into()),
        Op::Bind(node(2), Some(handler(2))),
        Op::Splice(node(1), 0, 1, vec![]),
        Op::Splice(node(2), 0, 1, vec![]),
        Op::SetTableCell(
            node(3),
            Cell {
                column: "value".into(),
                copy_text: String::new(),
            },
        ),
    ] {
        assert!(
            tree.apply(&tx(
                2,
                vec![Op::SetText(node(3), "must roll back".into()), invalid]
            ))
            .is_err()
        );
        assert_eq!(tree.revision(), 2);
        assert_eq!(tree.get(node(3)).unwrap().text.as_ref(), "display");
    }
    let mut orphan = Tree::new(window());
    assert!(
        orphan
            .apply(&tx(
                0,
                vec![
                    Op::Create(node(0), Kind::Container, String::new(), None),
                    Op::Create(node(1), Kind::Text, String::new(), None),
                    Op::Splice(node(0), 0, 0, vec![node(1)]),
                    Op::SetTableCell(
                        node(0),
                        Cell {
                            column: "value".into(),
                            copy_text: String::new()
                        }
                    ),
                    Op::SetRoot(Some(node(0))),
                ]
            ))
            .is_err()
    );
}
#[test]
fn schema_query_and_command_admission_is_atomic_and_ordered() {
    let mut tree = Tree::new(window());
    tree.apply(&tx(0, initial())).unwrap();
    let mut changed = config();
    changed.schema.columns[0].width = 200.;
    assert!(
        tree.apply(&tx(1, vec![Op::SetTable(node(0), changed.clone())]))
            .is_err()
    );
    changed.schema_revision = 2;
    tree.apply(&tx(1, vec![Op::SetTable(node(0), changed.clone())]))
        .unwrap();
    let mut query = changed.clone();
    query.query_generation = 1;
    assert!(
        tree.apply(&tx(2, vec![Op::SetTable(node(0), query.clone())]))
            .is_err()
    );
    // Final handler/config govern admission, independently of operation order.
    let action = Command {
        serial: 1,
        query_generation: 1,
        target: Target::Reveal(100_001, Some("value".into())),
    };
    let applied = tree
        .apply(&tx(
            2,
            vec![
                Op::TableCommand(node(0), action.clone()),
                Op::SetListOrder(node(0), order(2, 100_001)),
                Op::SetTable(node(0), query.clone()),
                Op::Bind(node(0), Some(handler(2))),
            ],
        ))
        .unwrap();
    assert_eq!(applied.tables, vec![(node(0), action.clone())]);
    assert_eq!(tree.get(node(0)).unwrap().table_serial, 1);
    let mut old = action.clone();
    old.query_generation = 0;
    old.serial = 2;
    for action in [action, old] {
        assert!(
            tree.apply(&tx(3, vec![Op::TableCommand(node(0), action)]))
                .is_err()
        );
    }
    // A failed later action rolls back the first serial and unrelated text too.
    let first = Command {
        serial: 2,
        query_generation: 1,
        target: Target::ScrollToEnd,
    };
    let bad = Command {
        serial: 3,
        query_generation: 1,
        target: Target::Reveal(100_002, None),
    };
    assert!(
        tree.apply(&tx(
            3,
            vec![
                Op::TableCommand(node(0), first.clone()),
                Op::TableCommand(node(0), bad)
            ]
        ))
        .is_err()
    );
    assert_eq!(tree.get(node(0)).unwrap().table_serial, 1);
    tree.apply(&tx(3, vec![Op::TableCommand(node(0), first)]))
        .unwrap();
    assert!(
        tree.apply(&tx(4, vec![Op::SetText(node(3), "later".into())]))
            .unwrap()
            .tables
            .is_empty()
    );
    assert!(
        tree.apply(&tx(5, vec![Op::SetTable(node(0), changed)]))
            .is_err()
    );
    let mut geometry = query.clone();
    geometry.row_height = 48.;
    assert!(
        tree.apply(&tx(5, vec![Op::SetTable(node(0), geometry.clone())]))
            .is_err()
    );
    tree.apply(&tx(
        5,
        vec![
            Op::SetTable(node(0), geometry.clone()),
            Op::SetListConfig(node(0), geometry.list_config()),
        ],
    ))
    .unwrap();
}
#[test]
fn commands_validate_logical_membership_not_just_the_active_cache() {
    let mut tree = Tree::new(window());
    tree.apply(&tx(0, initial())).unwrap();
    for target in [
        Target::Reveal(100_000, None),
        Target::SetSelection(Selection::Cell(100_000, "value".into())),
        Target::ScrollTo(1, 31.5),
    ] {
        let serial = tree.revision();
        tree.apply(&tx(serial, vec![command(serial, target)]))
            .unwrap();
    }
    for target in [
        Target::ScrollTo(1, 32.),
        Target::Reveal(0, None),
        Target::ScrollToColumn("missing".into()),
        Target::SetSelection(Selection::Cell(100_001, "value".into())),
    ] {
        assert!(tree.apply(&tx(4, vec![command(4, target)])).is_err());
    }
    assert!(
        tree.apply(&tx(
            4,
            vec![
                command(4, Target::ResetColumns),
                command(4, Target::ScrollToEnd)
            ]
        ))
        .is_err()
    );
    assert_eq!(tree.get(node(0)).unwrap().table_serial, 3);
}
fn session() -> Session {
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window(), "Table", 400., 300.).unwrap();
    session.apply(&tx(0, initial())).unwrap();
    session
}
#[test]
fn input_obeys_live_schema_query_handler_policy_and_window_lifetime() {
    let mut session = session();
    let input = |request| Input {
        schema_revision: 1,
        query_generation: 0,
        request,
    };
    let send = |session: &Session, request| {
        session.table_input(window(), node(0), handler(1), 1, input(request))
    };
    for valid in [
        Request::Select(Selection::Cell(100_000, "value".into())),
        Request::Copy(Selection::Cell(1, "value".into())),
        Request::Resize(vec![("value".into(), 240.)]),
        Request::Sort("value".into(), Some(Direction::Descending)),
    ] {
        assert!(send(&session, valid).is_some());
    }
    for invalid in [
        Request::Select(Selection::Row(100_001)),
        Request::Activate(1, Some("missing".into())),
        Request::Copy(Selection::Empty),
        Request::Resize(vec![("value".into(), 401.)]),
        Request::Move("missing".into(), None),
    ] {
        assert!(send(&session, invalid).is_none());
    }
    let event = input(Request::Select(Selection::Row(1)));
    assert!(
        session
            .table_input(window(), node(0), handler(2), 1, event.clone())
            .is_none()
    );
    assert!(
        session
            .table_input(window(), node(0), handler(1), 2, event.clone())
            .is_none()
    );
    let mut next = config();
    next.schema_revision = 2;
    next.schema.columns[0].sortable = false;
    session
        .apply(&tx(1, vec![Op::SetTable(node(0), next.clone())]))
        .unwrap();
    assert!(send(&session, Request::Select(Selection::Row(1))).is_none());
    let event = Input {
        schema_revision: 2,
        ..event
    };
    assert!(
        session
            .table_input(window(), node(0), handler(1), 1, event.clone())
            .is_some()
    );
    assert!(
        session
            .table_input(
                window(),
                node(0),
                handler(1),
                2,
                Input {
                    request: Request::Sort("value".into(), None),
                    ..event.clone()
                }
            )
            .is_none()
    );
    next.query_generation = 1;
    session
        .apply(&tx(
            2,
            vec![
                Op::Bind(node(0), Some(handler(2))),
                Op::SetTable(node(0), next.clone()),
            ],
        ))
        .unwrap();
    assert!(
        session
            .table_input(window(), node(0), handler(2), 3, event.clone())
            .is_none()
    );
    let event = Input {
        query_generation: 1,
        ..event
    };
    assert!(
        session
            .table_input(window(), node(0), handler(2), 3, event.clone())
            .is_some()
    );
    next.disabled = true;
    session
        .apply(&tx(3, vec![Op::SetTable(node(0), next)]))
        .unwrap();
    assert!(
        session
            .table_input(window(), node(0), handler(2), 4, event.clone())
            .is_none()
    );
    session.close(window()).unwrap();
    assert!(
        session
            .table_input(window(), node(0), handler(2), 4, event)
            .is_none()
    );
}

#[test]
fn query_reset_retires_old_viewports_and_eviction_releases_cell_bytes() {
    let mut session = session();
    let viewport = list::Viewport {
        order_revision: 1,
        visible_first: 0,
        visible_last: 1,
        requested: vec![1],
        pinned: vec![],
        anchor: Some((1, 0.)),
        following_tail: false,
        at_start: true,
        at_end: false,
        budget_exhausted: false,
    };
    assert!(
        session
            .list_viewport(window(), node(0), handler(1), 1, viewport.clone())
            .is_some()
    );
    let mut next = config();
    next.query_generation = 1;
    session
        .apply(&tx(
            1,
            vec![
                Op::SetTable(node(0), next),
                Op::Bind(node(0), Some(handler(2))),
            ],
        ))
        .unwrap();
    assert!(
        session
            .list_viewport(window(), node(0), handler(1), 1, viewport.clone())
            .is_none()
    );
    assert!(
        session
            .list_viewport(window(), node(0), handler(2), 2, viewport)
            .is_some()
    );
    let before = session.tree(window()).unwrap().retained_bytes();
    session
        .apply(&tx(
            2,
            vec![
                Op::SetListRows(node(0), vec![]),
                Op::Splice(node(0), 0, 1, vec![]),
                Op::Remove(node(3)),
                Op::Remove(node(2)),
                Op::Remove(node(1)),
            ],
        ))
        .unwrap();
    let tree = session.tree(window()).unwrap();
    assert_eq!(tree.len(), 1);
    assert!(tree.retained_bytes() < before);
    // Eviction is not data deletion; the logical 100k rows remain navigable.
    assert_eq!(
        tree.get(node(0))
            .unwrap()
            .list_index
            .as_ref()
            .unwrap()
            .len(),
        100_000
    );
    assert!(
        session
            .table_input(
                window(),
                node(0),
                handler(2),
                3,
                Input {
                    schema_revision: 1,
                    query_generation: 1,
                    request: Request::Select(Selection::Row(1)),
                }
            )
            .is_some()
    );
    session
        .apply(&tx(
            3,
            vec![Op::SetListOrder(
                node(0),
                list::Order {
                    revision: 2,
                    runs: vec![],
                },
            )],
        ))
        .unwrap();
    assert!(
        session
            .table_input(
                window(),
                node(0),
                handler(2),
                3,
                Input {
                    schema_revision: 1,
                    query_generation: 1,
                    request: Request::Select(Selection::Row(1)),
                }
            )
            .is_none()
    );
}
