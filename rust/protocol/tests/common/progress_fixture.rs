use gpuio_protocol::{NodeId, WindowId, v1::*};
pub fn request() -> Message {
    let node = NodeId::from_parts(0, 1).unwrap();
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(node, Kind::Progress, "".into(), None),
            Op::SetProgress(
                node,
                ProgressConfig {
                    label: "Download".into(),
                    fraction: Some(0.25),
                },
            ),
            Op::SetRoot(Some(node)),
        ],
    })
}
