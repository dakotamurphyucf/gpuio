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

#[test]
fn rich_fallback_revalidates_descendants_atomically() {
    use gpuio_native::tree::Tree;
    let window = WindowId::from_parts(0, 1).unwrap();
    let id = |slot| NodeId::from_parts(slot, 1).unwrap();
    let mut tree = Tree::new(window);
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    tree.apply(&tx(
        0,
        vec![
            Op::Create(id(0), Kind::Avatar, "".into(), None),
            Op::SetAvatar(
                id(0),
                Config {
                    source: None,
                    fit: ImageFit::Cover,
                    label: Some("Owner".into()),
                    fallback: "?".into(),
                },
            ),
            Op::Create(id(1), Kind::Container, "".into(), None),
            Op::Create(id(2), Kind::Text, "Decoration".into(), None),
            Op::Splice(id(0), 0, 0, vec![id(1)]),
            Op::Splice(id(1), 0, 0, vec![id(2)]),
            Op::SetRoot(Some(id(0))),
        ],
    ))
    .unwrap();
    let retained = tree.retained_bytes();
    let mut invalid = vec![
        vec![Op::Bind(id(2), Some(HandlerId::from_parts(0, 1).unwrap()))],
        vec![
            Op::Create(
                id(3),
                Kind::Button,
                "Bad".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::Splice(id(1), 1, 0, vec![id(3)]),
        ],
        vec![
            Op::Create(id(3), Kind::Text, "Second slot".into(), None),
            Op::Splice(id(0), 1, 0, vec![id(3)]),
        ],
    ];
    for field in [
        Field::UserSelect(true),
        Field::OverflowX(3),
        Field::Disabled(true),
        Field::Inert(true),
        Field::PointerOcclusion(1),
    ] {
        invalid.push(vec![Op::SetStyle(id(2), vec![Style::Fields(vec![field])])]);
    }
    for mut operations in invalid {
        operations.insert(0, Op::SetText(id(2), "Must roll back".into()));
        assert_eq!(tree.apply(&tx(1, operations)), Err(ErrorCode::InvalidTree));
        assert_eq!(tree.revision(), 1);
        assert_eq!(tree.retained_bytes(), retained);
        assert_eq!(&*tree.get(id(2)).unwrap().text, "Decoration");
        assert!(tree.get(id(3)).is_none());
    }
    tree.apply(&tx(
        1,
        vec![
            Op::Splice(id(0), 0, 1, vec![]),
            Op::Remove(id(2)),
            Op::Remove(id(1)),
        ],
    ))
    .unwrap();
    assert!(tree.get(id(2)).is_none());
}

#[test]
fn rich_fallback_negotiation_has_an_independent_bit() {
    use binprot::BinProtWrite;
    assert_eq!(CAP_AVATAR_FALLBACK, 1_i64 << 56);
    assert_eq!(CAPABILITIES, i64::MAX);
    let mut bytes = vec![];
    Message::Hello(VERSION, CAP_AVATAR_FALLBACK)
        .binprot_write(&mut bytes)
        .unwrap();
    assert_eq!(bytes, [0, 3, 252, 0, 0, 0, 0, 0, 0, 0, 1]);
}

#[test]
fn rich_fallback_node_budget_counts_the_slot_root() {
    use gpuio_native::tree::Tree;
    let window = WindowId::from_parts(0, 1).unwrap();
    let id = |slot| NodeId::from_parts(slot, 1).unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let mut tree = Tree::new(window);
    tree.apply(&tx(
        0,
        vec![
            Op::Create(id(0), Kind::Avatar, "".into(), None),
            Op::SetAvatar(
                id(0),
                Config {
                    source: None,
                    fit: ImageFit::Cover,
                    label: None,
                    fallback: "?".into(),
                },
            ),
            Op::Create(id(1), Kind::Container, "".into(), None),
            Op::Splice(id(0), 0, 0, vec![id(1)]),
            Op::SetRoot(Some(id(0))),
        ],
    ))
    .unwrap();
    let mut operations = (2..=4096)
        .map(|slot| Op::Create(id(slot), Kind::Text, "".into(), None))
        .collect::<Vec<_>>();
    operations.push(Op::Splice(id(1), 0, 0, (2..=4096).map(id).collect()));
    tree.apply(&tx(1, operations)).unwrap();
    let retained = tree.retained_bytes();
    assert_eq!(
        tree.apply(&tx(
            2,
            vec![
                Op::Create(id(4097), Kind::Text, "".into(), None),
                Op::Splice(id(1), 4095, 0, vec![id(4097)]),
            ]
        )),
        Err(ErrorCode::LimitExceeded)
    );
    assert_eq!(tree.revision(), 2);
    assert_eq!(tree.retained_bytes(), retained);
    assert!(tree.get(id(4097)).is_none());
}
