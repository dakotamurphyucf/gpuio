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
                Kind::Combobox,
                "é".into(),
                Some(HandlerId::from_parts(5, 4).unwrap()),
            ),
            Op::SetComboboxFilter(node, ComboboxFilter::Substring),
            Op::SetComboboxFilter(node, ComboboxFilter::Unfiltered),
        ],
    })
}
pub fn events() -> Vec<Event> {
    vec![Event::ComboboxSelected(
        WindowId::from_parts(3, 2).unwrap(),
        NodeId::from_parts(4, 3).unwrap(),
        HandlerId::from_parts(5, 4).unwrap(),
        128,
        "深い".into(),
        EditorSnapshot {
            revision: 129,
            text: "é界".into(),
            selection: EditorSelection { anchor: 5, head: 2 },
            composition: None,
            focused: true,
        },
    )]
}
