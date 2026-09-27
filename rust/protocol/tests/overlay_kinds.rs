use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, v1::*};

#[test]
fn sheet_and_alert_tags_match_independent_fixture() {
    let kinds = [
        OverlayKind::SheetLeft,
        OverlayKind::SheetRight,
        OverlayKind::SheetTop,
        OverlayKind::SheetBottom,
        OverlayKind::AlertDialog,
    ];
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: kinds
            .into_iter()
            .map(|kind| {
                Op::SetOverlay(
                    NodeId::from_parts(0, 1).unwrap(),
                    Some(OverlayConfig {
                        kind,
                        label: "Overlay".into(),
                        width: 220.,
                        dismiss_on_escape: true,
                        dismiss_on_outside_pointer: false,
                    }),
                )
            })
            .collect(),
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        hex,
        include_str!("../../../test/fixtures/overlay-kinds-request.hex").trim()
    );
    assert_eq!(gpuio_protocol::decode(&bytes), Ok(message));
    for length in 0..bytes.len() {
        assert!(gpuio_protocol::decode(&bytes[..length]).is_err());
    }
    let mut invalid = bytes.clone();
    invalid[10] = 7; // First config's kind, following the Some tag.
    assert!(gpuio_protocol::decode(&invalid).is_err());
    bytes.push(0);
    assert!(gpuio_protocol::decode(&bytes).is_err());
}
