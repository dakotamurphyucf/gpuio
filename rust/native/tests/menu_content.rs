use gpuio_native::tree::Tree;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn config(presentation: MenuPresentation) -> MenuConfig {
    MenuConfig {
        presentation,
        menus: vec![MenuDefinition {
            label: "Menu".into(),
            disabled: false,
            items: vec![
                MenuItem::Command("run".into()),
                MenuItem::Submenu(MenuDefinition {
                    label: "Nested".into(),
                    disabled: false,
                    items: vec![
                        if presentation == MenuPresentation::PlatformBar {
                            MenuItem::Command("run".into())
                        } else {
                            MenuItem::Label("Section".into())
                        },
                        MenuItem::Command("run".into()),
                    ],
                }),
                MenuItem::Separator,
            ],
        }],
    }
}
fn tx(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base,
        revision: base + 1,
        operations,
    }
}
fn initial(presentation: MenuPresentation) -> Vec<Op> {
    assert!(config(presentation).is_valid());
    let mut ops = vec![
        Op::Create(id(0), Kind::Menu, "".into(), None),
        Op::SetMenu(id(0), config(presentation)),
    ];
    let mut children = vec![];
    ops.push(Op::Create(id(1), Kind::Text, "Target".into(), None));
    if presentation.is_context() {
        children.push(id(1));
    }
    for index in 0..5 {
        ops.push(Op::Create(id(2 + index), Kind::Container, "".into(), None));
        children.push(id(2 + index));
    }
    for index in 0..4 {
        ops.push(Op::Create(
            id(7 + index),
            Kind::Text,
            format!("Rich {index}"),
            None,
        ));
        ops.push(Op::Splice(id(2 + index), 0, 0, vec![id(7 + index)]));
    }
    ops.extend([
        Op::Splice(id(0), 0, 0, children),
        Op::Create(
            id(11),
            Kind::CommandScope,
            "".into(),
            Some(HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetCommands(
            id(11),
            vec![CommandConfig {
                id: "run".into(),
                label: "Run".into(),
                generation: 1,
                enabled: true,
                checked: None,
                shortcuts: vec![],
                target: CommandTarget::Callback,
            }],
        ),
        Op::Splice(
            id(11),
            0,
            0,
            if presentation.is_context() {
                vec![id(0)]
            } else {
                vec![id(0), id(1)]
            },
        ),
        Op::SetRoot(Some(id(11))),
    ]);
    ops
}
#[test]
fn content_slot_projection_preserves_nested_and_duplicate_command_positions() {
    let mut config = config(MenuPresentation::Bar);
    config.menus.push(config.menus[0].clone());
    assert_eq!(config.items_preorder().len(), 10);
    assert_eq!(config.row_content_indices(&[0]), Some(vec![0, 1, 4]));
    assert_eq!(config.row_content_indices(&[0, 1]), Some(vec![2, 3]));
    assert_eq!(config.row_content_indices(&[1]), Some(vec![5, 6, 9]));
    assert_eq!(config.row_content_indices(&[1, 1]), Some(vec![7, 8]));
    for path in [vec![], vec![2], vec![0, 0], vec![0, 9]] {
        assert_eq!(config.row_content_indices(&path), None);
    }
}

#[test]
fn platform_icons_admit_only_passive_decorations_and_reject_changes_atomically() {
    for presentation in [
        MenuPresentation::PlatformContext,
        MenuPresentation::PlatformBar,
    ] {
        let mut operations = initial(presentation);
        for op in &mut operations {
            if let Op::Create(node, kind, text, _) = op
                && (7..11).any(|slot| *node == id(slot))
            {
                *kind = Kind::Icon;
                text.clear();
            }
        }
        let image = ImageConfig {
            source: ImageSource::Unavailable(ImageError::Released),
            fit: ImageFit::Contain,
            label: None,
        };
        for slot in 7..11 {
            operations.push(Op::SetImage(id(slot), image.clone()));
        }
        let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
        tree.apply(&tx(0, operations)).unwrap();
        let bytes = tree.retained_bytes();
        for invalid in [
            Op::SetHoverObserver(id(7), Some(HandlerId::from_parts(1, 1).unwrap())),
            Op::SetImage(
                id(7),
                ImageConfig {
                    label: Some("Interactive label".into()),
                    ..image.clone()
                },
            ),
            Op::SetText(id(7), "Rich text".into()),
            Op::SetStyle(id(7), vec![Style::Fields(vec![Field::UserSelect(true)])]),
            Op::Splice(id(6), 0, 0, vec![id(7)]),
        ] {
            assert!(tree.apply(&tx(1, vec![invalid])).is_err());
            assert_eq!(tree.revision(), 1);
            assert_eq!(tree.retained_bytes(), bytes);
        }
    }
}
#[test]
fn menu_content_is_passive_atomic_and_platform_explicit() {
    for presentation in [
        MenuPresentation::Button,
        MenuPresentation::Context,
        MenuPresentation::Bar,
    ] {
        let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
        tree.apply(&tx(0, initial(presentation))).unwrap();
        let bytes = tree.retained_bytes();
        for invalid in [
            Op::SetStyle(id(7), vec![Style::Fields(vec![Field::UserSelect(true)])]),
            Op::SetStyle(
                id(2),
                vec![Style::Fields(vec![Field::Width(Length::Px(30.))])],
            ),
            Op::Splice(id(6), 0, 0, vec![id(7)]),
            Op::Splice(id(2), 1, 0, vec![id(8)]),
            Op::SetMenu(
                id(0),
                MenuConfig {
                    presentation,
                    menus: vec![],
                },
            ),
        ] {
            assert!(
                tree.apply(&tx(1, vec![Op::SetText(id(7), "Rollback".into()), invalid]))
                    .is_err()
            );
            assert_eq!(tree.revision(), 1);
            assert_eq!(tree.get(id(7)).unwrap().text.as_ref(), "Rich 0");
            assert_eq!(tree.retained_bytes(), bytes);
        }
        let skip = usize::from(presentation.is_context());
        tree.apply(&tx(
            1,
            std::iter::once(Op::Splice(id(0), skip as i64, 5, vec![]))
                .chain((2..11).map(|slot| Op::Remove(id(slot))))
                .collect(),
        ))
        .unwrap();
        assert_eq!(tree.get(id(0)).unwrap().children.len(), skip);
    }
    let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
    assert!(
        tree.apply(&tx(0, initial(MenuPresentation::PlatformBar)))
            .is_err()
    );
    assert_eq!(tree.revision(), 0);
}

#[test]
fn exact_ocaml_public_transactions_admit_all_three_drawn_presentations() {
    let hex = include_str!("../../../test/fixtures/menu-content-public.hex").trim();
    let bytes: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let Message::Apply(tx) = gpuio_protocol::decode(&bytes).unwrap() else {
        panic!("expected transaction");
    };
    let mut tree = Tree::new(tx.window);
    tree.apply(&tx).unwrap();
    for (slot, presentation, count) in [
        (1, MenuPresentation::Button, 3),
        (6, MenuPresentation::Context, 4),
        (12, MenuPresentation::Bar, 3),
    ] {
        let node = tree.get(id(slot)).unwrap();
        assert_eq!(node.menu.as_ref().unwrap().presentation, presentation);
        assert_eq!(node.children.len(), count);
        let skip = usize::from(presentation.is_context());
        for child in &node.children[skip..] {
            assert!(tree.get(*child).unwrap().style.is_empty());
        }
    }
    assert_eq!(tree.get(id(7)).unwrap().text.as_ref(), "Target");
}

#[test]
fn platform_context_rejects_rich_slots_but_admits_one_target_and_passive_labels() {
    let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
    let invalid = initial(MenuPresentation::PlatformContext);
    assert!(tree.apply(&tx(0, invalid)).is_err());
    assert_eq!(tree.revision(), 0);
    let mut valid = initial(MenuPresentation::PlatformContext);
    for op in &mut valid {
        if let Op::Splice(owner, _, _, children) = op {
            if *owner == id(0) {
                *children = vec![id(1)];
            }
            // Keep unused rich slots reachable as ordinary siblings.
            if *owner == id(11) {
                children.extend((2..7).map(id));
            }
        }
    }
    tree.apply(&tx(0, valid)).unwrap();
    assert_eq!(tree.revision(), 1);
    assert!(
        tree.apply(&tx(1, vec![Op::Splice(id(0), 0, 1, vec![])]))
            .is_err()
    );
    assert_eq!(tree.revision(), 1);
}
