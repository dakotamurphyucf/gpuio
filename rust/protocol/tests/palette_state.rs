use binprot::BinProtWrite;
use gpuio_protocol::{HandlerId, NodeId, WindowId, decode, palette_state::Snapshot, v1::*};
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
#[test]
fn observation_operation_and_event_have_independent_ocaml_bytes() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(1, 1).unwrap();
    let handler = HandlerId::from_parts(2, 1).unwrap();
    let message = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![Op::SetPaletteObserved(node, true)],
    });
    let expected = b"\x03\x00\x01\x00\x01\x01\x7f\x01\x01\x01";
    assert_eq!(bytes(&message), expected);
    assert_eq!(decode(expected), Ok(message));
    for end in 0..expected.len() {
        assert!(decode(&expected[..end]).is_err());
    }
    let mut invalid = expected.to_vec();
    *invalid.last_mut().unwrap() = 2;
    assert!(decode(&invalid).is_err());
    let snapshot = Snapshot {
        sequence: 3,
        query_revision: 2,
        query: "λ".into(),
        composing: false,
        selected: Some("run".into()),
        matched_count: 2,
    };
    assert!(snapshot.is_valid());
    assert_eq!(
        bytes(&vec![Event::PaletteObserved(
            window,
            node,
            handler,
            7,
            snapshot.clone()
        )]),
        b"\x01\x4f\x00\x01\x01\x01\x02\x01\x07\x03\x02\x02\xce\xbb\x00\x01\x03run\x02"
    );
    for candidate in [
        Snapshot {
            sequence: 0,
            ..snapshot.clone()
        },
        Snapshot {
            query_revision: 4,
            ..snapshot.clone()
        },
        Snapshot {
            query: "a\nb".into(),
            ..snapshot.clone()
        },
        Snapshot {
            query: "x".repeat(4097),
            ..snapshot.clone()
        },
        Snapshot {
            selected: Some(" ".into()),
            ..snapshot.clone()
        },
        Snapshot {
            matched_count: 0,
            ..snapshot.clone()
        },
        Snapshot {
            matched_count: 1025,
            ..snapshot
        },
    ] {
        assert!(!candidate.is_valid());
    }
}

#[test]
fn palette_command_and_reply_match_independent_ocaml_fixtures() {
    use gpuio_protocol::palette_command::{Command, Response};
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(1, 1).unwrap();
    let handler = HandlerId::from_parts(2, 1).unwrap();
    let command = Message::PaletteCommand(
        9,
        window,
        node,
        handler,
        Some(2),
        Command::SetQuery("λ".into()),
    );
    let expected = b"\x16\x09\x00\x01\x01\x01\x02\x01\x01\x02\x02\x02\xce\xbb";
    assert_eq!(bytes(&command), expected);
    assert_eq!(decode(expected), Ok(command));
    for length in 0..expected.len() {
        assert!(decode(&expected[..length]).is_err());
    }
    let mut trailing = expected.to_vec();
    trailing.push(0);
    assert!(decode(&trailing).is_err());
    for (expected, command) in [
        (Some(0), Command::Focus),
        (None, Command::SetQuery("x\n".into())),
        (None, Command::SetQuery("x".repeat(4097))),
        (None, Command::Highlight(Some(" ".into()))),
    ] {
        assert!(
            decode(&bytes(&Message::PaletteCommand(
                9, window, node, handler, expected, command
            )))
            .is_err()
        );
    }
    let snapshot = Snapshot {
        sequence: 3,
        query_revision: 2,
        query: "λ".into(),
        composing: false,
        selected: Some("run".into()),
        matched_count: 2,
    };
    assert_eq!(
        bytes(&vec![Event::PaletteResult(
            9,
            window,
            node,
            handler,
            Response::Applied(snapshot)
        )]),
        b"\x01\x50\x09\x00\x01\x01\x01\x02\x01\x00\x03\x02\x02\xce\xbb\x00\x01\x03run\x02"
    );
}
