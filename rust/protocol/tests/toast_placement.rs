use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId, decode,
    toast_placement::{Anchor, Placement},
    v1::*,
};
#[test]
fn independent_placement_reset_fixture_and_malformed_values() {
    let id = NodeId::from_parts(0, 1).unwrap();
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetToastPlacement(
                id,
                Some(Placement {
                    anchor: Anchor::TopCenter,
                    top: 34.,
                    right: 8.,
                    bottom: 12.,
                    left: 20.,
                }),
            ),
            Op::SetToastPlacement(id, None),
        ],
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/toast-placement.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    for start in [11, 19, 27, 35] {
        for bad in [-1., 16385., f64::NAN, f64::INFINITY] {
            let mut invalid = bytes.clone();
            invalid[start..start + 8].copy_from_slice(&bad.to_le_bytes());
            assert!(decode(&invalid).is_err());
        }
    }
    let mut invalid = bytes.clone();
    invalid[10] = 8;
    assert!(decode(&invalid).is_err());
    bytes.push(0);
    assert!(decode(&bytes).is_err());
}
