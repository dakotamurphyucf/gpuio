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
        loading: false,
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
        b"\x01\x4f\x00\x01\x01\x01\x02\x01\x07\x03\x02\x02\xce\xbb\x00\x01\x03run\x02\x00"
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
        loading: false,
    };
    assert_eq!(
        bytes(&vec![Event::PaletteResult(
            9,
            window,
            node,
            handler,
            Response::Applied(snapshot)
        )]),
        b"\x01\x50\x09\x00\x01\x01\x01\x02\x01\x00\x03\x02\x02\xce\xbb\x00\x01\x03run\x02\x00"
    );
}

#[test]
fn palette_loading_has_a_checked_boolean_and_independent_command_fixture() {
    use gpuio_protocol::palette_command::Command;
    let message = Message::PaletteCommand(
        9,
        WindowId::from_parts(0, 1).unwrap(),
        NodeId::from_parts(1, 1).unwrap(),
        HandlerId::from_parts(2, 1).unwrap(),
        Some(2),
        Command::SetLoading(true),
    );
    let expected = b"\x16\x09\x00\x01\x01\x01\x02\x01\x01\x02\x04\x01";
    assert_eq!(bytes(&message), expected);
    assert_eq!(decode(expected), Ok(message));
    for length in 0..expected.len() {
        assert!(decode(&expected[..length]).is_err());
    }
    let mut invalid = expected.to_vec();
    *invalid.last_mut().unwrap() = 2;
    assert!(decode(&invalid).is_err());
}

#[test]
fn external_results_have_independent_bytes_and_require_bounded_fenced_metadata() {
    use gpuio_protocol::{palette_command::Command, palette_results::Results};
    let message = |expected, results| {
        Message::PaletteCommand(
            9,
            WindowId::from_parts(0, 1).unwrap(),
            NodeId::from_parts(1, 1).unwrap(),
            HandlerId::from_parts(2, 1).unwrap(),
            expected,
            Command::PublishResults(results),
        )
    };
    let results = Results {
        commands: vec!["run".into()],
        layout: None,
    };
    let valid = message(Some(2), results.clone());
    let expected = b"\x16\x09\x00\x01\x01\x01\x02\x01\x01\x02\x05\x01\x03run\x00";
    assert_eq!(bytes(&valid), expected);
    assert_eq!(decode(expected), Ok(valid));
    assert!(decode(&bytes(&message(None, results))).is_err());
    for end in 0..expected.len() {
        assert!(decode(&expected[..end]).is_err());
    }
    for commands in [
        vec!["run".into(), "run".into()],
        vec![" ".into()],
        vec!["x".repeat(257)],
        vec!["run".into(); 1025],
    ] {
        assert!(
            decode(&bytes(&message(
                Some(2),
                Results {
                    commands,
                    layout: None
                }
            )))
            .is_err()
        );
    }
}

#[test]
fn external_group_shape_and_retained_capacity_are_bounded() {
    use gpuio_protocol::{
        palette_layout::{Config, Entry},
        palette_options::{self, Search},
        palette_results::{RESERVATION_BYTES, Results},
    };
    for layout in [
        Config(vec![Entry::Command(1)]),
        Config(vec![Entry::Command(0), Entry::Command(0)]),
        Config(vec![
            Entry::Group("g".into(), None, vec![0]),
            Entry::Group("g".into(), None, vec![]),
        ]),
        Config(vec![Entry::Group(
            "g".into(),
            Some("x".repeat(4097)),
            vec![0],
        )]),
    ] {
        assert!(
            !Results {
                commands: vec!["run".into()],
                layout: Some(layout)
            }
            .is_valid()
        );
    }
    let defaults = palette_options::Config::default();
    let external = palette_options::Config {
        search: Search::External,
        ..Default::default()
    };
    assert_eq!(
        external.retained_bytes() - defaults.retained_bytes(),
        RESERVATION_BYTES
    );
    let ids = (0..1024).map(|i| format!("{i:0256}")).collect::<Vec<_>>();
    assert!(
        !Results {
            commands: ids,
            layout: None
        }
        .is_valid(),
        "metadata includes envelope label allowance"
    );
    assert!(
        Results {
            commands: vec![],
            layout: Some(Config(vec![]))
        }
        .is_valid()
    );
}
