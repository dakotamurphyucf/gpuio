use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
pub fn request() -> Message {
    let node = NodeId::from_parts(4, 3).unwrap();
    Message::Apply(Transaction {
        window: WindowId::from_parts(3, 2).unwrap(),
        base: 127,
        revision: 128,
        operations: vec![
            Op::Create(
                node,
                Kind::RadioGroup,
                "".into(),
                Some(HandlerId::from_parts(5, 4).unwrap()),
            ),
            Op::SetChoice(
                node,
                ChoiceConfig {
                    label: "Mode".into(),
                    items: vec![
                        ChoiceItem {
                            id: "fast".into(),
                            label: "Fast".into(),
                            disabled: false,
                        },
                        ChoiceItem {
                            id: "深い".into(),
                            label: "Deep".into(),
                            disabled: true,
                        },
                    ],
                    selected: Some("深い".into()),
                    disabled: false,
                },
            ),
        ],
    })
}
pub fn events() -> Vec<Event> {
    vec![Event::Choice(
        WindowId::from_parts(3, 2).unwrap(),
        NodeId::from_parts(4, 3).unwrap(),
        HandlerId::from_parts(5, 4).unwrap(),
        128,
        "深い".into(),
    )]
}
