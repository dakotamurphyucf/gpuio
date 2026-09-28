use gpuio_protocol::{NodeId, WindowId, v1::*};

pub fn request() -> Message {
    let node = NodeId::from_parts(0, 1).unwrap();
    let mut styles: Vec<_> = (0..=21)
        .map(|value| Style::Fields(vec![Field::Cursor(value)]))
        .collect();
    styles.extend((0..=2).map(|value| Style::Fields(vec![Field::TextOverflow(value)])));
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(node, Kind::Text, "long/path/to/main.ml".into(), None),
            Op::SetStyle(node, styles),
            Op::SetRoot(Some(node)),
        ],
    })
}
