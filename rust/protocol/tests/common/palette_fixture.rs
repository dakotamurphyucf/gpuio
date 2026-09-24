use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn handler(slot: i64) -> HandlerId {
    HandlerId::from_parts(slot, 1).unwrap()
}
pub fn config() -> PaletteConfig {
    PaletteConfig {
        label: "Actions".into(),
        placeholder: "Find…".into(),
        commands: vec!["run".into(), "copy".into()],
        dismiss_on_outside_pointer: false,
    }
}
pub fn request() -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(node(0), Kind::CommandScope, "".into(), Some(handler(0))),
            Op::SetCommands(
                node(0),
                vec![
                    CommandConfig {
                        id: "run".into(),
                        label: "Run".into(),
                        generation: 1,
                        enabled: true,
                        checked: Some(true),
                        shortcuts: vec![],
                        target: CommandTarget::Callback,
                    },
                    CommandConfig {
                        id: "copy".into(),
                        label: "Copy".into(),
                        generation: 2,
                        enabled: true,
                        checked: None,
                        shortcuts: vec![],
                        target: CommandTarget::Native(NativeCommand::Copy),
                    },
                ],
            ),
            Op::Create(node(1), Kind::CommandPalette, "".into(), Some(handler(1))),
            Op::SetPalette(node(1), config()),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    })
}
pub fn events() -> Vec<Event> {
    let window = WindowId::from_parts(0, 1).unwrap();
    vec![
        Event::CommandInvoked(
            window,
            node(0),
            handler(0),
            1,
            "run".into(),
            1,
            CommandSource::Palette(node(1)),
        ),
        Event::PaletteDismissed(
            window,
            node(1),
            handler(1),
            1,
            PaletteDismissal::Selected("run".into()),
        ),
        Event::PaletteDismissed(window, node(1), handler(1), 1, PaletteDismissal::Escape),
        Event::PaletteDismissed(
            window,
            node(1),
            handler(1),
            1,
            PaletteDismissal::OutsidePointer,
        ),
    ]
}
