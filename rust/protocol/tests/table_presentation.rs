use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, table_presentation::valid_scope, v1::*};

fn bytes<T: BinProtWrite>(value: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn message(operation: Op) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![operation],
    })
}
#[test]
fn scoped_presentation_operations_have_independent_paired_bytes() {
    let node = NodeId::from_parts(0, 1).unwrap();
    let header = vec![
        Style::Fields(vec![Field::FontWeight(400)]),
        Style::State(2, vec![Field::Foreground(Color::Rgba(1))]),
    ];
    let row = vec![
        Style::Fields(vec![Field::Foreground(Color::Rgba(1))]),
        Style::State(7, vec![Field::Foreground(Color::Rgba(2))]),
    ];
    for (operation, fixture) in [
        (
            Op::SetTableHeaderStyle(node, header),
            include_str!("../../../test/fixtures/table-header-style.hex"),
        ),
        (
            Op::SetTableHeaderStyle(node, vec![]),
            include_str!("../../../test/fixtures/table-header-style-clear.hex"),
        ),
        (
            Op::SetTableRowStyle(node, row),
            include_str!("../../../test/fixtures/table-row-style.hex"),
        ),
        (
            Op::SetTableRowStyle(node, vec![]),
            include_str!("../../../test/fixtures/table-row-style-clear.hex"),
        ),
    ] {
        assert_eq!(
            bytes(&operation)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            fixture.trim()
        );
        let value = message(operation);
        let mut encoded = bytes(&value);
        assert_eq!(gpuio_protocol::decode(&encoded), Ok(value));
        for end in 0..encoded.len() {
            assert!(gpuio_protocol::decode(&encoded[..end]).is_err());
        }
        encoded.push(0);
        assert!(gpuio_protocol::decode(&encoded).is_err());
    }
}
#[test]
fn every_layer_is_bounded_and_scope_checked_before_admission() {
    let node = NodeId::from_parts(0, 1).unwrap();
    for row in [false, true] {
        for bad in [
            vec![Style::Fields(vec![Field::Width(Length::Px(100.))])],
            vec![Style::State(2, vec![Field::Disabled(false)])],
            vec![Style::State(4, vec![Field::Foreground(Color::Rgba(1))])],
            vec![Style::Fields(vec![Field::Opacity(0.)])],
            vec![Style::Fields(vec![Field::BorderBottomWidth(30.)])],
            vec![Style::Foreground(Color::Rgba(1))],
            vec![Style::Fields(vec![Field::Foreground(Color::Rgba(1)); 129])],
            vec![
                Style::Fields(vec![Field::Foreground(Color::Rgba(1)); 65]),
                Style::State(2, vec![Field::Foreground(Color::Rgba(1)); 64]),
            ],
        ] {
            assert!(!valid_scope(&bad, row));
            let op = if row {
                Op::SetTableRowStyle(node, bad)
            } else {
                Op::SetTableHeaderStyle(node, bad)
            };
            assert!(gpuio_protocol::decode(&bytes(&message(op))).is_err());
        }
    }
    let selected = vec![Style::State(7, vec![Field::Foreground(Color::Rgba(1))])];
    assert!(valid_scope(&selected, true) && !valid_scope(&selected, false));
    let maximum = vec![Style::Fields(vec![Field::Foreground(Color::Rgba(1)); 128])];
    let value = message(Op::SetTableRowStyle(node, maximum));
    assert_eq!(gpuio_protocol::decode(&bytes(&value)), Ok(value));
}
