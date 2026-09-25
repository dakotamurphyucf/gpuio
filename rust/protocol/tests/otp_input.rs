use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, decode_otp_input_command, decode_otp_input_config, decode_otp_input_event,
    decode_otp_input_response, otp_input::*,
};

fn bytes<T: BinProtWrite>(value: &T) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn composing() -> Snapshot {
    Snapshot {
        revision: 7,
        policy: Policy::new(6, Alphabet::Digits).unwrap(),
        value: "12".into(),
        draft: "12３".into(),
        selection: Selection { anchor: 5, head: 2 },
        composition: Some(Selection { anchor: 2, head: 5 }),
        focused: true,
        can_undo: false,
        can_redo: true,
    }
}
fn full() -> Snapshot {
    Snapshot {
        revision: 9,
        value: "123456".into(),
        draft: "123456".into(),
        selection: Selection { anchor: 6, head: 6 },
        composition: None,
        can_undo: true,
        can_redo: false,
        ..composing()
    }
}
fn config() -> Config {
    Config {
        policy: Policy::new(6, Alphabet::Digits).unwrap(),
        label: "Code".into(),
        masked: true,
        disabled: false,
        read_only: false,
        auto_focus: true,
    }
}
fn check<T: BinProtWrite + PartialEq + std::fmt::Debug>(
    value: T,
    fixture: &str,
    decode: fn(&[u8]) -> Result<T, DecodeError>,
) {
    let expected = hex(fixture);
    assert_eq!(bytes(&value), expected);
    assert_eq!(decode(&expected), Ok(value));
    for end in 0..expected.len() {
        assert!(decode(&expected[..end]).is_err(), "truncated at {end}");
    }
    let mut extra = expected;
    extra.push(0);
    assert_eq!(decode(&extra), Err(DecodeError::Malformed));
}

#[test]
fn independent_ocaml_fixtures_and_truncation_checks() {
    check(config(), "060004436f646501000001", decode_otp_input_config);
    check(
        Event::Observed(composing()),
        "00070600023132053132efbc930502010205010001",
        decode_otp_input_event,
    );
    check(
        Event::Complete(full()),
        "020906000631323334353606313233343536060600010100",
        decode_otp_input_event,
    );
    check(
        Response::Failed(Error::StaleRevision),
        "0103",
        decode_otp_input_response,
    );
    check(
        Event::Rejected(
            InputError::UnexpectedCharacter { byte_offset: 4095 },
            full(),
        ),
        "0303feff0f0906000631323334353606313233343536060600010100",
        decode_otp_input_event,
    );
    check(
        Command::Replace {
            value: "1234".into(),
            selection: SelectionPolicy::Select(Selection { anchor: 4, head: 1 }),
            undo: UndoPolicy::Reset,
            if_revision: Some(7),
        },
        "000431323334030401010107",
        decode_otp_input_command,
    );
    check(
        Command::Clear {
            undo: UndoPolicy::Record,
            if_revision: None,
        },
        "010000",
        decode_otp_input_command,
    );
    for (command, fixture) in [
        (Command::Select(Selection { anchor: 2, head: 0 }), "020200"),
        (Command::Focus, "03"),
        (Command::Undo, "04"),
        (Command::Redo, "05"),
        (Command::CancelComposition, "06"),
        (Command::ReadSnapshot, "07"),
    ] {
        check(command, fixture, decode_otp_input_command);
    }
}

#[test]
fn untrusted_snapshots_events_and_commands_reject_invalid_states() {
    let bad = vec![
        Snapshot {
            revision: -1,
            ..composing()
        },
        Snapshot {
            value: "AB".into(),
            ..composing()
        },
        Snapshot {
            value: "1234567".into(),
            ..composing()
        },
        Snapshot {
            draft: "\0".into(),
            ..composing()
        },
        Snapshot {
            draft: "x".repeat(4097),
            ..composing()
        },
        Snapshot {
            composition: None,
            ..composing()
        },
        Snapshot {
            composition: Some(Selection { anchor: 2, head: 2 }),
            ..composing()
        },
        Snapshot {
            composition: Some(Selection { anchor: 5, head: 2 }),
            ..composing()
        },
        Snapshot {
            composition: Some(Selection { anchor: 3, head: 5 }),
            ..composing()
        },
        Snapshot {
            selection: Selection {
                anchor: -1,
                head: 2,
            },
            ..composing()
        },
        Snapshot {
            selection: Selection { anchor: 6, head: 2 },
            ..composing()
        },
        Snapshot {
            selection: Selection { anchor: 3, head: 2 },
            ..composing()
        },
    ];
    for snapshot in bad {
        assert!(!snapshot.is_valid());
        assert!(decode_otp_input_event(&bytes(&Event::Observed(snapshot.clone()))).is_err());
        assert!(decode_otp_input_response(&bytes(&Response::Applied(snapshot))).is_err());
    }
    for event in [
        Event::Complete(composing()),
        Event::Changed(Snapshot {
            revision: 0,
            ..full()
        }),
        Event::Complete(Snapshot {
            revision: 0,
            ..full()
        }),
        Event::Complete(Snapshot {
            value: "12".into(),
            draft: "12".into(),
            selection: Selection { anchor: 2, head: 2 },
            ..full()
        }),
        Event::Rejected(InputError::TooLong, composing()),
        Event::Rejected(
            InputError::UnexpectedCharacter { byte_offset: 4096 },
            full(),
        ),
    ] {
        assert!(!event.is_valid());
        assert!(decode_otp_input_event(&bytes(&event)).is_err());
    }
    for value in ["１２".to_owned(), "a-b".into(), "a".repeat(33)] {
        let command = Command::Replace {
            value,
            selection: SelectionPolicy::End,
            undo: UndoPolicy::Record,
            if_revision: None,
        };
        assert!(!command.is_valid());
        assert!(decode_otp_input_command(&bytes(&command)).is_err());
    }
    for command in [
        Command::Replace {
            value: "12".into(),
            selection: SelectionPolicy::Select(Selection { anchor: 3, head: 0 }),
            undo: UndoPolicy::Reset,
            if_revision: None,
        },
        Command::Clear {
            undo: UndoPolicy::Reset,
            if_revision: Some(-1),
        },
        Command::Select(Selection {
            anchor: 4097,
            head: 0,
        }),
    ] {
        assert!(!command.is_valid());
        assert!(decode_otp_input_command(&bytes(&command)).is_err());
    }
}

#[test]
fn all_error_and_event_variants_roundtrip_and_bounds_precede_allocations() {
    for (tag, error) in [
        Error::NotMounted,
        Error::Closed,
        Error::StaleInput,
        Error::StaleRevision,
        Error::Composing,
        Error::InvalidSelection,
        Error::LimitExceeded,
        Error::Busy,
        Error::NativeFailure,
        Error::InvalidValue,
        Error::FocusBlocked,
        Error::Disabled,
        Error::ReadOnly,
        Error::InvalidConfig,
    ]
    .into_iter()
    .enumerate()
    {
        let response = Response::Failed(error);
        assert_eq!(bytes(&response), vec![1, tag as u8]);
        assert_eq!(decode_otp_input_response(&bytes(&response)), Ok(response));
    }
    for reason in [
        InputError::InvalidPolicy,
        InputError::InputTooLarge,
        InputError::InvalidUtf8,
        InputError::UnexpectedCharacter { byte_offset: 4095 },
        InputError::TooLong,
        InputError::InvalidValue,
        InputError::InvalidSelection,
    ] {
        let event = Event::Rejected(reason, full());
        assert!(event.is_valid());
        assert_eq!(decode_otp_input_event(&bytes(&event)), Ok(event));
    }
    let event = Event::Changed(composing());
    assert_eq!(decode_otp_input_event(&bytes(&event)), Ok(event));
    let response = Response::Applied(full());
    assert_eq!(decode_otp_input_response(&bytes(&response)), Ok(response));
    let maximum = Event::Observed(Snapshot {
        revision: i64::MAX,
        policy: Policy::new(32, Alphabet::AsciiAlphanumeric).unwrap(),
        value: "A".repeat(32),
        draft: "x".repeat(4096),
        selection: Selection {
            anchor: 4096,
            head: 0,
        },
        composition: Some(Selection {
            anchor: 0,
            head: 4096,
        }),
        ..composing()
    });
    assert!(bytes(&maximum).len() <= MAX_EVENT_BYTES);
    assert_eq!(decode_otp_input_event(&bytes(&maximum)), Ok(maximum));
    let maximum = Config {
        label: "x".repeat(4096),
        ..config()
    };
    assert_eq!(decode_otp_input_config(&bytes(&maximum)), Ok(maximum));
    // Declared length exceeds the field bound, with no supplied text bytes.
    assert_eq!(
        decode_otp_input_command(&[0, 33]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_otp_input_config(&[6, 0, 0xfe, 1, 16]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_otp_input_event(&vec![0; MAX_EVENT_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_otp_input_command(&[0; MAX_COMMAND_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_otp_input_config(&vec![0; MAX_CONFIG_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    for (index, value) in [(0, 0), (0, 33), (1, 2)] {
        let mut invalid = bytes(&config());
        invalid[index] = value;
        assert!(decode_otp_input_config(&invalid).is_err());
    }
    assert!(decode_otp_input_command(&[255]).is_err());
    assert!(decode_otp_input_event(&[255]).is_err());
    assert!(decode_otp_input_response(&[1, 255]).is_err());
    let mut invalid_utf8 = bytes(&Command::Replace {
        value: "A".into(),
        selection: SelectionPolicy::End,
        undo: UndoPolicy::Record,
        if_revision: None,
    });
    invalid_utf8[2] = 255;
    assert!(decode_otp_input_command(&invalid_utf8).is_err());
    let mut invalid_bool = bytes(&config());
    *invalid_bool.last_mut().unwrap() = 2;
    assert!(decode_otp_input_config(&invalid_bool).is_err());
    // Unexpected-character offset is a signed OCaml int, not an unsigned length.
    let mut negative_offset = vec![3, 3, 255, 255];
    negative_offset.extend(bytes(&full()));
    assert!(decode_otp_input_event(&negative_offset).is_err());
}

#[test]
fn configuration_label_validation_matches_core() {
    for label in ["", " \t\u{b}\u{c}", "bad\n", "bad\r", "bad\0"] {
        let config = Config {
            label: label.into(),
            ..config()
        };
        assert!(!config.is_valid());
        assert!(decode_otp_input_config(&bytes(&config)).is_err());
    }
    let config = Config {
        label: "-".into(),
        ..config()
    };
    assert!(config.is_valid());
    assert_eq!(decode_otp_input_config(&bytes(&config)), Ok(config));
}
