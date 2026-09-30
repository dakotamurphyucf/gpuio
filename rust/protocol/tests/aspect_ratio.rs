use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, v1::*};

#[test]
fn aspect_ratio_matches_independent_ocaml_fixtures_and_bounded_decoding() {
    for (ratio, expected) in [
        (0.5, "44000000000000e03f"),
        (1., "44000000000000f03f"),
        (2., "440000000000000040"),
    ] {
        let field = Field::AspectRatio(ratio);
        let mut bytes = vec![];
        field.binprot_write(&mut bytes).unwrap();
        let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        assert_eq!(hex, expected);
        let message = Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![Op::SetStyle(
                NodeId::from_parts(0, 1).unwrap(),
                vec![Style::Fields(vec![field])],
            )],
        });
        bytes.clear();
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
fn aspect_ratio_requires_a_matching_host() {
    assert_eq!(CAP_ASPECT_RATIO, 1_i64 << 50);
    assert_eq!(CAPABILITIES, (1_i64 << 51) - 1);
    for (required, expected) in [
        (CAP_ASPECT_RATIO, "0001fc0000000000000400"),
        (CAPABILITIES, "0001fcffffffffffff0700"),
    ] {
        let hello = Message::Hello(VERSION, required);
        let mut bytes = vec![];
        hello.binprot_write(&mut bytes).unwrap();
        assert_eq!(
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            expected
        );
        assert_eq!(gpuio_protocol::decode(&bytes), Ok(hello));
    }
}
