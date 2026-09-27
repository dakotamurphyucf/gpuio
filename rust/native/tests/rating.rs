use gpuio_native::session::Session;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    rating::{Config, Request},
    v1::*,
};
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
