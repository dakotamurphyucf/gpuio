use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
pub fn config() -> ToastConfig {
    ToastConfig {
        label: "Saved".into(),
        close_label: "Dismiss".into(),
        timeout_ns: Some(5_000_000_000),
        politeness: ToastPoliteness::Assertive,
    }
}
pub fn stack() -> ToastStackConfig {
    ToastStackConfig {
        label: "Notifications".into(),
        corner: ToastCorner::BottomRight,
        width: 360.,
        max_visible: 3,
    }
}
pub fn request() -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(node(0), Kind::ToastStack, "".into(), None),
            Op::SetToastStack(node(0), stack()),
            Op::Create(
                node(1),
                Kind::Toast,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetToast(node(1), config()),
            Op::Create(node(2), Kind::Text, "Draft saved".into(), None),
            Op::Splice(node(1), 0, 0, vec![node(2)]),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    })
}
pub fn events() -> Vec<Event> {
    [
        ToastDismissal::Timeout,
        ToastDismissal::CloseButton,
        ToastDismissal::Escape,
        ToastDismissal::Overflow,
    ]
    .into_iter()
    .map(|reason| {
        Event::ToastDismissed(
            WindowId::from_parts(0, 1).unwrap(),
            node(1),
            HandlerId::from_parts(0, 1).unwrap(),
            1,
            reason,
        )
    })
    .collect()
}
