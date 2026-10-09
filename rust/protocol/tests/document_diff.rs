use binprot::BinProtWrite;
use gpuio_protocol::{HandlerId, NodeId, ResourceId, WindowId, document_diff::*, v1::*};
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex(value: &impl BinProtWrite) -> String {
    bytes(value).iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn independent_live_envelopes_append_op58_event65() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let source = ResourceId::from_parts(0, 1).unwrap();
    let packet = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetDocumentDiff(node, 2, Some(Config::default())),
            Op::SetDocumentDiff(node, 3, None),
        ],
    });
    assert_eq!(
        hex(&packet),
        "0300010001023a0001020100000000fec800013a00010300"
    );
    let encoded = bytes(&packet);
    assert_eq!(gpuio_protocol::decode(&encoded).unwrap(), packet);
    for end in 0..encoded.len() {
        assert!(gpuio_protocol::decode(&encoded[..end]).is_err());
    }
    let mut trailing = encoded;
    trailing.push(0);
    assert!(gpuio_protocol::decode(&trailing).is_err());
    let event = gpuio_protocol::document_diff::Event {
        config_epoch: 2,
        source_revision: 7,
        source_generation: 3,
        observation: Observation::ShowMore {
            visible: 0,
            hidden: 7,
            applied_limit: None,
        },
    };
    let events = vec![gpuio_protocol::v1::Event::DocumentDiffEvent(
        window, node, handler, 1, source, event,
    )];
    assert_eq!(hex(&events), "014100010001000101000102070301000700");
    for epoch in [0, -1] {
        let invalid = Message::Apply(Transaction {
            window,
            base: 0,
            revision: 1,
            operations: vec![Op::SetDocumentDiff(node, epoch, None)],
        });
        assert!(gpuio_protocol::decode(&bytes(&invalid)).is_err());
    }
}
