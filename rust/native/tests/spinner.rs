use gpuio_native::session::Session;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    animation::Easing,
    image::{ImageError, ImageSource},
    spinner::Config,
    v1::*,
};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
#[test]
fn spinner_admission_is_atomic_bounded_and_legacy_reset_clears_image_ownership() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Spinner", 100., 100.).unwrap();
    let config = Config {
        label: "Working".into(),
        animated: true,
        period_ms: 800,
        easing: Easing::EaseInOut,
        source: Some(ImageSource::Unavailable(ImageError::Released)),
    };
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let handler = HandlerId::from_parts(0, 1).unwrap();
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(id(0), Kind::Container, "".into(), None),
                Op::Create(id(1), Kind::Loading, "".into(), Some(handler)),
                Op::SetSpinner(id(1), config.clone()),
                Op::Splice(id(0), 0, 0, vec![id(1)]),
                Op::SetRoot(Some(id(0))),
            ],
        ))
        .unwrap();
    let bytes = session.retained_bytes();
    assert!(
        bytes
            >= config.retained_bytes()
                + config.loading().retained_bytes()
                + gpuio_native::spinner_clock::RESERVED_BYTES
    );
    for operations in [
        vec![Op::SetSpinner(id(0), config.clone())],
        vec![Op::SetSpinner(
            id(1),
            Config {
                period_ms: 99,
                ..config.clone()
            },
        )],
        vec![Op::SetSpinner(
            id(1),
            Config {
                easing: Easing::CubicBezier(2., 0., 0., 1.),
                ..config.clone()
            },
        )],
        vec![
            Op::SetSpinner(
                id(1),
                Config {
                    label: "Changed".into(),
                    ..config.clone()
                },
            ),
            Op::SetText(id(1), "illegal".into()),
        ],
        vec![Op::SetImage(id(1), config.image().unwrap())],
        vec![Op::SetLoading(id(1), config.loading())], // cannot leave an orphan callback
    ] {
        assert!(session.apply(&tx(1, operations)).is_err());
        assert_eq!(
            session
                .tree(window)
                .unwrap()
                .get(id(1))
                .unwrap()
                .spinner
                .as_deref(),
            Some(&config)
        );
        assert_eq!(session.retained_bytes(), bytes);
    }
    let source = config.source.unwrap();
    assert!(
        session
            .image_state(window, id(1), handler, 1, source, ImageState::Loading)
            .is_some()
    );
    session
        .apply(&tx(
            1,
            vec![
                Op::Bind(id(1), None),
                Op::SetLoading(id(1), config.loading()),
            ],
        ))
        .unwrap();
    let node = session.tree(window).unwrap().get(id(1)).unwrap();
    assert!(node.spinner.is_none() && node.image.is_none());
    assert_eq!(node.loading.as_deref(), Some(&config.loading()));
    assert!(session.retained_bytes() < bytes);
    assert!(
        session
            .image_state(window, id(1), handler, 1, source, ImageState::Loading)
            .is_none()
    );
    session
        .apply(&tx(
            2,
            vec![Op::SetSpinner(
                id(1),
                Config {
                    source: None,
                    ..config
                },
            )],
        ))
        .unwrap();
    session.close(window).unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
