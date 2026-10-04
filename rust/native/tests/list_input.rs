use gpuio_native::{mailbox::Mailbox, session::Session};
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    accessibility::{Config as Metadata, Live, OptionItem, Role},
    list::{Config as ListConfig, IdRun, Order, Row, ScrollPolicy},
    list_input::{Config, Confirmation, Gesture, Navigation, Request},
    v1::*,
};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn handler() -> HandlerId {
    HandlerId::from_parts(0, 1).unwrap()
}
fn metadata(role: Role) -> Metadata {
    Metadata {
        role: Some(role),
        label: Some("Options".into()),
        description: None,
        live: Live::Off,
        field: None,
        current: None,
    }
}
fn item(disabled: bool) -> Metadata {
    metadata(Role::OptionItem(OptionItem {
        index: 0,
        count: Some(100_000),
        selected: false,
        disabled,
    }))
}
fn config() -> Config {
    Config {
        generation: 1,
        cursor: Some(1),
        query: Some(node(3)),
        selection_on_navigation: false,
        disabled: false,
        busy: false,
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
fn list_config() -> ListConfig {
    ListConfig {
        estimated_height: 24.,
        overscan: 0.,
        max_active: 4,
        scroll_policy: ScrollPolicy::KeepPosition,
        scrollbar: false,
        managed: true,
    }
}
fn session() -> Session {
    let mut s = Session::default();
    s.hello(VERSION, CAPABILITIES).unwrap();
    s.open(1, window(), "List", 400., 300.).unwrap();
    s.apply(&tx(
        0,
        vec![
            Op::Create(node(0), Kind::Container, String::new(), None),
            Op::Create(node(1), Kind::VirtualList, String::new(), Some(handler())),
            Op::SetListConfig(node(1), list_config()),
            Op::SetListOrder(
                node(1),
                Order {
                    revision: 1,
                    runs: vec![IdRun {
                        first: 1,
                        count: 100_000,
                    }],
                },
            ),
            Op::SetAccessibility(node(1), Some(metadata(Role::ListBox(true)))),
            Op::SetListInput(node(1), Some(config())),
            Op::Create(node(2), Kind::Container, String::new(), None),
            Op::SetAccessibility(node(2), Some(item(false))),
            Op::SetListRows(
                node(1),
                vec![Row {
                    id: 1,
                    node: node(2),
                }],
            ),
            Op::Splice(node(1), 0, 0, vec![node(2)]),
            Op::Create(node(3), Kind::Input, String::new(), Some(handler())),
            Op::SetEditor(
                node(3),
                EditorConfig {
                    label: "Search".into(),
                    placeholder: String::new(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: false,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::Create(node(4), Kind::Container, String::new(), None),
            Op::Splice(node(0), 0, 0, vec![node(1), node(3), node(4)]),
            Op::SetRoot(Some(node(0))),
        ],
    ))
    .unwrap();
    s
}
fn event(s: &Session, generation: i64, request: Request) -> Option<Event> {
    s.list_input(window(), node(1), handler(), 1, generation, request)
}
fn rejects(s: &mut Session, operations: Vec<Op>) {
    rejects_with(s, operations, ErrorCode::InvalidTree);
}
fn rejects_with(s: &mut Session, operations: Vec<Op>, error: ErrorCode) {
    let tree = s.tree(window()).unwrap();
    let revision = tree.revision();
    let retained = tree.retained_bytes();
    assert_eq!(s.apply(&tx(revision, operations)), Err(error));
    assert_eq!(s.tree(window()).unwrap().revision(), revision);
    assert_eq!(s.tree(window()).unwrap().retained_bytes(), retained);
}
#[test]
fn admission_is_atomic_and_requires_listbox_handler_order_and_input_exclusivity() {
    let mut s = session();
    for ops in [
        vec![Op::SetAccessibility(node(1), None)],
        vec![Op::SetAccessibility(node(1), Some(metadata(Role::Log)))],
        vec![Op::Bind(node(1), None)],
        vec![Op::SetListInput(node(0), Some(config()))],
        vec![Op::SetTreeInput(node(1), true)],
        vec![Op::SetListInput(
            node(1),
            Some(Config {
                cursor: Some(100_001),
                ..config()
            }),
        )],
        vec![Op::SetListInput(
            node(1),
            Some(Config {
                generation: 0,
                ..config()
            }),
        )],
        vec![Op::SetListInput(
            node(1),
            Some(Config {
                query: Some(node(4)),
                generation: 2,
                ..config()
            }),
        )],
        vec![Op::SetAccessibility(node(2), Some(item(true)))],
        vec![Op::SetAccessibility(node(2), None)],
    ] {
        rejects(&mut s, ops);
    }
    assert!(event(&s, 1, Request::Focus(1)).is_some());
    // An unmounted logical cursor is legal: reveal/materialization follows the accepted model.
    let applied = s
        .apply(&tx(
            1,
            vec![Op::SetListInput(
                node(1),
                Some(Config {
                    cursor: Some(100_000),
                    ..config()
                }),
            )],
        ))
        .unwrap();
    assert!(applied.touched_records <= 2);
    assert_eq!(applied.validated_nodes, 0);
    assert!(event(&s, 1, Request::Focus(100_000)).is_none());
    s.apply(&tx(
        2,
        vec![Op::SetListAxis(
            node(1),
            gpuio_protocol::list::Axis::Horizontal,
        )],
    ))
    .unwrap();
}
#[test]
fn query_links_are_unique_and_removing_or_reparenting_a_query_revalidates_owners() {
    let mut s = session();
    rejects_with(
        &mut s,
        vec![Op::Splice(node(0), 1, 1, vec![]), Op::Remove(node(3))],
        ErrorCode::StaleHandle,
    );
    rejects(
        &mut s,
        vec![
            Op::Splice(node(0), 1, 1, vec![]),
            Op::Splice(node(4), 0, 0, vec![node(3)]),
        ],
    );
    rejects(
        &mut s,
        vec![
            Op::Create(node(5), Kind::VirtualList, String::new(), Some(handler())),
            Op::SetListConfig(node(5), list_config()),
            Op::SetListOrder(
                node(5),
                Order {
                    revision: 1,
                    runs: vec![],
                },
            ),
            Op::SetAccessibility(node(5), Some(metadata(Role::ListBox(false)))),
            Op::SetListInput(
                node(5),
                Some(Config {
                    cursor: None,
                    ..config()
                }),
            ),
            Op::Splice(node(0), 3, 0, vec![node(5)]),
        ],
    );
    // Detach the link in the same transaction before moving the editor.
    s.apply(&tx(
        1,
        vec![
            Op::SetListInput(
                node(1),
                Some(Config {
                    query: None,
                    generation: 2,
                    ..config()
                }),
            ),
            Op::Splice(node(0), 1, 1, vec![]),
            Op::Splice(node(4), 0, 0, vec![node(3)]),
        ],
    ))
    .unwrap();
    assert!(event(&s, 1, Request::Cancel).is_none());
    assert!(event(&s, 2, Request::Cancel).is_some());
}
#[test]
fn generation_watermark_survives_clear_and_busy_does_not_retire_queued_navigation() {
    let mut s = session();
    for changed in [
        Config {
            query: None,
            ..config()
        },
        Config {
            disabled: true,
            ..config()
        },
        Config {
            selection_on_navigation: true,
            ..config()
        },
    ] {
        rejects(&mut s, vec![Op::SetListInput(node(1), Some(changed))]);
    }
    s.apply(&tx(
        1,
        vec![Op::SetListInput(
            node(1),
            Some(Config {
                busy: true,
                cursor: None,
                ..config()
            }),
        )],
    ))
    .unwrap();
    assert!(event(&s, 1, Request::Navigate(Navigation::Next, None)).is_some());
    s.apply(&tx(
        2,
        vec![Op::SetListInput(
            node(1),
            Some(Config {
                generation: 2,
                disabled: true,
                ..config()
            }),
        )],
    ))
    .unwrap();
    assert!(event(&s, 2, Request::Cancel).is_none());
    s.apply(&tx(3, vec![Op::SetListInput(node(1), None)]))
        .unwrap();
    assert!(event(&s, 2, Request::Cancel).is_none());
    rejects(
        &mut s,
        vec![Op::SetListInput(
            node(1),
            Some(Config {
                generation: 2,
                ..config()
            }),
        )],
    );
    s.apply(&tx(
        4,
        vec![Op::SetListInput(
            node(1),
            Some(Config {
                generation: 3,
                ..config()
            }),
        )],
    ))
    .unwrap();
    assert!(event(&s, 1, Request::Cancel).is_none());
    assert!(event(&s, 3, Request::Cancel).is_some());
}
#[test]
fn events_fence_eligible_mounted_rows_handler_revision_and_window_lifetime() {
    let mut s = session();
    for request in [
        Request::Focus(1),
        Request::Select(1, Gesture::Toggle),
        Request::SetSelected(1, false),
        Request::Confirm(1, Confirmation::Secondary),
        Request::Context(1),
    ] {
        assert!(event(&s, 1, request).is_some());
    }
    assert!(event(&s, 1, Request::Focus(0)).is_none());
    assert!(event(&s, 1, Request::Focus(2)).is_none());
    for revision in [-1, 2] {
        assert!(
            s.list_input(window(), node(1), handler(), revision, 1, Request::Cancel)
                .is_none()
        );
    }
    assert!(
        s.list_input(
            window(),
            node(1),
            HandlerId::from_parts(0, 2).unwrap(),
            1,
            1,
            Request::Cancel
        )
        .is_none()
    );
    s.apply(&tx(
        1,
        vec![
            Op::SetListInput(
                node(1),
                Some(Config {
                    cursor: None,
                    ..config()
                }),
            ),
            Op::SetAccessibility(node(2), Some(item(true))),
        ],
    ))
    .unwrap();
    for request in [
        Request::Focus(1),
        Request::Select(1, Gesture::Replace),
        Request::SetSelected(1, true),
        Request::Context(1),
    ] {
        assert!(event(&s, 1, request).is_none());
    }
    assert!(event(&s, 1, Request::Navigate(Navigation::Next, None)).is_some());
    s.close(window()).unwrap();
    assert!(event(&s, 1, Request::Cancel).is_none());
}
#[test]
fn repeated_relative_inputs_and_distinct_confirmation_kinds_remain_ordered() {
    let s = session();
    let mut mailbox = Mailbox::default();
    let requests = [
        Request::Navigate(Navigation::Next, None),
        Request::Navigate(Navigation::Next, None),
        Request::Navigate(Navigation::Previous, Some(Gesture::Range { extend: true })),
        Request::ConfirmActive(Confirmation::Primary),
        Request::ConfirmActive(Confirmation::Secondary),
        Request::ContextActive,
        Request::Cancel,
    ];
    for request in requests {
        mailbox.input(event(&s, 1, request).unwrap()).unwrap();
    }
    assert!(mailbox.has_window_output(0));
    let output = mailbox
        .drain(128)
        .into_iter()
        .map(|event| match event {
            Event::ListInput(_, _, _, _, _, request) => request,
            _ => panic!("unexpected event"),
        })
        .collect::<Vec<_>>();
    assert_eq!(output, requests);
    assert!(!mailbox.has_window_output(0));
}
#[test]
fn atomic_query_transfer_is_order_independent_and_reset_releases_registration_bytes() {
    let mut s = session();
    s.apply(&tx(
        1,
        vec![
            Op::Create(node(5), Kind::VirtualList, String::new(), Some(handler())),
            Op::SetListConfig(node(5), list_config()),
            Op::SetListOrder(
                node(5),
                Order {
                    revision: 1,
                    runs: vec![],
                },
            ),
            Op::SetAccessibility(node(5), Some(metadata(Role::ListBox(false)))),
            Op::SetListInput(
                node(5),
                Some(Config {
                    query: None,
                    cursor: None,
                    ..config()
                }),
            ),
            Op::Splice(node(0), 3, 0, vec![node(5)]),
        ],
    ))
    .unwrap();
    let bytes = s.tree(window()).unwrap().retained_bytes();
    s.apply(&tx(
        2,
        vec![
            Op::SetListInput(
                node(5),
                Some(Config {
                    generation: 2,
                    cursor: None,
                    ..config()
                }),
            ),
            Op::SetListInput(
                node(1),
                Some(Config {
                    generation: 2,
                    query: None,
                    ..config()
                }),
            ),
        ],
    ))
    .unwrap();
    assert_eq!(s.tree(window()).unwrap().retained_bytes(), bytes);
    s.apply(&tx(
        3,
        vec![
            Op::SetListInput(
                node(1),
                Some(Config {
                    generation: 3,
                    ..config()
                }),
            ),
            Op::SetListInput(node(5), None),
        ],
    ))
    .unwrap();
    assert_eq!(s.tree(window()).unwrap().retained_bytes(), bytes);
    s.apply(&tx(4, vec![Op::SetListInput(node(1), None)]))
        .unwrap();
    assert_eq!(s.tree(window()).unwrap().retained_bytes(), bytes - 128);
    // No stale registration survives input reset; deleting the query is legal.
    s.apply(&tx(
        5,
        vec![Op::Splice(node(0), 1, 1, vec![]), Op::Remove(node(3))],
    ))
    .unwrap();
}
#[test]
fn query_value_updates_touch_only_the_editor_and_owner_without_expanding_logical_rows() {
    let mut s = session();
    let editor = s
        .tree(window())
        .unwrap()
        .get(node(3))
        .unwrap()
        .editor
        .as_ref()
        .unwrap()
        .as_ref()
        .clone();
    let applied = s
        .apply(&tx(
            1,
            vec![Op::SetEditor(
                node(3),
                EditorConfig {
                    placeholder: "Filter".into(),
                    ..editor
                },
            )],
        ))
        .unwrap();
    assert_eq!(applied.validated_nodes, 0);
    assert_eq!(applied.touched_records, 2);
    assert!(applied.dirty.contains(&node(1)));
    assert!(event(&s, 1, Request::Navigate(Navigation::Next, None)).is_some());
}

#[test]
fn exact_public_core_frames_link_late_query_preserve_cursor_epoch_clear_and_dispose() {
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session
        .open(1, window(), "Public list", 400., 300.)
        .unwrap();
    let fixtures = [
        include_str!("../../../test/fixtures/list-input-public-0.hex"),
        include_str!("../../../test/fixtures/list-input-public-1.hex"),
        include_str!("../../../test/fixtures/list-input-public-2.hex"),
        include_str!("../../../test/fixtures/list-input-public-3.hex"),
        include_str!("../../../test/fixtures/list-input-public-4.hex"),
        include_str!("../../../test/fixtures/list-input-public-5.hex"),
    ];
    for (index, hex) in fixtures.into_iter().enumerate() {
        let bytes: Vec<_> = hex
            .trim()
            .as_bytes()
            .chunks_exact(2)
            .map(|b| u8::from_str_radix(std::str::from_utf8(b).unwrap(), 16).unwrap())
            .collect();
        let Message::Apply(tx) = gpuio_protocol::decode(&bytes).unwrap() else {
            panic!("public transaction")
        };
        session.apply(&tx).unwrap();
        let tree = session.tree(window()).unwrap();
        if index == 5 {
            assert!(tree.root().is_none());
            assert!(tree.get(node(1)).is_none());
            assert!(tree.list_query_owner(node(8)).is_none());
            continue;
        }
        let list = tree.get(node(1)).unwrap();
        assert_eq!(list.kind, Kind::VirtualList);
        assert_eq!(list.list_rows.len(), 2);
        if index == 3 {
            assert!(list.list_input.is_none());
            assert_eq!(list.list_input_generation, 2);
            assert!(tree.get(node(8)).is_none());
            assert!(tree.list_query_owner(node(8)).is_none());
        } else {
            let config = list.list_input.unwrap();
            assert_eq!(config.generation, [1, 1, 2, 0, 3][index]);
            assert_eq!(config.cursor, Some(if index == 1 { 2 } else { 1 }));
            assert_eq!(config.busy, index == 1);
            assert_eq!(config.query, (index < 3).then_some(node(8)));
            assert_eq!(
                tree.list_query_owner(node(8)),
                (index < 3).then_some(node(1))
            );
            assert!(
                session
                    .list_input(
                        window(),
                        node(1),
                        handler(),
                        tx.revision,
                        config.generation,
                        Request::ConfirmActive(Confirmation::Secondary)
                    )
                    .is_some()
            );
            if config.generation > 1 {
                assert!(
                    session
                        .list_input(
                            window(),
                            node(1),
                            handler(),
                            tx.revision,
                            config.generation - 1,
                            Request::Cancel
                        )
                        .is_none()
                );
            }
        }
    }
}
