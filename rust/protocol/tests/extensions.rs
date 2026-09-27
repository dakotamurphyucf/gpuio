use binprot::BinProtWrite;
use gpuio_protocol::{HandlerId, NodeId, WindowId, decode, extension::*, v1::*};
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn extension_request_and_event_match_independent_ocaml_fixture() {
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let window = WindowId::from_parts(0, 1).unwrap();
    let config = Config {
        schema: Schema {
            name: "test.counter".into(),
            version: 1,
            fingerprint: "a".repeat(64),
        },
        generation: 1,
        label: "x".into(),
        disabled: false,
        properties: Payload(vec![0, 255, 128]),
        command: Some(Command {
            sequence: 2,
            payload: Payload(vec![9]),
        }),
    };
    let message = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(node, Kind::Extension, "".into(), Some(handler)),
            Op::SetExtension(node, config),
        ],
    });
    let encoded = bytes(&message);
    assert_eq!(
        hex(&encoded),
        format!(
            "0300010001020000011e000100012300010c746573742e636f756e7465720140{}010178000300ff8001020109",
            "61".repeat(64)
        )
    );
    assert_eq!(decode(&encoded).unwrap(), message);
    for end in 0..encoded.len() {
        assert!(decode(&encoded[..end]).is_err());
    }
    assert_eq!(
        hex(&bytes(&vec![Event::ExtensionEvent(
            window,
            node,
            handler,
            1,
            1,
            Signal::Data(Payload(vec![0, 255, 128]))
        )])),
        "01260001000100010101000300ff80"
    );
}
