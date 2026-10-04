use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, decode, toast_layering::Layering, v1::*};
#[test]
fn independent_layering_reset_fixture_and_malformed_values() {
    let id = NodeId::from_parts(0, 1).unwrap();
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetToastLayering(
                id,
                Some(Layering {
                    peek: 14.,
                    gap: 14.,
                    width_step: 0.05,
                    visible: 3,
                }),
            ),
            Op::SetToastLayering(id, None),
        ],
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/toast-layering.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    for start in [10, 18, 26] {
        for bad in [
            -1.,
            f64::NAN,
            f64::INFINITY,
            if start == 26 { 0.1001 } else { 16385. },
        ] {
            let mut invalid = bytes.clone();
            invalid[start..start + 8].copy_from_slice(&bad.to_le_bytes());
            assert!(decode(&invalid).is_err());
        }
    }
    for invalid_count in [0, 9] {
        let mut invalid = bytes.clone();
        invalid[34] = invalid_count;
        assert!(decode(&invalid).is_err());
    }
    bytes.push(0);
    assert!(decode(&bytes).is_err());
}
