use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, NodeId, WindowId, decode, v1::*};

#[test]
fn valid_messages_round_trip_and_all_truncations_fail() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    for message in [
        Message::Hello(VERSION, CAPABILITIES),
        Message::Open(9, window, "λ 🦀".into(), 800., 600.),
        Message::Apply(Transaction {
            window,
            base: 0,
            revision: 1,
            operations: vec![
                Op::Create(node, Kind::Container, "hello".into(), None),
                Op::SetStyle(
                    node,
                    vec![
                        Style::Width(Length::Percent(100.)),
                        Style::Background(Color::Rgba(0x123456ff)),
                    ],
                ),
                Op::SetRoot(Some(node)),
            ],
        }),
        Message::Close(10, window),
        Message::RequestFrame(11, window),
        Message::Shutdown,
    ] {
        let mut bytes = Vec::new();
        message.binprot_write(&mut bytes).unwrap();
        assert_eq!(decode(&bytes).unwrap(), message);
        for end in 0..bytes.len() {
            assert!(decode(&bytes[..end]).is_err());
        }
        bytes.push(0);
        assert_eq!(decode(&bytes), Err(DecodeError::Malformed));
    }
}

#[test]
fn reject_allocation_bombs_and_invalid_tags_or_numbers() {
    assert_eq!(
        decode(&vec![0; MAX_MESSAGE_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(decode(&[255]), Err(DecodeError::Malformed));
    let mut bytes = vec![3, 0, 1, 0, 1];
    binprot::Nat0(u64::MAX).binprot_write(&mut bytes).unwrap();
    assert_eq!(decode(&bytes), Err(DecodeError::LimitExceeded));
    let mut bytes = Vec::new();
    Message::Open(
        1,
        WindowId::from_parts(0, 1).unwrap(),
        String::new(),
        f64::NAN,
        2.,
    )
    .binprot_write(&mut bytes)
    .unwrap();
    assert_eq!(decode(&bytes), Err(DecodeError::Malformed));
    assert!(NodeId::from_parts(0, 0).is_none());
    assert!(WindowId::from_parts(-1, 1).is_none());
}
