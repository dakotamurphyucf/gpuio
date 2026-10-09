use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, decode, v1::*};

#[test]
fn escape_operation_has_independent_bytes_and_strict_boolean_decode() {
    let node = NodeId::from_parts(1, 2).unwrap();
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetEditorClearOnEscape(node, false),
            Op::SetEditorClearOnEscape(node, true),
        ],
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    let expected: Vec<_> = include_str!("../../../test/fixtures/editor-escape-operation.hex")
        .trim()
        .as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(bytes, expected);
    assert_eq!(decode(&bytes).unwrap(), message);
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let last = bytes.len() - 1;
    bytes[last] = 2;
    assert!(decode(&bytes).is_err());
    bytes[last] = 1;
    bytes.push(0);
    assert!(decode(&bytes).is_err());
}
