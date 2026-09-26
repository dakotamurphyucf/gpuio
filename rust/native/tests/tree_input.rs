use gpuio_native::{mailbox::Mailbox, session::Session};
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    accessibility::{Config as Metadata, Live, Role, TreeItem},
    list::{Config, IdRun, Order, Row, ScrollPolicy},
    tree_input::{Navigation, Request, Selection},
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
        label: Some("Tree".into()),
        description: None,
        live: Live::Off,
        field: None,
        current: None,
    }
}
fn item(disabled: bool) -> Metadata {
    metadata(Role::TreeItem(TreeItem {
        level: 1,
        index: 0,
        count: Some(1),
        expanded: None,
        selected: false,
        disabled,
        busy: false,
    }))
}
fn tx(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: window(),
        base,
        revision: base + 1,
        operations,
    }
}
fn session() -> Session {
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window(), "Tree", 400., 300.).unwrap();
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(node(0), Kind::VirtualList, String::new(), Some(handler())),
                Op::SetListConfig(
                    node(0),
                    Config {
                        estimated_height: 24.,
                        overscan: 0.,
                        max_active: 4,
                        scroll_policy: ScrollPolicy::KeepPosition,
                        scrollbar: false,
                        managed: true,
                    },
                ),
                Op::SetListOrder(
                    node(0),
                    Order {
                        revision: 1,
                        runs: vec![IdRun { first: 1, count: 1 }],
                    },
                ),
                Op::SetAccessibility(node(0), Some(metadata(Role::Tree(false)))),
                Op::SetTreeInput(node(0), true),
                Op::Create(node(1), Kind::Container, String::new(), None),
                Op::SetAccessibility(node(1), Some(item(false))),
                Op::SetListRows(
                    node(0),
                    vec![Row {
                        id: 1,
                        node: node(1),
                    }],
                ),
                Op::Splice(node(0), 0, 0, vec![node(1)]),
                Op::SetRoot(Some(node(0))),
            ],
        ))
        .unwrap();
    session
}
#[test]
fn input_requires_opt_in_tree_metadata_handler_and_current_eligible_row() {
    let mut session = session();
    let request = |s: &Session, r| s.tree_input(window(), node(0), handler(), 1, r);
    assert!(request(&session, Request::Focus(1)).is_some());
    assert!(request(&session, Request::Focus(2)).is_none());
    assert!(request(&session, Request::SetExpanded(1, true)).is_none());
    assert!(
        session
            .tree_input(
                window(),
                node(0),
                HandlerId::from_parts(0, 2).unwrap(),
                1,
                Request::ActivateActive
            )
            .is_none()
    );
    assert!(
        session
            .tree_input(window(), node(0), handler(), 2, Request::ActivateActive)
            .is_none()
    );
    for operations in [
        vec![Op::SetAccessibility(node(0), None)],
        vec![Op::Bind(node(0), None)],
        vec![Op::SetTreeInput(node(1), true)],
    ] {
        assert_eq!(
            session.apply(&tx(1, operations)),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(session.tree(window()).unwrap().revision(), 1);
    }
    session
        .apply(&tx(
            1,
            vec![Op::SetAccessibility(node(1), Some(item(true)))],
        ))
        .unwrap();
    assert!(request(&session, Request::Select(1, Selection::Replace)).is_none());
    assert!(request(&session, Request::Navigate(Navigation::Next, None)).is_some());
    session
        .apply(&tx(
            2,
            vec![
                Op::SetTreeInput(node(0), false),
                Op::SetAccessibility(node(0), None),
            ],
        ))
        .unwrap();
    assert!(request(&session, Request::ActivateActive).is_none());
}
#[test]
fn relative_intents_are_ordered_not_coalesced_and_close_retains_pending_output() {
    let session = session();
    let mut mailbox = Mailbox::default();
    for direction in [Navigation::Next, Navigation::Next, Navigation::Previous] {
        let event = session
            .tree_input(
                window(),
                node(0),
                handler(),
                1,
                Request::Navigate(direction, None),
            )
            .unwrap();
        assert!(mailbox.input(event).is_ok());
    }
    assert!(mailbox.has_window_output(0));
    let events = mailbox.drain(128);
    let directions: Vec<_> = events
        .into_iter()
        .map(|event| match event {
            Event::TreeInput(_, _, _, _, Request::Navigate(direction, None)) => direction,
            _ => panic!("unexpected event"),
        })
        .collect();
    assert_eq!(
        directions,
        vec![Navigation::Next, Navigation::Next, Navigation::Previous]
    );
    assert!(!mailbox.has_window_output(0));
}
