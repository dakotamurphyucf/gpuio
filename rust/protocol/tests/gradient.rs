use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, decode, v1::*};
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut out = vec![];
    value.binprot_write(&mut out).unwrap();
    out
}
#[test]
fn paired_gradient_bytes_and_bounded_live_decoder() {
    for (fill, hex) in [
        (
            Fill::LinearGradient(90., Color::Rgba(0xff0000ff), 0., Color::Rgba(0xffff), 1.),
            "01000000000080564000fcff0000ff00000000000000000000000000fdffff0000000000000000f03f",
        ),
        (
            Fill::LinearGradientIn(1, 90., Color::Rgba(0xff0000ff), 0., Color::Rgba(0xffff), 1.),
            "0201000000000080564000fcff0000ff00000000000000000000000000fdffff0000000000000000f03f",
        ),
    ] {
        let expected: Vec<_> = hex
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        assert_eq!(bytes(&fill), expected);
        let request = Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 1,
            revision: 2,
            operations: vec![Op::SetStyle(
                NodeId::from_parts(0, 1).unwrap(),
                vec![Style::Fields(vec![Field::Background(fill)])],
            )],
        });
        let encoded = bytes(&request);
        assert_eq!(decode(&encoded), Ok(request));
        for end in 0..encoded.len() {
            assert!(decode(&encoded[..end]).is_err());
        }
        let mut trailing = encoded;
        trailing.push(0);
        assert!(decode(&trailing).is_err());
    }
}
