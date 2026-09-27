use binprot::BinProtWrite;
use gpuio_protocol::{
    decode_notification_event, decode_notification_request, decode_notification_response,
    notification::*,
};
fn encode<T: BinProtWrite>(value: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn content() -> Content {
    Content {
        title: "Ready".into(),
        body: "OK".into(),
        actions: vec![Action {
            id: "open".into(),
            label: "Open".into(),
        }],
        sound: Sound::Silent,
    }
}
fn receipt() -> Receipt {
    Receipt {
        id: 7,
        tag: "build".into(),
    }
}
#[test]
fn requests_match_independent_ocaml_bytes() {
    let cases = [
        (Request::Capabilities, vec![0]),
        (Request::Authorization, vec![1]),
        (Request::RequestAuthorization, vec![2]),
        (
            Request::Post("build".into(), content()),
            b"\x03\x05build\x05Ready\x02OK\x01\x04open\x04Open\x00".to_vec(),
        ),
        (
            Request::Replace(receipt(), content()),
            b"\x04\x07\x05build\x05Ready\x02OK\x01\x04open\x04Open\x00".to_vec(),
        ),
        (Request::Dismiss(receipt()), b"\x05\x07\x05build".to_vec()),
        (Request::TakeEvents, vec![6]),
        (Request::Close, vec![7]),
    ];
    for (value, bytes) in cases {
        assert_eq!(encode(&value), bytes);
        assert_eq!(decode_notification_request(&bytes), Ok(value));
        for end in 0..bytes.len() {
            assert!(decode_notification_request(&bytes[..end]).is_err());
        }
        let mut trailing = bytes;
        trailing.push(0);
        assert!(decode_notification_request(&trailing).is_err());
    }
}
#[test]
fn responses_match_independent_ocaml_bytes() {
    let cases = [
        (
            Response::Capabilities(Capabilities {
                body: true,
                actions: true,
                activation: true,
                replacement: true,
                dismissal: true,
                permission_request: false,
                sound: true,
            }),
            vec![0, 1, 1, 1, 1, 1, 0, 1],
        ),
        (
            Response::Authorization(Authorization::NotRequired),
            vec![1, 4],
        ),
        (Response::Posted(receipt()), b"\x02\x07\x05build".to_vec()),
        (Response::Replaced, vec![3]),
        (Response::DismissRequested, vec![4]),
        (
            Response::Events(vec![
                Event::Activated(receipt()),
                Event::Action(receipt(), "open".into()),
                Event::Closed(receipt(), ClosedReason::User),
                Event::Failed(Error::Unavailable),
            ]),
            b"\x05\x04\x00\x07\x05build\x01\x07\x05build\x04open\x02\x07\x05build\x01\x03\x03"
                .to_vec(),
        ),
        (Response::Closed, vec![6]),
        (Response::Failed(Error::NativeFailure), vec![7, 8]),
    ];
    for (value, bytes) in cases {
        assert_eq!(encode(&value), bytes);
        assert_eq!(decode_notification_response(&bytes), Ok(value));
        for end in 0..bytes.len() {
            assert!(decode_notification_response(&bytes[..end]).is_err());
        }
        let mut trailing = bytes;
        trailing.push(0);
        assert!(decode_notification_response(&trailing).is_err());
    }
}
#[test]
fn malformed_payloads_and_budget_claims_are_rejected_before_allocation() {
    for bytes in [
        vec![8],
        vec![3, 1, 255],
        vec![3, 0xfe, 255, 127],
        b"\x05\x00\x05build".to_vec(),
        b"\x05\x07\x01\x00".to_vec(),
    ] {
        assert!(decode_notification_request(&bytes).is_err());
    }
    for bytes in [
        vec![0, 2, 1, 1, 1, 1, 0, 1],
        vec![1, 5],
        vec![7, 9],
        vec![5, 0xfe, 255, 127],
        vec![5, 1, 3, 9],
    ] {
        assert!(decode_notification_response(&bytes).is_err());
    }
    for bytes in [
        vec![4],
        vec![3, 9],
        b"\x02\x07\x05build\x03".to_vec(),
        b"\x01\x07\x05build\x01/".to_vec(),
    ] {
        assert!(decode_notification_event(&bytes).is_err());
    }
    let mut invalid = content();
    invalid.actions.push(invalid.actions[0].clone());
    assert!(decode_notification_request(&encode(&Request::Post("tag".into(), invalid))).is_err());
    let mut invalid = content();
    invalid.body = "x".repeat(MAX_BODY_BYTES + 1);
    assert!(decode_notification_request(&encode(&Request::Post("tag".into(), invalid))).is_err());
    let mut invalid = content();
    invalid.actions = (0..=MAX_ACTIONS)
        .map(|i| Action {
            id: i.to_string(),
            label: "Open".into(),
        })
        .collect();
    assert!(decode_notification_request(&encode(&Request::Post("tag".into(), invalid))).is_err());
    assert!(
        decode_notification_response(&encode(&Response::Events(vec![
            Event::Failed(Error::Busy);
            MAX_EVENTS + 1
        ])))
        .is_err()
    );
    assert!(
        decode_notification_response(&encode(&Response::Events(vec![
            Event::Failed(Error::Busy);
            MAX_EVENTS
        ])))
        .is_ok()
    );
}
#[test]
fn unicode_and_exact_limits_round_trip() {
    for title in ["日本語 🦀".into(), "x".repeat(MAX_TITLE_BYTES)] {
        let mut value = content();
        value.title = title;
        value.body = "x".repeat(MAX_BODY_BYTES);
        let request = Request::Post("x".repeat(MAX_TAG_BYTES), value);
        assert_eq!(decode_notification_request(&encode(&request)), Ok(request));
    }
    for title in ["", " ", "x\0", "x\n", "x\x7f"] {
        let mut value = content();
        value.title = title.into();
        assert!(!value.is_valid());
    }
}

#[test]
fn application_envelopes_keep_correlation_and_distinct_availability() {
    use gpuio_protocol::{
        decode,
        v1::{Event as BridgeEvent, Message},
    };
    let value = Message::Notification(7, Request::Capabilities);
    assert_eq!(encode(&value), vec![20, 7, 0]);
    assert_eq!(decode(&[20, 7, 0]), Ok(value));
    assert!(decode(&[20, 0, 0]).is_err());
    assert_eq!(
        encode(&vec![
            BridgeEvent::NotificationResponse(7, Response::Replaced),
            BridgeEvent::NotificationPending
        ]),
        vec![2, 59, 7, 3, 60]
    );
}
