use binprot::BinProtWrite;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
#[test]
fn hover_has_independent_appended_operation_and_event_bytes() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(1, 2).unwrap();
    let handler = HandlerId::from_parts(3, 4).unwrap();
    let message = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetHoverObserver(node, Some(handler)),
            Op::SetHoverObserver(node, None),
        ],
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        hex(&bytes),
        include_str!("../../../test/fixtures/hover-operation.hex").trim()
    );
    assert_eq!(gpuio_protocol::decode(&bytes), Ok(message));
    for size in 0..bytes.len() {
        assert!(gpuio_protocol::decode(&bytes[..size]).is_err());
    }
    let mut invalid = bytes.clone();
    invalid[9] = 2;
    assert!(gpuio_protocol::decode(&invalid).is_err());
    bytes.push(0);
    assert!(gpuio_protocol::decode(&bytes).is_err());
    let mut events = vec![];
    vec![Event::HoverChanged(window, node, handler, 5, true)]
        .binprot_write(&mut events)
        .unwrap();
    assert_eq!(
        hex(&events),
        include_str!("../../../test/fixtures/hover-event.hex").trim()
    );
}
