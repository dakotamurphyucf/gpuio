use binprot::BinProtWrite;
use gpuio_protocol::editor_search::*;
use gpuio_protocol::{NodeId, WindowId, decode, v1::*};

fn fixture(hex: &str) -> Vec<u8> {
    hex.trim()
        .as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}

#[test]
fn search_commands_are_bounded_and_match_independent_fixtures() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(1, 2).unwrap();
    let stamp = Stamp {
        editor_revision: 7,
        search_revision: 9,
    };
    let commands = [
        Command::Read,
        Command::Open(false),
        Command::Close,
        Command::SetQuery("é".into(), Case::Sensitive),
        Command::Next,
        Command::Previous,
        Command::ReplaceCurrent(stamp, "é".into()),
        Command::ReplaceAll(stamp, "".into()),
        Command::CloseAndFocus(2),
        Command::SetQueryText("é".into()),
        Command::SetCase(Case::Sensitive),
        Command::ToggleCase,
    ];
    for (index, (command, expected)) in commands
        .into_iter()
        .zip(include_str!("../../../test/fixtures/editor-search-commands.hex").lines())
        .enumerate()
    {
        let message = Message::EditorCommand(
            7 + index as i64,
            window,
            node,
            EditorCommand::Search(command),
        );
        let data = bytes(&message);
        assert_eq!(data, fixture(expected));
        assert_eq!(decode(&data).unwrap(), message);
        for n in 0..data.len() {
            assert!(decode(&data[..n]).is_err());
        }
        assert!(decode(&[data, vec![0]].concat()).is_err());
    }
    for command in [
        Command::CloseAndFocus(-1),
        Command::SetQueryText("x".repeat(2049)),
        Command::SetQuery("x".repeat(2049), Case::Sensitive),
        Command::SetQuery("\0".into(), Case::Sensitive),
        Command::ReplaceAll(
            Stamp {
                search_revision: -1,
                ..stamp
            },
            "".into(),
        ),
        Command::ReplaceCurrent(stamp, "\0".into()),
    ] {
        assert!(
            decode(&bytes(&Message::EditorCommand(
                7,
                window,
                node,
                EditorCommand::Search(command)
            )))
            .is_err()
        );
    }
    let tx = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetEditorSearchable(node, false),
            Op::SetEditorSearchable(node, true),
        ],
    });
    assert_eq!(
        bytes(&tx),
        fixture(include_str!(
            "../../../test/fixtures/editor-search-operation.hex"
        ))
    );
    assert_eq!(decode(&bytes(&tx)).unwrap(), tx);
}

#[test]
fn search_observation_matches_independent_bytes_and_validates_bounds() {
    let value = Snapshot {
        stamp: Stamp {
            editor_revision: 7,
            search_revision: 9,
        },
        activation_revision: 2,
        mode: Mode::Replace,
        query: "é".into(),
        case: Case::AsciiInsensitive,
        text_bytes: 8,
        match_count: 3,
        current: Some(Occurrence {
            index: 1,
            byte_start: 3,
            byte_end: 5,
        }),
        can_replace: true,
    };
    assert!(value.is_valid());
    assert_eq!(
        bytes(&vec![Event::EditorSearchObserved(
            WindowId::from_parts(0, 1).unwrap(),
            NodeId::from_parts(1, 2).unwrap(),
            gpuio_protocol::HandlerId::from_parts(3, 1).unwrap(),
            4,
            value.clone(),
        )]),
        fixture(include_str!(
            "../../../test/fixtures/editor-search-observation.hex"
        )),
    );
    let hex = include_str!("../../../test/fixtures/editor-search-snapshot.hex").trim();
    let expected: Vec<u8> = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect();
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    assert_eq!(bytes, expected);
    let editor = EditorSnapshot {
        revision: 8,
        text: "é é é".into(),
        selection: EditorSelection { anchor: 5, head: 5 },
        composition: None,
        focused: true,
    };
    let after = Snapshot {
        stamp: Stamp {
            editor_revision: 8,
            search_revision: 10,
        },
        ..value.clone()
    };
    let events = vec![
        Event::EditorResult(
            10,
            WindowId::from_parts(0, 1).unwrap(),
            NodeId::from_parts(1, 2).unwrap(),
            EditorResult::SearchObserved(value.clone()),
        ),
        Event::EditorResult(
            13,
            WindowId::from_parts(0, 1).unwrap(),
            NodeId::from_parts(1, 2).unwrap(),
            EditorResult::SearchReplaced(editor, after, 1),
        ),
        Event::EditorResult(
            14,
            WindowId::from_parts(0, 1).unwrap(),
            NodeId::from_parts(1, 2).unwrap(),
            EditorResult::Failed(EditorError::StaleSearch),
        ),
    ];
    let mut encoded = vec![];
    events.binprot_write(&mut encoded).unwrap();
    assert_eq!(
        encoded,
        fixture(include_str!(
            "../../../test/fixtures/editor-search-events.hex"
        ))
    );
    for invalid in [
        Snapshot {
            stamp: Stamp {
                editor_revision: -1,
                ..value.stamp
            },
            ..value.clone()
        },
        Snapshot {
            activation_revision: 10,
            ..value.clone()
        },
        Snapshot {
            mode: Mode::Closed,
            ..value.clone()
        },
        Snapshot {
            query: "".into(),
            ..value.clone()
        },
        Snapshot {
            query: "abc".into(),
            ..value.clone()
        },
        Snapshot {
            query: "a\0b".into(),
            ..value.clone()
        },
        Snapshot {
            text_bytes: 262145,
            ..value.clone()
        },
        Snapshot {
            match_count: i64::MAX,
            ..value.clone()
        },
        Snapshot {
            current: None,
            ..value.clone()
        },
        Snapshot {
            current: Some(Occurrence {
                index: 3,
                byte_start: 3,
                byte_end: 5,
            }),
            ..value.clone()
        },
        Snapshot {
            current: Some(Occurrence {
                index: 1,
                byte_start: 7,
                byte_end: 9,
            }),
            ..value.clone()
        },
    ] {
        assert!(!invalid.is_valid());
    }
    assert!(
        Snapshot {
            query: "".into(),
            current: None,
            match_count: 0,
            ..value
        }
        .is_valid()
    );
    assert!(valid_query(&"é".repeat(1024)));
    assert!(!valid_query(&"é".repeat(1025)));
}
