use gpuio_native::{session::Session, tree::Tree};
use gpuio_protocol::{HandlerId, NodeId, WindowId, link::Config, v1::*};

fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn handler() -> HandlerId {
    HandlerId::from_parts(0, 1).unwrap()
}
fn config(disabled: bool) -> Config {
    Config {
        label: "Guide 世界".into(),
        disabled,
        tab_stop: true,
        tab_index: -2,
    }
}
fn tx(tree: &Tree, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: window(),
        base: tree.revision(),
        revision: tree.revision() + 1,
        operations,
    }
}
fn mount() -> Vec<Op> {
    vec![
        Op::Create(node(0), Kind::Link, String::new(), Some(handler())),
        Op::SetLink(node(0), config(false)),
        Op::Create(node(1), Kind::Container, String::new(), None),
        Op::Create(node(2), Kind::Text, "Read the guide".into(), None),
        Op::Splice(node(0), 0, 0, vec![node(1)]),
        Op::Splice(node(1), 0, 0, vec![node(2)]),
        Op::SetRoot(Some(node(0))),
    ]
}
fn initial() -> Tree {
    let mut tree = Tree::new(window());
    tree.apply(&tx(&tree, mount())).unwrap();
    tree
}

#[test]
fn passive_content_limits_include_deep_and_wide_ancestor_updates() {
    for (deep, count, valid) in [
        (true, 128, true),
        (true, 129, false),
        (false, 4096, true),
        (false, 4097, false),
    ] {
        let mut tree = Tree::new(window());
        let mut operations = vec![
            Op::Create(node(0), Kind::Link, String::new(), Some(handler())),
            Op::SetLink(node(0), config(false)),
            Op::SetRoot(Some(node(0))),
        ];
        let start = if deep {
            1
        } else {
            for slot in 1..=2048 {
                operations.push(Op::Create(node(slot), Kind::Container, String::new(), None));
            }
            operations.push(Op::Splice(node(0), 0, 0, (1..=2048).map(node).collect()));
            tree.apply(&tx(&tree, operations)).unwrap();
            operations = vec![];
            2049
        };
        for slot in start..=count {
            operations.push(Op::Create(node(slot), Kind::Container, String::new(), None));
        }
        if deep {
            for slot in 1..=count {
                operations.push(Op::Splice(node(slot - 1), 0, 0, vec![node(slot)]));
            }
        } else {
            operations.push(Op::Splice(
                node(0),
                2048,
                0,
                (start..=count).map(node).collect(),
            ));
        }
        let revision = tree.revision();
        let retained = tree.retained_bytes();
        let result = tree.apply(&tx(&tree, operations));
        // The global tree depth guard runs first; link width has its own quota.
        let error = if deep {
            ErrorCode::InvalidTree
        } else {
            ErrorCode::LimitExceeded
        };
        if valid {
            assert!(result.is_ok(), "{deep}/{count}: {result:?}");
            assert_eq!(
                tree.apply(&tx(
                    &tree,
                    vec![
                        Op::Create(node(count + 1), Kind::Text, "one too many".into(), None),
                        Op::Splice(node(count), 0, 0, vec![node(count + 1)]),
                    ]
                )),
                Err(error)
            );
            assert!(tree.get(node(count + 1)).is_none());
        } else {
            assert_eq!(result, Err(error));
            assert_eq!(tree.revision(), revision);
            assert_eq!(tree.retained_bytes(), retained);
        }
    }
}

#[test]
fn mutations_revalidate_link_ancestors_and_roll_back_atomically() {
    let mut tree = initial();
    let retained = tree.retained_bytes();
    let original = tree.get(node(2)).unwrap().clone();
    let revision = tree.revision();
    let mut invalid_ops = vec![
        Op::Bind(node(2), Some(handler())),
        Op::Bind(node(0), None),
        Op::SetText(node(0), "not a content leaf".into()),
        Op::SetLink(node(2), config(false)),
        Op::SetLink(
            node(0),
            Config {
                label: String::new(),
                ..config(false)
            },
        ),
    ];
    for field in [
        Field::UserSelect(true),
        Field::Inert(true),
        Field::OverflowX(3),
        Field::OverflowY(3),
        Field::PointerOcclusion(1),
        Field::PointerOcclusion(2),
    ] {
        invalid_ops.push(Op::SetStyle(
            node(1),
            vec![Style::Fields(vec![field.clone()])],
        ));
        if matches!(field, Field::OverflowX(_) | Field::OverflowY(_)) {
            invalid_ops.push(Op::SetStyle(node(1), vec![Style::State(1, vec![field])]));
        }
    }
    for invalid in invalid_ops {
        assert_eq!(
            tree.apply(&tx(
                &tree,
                vec![Op::SetText(node(2), "rollback".into()), invalid]
            )),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(tree.get(node(2)).unwrap(), &original);
        assert_eq!(tree.revision(), revision);
        assert_eq!(tree.retained_bytes(), retained);
    }
    // Mutation through a deep child must also reject a newly nested control.
    assert_eq!(
        tree.apply(&tx(
            &tree,
            vec![
                Op::Create(node(3), Kind::Button, "nested".into(), Some(handler())),
                Op::Splice(node(1), 1, 0, vec![node(3)]),
            ]
        )),
        Err(ErrorCode::InvalidTree)
    );
    assert!(tree.get(node(3)).is_none());
    tree.apply(&tx(
        &tree,
        vec![Op::SetStyle(
            node(1),
            vec![Style::Fields(vec![
                Field::UserSelect(false),
                Field::OverflowX(2),
                Field::OverflowY(1),
            ])],
        )],
    ))
    .unwrap();
}

#[test]
fn config_budget_and_generation_are_admitted_transactionally() {
    let mut tree = initial();
    let retained = tree.retained_bytes();
    let larger = Config {
        label: "x".repeat(4096),
        ..config(false)
    };
    let expected = retained + larger.label.len() - config(false).label.len();
    let update = tx(&tree, vec![Op::SetLink(node(0), larger.clone())]);
    assert_eq!(
        tree.apply_with_budget(&update, expected - 1),
        Err(ErrorCode::LimitExceeded)
    );
    assert_eq!(tree.retained_bytes(), retained);
    tree.apply_with_budget(&update, expected).unwrap();
    assert_eq!(tree.get(node(0)).unwrap().link.as_deref(), Some(&larger));
    assert_eq!(tree.retained_bytes(), expected);
    tree.apply(&tx(
        &tree,
        vec![
            Op::SetRoot(None),
            Op::Remove(node(2)),
            Op::Remove(node(1)),
            Op::Remove(node(0)),
        ],
    ))
    .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
    let next = NodeId::from_parts(0, 2).unwrap();
    tree.apply(&tx(
        &tree,
        vec![
            Op::Create(next, Kind::Link, String::new(), None),
            Op::SetLink(next, config(true)),
            Op::SetRoot(Some(next)),
        ],
    ))
    .unwrap();
    assert_eq!(
        tree.apply(&tx(&tree, vec![Op::SetLink(node(0), config(false))])),
        Err(ErrorCode::StaleHandle)
    );
    assert!(tree.get(next).unwrap().link.as_ref().unwrap().disabled);
}

#[test]
fn disabled_and_retired_links_reject_queued_events_even_with_retained_handler() {
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window(), "link", 400., 200.).unwrap();
    session
        .apply(&tx(session.tree(window()).unwrap(), mount()))
        .unwrap();
    assert!(session.press(window(), node(0), handler(), 1).is_some());
    let update = tx(
        session.tree(window()).unwrap(),
        vec![Op::SetLink(node(0), config(true))],
    );
    session.apply(&update).unwrap();
    assert!(session.press(window(), node(0), handler(), 1).is_none());
    let next_handler = HandlerId::from_parts(0, 2).unwrap();
    let update = tx(
        session.tree(window()).unwrap(),
        vec![
            Op::SetLink(node(0), config(false)),
            Op::Bind(node(0), Some(next_handler)),
        ],
    );
    session.apply(&update).unwrap();
    assert!(session.press(window(), node(0), handler(), 1).is_none());
    assert!(session.press(window(), node(0), next_handler, 3).is_some());
    assert!(session.press(window(), node(0), next_handler, 4).is_none());
    session.close(window()).unwrap();
    assert!(session.press(window(), node(0), next_handler, 3).is_none());
    assert_eq!(session.retained_bytes(), 0);
}
