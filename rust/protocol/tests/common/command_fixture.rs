use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn shortcut(
    key: &str,
    modifiers: Vec<ShortcutModifier>,
    priority: ShortcutPriority,
    text_input: ShortcutTextInput,
    during_composition: bool,
) -> Shortcut {
    Shortcut {
        key: key.into(),
        modifiers,
        priority,
        text_input,
        during_composition,
    }
}
pub fn request() -> Message {
    let mut commands = vec![CommandConfig {
        id: "run".into(),
        generation: 1,
        label: "run".into(),
        enabled: true,
        checked: Some(false),
        shortcuts: vec![
            shortcut(
                "k",
                vec![ShortcutModifier::Primary, ShortcutModifier::Shift],
                ShortcutPriority::Override,
                ShortcutTextInput::Always,
                true,
            ),
            shortcut(
                "enter",
                vec![],
                ShortcutPriority::NativeFirst,
                ShortcutTextInput::Never,
                false,
            ),
        ],
        target: CommandTarget::Callback,
    }];
    for (index, (label, target)) in [
        ("copy", NativeCommand::Copy),
        ("cut", NativeCommand::Cut),
        ("paste", NativeCommand::Paste),
        ("select-all", NativeCommand::SelectAll),
        ("undo", NativeCommand::Undo),
        ("redo", NativeCommand::Redo),
    ]
    .into_iter()
    .enumerate()
    {
        commands.push(CommandConfig {
            id: label.into(),
            generation: index as i64 + 2,
            label: label.into(),
            enabled: true,
            checked: None,
            shortcuts: if index == 0 {
                vec![shortcut(
                    "c",
                    vec![ShortcutModifier::Primary],
                    ShortcutPriority::NativeFirst,
                    ShortcutTextInput::ModifiedOnly,
                    false,
                )]
            } else {
                vec![]
            },
            target: CommandTarget::Native(target),
        });
    }
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(
                node(0),
                Kind::CommandScope,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetCommands(node(0), commands),
            Op::Create(node(1), Kind::CommandButton, "".into(), None),
            Op::SetCommandRef(node(1), "run".into()),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    })
}
pub fn events() -> Vec<Event> {
    [
        CommandSource::Button(node(1)),
        CommandSource::Shortcut,
        CommandSource::Menu,
        CommandSource::Palette(node(2)),
    ]
    .into_iter()
    .map(|source| {
        Event::CommandInvoked(
            WindowId::from_parts(0, 1).unwrap(),
            node(0),
            HandlerId::from_parts(0, 1).unwrap(),
            1,
            "run".into(),
            1,
            source,
        )
    })
    .collect()
}
