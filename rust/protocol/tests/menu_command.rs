use binprot::BinProtWrite;
use gpuio_protocol::{HandlerId, NodeId, WindowId, decode, menu_command::*, v1::*};
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
#[test]
fn positioned_menu_codec_and_bounds() {
    let message = |position| {
        Message::MenuCommand(
            7,
            WindowId::from_parts(0, 1).unwrap(),
            NodeId::from_parts(1, 1).unwrap(),
            HandlerId::from_parts(2, 1).unwrap(),
            Command::Show(position),
        )
    };
    let good = message(Position { x: 120., y: 80. });
    let expected = b"\x17\x07\x00\x01\x01\x01\x02\x01\x00\x00\x00\x00\x00\x00\x00\x5e\x40\x00\x00\x00\x00\x00\x00\x54\x40";
    assert_eq!(bytes(&good), expected);
    assert_eq!(decode(expected), Ok(good));
    for end in 0..expected.len() {
        assert!(decode(&expected[..end]).is_err());
    }
    for bad in [
        f64::NAN,
        f64::INFINITY,
        -f64::INFINITY,
        1_000_001.,
        -1_000_001.,
    ] {
        assert!(decode(&bytes(&message(Position { x: bad, y: 0. }))).is_err());
        assert!(decode(&bytes(&message(Position { x: 0., y: bad }))).is_err());
    }
    assert!(
        decode(&bytes(&message(Position {
            x: -1_000_000.,
            y: 1_000_000.
        })))
        .is_ok()
    );
    let mut invalid = expected.to_vec();
    invalid[1] = 0;
    assert!(decode(&invalid).is_err());
    let mut invalid = expected.to_vec();
    invalid[8] = 2;
    assert!(decode(&invalid).is_err());
    let mut trailing = expected.to_vec();
    trailing.push(0);
    assert!(decode(&trailing).is_err());
    assert_eq!(
        bytes(&vec![Event::MenuResult(
            7,
            WindowId::from_parts(0, 1).unwrap(),
            NodeId::from_parts(1, 1).unwrap(),
            HandlerId::from_parts(2, 1).unwrap(),
            Response::Applied
        )]),
        b"\x01\x51\x07\x00\x01\x01\x01\x02\x01\x00"
    );
}
