use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
pub fn definition() -> MenuDefinition {
    MenuDefinition {
        label: "File".into(),
        disabled: false,
        items: vec![
            MenuItem::Command("run".into()),
            MenuItem::Separator,
            MenuItem::Submenu(MenuDefinition {
                label: "More".into(),
                disabled: true,
                items: vec![MenuItem::Command("run".into())],
            }),
        ],
    }
}
pub fn request() -> Message {
    let mut operations = vec![
        Op::Create(
            node(0),
            Kind::CommandScope,
            "".into(),
            Some(HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetCommands(
            node(0),
            vec![CommandConfig {
                id: "run".into(),
                generation: 1,
                label: "Run".into(),
                enabled: true,
                checked: None,
                shortcuts: vec![],
                target: CommandTarget::Callback,
            }],
        ),
    ];
    for (slot, presentation) in [
        (1, MenuPresentation::Button),
        (2, MenuPresentation::Context),
        (3, MenuPresentation::Bar),
        (4, MenuPresentation::PlatformBar),
    ] {
        operations.extend([
            Op::Create(node(slot), Kind::Menu, "".into(), None),
            Op::SetMenu(
                node(slot),
                MenuConfig {
                    presentation,
                    menus: vec![definition()],
                },
            ),
        ]);
    }
    operations.extend([
        Op::Create(node(5), Kind::Text, "Anchor".into(), None),
        Op::Splice(node(2), 0, 0, vec![node(5)]),
        Op::Splice(node(0), 0, 0, (1..5).map(node).collect()),
        Op::SetRoot(Some(node(0))),
    ]);
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations,
    })
}
