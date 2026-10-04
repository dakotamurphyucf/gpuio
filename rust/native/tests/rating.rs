use gpuio_native::session::Session;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    rating::{Config, Request},
    v1::*,
};

#[test]
fn appearance_updates_reset_and_reject_atomically_without_changing_rating() {
    use gpuio_protocol::rating::Appearance;
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let other = NodeId::from_parts(1, 1).unwrap();
    let root = NodeId::from_parts(2, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session
        .open(1, window, "Rating appearance", 320., 200.)
        .unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let config = Config {
        label: "Quality".into(),
        value: 2,
        maximum: 5,
        star_size: 24.,
        disabled: false,
        read_only: false,
    };
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(node, Kind::Rating, "".into(), Some(handler)),
                Op::SetRating(node, config.clone()),
                Op::Create(other, Kind::Text, "unchanged".into(), None),
                Op::Create(root, Kind::Container, "".into(), None),
                Op::Splice(root, 0, 0, vec![node, other]),
                Op::SetRoot(Some(root)),
            ],
        ))
        .unwrap();
    let initial_bytes = session.retained_bytes();
    let appearance = Appearance {
        active: Some(0x11223344),
        inactive: Some(0xffffffff),
    };
    session
        .apply(&tx(
            1,
            vec![Op::SetRatingAppearance(node, Some(appearance))],
        ))
        .unwrap();
    assert_eq!(
        session.retained_bytes(),
        initial_bytes + std::mem::size_of::<Appearance>()
    );
    for (target, value) in [
        (
            node,
            Some(Appearance {
                active: Some(-1),
                ..appearance
            }),
        ),
        (
            node,
            Some(Appearance {
                inactive: Some(0x100000000),
                ..appearance
            }),
        ),
        (other, Some(appearance)),
        (other, None),
    ] {
        let bytes = session.retained_bytes();
        assert!(
            session
                .apply(&tx(
                    2,
                    vec![
                        Op::SetText(other, "must roll back".into()),
                        Op::SetRatingAppearance(target, value),
                    ]
                ))
                .is_err()
        );
        let tree = session.tree(window).unwrap();
        assert_eq!(tree.revision(), 2);
        assert_eq!(&*tree.get(other).unwrap().text, "unchanged");
        assert_eq!(
            tree.get(node).unwrap().rating_appearance.as_deref(),
            Some(&appearance)
        );
        assert_eq!(tree.get(node).unwrap().rating.as_deref(), Some(&config));
        assert_eq!(tree.get(node).unwrap().handler, Some(handler));
        assert_eq!(session.retained_bytes(), bytes);
    }
    session
        .apply(&tx(2, vec![Op::SetRatingAppearance(node, None)]))
        .unwrap();
    assert!(
        session
            .tree(window)
            .unwrap()
            .get(node)
            .unwrap()
            .rating_appearance
            .is_none()
    );
    assert_eq!(session.retained_bytes(), initial_bytes);
    session.close(window).unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
#[test]
fn rating_admission_rollback_and_requests_obey_current_policy() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Rating", 320., 200.).unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let config = Config {
        label: "Quality".into(),
        value: 2,
        maximum: 5,
        star_size: 24.,
        disabled: false,
        read_only: false,
    };
    assert!(
        session
            .apply(&tx(
                0,
                vec![
                    Op::Create(node, Kind::Rating, "".into(), None),
                    Op::SetRating(node, config.clone())
                ]
            ))
            .is_err()
    );
    assert_eq!(session.retained_bytes(), 0);
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(node, Kind::Rating, "".into(), Some(handler)),
                Op::SetRating(node, config.clone()),
                Op::SetRoot(Some(node)),
            ],
        ))
        .unwrap();
    let bytes = session.retained_bytes();
    assert!(bytes >= config.retained_bytes());
    for operations in [
        vec![Op::SetRating(
            node,
            Config {
                value: 6,
                ..config.clone()
            },
        )],
        vec![
            Op::SetRating(
                node,
                Config {
                    value: 4,
                    ..config.clone()
                },
            ),
            Op::SetText(node, "invalid leaf text".into()),
        ],
    ] {
        assert!(session.apply(&tx(1, operations)).is_err());
        assert_eq!(session.retained_bytes(), bytes);
        assert_eq!(session.tree(window).unwrap().revision(), 1);
        assert_eq!(
            session
                .tree(window)
                .unwrap()
                .get(node)
                .unwrap()
                .rating
                .as_deref(),
            Some(&config)
        );
    }
    assert_eq!(
        session.request_rating(window, node, handler, 1, Request::Increase),
        Some(Event::RatingRequested(
            window,
            node,
            handler,
            1,
            Request::Increase
        ))
    );
    assert!(
        session
            .request_rating(window, node, handler, 2, Request::Increase)
            .is_none()
    );
    assert!(
        session
            .request_rating(window, node, handler, 1, Request::Set(6))
            .is_none()
    );
    assert!(
        session
            .request_rating(
                window,
                node,
                HandlerId::from_parts(0, 2).unwrap(),
                1,
                Request::Increase
            )
            .is_none()
    );
    session
        .apply(&tx(
            1,
            vec![Op::SetRating(
                node,
                Config {
                    read_only: true,
                    ..config.clone()
                },
            )],
        ))
        .unwrap();
    assert!(
        session
            .request_rating(window, node, handler, 1, Request::Increase)
            .is_none()
    );
    session
        .apply(&tx(
            2,
            vec![
                Op::SetRating(
                    node,
                    Config {
                        disabled: true,
                        ..config.clone()
                    },
                ),
                Op::Bind(node, None),
            ],
        ))
        .unwrap();
    assert!(
        session
            .request_rating(window, node, handler, 1, Request::Increase)
            .is_none()
    );
    session
        .apply(&tx(3, vec![Op::SetRoot(None), Op::Remove(node)]))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
    assert!(
        session
            .request_rating(window, node, handler, 1, Request::Increase)
            .is_none()
    );
}
