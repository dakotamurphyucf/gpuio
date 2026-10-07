use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, decode, v1::*};

fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}

#[test]
fn independent_pattern_bytes_and_live_envelope_decoder() {
    for (fill, expected) in [
        (
            Fill::PatternSlash(Color::Rgba(0), 2., 4.),
            "03000000000000000000400000000000001040",
        ),
        (
            Fill::Checkerboard(Color::Rgba(0), 8.),
            "0400000000000000002040",
        ),
    ] {
        let encoded = bytes(&fill);
        assert_eq!(
            encoded
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            expected
        );
        let message = Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![Op::SetStyle(
                NodeId::from_parts(0, 1).unwrap(),
                vec![Style::Fields(vec![Field::Background(fill)])],
            )],
        });
        let encoded = bytes(&message);
        assert_eq!(decode(&encoded), Ok(message));
        for end in 0..encoded.len() {
            assert!(decode(&encoded[..end]).is_err());
        }
        let mut trailing = encoded;
        trailing.push(0);
        assert!(decode(&trailing).is_err());
    }
}
