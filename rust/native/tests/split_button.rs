use gpuio_native::session::Session;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    split_button::{Config, Parts},
    v1::*,
};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn menu(presentation: MenuPresentation) -> MenuConfig {
    MenuConfig {
        presentation,
        menus: vec![MenuDefinition {
            label: "More".into(),
            disabled: false,
            items: vec![],
        }],
    }
}
#[test]
fn split_admission_revalidates_parts_and_rolls_back_then_resets() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Split", 400., 300.).unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let config = Config {
        parts: Parts::Split,
        surface: vec![Style::Fields(vec![Field::Foreground(Color::Rgba(
            0x112233ff,
        ))])],
        menu_open: vec![],
    };
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(node(0), Kind::Container, "".into(), None),
                Op::Create(node(1), Kind::Container, "".into(), None),
                Op::Create(
                    node(2),
                    Kind::Button,
                    "Run".into(),
                    Some(HandlerId::from_parts(0, 1).unwrap()),
                ),
                Op::Create(node(3), Kind::Container, "".into(), None),
                Op::Create(node(4), Kind::Menu, "".into(), None),
                Op::SetMenu(node(4), menu(MenuPresentation::Button)),
                Op::Splice(node(1), 0, 0, vec![node(2)]),
                Op::Splice(node(3), 0, 0, vec![node(4)]),
                Op::Splice(node(0), 0, 0, vec![node(1), node(3)]),
                Op::SetSplitButton(node(0), Some(config.clone())),
                Op::SetRoot(Some(node(0))),
            ],
        ))
        .unwrap();
    let bytes = session.tree(window).unwrap().retained_bytes();
    for operations in [
        vec![Op::SetMenu(node(4), menu(MenuPresentation::Bar))],
        vec![Op::Bind(
            node(1),
            Some(HandlerId::from_parts(1, 1).unwrap()),
        )],
        vec![Op::SetSplitButton(node(2), Some(config.clone()))],
        vec![Op::SetSplitButton(
            node(0),
            Some(Config {
                parts: Parts::Primary,
                ..config.clone()
            }),
        )],
        vec![Op::SetSplitButton(
            node(0),
            Some(Config {
                surface: vec![Style::Fields(vec![Field::Opacity(0.5)])],
                ..config.clone()
            }),
        )],
        vec![Op::SetSplitButton(
            node(0),
            Some(Config {
                surface: vec![Style::Fields(vec![Field::Foreground(Color::Token(0))])],
                ..config.clone()
            }),
        )],
    ] {
        assert!(session.apply(&tx(1, operations)).is_err());
        let tree = session.tree(window).unwrap();
        assert_eq!(tree.revision(), 1);
        assert_eq!(tree.retained_bytes(), bytes);
        assert_eq!(
            tree.get(node(4))
                .unwrap()
                .menu
                .as_ref()
                .unwrap()
                .presentation,
            MenuPresentation::Button
        );
    }
    session
        .apply(&tx(
            1,
            vec![
                Op::SetSplitButton(node(0), None),
                Op::SetMenu(node(4), menu(MenuPresentation::Bar)),
            ],
        ))
        .unwrap();
    assert!(session.tree(window).unwrap().retained_bytes() < bytes);
    assert!(
        session
            .tree(window)
            .unwrap()
            .get(node(0))
            .unwrap()
            .split_button
            .is_none()
    );
    session.close(window).unwrap();
}
