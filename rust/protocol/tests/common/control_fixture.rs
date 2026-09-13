use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
pub fn request() -> Message {
    let node = |slot| NodeId::from_parts(slot, 2).unwrap();
    Message::Apply(Transaction {
        window: WindowId::from_parts(3, 2).unwrap(),
        base: 127,
        revision: 128,
        operations: vec![
            Op::Create(
                node(4),
                Kind::Checkbox,
                "界".into(),
                Some(HandlerId::from_parts(5, 4).unwrap()),
            ),
            Op::Create(node(6), Kind::Switch, "".into(), None),
            Op::SetControl(node(4), Control::Checkbox(CheckState::Unchecked, false)),
            Op::SetControl(node(4), Control::Checkbox(CheckState::Checked, true)),
            Op::SetControl(node(4), Control::Checkbox(CheckState::Indeterminate, false)),
            Op::SetControl(node(6), Control::Switch(true, false)),
            Op::SetControl(node(6), Control::Switch(false, true)),
            Op::SetControl(node(7), Control::Button(false)),
            Op::SetControl(node(7), Control::Button(true)),
            Op::SetStyle(
                node(4),
                (4..=7)
                    .map(|state| Style::State(state, vec![Field::Opacity(0.5)]))
                    .collect(),
            ),
        ],
    })
}
