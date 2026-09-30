use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, decode, v1::*};
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex(value: &[u8]) -> String {
    value.iter().map(|b| format!("{b:02x}")).collect()
}
fn message(draft: Option<String>) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetNumberInputDraft(
            NodeId::from_parts(2, 3).unwrap(),
            draft,
        )],
    })
}
#[test]
fn independent_mount_seed_envelopes_and_strict_decoding() {
    let request = message(Some("1e-".into()));
    let encoded = bytes(&request);
    assert_eq!(hex(&encoded), "0300010001013f0203010331652d");
    assert_eq!(decode(&encoded).unwrap(), request);
    assert_eq!(hex(&bytes(&message(None))), "0300010001013f020300");
    for n in 0..encoded.len() {
        assert!(decode(&encoded[..n]).is_err());
    }
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert!(decode(&trailing).is_err());
    let mut invalid_utf8 = encoded.clone();
    *invalid_utf8.last_mut().unwrap() = 255;
    assert!(decode(&invalid_utf8).is_err());
    for text in [
        "".to_owned(),
        "-".into(),
        "not numeric".into(),
        "é".repeat(2048),
    ] {
        let request = message(Some(text));
        assert_eq!(decode(&bytes(&request)).unwrap(), request);
    }
    for text in [
        "\0".to_owned(),
        "a\nb".into(),
        "a\rb".into(),
        "x".repeat(4097),
    ] {
        assert!(decode(&bytes(&message(Some(text)))).is_err());
    }
}
#[test]
fn negotiated_mount_seed_has_independent_hello_bytes() {
    assert_eq!(CAP_NUMBER_INPUT_DRAFT, 1_i64 << 53);
    assert_eq!(
        hex(&bytes(&Message::Hello(VERSION, CAP_NUMBER_INPUT_DRAFT))),
        "0001fc0000000000002000"
    );
    assert_eq!(
        hex(&bytes(&Message::Hello(VERSION, CAPABILITIES))),
        "0001fcffffffffffff3f00"
    );
}
