use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, decode, v1::*};
#[test]
fn independent_rgba_transparent_and_default_operation_bytes() {
    let node = NodeId::from_parts(0, 1).unwrap();
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetOverlayBackdrop(node, Some(0x12345678)),
            Op::SetOverlayBackdrop(node, Some(0)),
            Op::SetOverlayBackdrop(node, None),
        ],
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/overlay-backdrop.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut bad = bytes.clone();
    bad[9] = 2;
    assert!(decode(&bad).is_err());
    bytes.push(0);
    assert!(decode(&bytes).is_err());
}
