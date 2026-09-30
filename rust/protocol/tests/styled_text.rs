use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId,
    text_content::{Content, Span},
    v1::*,
};

#[test]
fn op59_matches_independent_ocaml_transaction_and_validates_content() {
    let node = NodeId::from_parts(0, 1).unwrap();
    let content = Content {
        text: "Aé世界".into(),
        spans: vec![
            Span {
                start_byte: 1,
                end_byte: 3,
                foreground: 0x11223344,
            },
            Span {
                start_byte: 3,
                end_byte: 9,
                foreground: 0xaabbccdd,
            },
        ],
    };
    let packet = |content| {
        Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![Op::SetStyledText(node, content)],
        })
    };
    let encode = |message: &Message| {
        let mut bytes = vec![];
        message.binprot_write(&mut bytes).unwrap();
        bytes
    };
    let message = packet(content.clone());
    let bytes = encode(&message);
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        "0300010001013b00010941c3a9e4b896e7958c020103fd443322110309fcddccbbaa00000000"
    );
    assert_eq!(gpuio_protocol::decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(gpuio_protocol::decode(&bytes[..end]).is_err());
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(gpuio_protocol::decode(&trailing).is_err());
    let mut invalid = content;
    invalid.spans[0].start_byte = 2;
    assert!(gpuio_protocol::decode(&encode(&packet(invalid))).is_err());
}

#[test]
fn styled_text_has_a_distinct_negotiated_capability() {
    assert_eq!(CAP_STYLED_TEXT, 1_i64 << 47);
    assert_eq!(CAPABILITIES, (1_i64 << 55) - 1);
    assert_ne!(CAPABILITIES & CAP_STYLED_TEXT, 0);
    let mut bytes = vec![];
    Message::Hello(VERSION, CAPABILITIES)
        .binprot_write(&mut bytes)
        .unwrap();
    assert_eq!(
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "0001fcffffffffffff7f00"
    );
}
