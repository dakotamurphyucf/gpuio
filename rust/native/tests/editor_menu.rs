use gpuio_native::tree::Tree;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn command(target: CommandTarget) -> CommandConfig {
    CommandConfig {
        id: "copy".into(),
        generation: 1,
        label: "Copy".into(),
        enabled: true,
        checked: None,
        shortcuts: vec![],
        target,
    }
}
fn transaction(kind: Kind, target: CommandTarget) -> Transaction {
    let mut operations = vec![
        Op::Create(
            id(0),
            Kind::CommandScope,
            "".into(),
            Some(HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetCommands(id(0), vec![command(target)]),
        Op::Create(id(1), Kind::Menu, "".into(), None),
        Op::SetMenu(
            id(1),
            MenuConfig {
                presentation: MenuPresentation::EditorContext,
                menus: vec![MenuDefinition {
                    label: "Edit".into(),
                    disabled: false,
                    items: vec![MenuItem::Command("copy".into())],
                }],
            },
        ),
        Op::Create(
            id(2),
            kind,
            "".into(),
            if kind == Kind::Container {
                None
            } else {
                Some(HandlerId::from_parts(1, 1).unwrap())
            },
        ),
    ];
    if kind != Kind::Container {
        operations.push(Op::SetEditor(
            id(2),
            EditorConfig {
                label: "Input".into(),
                placeholder: "".into(),
                read_only: false,
                disabled: false,
                submit_on_enter: false,
                auto_focus: false,
                min_rows: 1,
                max_rows: 1,
            },
        ));
    }
    operations.extend([
        Op::Splice(id(0), 0, 0, vec![id(1)]),
        Op::Splice(id(1), 0, 0, vec![id(2)]),
        Op::SetRoot(Some(id(0))),
    ]);
    Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations,
    }
}
#[test]
fn editor_menus_admit_only_plain_editors_and_native_commands_atomically() {
    for kind in [Kind::Input, Kind::Textarea, Kind::Container] {
        for target in [
            CommandTarget::Native(NativeCommand::Copy),
            CommandTarget::Callback,
        ] {
            let tx = transaction(kind, target);
            let mut tree = Tree::new(tx.window);
            if kind == Kind::Container || target == CommandTarget::Callback {
                assert_eq!(tree.apply(&tx), Err(ErrorCode::InvalidTree));
                assert!(tree.is_empty());
                continue;
            }
            tree.apply(&tx).unwrap();
            let retained = tree.retained_bytes();
            assert_eq!(
                tree.apply(&Transaction {
                    window: tx.window,
                    base: 1,
                    revision: 2,
                    operations: vec![Op::SetCommands(
                        id(0),
                        vec![command(CommandTarget::Callback)]
                    )]
                }),
                Err(ErrorCode::InvalidTree)
            );
            assert_eq!(tree.revision(), 1);
            assert_eq!(tree.retained_bytes(), retained);
            assert_eq!(tree.command(id(1), "copy").unwrap().1.target, target);
        }
    }
}
