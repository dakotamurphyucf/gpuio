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

#[test]
fn horizontal_list_metadata_cannot_reinterpret_a_managed_table() {
    let mut tree = Tree::new(window());
    tree.apply(&tx(0, initial())).unwrap();
    let bytes = tree.retained_bytes();
    assert_eq!(
        tree.apply(&tx(
            1,
            vec![
                Op::SetText(node(3), "must roll back".into()),
                Op::SetListAxis(node(0), list::Axis::Horizontal),
            ]
        )),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(tree.revision(), 1);
    assert_eq!(tree.retained_bytes(), bytes);
    assert_eq!(tree.get(node(3)).unwrap().text.as_ref(), "display");
    assert_eq!(tree.get(node(0)).unwrap().list_axis, list::Axis::Vertical);
}

#[test]
fn behavior_policy_is_atomic_charged_and_fences_stale_input() {
    let restricted = Behavior {
        row_header: false,
        boundary: Boundary::Stop,
        selectable_headers: Some(vec![]),
    };
    for reverse in [false, true] {
        let mut session = session();
        let before = session.tree(window()).unwrap().retained_bytes();
        let behavior_op = Op::SetTableBehavior(node(0), Some(restricted.clone()));
        assert!(
            session.apply(&tx(1, vec![behavior_op.clone()])).is_err(),
            "policy requires fresh schema revision"
        );
        let mut next = config();
        next.schema_revision = 2;
        let mut operations = vec![Op::SetTable(node(0), next.clone()), behavior_op];
        if reverse {
            operations.reverse();
        }
        session.apply(&tx(1, operations)).unwrap();
        assert!(session.tree(window()).unwrap().retained_bytes() > before);
        let send = |revision, request| {
            session.table_input(
                window(),
                node(0),
                handler(1),
                1,
                Input {
                    schema_revision: revision,
                    query_generation: 0,
                    request,
                },
            )
        };
        assert!(send(1, Request::Select(Selection::Row(1))).is_none());
        for request in [
            Request::Select(Selection::Column("value".into())),
            Request::Copy(Selection::Column("value".into())),
            Request::Context(Selection::Column("value".into())),
        ] {
            assert!(send(2, request).is_none());
        }
        for request in [
            Request::Select(Selection::Cell(1, "value".into())),
            Request::Sort("value".into(), Some(Direction::Ascending)),
            Request::Select(Selection::Row(1)),
        ] {
            assert!(send(2, request).is_some());
        }
        assert!(
            session
                .apply(&tx(
                    2,
                    vec![command(
                        1,
                        Target::SetSelection(Selection::Column("value".into()))
                    )]
                ))
                .is_err()
        );
        assert!(
            session
                .apply(&tx(2, vec![Op::SetTableBehavior(node(0), None)]))
                .is_err()
        );
        next.schema_revision = 3;
        session
            .apply(&tx(
                2,
                vec![
                    Op::SetTableBehavior(node(0), None),
                    Op::SetTable(node(0), next),
                    command(1, Target::SetSelection(Selection::Column("value".into()))),
                ],
            ))
            .unwrap();
        assert_eq!(session.tree(window()).unwrap().retained_bytes(), before);
    }
}

#[test]
fn behavior_rejects_orphans_unknown_headers_and_budget_overflow_without_mutation() {
    let mut tree = Tree::new(window());
    tree.apply(&tx(0, initial())).unwrap();
    let baseline = tree.retained_bytes();
    let mut next = config();
    next.schema_revision = 2;
    for (owner, headers) in [
        (node(0), vec!["missing".into()]),
        (node(0), vec!["value".into(), "value".into()]),
        (node(1), vec![]),
    ] {
        assert!(
            tree.apply(&tx(
                1,
                vec![
                    Op::SetText(node(3), "rollback".into()),
                    Op::SetTable(node(0), next.clone()),
                    Op::SetTableBehavior(
                        owner,
                        Some(Behavior {
                            selectable_headers: Some(headers),
                            ..Behavior::default()
                        })
                    )
                ]
            ))
            .is_err()
        );
        assert_eq!(tree.revision(), 1);
        assert_eq!(tree.get(node(3)).unwrap().text.as_ref(), "display");
    }
    let operations = vec![
        Op::SetTable(node(0), next),
        Op::SetTableBehavior(
            node(0),
            Some(Behavior {
                selectable_headers: Some(vec!["value".into()]),
                ..Behavior::default()
            }),
        ),
    ];
    assert_eq!(
        tree.apply_with_budget(&tx(1, operations.clone()), baseline),
        Err(ErrorCode::LimitExceeded)
    );
    assert_eq!(tree.retained_bytes(), baseline);
    tree.apply(&tx(1, operations)).unwrap();
    let mut orphan = Tree::new(window());
    assert!(
        orphan
            .apply(&tx(
                0,
                vec![
                    Op::Create(node(0), Kind::VirtualList, String::new(), Some(handler(1))),
                    Op::SetTableBehavior(node(0), Some(Behavior::default()))
                ]
            ))
            .is_err()
    );
}

#[test]
fn appearance_separates_paint_from_geometry_and_checks_quota_and_membership() {
    let mut tree = Tree::new(window());
    tree.apply(&tx(0, initial())).unwrap();
    let baseline = tree.retained_bytes();
    let paint = Appearance {
        striped: true,
        colors: vec![(Part::HeaderBackground, 0xff000080)],
        ..Appearance::default()
    };
    let op = Op::SetTableAppearance(node(0), Some(paint.clone()));
    assert_eq!(
        tree.apply_with_budget(&tx(1, vec![op.clone()]), baseline),
        Err(ErrorCode::LimitExceeded)
    );
    tree.apply(&tx(1, vec![op])).unwrap();
    assert_eq!(
        tree.get(node(0))
            .unwrap()
            .table
            .as_ref()
            .unwrap()
            .schema_revision,
        1
    );
    let padding = Padding {
        top: 1.,
        right: 2.,
        bottom: 3.,
        left: 4.,
    };
    let padded = Appearance {
        padding: Some(padding),
        ..paint
    };
    assert!(
        tree.apply(&tx(
            2,
            vec![Op::SetTableAppearance(node(0), Some(padded.clone()))]
        ))
        .is_err()
    );
    let mut next = config();
    next.schema_revision = 2;
    tree.apply(&tx(
        2,
        vec![
            Op::SetTableAppearance(node(0), Some(padded.clone())),
            Op::SetTable(node(0), next.clone()),
        ],
    ))
    .unwrap();
    next.schema_revision = 3;
    assert!(
        tree.apply(&tx(
            3,
            vec![
                Op::SetTable(node(0), next.clone()),
                Op::SetTableAppearance(
                    node(0),
                    Some(Appearance {
                        column_padding: vec![("missing".into(), padding)],
                        ..padded
                    })
                )
            ]
        ))
        .is_err()
    );
    assert_eq!(tree.revision(), 3);
    assert!(
        tree.apply(&tx(3, vec![Op::SetTableAppearance(node(0), None)]))
            .is_err()
    );
    tree.apply(&tx(
        3,
        vec![
            Op::SetTableAppearance(node(0), None),
            Op::SetTable(node(0), next),
        ],
    ))
    .unwrap();
    assert_eq!(tree.retained_bytes(), baseline);
    assert!(
        tree.apply(&tx(
            4,
            vec![Op::SetTableAppearance(node(1), Some(Appearance::default()))]
        ))
        .is_err()
    );
}

#[test]
fn column_observation_admission_checks_current_owner_schema_and_pin_identity() {
    let mut session = session();
    let viewport = ColumnViewport {
        schema_revision: 1,
        query_generation: 0,
        columns: vec![("value".into(), Pin::Unpinned, false)],
    };
    let send = |s: &Session, h, rev, v| s.table_columns_observed(window(), node(0), h, rev, v);
    assert!(send(&session, handler(1), 1, viewport.clone()).is_some());
    assert!(send(&session, handler(2), 1, viewport.clone()).is_none());
    assert!(send(&session, handler(1), 2, viewport.clone()).is_none());
    for columns in [
        vec![("missing".into(), Pin::Unpinned, true)],
        vec![("value".into(), Pin::Left, true)],
    ] {
        assert!(
            send(
                &session,
                handler(1),
                1,
                ColumnViewport {
                    columns,
                    ..viewport.clone()
                }
            )
            .is_none()
        );
    }
    let mut next = config();
    next.schema_revision = 2;
    next.schema.columns[0].width = 200.;
    session
        .apply(&tx(1, vec![Op::SetTable(node(0), next)]))
        .unwrap();
    assert!(send(&session, handler(1), 1, viewport.clone()).is_none());
    assert!(
        send(
            &session,
            handler(1),
            2,
            ColumnViewport {
                schema_revision: 2,
                columns: vec![],
                ..viewport
            }
        )
        .is_some()
    );
}

fn header_ops(slot: i64) -> Vec<Op> {
    vec![
        Op::Create(node(slot), Kind::Container, String::new(), None),
        Op::SetTableHeader(
            node(slot),
            Some(gpuio_protocol::table_header::Target::Column("value".into())),
        ),
        Op::Create(node(slot + 1), Kind::Text, "Rich header".into(), None),
        Op::Splice(node(slot), 0, 0, vec![node(slot + 1)]),
        Op::Splice(node(0), 0, 0, vec![node(slot)]),
    ]
}

#[test]
fn header_ownership_budgets_and_mapping_fences_are_atomic() {
    let mut tree = Tree::new(window());
    tree.apply(&tx(0, initial())).unwrap();
    let baseline = tree.retained_bytes();
    let mut next = config();
    next.schema_revision = 2;
    let mut add = header_ops(4);
    assert_eq!(tree.apply(&tx(1, add.clone())), Err(ErrorCode::InvalidTree));
    assert_eq!(tree.len(), 4);
    add.push(Op::SetTable(node(0), next.clone()));
    assert_eq!(
        tree.apply_with_budget(&tx(1, add.clone()), baseline),
        Err(ErrorCode::LimitExceeded)
    );
    assert_eq!(tree.retained_bytes(), baseline);
    tree.apply(&tx(1, add)).unwrap();
    assert_eq!(tree.len(), 6);
    assert_eq!(tree.get(node(0)).unwrap().list_rows.len(), 1);
    let retained = tree.retained_bytes();
    // Child-only mutations must revalidate unchanged ancestor ownership.
    for bad in [
        vec![Op::SetTableHeader(node(4), None)],
        vec![Op::SetTableHeader(
            node(4),
            Some(gpuio_protocol::table_header::Target::Column(
                "missing".into(),
            )),
        )],
        vec![Op::SetStyle(node(4), vec![Style::Width(Length::Px(10.))])],
        vec![Op::Bind(node(4), Some(handler(2)))],
        vec![Op::SetListRows(
            node(0),
            vec![list::Row {
                id: 1,
                node: node(4),
            }],
        )],
        header_ops(6),
    ] {
        assert_eq!(tree.apply(&tx(2, bad)), Err(ErrorCode::InvalidTree));
        assert_eq!(tree.revision(), 2);
        assert_eq!(tree.len(), 6);
        assert_eq!(tree.retained_bytes(), retained);
    }
    tree.apply(&tx(2, vec![Op::SetText(node(5), "Updated".into())]))
        .unwrap();
    let mut remove = vec![
        Op::Splice(node(0), 0, 1, vec![]),
        Op::Remove(node(5)),
        Op::Remove(node(4)),
    ];
    assert_eq!(
        tree.apply(&tx(3, remove.clone())),
        Err(ErrorCode::InvalidTree)
    );
    next.schema_revision = 3;
    remove.push(Op::SetTable(node(0), next));
    tree.apply(&tx(3, remove)).unwrap();
    assert_eq!(tree.retained_bytes(), baseline);
}

#[test]
fn header_marker_cannot_turn_an_ordinary_list_row_into_table_content() {
    let mut tree = Tree::new(window());
    let mut operations = initial();
    operations.retain(|op| !matches!(op, Op::SetTable(..) | Op::SetTableCell(..)));
    operations.extend(header_ops(4));
    assert_eq!(tree.apply(&tx(0, operations)), Err(ErrorCode::InvalidTree));
    assert_eq!(tree.len(), 0);
    assert_eq!(tree.revision(), 0);
}

#[test]
fn scoped_header_and_row_styles_are_owned_charged_and_reversible_without_schema_change() {
    let mut tree = Tree::new(window());
    tree.apply(&tx(0, initial())).unwrap();
    let baseline = tree.retained_bytes();
    let style = vec![Style::Fields(vec![Field::Foreground(Color::Rgba(
        0xff0000ff,
    ))])];
    let update = tx(
        1,
        vec![
            Op::SetTableHeaderStyle(node(0), style.clone()),
            Op::SetTableRowStyle(node(1), style.clone()),
        ],
    );
    assert_eq!(
        tree.apply_with_budget(&update, baseline),
        Err(ErrorCode::LimitExceeded)
    );
    assert_eq!(tree.retained_bytes(), baseline);
    tree.apply(&update).unwrap();
    assert!(tree.retained_bytes() > baseline);
    assert_eq!(
        tree.get(node(0))
            .unwrap()
            .table
            .as_ref()
            .unwrap()
            .schema_revision,
        1
    );
    for operation in [
        Op::SetTableRowStyle(node(2), style.clone()),
        Op::SetTableRowStyle(node(0), style.clone()),
        Op::SetTableHeaderStyle(node(1), style.clone()),
        Op::SetTableRowStyle(
            node(1),
            vec![Style::State(2, vec![Field::Height(Length::Px(90.))])],
        ),
        Op::SetTableHeaderStyle(
            node(0),
            vec![Style::State(7, vec![Field::Foreground(Color::Rgba(1))])],
        ),
        Op::SetTableRowStyle(
            node(1),
            vec![Style::Fields(vec![Field::FontSize(f64::NAN)])],
        ),
    ] {
        assert!(tree.apply(&tx(2, vec![operation])).is_err());
        assert_eq!(tree.revision(), 2);
    }
    tree.apply(&tx(
        2,
        vec![
            Op::SetTableHeaderStyle(node(0), vec![]),
            Op::SetTableRowStyle(node(1), vec![]),
        ],
    ))
    .unwrap();
    assert_eq!(tree.retained_bytes(), baseline);
}

fn compact_initial() -> Vec<Op> {
    let mut operations = initial();
    operations.retain(|op| {
        !matches!(op,
        Op::Create(id, ..) if *id == node(2) || *id == node(3))
    });
    operations.retain(|op| match op {
        Op::SetTableCell(..) => false,
        Op::Splice(id, ..) => *id != node(2),
        _ => true,
    });
    operations.insert(
        5,
        Op::CreateTableText(
            node(2),
            Cell {
                column: "value".into(),
                copy_text: "日本語👨‍👩‍👧‍👦".into(),
            },
        ),
    );
    operations
}

#[test]
fn compact_text_cells_preserve_ownership_atomicity_and_payload_limits() {
    let mut tree = Tree::new(window());
    tree.apply(&tx(0, compact_initial())).unwrap();
    assert_eq!(tree.len(), 3);
    let baseline = tree.retained_bytes();
    let before = tree.get(node(2)).unwrap().text.clone();
    let copy = "x".repeat(MAX_COPY_BYTES);
    let update = tx(
        1,
        vec![Op::SetTableText(
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
    assert_eq!(tree.get(node(2)).unwrap().text, before);
    assert_eq!(tree.retained_bytes(), baseline);
    tree.apply(&update).unwrap();
    assert_eq!(
        tree.retained_bytes() - baseline,
        2 * (copy.len() - before.len())
    );
    assert_eq!(&*tree.get(node(2)).unwrap().text, copy.as_str());
    assert_eq!(
        tree.get(node(2))
            .unwrap()
            .table_cell
            .as_ref()
            .unwrap()
            .copy_text,
        copy
    );
    for invalid in [
        Op::SetTableText(
            node(2),
            Cell {
                column: "other".into(),
                copy_text: "wrong".into(),
            },
        ),
        Op::SetTableText(
            node(1),
            Cell {
                column: "value".into(),
                copy_text: "row".into(),
            },
        ),
        Op::SetTableText(
            node(2),
            Cell {
                column: "value".into(),
                copy_text: "\0".into(),
            },
        ),
        Op::SetText(node(2), "display/copy mismatch".into()),
        Op::Bind(node(2), Some(handler(2))),
        Op::Splice(node(1), 0, 1, vec![]),
        Op::CreateTableText(
            node(2),
            Cell {
                column: "value".into(),
                copy_text: "duplicate".into(),
            },
        ),
    ] {
        assert!(tree.apply(&tx(2, vec![invalid])).is_err());
        assert_eq!(tree.revision(), 2);
        assert_eq!(&*tree.get(node(2)).unwrap().text, copy.as_str());
    }
    let mut orphan = Tree::new(window());
    assert!(
        orphan
            .apply(&tx(
                0,
                vec![
                    Op::CreateTableText(
                        node(0),
                        Cell {
                            column: "value".into(),
                            copy_text: "orphan".into()
                        }
                    ),
                    Op::SetRoot(Some(node(0))),
                ]
            ))
            .is_err()
    );
    assert_eq!(orphan.len(), 0);
}
