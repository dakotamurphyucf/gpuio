use gpuio_native::{mailbox::Mailbox, session::Session};
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    carousel::{Axis, Config, Direction, Request},
    navigation_stack::{Config as Presentation, Motion},
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
fn config() -> Config {
    Config {
        revision: 0,
        ids: vec!["a".into(), "b".into()],
        selected: Some(0),
        looping: false,
        disabled: false,
        axis: Axis::Horizontal,
        auto_advance_ms: Some(1000),
        direction: Direction::Direct,
    }
}
fn presentation(selected: i64) -> Presentation {
    Presentation {
        selected: Some(selected),
        retain: true,
        motion: Motion::Slide,
        duration_ms: 200,
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
fn session() -> Session {
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window(), "Carousel", 400., 300.).unwrap();
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(node(0), Kind::Carousel, "Gallery".into(), Some(handler())),
                Op::Create(node(1), Kind::NavigationStack, "Gallery pages".into(), None),
                Op::Create(node(2), Kind::Panel, "A".into(), None),
                Op::Create(node(3), Kind::Panel, "B".into(), None),
                Op::SetCarousel(node(0), config()),
                Op::SetNavigationStack(node(1), presentation(0)),
                Op::Splice(node(1), 0, 0, vec![node(2), node(3)]),
                Op::Splice(node(0), 0, 0, vec![node(1)]),
                Op::SetRoot(Some(node(0))),
            ],
        ))
        .unwrap();
    session
}
#[test]
fn owner_viewport_consistency_revision_guards_and_atomic_failure() {
    let mut session = session();
    let retained = session.retained_bytes();
    let selected = Config {
        revision: 1,
        selected: Some(1),
        direction: Direction::Next,
        ..config()
    };
    for operations in [
        // Crucially, owner is unchanged in this invalid viewport-only update.
        vec![Op::SetNavigationStack(node(1), presentation(1))],
        vec![Op::SetCarousel(node(0), selected.clone())],
        vec![
            Op::SetCarousel(
                node(0),
                Config {
                    selected: Some(1),
                    ..config()
                },
            ),
            Op::SetNavigationStack(node(1), presentation(1)),
        ],
        vec![Op::SetCarousel(
            node(0),
            Config {
                disabled: true,
                ..config()
            },
        )],
        vec![Op::SetCarousel(
            node(0),
            Config {
                ids: vec!["b".into(), "a".into()],
                ..config()
            },
        )],
        vec![Op::SetCarousel(node(1), config())],
        vec![Op::SetText(node(0), "".into())],
        vec![Op::Bind(node(0), None)],
        vec![Op::Splice(node(0), 0, 1, vec![])],
        vec![Op::Splice(node(1), 0, 1, vec![])],
    ] {
        assert_eq!(
            session.apply(&tx(1, operations)),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(session.retained_bytes(), retained);
        assert_eq!(session.tree(window()).unwrap().revision(), 1);
        assert_eq!(
            session
                .tree(window())
                .unwrap()
                .get(node(0))
                .unwrap()
                .carousel
                .as_deref(),
            Some(&config())
        );
    }
    session
        .apply(&tx(
            1,
            vec![
                Op::SetCarousel(node(0), selected),
                Op::SetNavigationStack(node(1), presentation(1)),
            ],
        ))
        .unwrap();
    assert_eq!(
        session.apply(&tx(
            2,
            vec![
                Op::SetCarousel(node(0), config()),
                Op::SetNavigationStack(node(1), presentation(0))
            ]
        )),
        Err(ErrorCode::InvalidTree)
    );
    let mut vertical = session
        .tree(window())
        .unwrap()
        .get(node(0))
        .unwrap()
        .carousel
        .as_ref()
        .unwrap()
        .as_ref()
        .clone();
    vertical.axis = Axis::Vertical;
    session
        .apply(&tx(2, vec![Op::SetCarousel(node(0), vertical)]))
        .unwrap();
    session
        .apply(&tx(
            3,
            vec![
                Op::SetRoot(None),
                Op::Remove(node(0)),
                Op::Remove(node(1)),
                Op::Remove(node(2)),
                Op::Remove(node(3)),
            ],
        ))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
#[test]
fn requests_preserve_manual_order_and_reject_obsolete_automatic_or_owner_identity() {
    let mut session = session();
    let event = |session: &Session, request| {
        session.request_carousel(window(), node(0), handler(), 1, request)
    };
    // At index zero, Previous is still delivered: a preceding queued Next can
    // make it meaningful before the application reduces it.
    let previous = event(&session, Request::Previous).unwrap();
    let next = event(&session, Request::Next).unwrap();
    let mut mailbox = Mailbox::default();
    assert!(mailbox.input(next.clone()).is_ok());
    assert!(mailbox.input(previous.clone()).is_ok());
    assert_eq!(mailbox.drain(8), vec![next, previous]);
    let automatic = config().automatic_request().unwrap();
    assert!(event(&session, automatic.clone()).is_some());
    assert!(event(&session, Request::Select("missing".into())).is_none());
    assert!(
        session
            .request_carousel(window(), node(0), handler(), 2, Request::Next)
            .is_none()
    );
    assert!(
        session
            .request_carousel(
                window(),
                node(0),
                HandlerId::from_parts(0, 2).unwrap(),
                1,
                Request::Next
            )
            .is_none()
    );
    assert!(
        session
            .request_carousel(
                window(),
                NodeId::from_parts(0, 2).unwrap(),
                handler(),
                1,
                Request::Next
            )
            .is_none()
    );
    session
        .apply(&tx(
            1,
            vec![Op::SetCarousel(
                node(0),
                Config {
                    revision: 1,
                    ..config()
                },
            )],
        ))
        .unwrap();
    assert!(event(&session, automatic).is_none());
    session
        .apply(&tx(
            2,
            vec![Op::SetCarousel(
                node(0),
                Config {
                    revision: 2,
                    disabled: true,
                    ..config()
                },
            )],
        ))
        .unwrap();
    assert!(event(&session, Request::Next).is_none());
    session.close(window()).unwrap();
    assert!(event(&session, Request::Next).is_none());
}
