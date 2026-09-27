use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, avatar::Config, v1::*};

#[test]
fn avatar_configuration_is_atomic_and_optional_sources_own_observations() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Avatar", 300., 200.).unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let fallback = Config {
        source: None,
        fit: ImageFit::Cover,
        label: Some("Dakota".into()),
        fallback: "DM".into(),
    };
    for operations in [
        vec![Op::Create(node, Kind::Avatar, "".into(), None)],
        vec![
            Op::Create(node, Kind::Avatar, "".into(), Some(handler)),
            Op::SetAvatar(node, fallback.clone()),
        ],
    ] {
        assert!(session.apply(&tx(0, operations)).is_err());
        assert_eq!(session.retained_bytes(), 0);
    }
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(node, Kind::Avatar, "".into(), None),
                Op::SetAvatar(node, fallback.clone()),
                Op::SetRoot(Some(node)),
            ],
        ))
        .unwrap();
    let bytes = session.retained_bytes();
    assert!(bytes >= fallback.retained_bytes());
    let supplied = Config {
        source: Some(ImageSource::Unavailable(ImageError::WrongApplication)),
        ..fallback.clone()
    };
    for operations in [
        vec![
            Op::SetAvatar(node, supplied.clone()),
            Op::SetText(node, "not a text leaf".into()),
        ],
        vec![Op::SetAvatar(
            node,
            Config {
                fallback: "\n".into(),
                ..fallback.clone()
            },
        )],
        vec![Op::SetImage(node, supplied.image().unwrap())],
    ] {
        assert!(session.apply(&tx(1, operations)).is_err());
        let tree = session.tree(window).unwrap();
        assert_eq!(tree.revision(), 1);
        let saved = tree.get(node).unwrap();
        assert_eq!(saved.avatar.as_deref(), Some(&fallback));
        assert!(saved.image.is_none());
        assert_eq!(session.retained_bytes(), bytes);
    }
    session
        .apply(&tx(
            1,
            vec![
                Op::SetAvatar(node, supplied.clone()),
                Op::Bind(node, Some(handler)),
            ],
        ))
        .unwrap();
    assert!(
        session.retained_bytes() > bytes,
        "derived image metadata must be charged"
    );
    assert!(
        supplied.matches_image(
            session
                .tree(window)
                .unwrap()
                .get(node)
                .unwrap()
                .image
                .as_deref()
        )
    );
    assert!(
        session
            .apply(&tx(2, vec![Op::SetAvatar(node, fallback.clone())]))
            .is_err(),
        "source removal must unbind observations in the same transaction"
    );
    assert!(
        supplied.matches_image(
            session
                .tree(window)
                .unwrap()
                .get(node)
                .unwrap()
                .image
                .as_deref()
        )
    );
    session
        .apply(&tx(
            2,
            vec![Op::SetAvatar(node, fallback), Op::Bind(node, None)],
        ))
        .unwrap();
    assert_eq!(session.retained_bytes(), bytes);
    session
        .apply(&tx(3, vec![Op::SetRoot(None), Op::Remove(node)]))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
