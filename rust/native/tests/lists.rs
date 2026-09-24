use gpuio_native::tree::{ListAction, Tree};
use gpuio_protocol::{NodeId, WindowId, list::*, v1::*};
use std::sync::Arc;

fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn tx(tree: &Tree, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: window(),
        base: tree.revision(),
        revision: tree.revision() + 1,
        operations,
    }
}
fn config() -> Config {
    Config {
        estimated_height: 100.,
        overscan: 200.,
        max_active: 32,
        scroll_policy: ScrollPolicy::FollowTailWhenAtEnd,
        scrollbar: true,
        managed: true,
    }
}
fn order(revision: i64, count: i64) -> Order {
    Order {
        revision,
        runs: vec![IdRun { first: 1, count }],
    }
}
fn initial(count: i64) -> Vec<Op> {
    vec![
        Op::Create(node(0), Kind::VirtualList, String::new(), None),
        Op::Create(node(1), Kind::Text, "one mounted row".into(), None),
        Op::SetListConfig(node(0), config()),
        Op::SetListOrder(node(0), order(1, count)),
        Op::SetListRows(
            node(0),
            vec![Row {
                id: 1,
                node: node(1),
            }],
        ),
        Op::Splice(node(0), 0, 0, vec![node(1)]),
        Op::SetRoot(Some(node(0))),
    ]
}

#[test]
fn logical_metadata_has_independent_ownership_and_is_charged_to_the_budget() {
    let mut tree = Tree::new(window());
    tree.apply(&tx(&tree, initial(100_000))).unwrap();
    assert_eq!(tree.len(), 2);
    assert_eq!(
        tree.get(node(0))
            .unwrap()
            .list_index
            .as_ref()
            .unwrap()
            .len(),
        100_000
    );
    assert!(tree.retained_bytes() >= 100_000 * 192);
    let index = tree.get(node(0)).unwrap().list_index.clone().unwrap();
    tree.apply(&tx(
        &tree,
        vec![
            Op::SetText(node(1), "streamed".into()),
            Op::InvalidateListRows(node(0), vec![1]),
        ],
    ))
    .unwrap();
    assert!(Arc::ptr_eq(
        &index,
        tree.get(node(0)).unwrap().list_index.as_ref().unwrap()
    ));
    let mut constrained = Tree::new(window());
    let rejected =
        constrained.apply_with_budget(&tx(&constrained, initial(100_000)), 100_000 * 192 - 1);
    assert_eq!(rejected, Err(ErrorCode::LimitExceeded));
    assert_eq!(constrained.len(), 0);
    assert_eq!(constrained.retained_bytes(), 0);
    assert_eq!(
        constrained.apply(&tx(&constrained, initial(1_000_000))),
        Err(ErrorCode::LimitExceeded)
    );
}

#[test]
fn malformed_mappings_and_revisions_roll_back_atomically() {
    let mut tree = Tree::new(window());
    tree.apply(&tx(&tree, initial(10))).unwrap();
    for invalid in [
        Op::SetListRows(
            node(0),
            vec![Row {
                id: 11,
                node: node(1),
            }],
        ),
        Op::SetListRows(
            node(0),
            vec![
                Row {
                    id: 1,
                    node: node(1),
                },
                Row {
                    id: 1,
                    node: node(1),
                },
            ],
        ),
        Op::SetListRows(node(0), vec![]),
        Op::SetListOrder(node(0), order(1, 11)),
        Op::SetListOrder(
            node(0),
            Order {
                revision: 2,
                runs: vec![IdRun {
                    first: i64::MAX,
                    count: 2,
                }],
            },
        ),
        Op::ScrollList(
            node(0),
            ScrollRequest {
                serial: 1,
                target: ScrollTarget::Offset(1, f64::NAN),
            },
        ),
        Op::InvalidateListRows(node(0), vec![11]),
        Op::ScrollList(
            node(1),
            ScrollRequest {
                serial: 1,
                target: ScrollTarget::End,
            },
        ),
    ] {
        let revision = tree.revision();
        let index = tree.get(node(0)).unwrap().list_index.clone().unwrap();
        assert!(
            tree.apply(&tx(
                &tree,
                vec![Op::SetText(node(1), "must roll back".into()), invalid]
            ))
            .is_err()
        );
        assert_eq!(tree.revision(), revision);
        assert_eq!(tree.get(node(1)).unwrap().text.as_ref(), "one mounted row");
        assert!(Arc::ptr_eq(
            &index,
            tree.get(node(0)).unwrap().list_index.as_ref().unwrap()
        ));
    }
}

#[test]
fn commands_validate_against_final_data_and_disappear_after_the_transaction() {
    let mut tree = Tree::new(window());
    tree.apply(&tx(&tree, initial(10))).unwrap();
    let request = ScrollRequest {
        serial: 1,
        target: ScrollTarget::Reveal(11),
    };
    let applied = tree
        .apply(&tx(
            &tree,
            vec![
                Op::ScrollList(node(0), request),
                Op::InvalidateListRows(node(0), vec![11]),
                Op::SetListOrder(node(0), order(2, 11)),
            ],
        ))
        .unwrap();
    assert_eq!(
        applied.lists,
        vec![
            ListAction::Scroll(node(0), request),
            ListAction::Invalidate(node(0), vec![11])
        ]
    );
    let applied = tree
        .apply(&tx(&tree, vec![Op::SetText(node(1), "later".into())]))
        .unwrap();
    assert!(applied.lists.is_empty());
}

#[test]
fn simple_lists_require_all_rows_and_managed_lists_enforce_their_active_budget() {
    let mut tree = Tree::new(window());
    tree.apply(&tx(&tree, initial(10))).unwrap();
    let mut simple = config();
    simple.managed = false;
    assert_eq!(
        tree.apply(&tx(&tree, vec![Op::SetListConfig(node(0), simple.clone())])),
        Err(ErrorCode::InvalidTree)
    );
    tree.apply(&tx(
        &tree,
        vec![
            Op::SetListOrder(node(0), order(2, 1)),
            Op::SetListConfig(node(0), simple),
        ],
    ))
    .unwrap();
    let mut managed = config();
    managed.max_active = 1;
    assert_eq!(
        tree.apply(&tx(
            &tree,
            vec![
                Op::SetListConfig(node(0), managed),
                Op::SetListOrder(node(0), order(3, 2)),
                Op::Create(node(2), Kind::Text, "two".into(), None),
                Op::Splice(node(0), 1, 0, vec![node(2)]),
                Op::SetListRows(
                    node(0),
                    vec![
                        Row {
                            id: 1,
                            node: node(1)
                        },
                        Row {
                            id: 2,
                            node: node(2)
                        }
                    ]
                ),
            ]
        )),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(tree.len(), 2);
}

#[test]
fn viewport_observations_validate_order_membership_and_coalesce() {
    use gpuio_native::{mailbox::Mailbox, session::Session};
    use gpuio_protocol::HandlerId;
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window(), "list", 600., 400.).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let mut operations = initial(10);
    operations.push(Op::Bind(node(0), Some(handler)));
    session
        .apply(&Transaction {
            window: window(),
            base: 0,
            revision: 1,
            operations,
        })
        .unwrap();
    let viewport = Viewport {
        order_revision: 1,
        visible_first: 0,
        visible_last: 3,
        requested: vec![1, 2, 3],
        pinned: vec![1],
        anchor: Some((1, 4.)),
        following_tail: false,
        at_start: false,
        at_end: false,
        budget_exhausted: false,
    };
    let event = session
        .list_viewport(window(), node(0), handler, 1, viewport.clone())
        .unwrap();
    let mut obsolete = viewport.clone();
    obsolete.order_revision = 2;
    assert!(
        session
            .list_viewport(window(), node(0), handler, 1, obsolete)
            .is_none()
    );
    let mut absent = viewport.clone();
    absent.requested.push(11);
    assert!(
        session
            .list_viewport(window(), node(0), handler, 1, absent)
            .is_none()
    );
    let mut mailbox = Mailbox::default();
    for _ in 0..1000 {
        mailbox.input(event.clone()).unwrap();
    }
    assert_eq!(mailbox.drain(10), vec![event.clone()]);
    mailbox.input(event.clone()).unwrap();
    mailbox
        .input(Event::Press(window(), node(1), handler, 1))
        .unwrap();
    mailbox.input(event).unwrap();
    assert_eq!(mailbox.drain(10).len(), 3);
    session.close(window()).unwrap();
    assert!(
        session
            .list_viewport(window(), node(0), handler, 1, viewport)
            .is_none()
    );
}

#[test]
fn native_pins_veto_stale_eviction_but_not_explicit_data_deletion() {
    use gpuio_native::tree::ApplyFailure;
    let mut tree = Tree::new(window());
    tree.apply(&tx(&tree, initial(10))).unwrap();
    let pins = vec![Retained {
        node: node(0),
        rows: vec![1],
    }];
    let eviction = vec![
        Op::Remove(node(1)),
        Op::Splice(node(0), 0, 1, vec![]),
        Op::SetListRows(node(0), vec![]),
    ];
    let revision = tree.revision();
    let bytes = tree.retained_bytes();
    assert_eq!(
        tree.apply_guarded(&tx(&tree, eviction.clone()), usize::MAX, &pins),
        Err(ApplyFailure::Retained(pins.clone()))
    );
    assert_eq!(tree.revision(), revision);
    assert_eq!(tree.retained_bytes(), bytes);
    assert!(tree.get(node(1)).is_some());
    let mut deletion = eviction;
    deletion.push(Op::SetListOrder(
        node(0),
        Order {
            revision: 2,
            runs: vec![IdRun { first: 2, count: 9 }],
        },
    ));
    tree.apply_guarded(&tx(&tree, deletion), usize::MAX, &pins)
        .unwrap();
    assert!(tree.get(node(1)).is_none());
    let mut other = Tree::new(window());
    other.apply(&tx(&other, initial(10))).unwrap();
    other
        .apply_guarded(
            &tx(
                &other,
                vec![Op::SetRoot(None), Op::Remove(node(1)), Op::Remove(node(0))],
            ),
            usize::MAX,
            &pins,
        )
        .unwrap();
    assert!(other.is_empty());
}

#[test]
fn retention_response_releases_the_single_inflight_reservation() {
    use gpuio_native::mailbox::Mailbox;
    let mut queue = Mailbox::default();
    let tree = Tree::new(window());
    let message = Message::Apply(tx(&tree, initial(10)));
    queue.submit(message.clone(), 100).unwrap();
    queue.pop().unwrap();
    let reply = Event::ListRetained(
        window(),
        1,
        vec![Retained {
            node: node(0),
            rows: vec![1],
        }],
    );
    queue.respond(reply.clone());
    assert_eq!(queue.submit(message.clone(), 100), Err(ErrorCode::Busy));
    assert_eq!(queue.drain(10), vec![reply]);
    queue.submit(message, 100).unwrap();
}
