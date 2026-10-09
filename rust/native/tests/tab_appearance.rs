use gpuio_native::tree::Tree;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    tab_appearance::{Config, Variant},
    v1::*,
};
fn id() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
#[test]
fn presentation_is_atomic_bounded_tab_only_and_releases_on_reset() {
    let wid = WindowId::from_parts(0, 1).unwrap();
    let mut tree = Tree::new(wid);
    let tx = |base, operations| Transaction {
        window: wid,
        base,
        revision: base + 1,
        operations,
    };
    tree.apply(&tx(
        0,
        vec![
            Op::Create(
                id(),
                Kind::TabBar,
                String::new(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetChoice(
                id(),
                ChoiceConfig {
                    label: "Tabs".into(),
                    items: vec![ChoiceItem {
                        id: "one".into(),
                        label: "One".into(),
                        disabled: false,
                    }],
                    selected: Some("one".into()),
                    disabled: false,
                },
            ),
            Op::SetRoot(Some(id())),
        ],
    ))
    .unwrap();
    let baseline = tree.retained_bytes();
    let config = Config {
        variant: Variant::Pill,
        item_styles: vec![(
            "one".into(),
            vec![Style::State(
                7,
                vec![Field::Background(Fill::Solid(Color::Rgba(0x11223344)))],
            )],
        )],
        ..Config::default()
    };
    tree.apply(&tx(
        1,
        vec![Op::SetTabAppearance(id(), Some(config.clone()))],
    ))
    .unwrap();
    let retained = tree.retained_bytes();
    assert!(retained > baseline);
    for invalid in [
        Config {
            height: 0.,
            ..config.clone()
        },
        Config {
            tab_style: vec![Style::Fields(vec![Field::Display(3)])],
            ..config.clone()
        },
        Config {
            item_styles: vec![("same".into(), vec![]), ("same".into(), vec![])],
            ..config.clone()
        },
    ] {
        assert!(
            tree.apply(&tx(
                2,
                vec![
                    Op::SetText(id(), "Must rollback".into()),
                    Op::SetTabAppearance(id(), Some(invalid))
                ]
            ))
            .is_err()
        );
        assert_eq!(tree.revision(), 2);
        assert_eq!(tree.retained_bytes(), retained);
        assert_eq!(tree.get(id()).unwrap().text.as_ref(), "");
    }
    tree.apply(&tx(2, vec![Op::SetTabAppearance(id(), None)]))
        .unwrap();
    assert_eq!(tree.retained_bytes(), baseline);
    assert_eq!(
        tree.get(id())
            .unwrap()
            .choice
            .as_ref()
            .unwrap()
            .selected
            .as_deref(),
        Some("one")
    );
    tree.apply(&tx(3, vec![Op::SetRoot(None), Op::Remove(id())]))
        .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
    let mut tree = Tree::new(wid);
    assert!(
        tree.apply(&tx(
            0,
            vec![
                Op::Create(id(), Kind::Container, String::new(), None),
                Op::SetTabAppearance(id(), Some(config)),
                Op::SetRoot(Some(id()))
            ]
        ))
        .is_err()
    );
    assert_eq!(tree.retained_bytes(), 0);
}
