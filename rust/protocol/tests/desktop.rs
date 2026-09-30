use binprot::BinProtWrite;
use gpuio_protocol::{decode_desktop_request, desktop::*, file_path::FilePath};

#[test]
fn envelopes_correlate_and_reject_invalid_request_ids() {
    use gpuio_protocol::{
        decode,
        v1::{Event, Message},
    };
    let request = Message::Desktop(7, Request::Capabilities);
    assert_eq!(encode(&request), vec![19, 7, 1]);
    assert_eq!(decode(&[19, 7, 1]).unwrap(), request);
    assert!(decode(&[19, 0, 1]).is_err());
    assert_eq!(
        encode(&vec![
            Event::DesktopResponse(7, Response::Configured),
            Event::DesktopPending
        ]),
        vec![2, 57, 7, 0, 58]
    );
}

fn encode<T: BinProtWrite>(value: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}

#[test]
fn requests_match_independent_ocaml_bytes() {
    let cases = [
        (
            Request::Configure(Identity {
                identifier: "com.example".into(),
                name: "Demo".into(),
                schemes: vec!["gpuio".into()],
            }),
            b"\x00\x0bcom.example\x04Demo\x01\x05gpuio".to_vec(),
        ),
        (Request::Capabilities, vec![1]),
        (Request::TakeLinks, vec![2]),
        (Request::Activate(true), vec![3, 1]),
        (
            Request::RevealFile(FilePath::new(b"/tmp/\xff".to_vec()).unwrap()),
            b"\x04\x06/tmp/\xff".to_vec(),
        ),
        (
            Request::OpenFile(FilePath::new(b"/a".to_vec()).unwrap()),
            b"\x05\x02/a".to_vec(),
        ),
        (
            Request::RegisterScheme("gpuio".into()),
            b"\x06\x05gpuio".to_vec(),
        ),
    ];
    for (request, expected) in cases {
        assert_eq!(encode(&request), expected);
        assert_eq!(decode_desktop_request(&expected).unwrap(), request);
        for end in 0..expected.len() {
            assert!(decode_desktop_request(&expected[..end]).is_err());
        }
        let mut trailing = expected;
        trailing.push(0);
        assert!(decode_desktop_request(&trailing).is_err());
    }
}

#[test]
fn invalid_requests_are_rejected_before_native_work() {
    for bytes in [
        b"\x03\x02".to_vec(),      // invalid Boolean
        b"\x04\x03rel".to_vec(),   // relative path
        b"\x04\x02/\0".to_vec(),   // path NUL
        b"\x06\x03APP".to_vec(),   // non-normalized scheme
        b"\x06\x01\xff".to_vec(),  // invalid UTF-8
        vec![6, 0xfe, 0xff, 0x7f], // oversized allocation claim
        vec![7],                   // unknown command
    ] {
        assert!(decode_desktop_request(&bytes).is_err());
    }
    for identifier in [
        "example", "com..app", "com.-app", "com.app-", "com.App", "com.a_b",
    ] {
        let request = Request::Configure(Identity {
            identifier: identifier.into(),
            name: "Demo".into(),
            schemes: vec![],
        });
        assert!(!request.is_valid());
        assert!(decode_desktop_request(&encode(&request)).is_err());
    }
    for schemes in [
        vec!["a".into(), "a".into()],
        vec!["a".into(); MAX_SCHEMES + 1],
    ] {
        let request = Request::Configure(Identity {
            identifier: "com.example".into(),
            name: "Demo".into(),
            schemes,
        });
        assert!(decode_desktop_request(&encode(&request)).is_err());
    }
}

#[test]
fn response_bytes_preserve_malformed_incoming_links_for_application_rejection() {
    let batch = LinkBatch {
        links: vec!["gpuio://doc/1".into(), "%".into()],
        dropped: 2,
    };
    assert!(batch.is_valid());
    assert_eq!(
        encode(&Response::Links(batch)),
        b"\x02\x02\x0dgpuio://doc/1\x01%\x02"
    );
    assert_eq!(encode(&Response::Failed(Error::Unavailable)), vec![5, 4]);
    assert_eq!(encode(&Response::Registered), vec![4]);
    let caps = Capabilities {
        incoming_links: true,
        runtime_registration: false,
        application_activation: true,
        file_reveal: true,
        file_open: true,
        document_metadata: false,
    };
    assert_eq!(
        encode(&Response::Capabilities(caps)),
        vec![1, 1, 0, 1, 1, 1, 0]
    );
}

#[test]
fn input_batches_bound_count_bytes_and_overflow_accounting() {
    assert!(
        !LinkBatch {
            links: vec![],
            dropped: -1
        }
        .is_valid()
    );
    assert!(
        !LinkBatch {
            links: vec![String::new(); MAX_LINKS + 1],
            dropped: 0
        }
        .is_valid()
    );
    assert!(
        !LinkBatch {
            links: vec!["x".repeat(MAX_LINK_BYTES + 1)],
            dropped: 0
        }
        .is_valid()
    );
    let mut batch = LinkBatch {
        links: vec!["x".repeat(MAX_LINK_BYTES); MAX_LINK_BATCH_BYTES / MAX_LINK_BYTES],
        dropped: i64::MAX,
    };
    assert!(batch.is_valid());
    batch.links.push("x".into());
    assert!(!batch.is_valid());
}

#[test]
fn launch_preflight_matches_ocaml_and_rejects_partial_or_unbounded_input() {
    use gpuio_protocol::decode_desktop_launch;
    let mut request = LaunchRequest {
        identity: Identity {
            identifier: "com.example".into(),
            name: "Demo".into(),
            schemes: vec!["gpuio".into()],
        },
        links: vec!["gpuio://a".into(), "bad".into()],
    };
    let expected = b"\x0bcom.example\x04Demo\x01\x05gpuio\x02\x09gpuio://a\x03bad";
    assert_eq!(encode(&request), expected);
    assert_eq!(decode_desktop_launch(expected).unwrap(), request);
    for end in 0..expected.len() {
        assert!(decode_desktop_launch(&expected[..end]).is_err());
    }
    let mut trailing = expected.to_vec();
    trailing.push(0);
    assert!(decode_desktop_launch(&trailing).is_err());
    for links in [
        vec!["bad\0input".into()],
        vec!["x".into(); MAX_LINKS + 1],
        vec!["x".repeat(MAX_LINK_BYTES + 1)],
        vec!["x".repeat(MAX_LINK_BYTES); 17],
    ] {
        request.links = links;
        assert!(decode_desktop_launch(&encode(&request)).is_err());
    }
    assert_eq!(encode(&LaunchResponse::Primary), [0]);
    assert_eq!(encode(&LaunchResponse::Forwarded), [1]);
    assert_eq!(encode(&LaunchResponse::Failed(Error::Busy)), [2, 6]);
}

#[test]
fn desktop_capability_uses_a_new_bit_and_round_trips_the_current_handshake() {
    use gpuio_protocol::{
        decode,
        v1::{CAP_DESKTOP, CAPABILITIES, Message, VERSION},
    };
    assert_eq!(CAPABILITIES & CAP_DESKTOP, 1_i64 << 41);
    assert_eq!(CAPABILITIES, 36_028_797_018_963_967);
    let hello = Message::Hello(VERSION, CAPABILITIES);
    assert_eq!(decode(&encode(&hello)).unwrap(), hello);
}
