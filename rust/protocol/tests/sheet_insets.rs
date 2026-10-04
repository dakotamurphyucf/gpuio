use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, decode, sheet_insets::Insets, v1::*};
#[test]
fn independent_sheet_edges_zero_reset_and_invalid_payloads() {
    let id = NodeId::from_parts(0, 1).unwrap();
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetSheetInsets(
                id,
                Some(Insets {
                    top: 34.,
                    right: 8.,
                    bottom: 12.,
                    left: 20.,
                }),
            ),
            Op::SetSheetInsets(id, Some(Insets::default())),
            Op::SetSheetInsets(id, None),
        ],
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/sheet-insets.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    for start in [10, 18, 26, 34] {
        for bad in [-1., 16385., f64::NAN, f64::INFINITY] {
            let mut invalid = bytes.clone();
            invalid[start..start + 8].copy_from_slice(&bad.to_le_bytes());
            assert!(decode(&invalid).is_err());
        }
    }
    bytes.push(0);
    assert!(decode(&bytes).is_err());
}
