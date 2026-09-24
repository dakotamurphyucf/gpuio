use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
pub fn config() -> PointerConfig {
    PointerConfig {
        label: "Resize".into(),
        button: PointerButton::Left,
        disabled: false,
        prevent_default: true,
        stop_propagation: true,
    }
}
pub fn sample(phase: PointerPhase) -> PointerSample {
    PointerSample {
        gesture: 7,
        phase,
        button: PointerButton::Left,
        window_x: 120.,
        window_y: 80.,
        local_x: -5.,
        local_y: 30.,
        modifiers: PointerModifiers {
            shift: true,
            ..Default::default()
        },
    }
}
pub fn request() -> Message {
    let node = NodeId::from_parts(0, 1).unwrap();
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(
                node,
                Kind::PointerArea,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetPointer(node, config()),
            Op::SetRoot(Some(node)),
        ],
    })
}
pub fn events() -> Vec<Event> {
    [
        PointerPhase::Started,
        PointerPhase::Moved,
        PointerPhase::Released,
        PointerPhase::Cancelled(PointerCancel::Hidden),
    ]
    .into_iter()
    .map(|phase| {
        Event::PointerEvent(
            WindowId::from_parts(0, 1).unwrap(),
            NodeId::from_parts(0, 1).unwrap(),
            HandlerId::from_parts(0, 1).unwrap(),
            1,
            sample(phase),
        )
    })
    .collect()
}
