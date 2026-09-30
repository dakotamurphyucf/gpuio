use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, v1::*};

#[test]
fn border_pattern_field_fixtures_and_bounded_request_decode() {
    for (pattern, expected) in [(0, [0x43, 0]), (1, [0x43, 1])] {
        let field = Field::BorderStyle(pattern);
        let mut bytes = vec![];
        field.binprot_write(&mut bytes).unwrap();
        assert_eq!(bytes, expected); // Independent OCaml fixture.
        let message = Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![Op::SetStyle(
                NodeId::from_parts(0, 1).unwrap(),
                vec![Style::Fields(vec![field])],
            )],
        });
        let mut bytes = vec![];
        message.binprot_write(&mut bytes).unwrap();
        assert_eq!(gpuio_protocol::decode(&bytes), Ok(message));
        for len in 0..bytes.len() {
            assert!(gpuio_protocol::decode(&bytes[..len]).is_err());
        }
        bytes.push(0);
        assert!(gpuio_protocol::decode(&bytes).is_err());
    }
}

#[test]
fn border_patterns_have_a_distinct_required_capability() {
    assert_eq!(CAP_BORDER_STYLES, 1_i64 << 49);
    assert_eq!(CAPABILITIES, (1_i64 << 53) - 1);
    for (required, expected) in [
        (CAP_BORDER_STYLES, "0001fc0000000000000200"),
        (CAPABILITIES, "0001fcffffffffffff1f00"),
    ] {
        let hello = Message::Hello(VERSION, required);
        let mut bytes = vec![];
        hello.binprot_write(&mut bytes).unwrap();
        assert_eq!(
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            expected
        );
        assert_eq!(gpuio_protocol::decode(&bytes), Ok(hello));
    }
}
