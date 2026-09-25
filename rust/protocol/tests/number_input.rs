use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, decode_number_input_command, decode_number_input_config,
    decode_number_input_event, decode_number_input_response,
    number_input::*,
    numeric::{Direction, Domain, Draft, DraftError},
};
use std::fmt::Debug;

fn config() -> Config {
    Config {
        domain: Domain::new(-2., 8., 0.5).unwrap(),
        label: "Temperature".into(),
        placeholder: "e.g. 1.5".into(),
        increment_label: "Higher".into(),
        decrement_label: "Lower".into(),
        step_controls: StepControls::Stacked,
        allow_empty: true,
        disabled: false,
        read_only: true,
        auto_focus: false,
    }
}
fn snapshot() -> Snapshot {
    Snapshot {
        revision: 7,
        domain: config().domain,
        draft: "é1e-".into(),
        committed: Value::Number(1.5),
        selection: Selection { anchor: 5, head: 2 },
        composition: Some(Selection { anchor: 2, head: 5 }),
        focused: true,
    }
}
fn settled() -> Snapshot {
    Snapshot {
        revision: 8,
        draft: "2.5".into(),
        committed: Value::Number(2.5),
        selection: Selection { anchor: 3, head: 3 },
        composition: None,
        ..snapshot()
    }
}
fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn fixture<T: BinProtWrite + PartialEq + Debug>(
    value: T,
    expected: &str,
    decode: fn(&[u8]) -> Result<T, DecodeError>,
) {
    let bytes = encode(&value);
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hex, expected.trim());
    assert_eq!(decode(&bytes), Ok(value));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err(), "accepted prefix {end}");
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(decode(&trailing).is_err());
}
#[test]
fn independent_fixtures_cover_layout_utf8_direction_guards_and_semantic_boundaries() {
    fixture(
        config(),
        include_str!("../../../test/fixtures/number-input-config.hex"),
        decode_number_input_config,
    );
    fixture(
        Event::Observed(snapshot()),
        include_str!("../../../test/fixtures/number-input-observed.hex"),
        decode_number_input_event,
    );
    fixture(
        Event::Committed(Source::Stepper, settled()),
        include_str!("../../../test/fixtures/number-input-committed.hex"),
        decode_number_input_event,
    );
    fixture(
        Command::ReplaceDraft {
            text: "1e-".into(),
            selection: SelectionPolicy::Select(Selection { anchor: 3, head: 0 }),
            undo: UndoPolicy::Reset,
            if_revision: Some(7),
        },
        include_str!("../../../test/fixtures/number-input-replace-draft.hex"),
        decode_number_input_command,
    );
    fixture(
        Command::ReplaceValue {
            value: Value::Number(4.),
            selection: SelectionPolicy::Preserve,
            undo: UndoPolicy::Record,
            if_revision: Some(8),
        },
        include_str!("../../../test/fixtures/number-input-replace-value.hex"),
        decode_number_input_command,
    );
    fixture(
        Response::Failed(Error::Rejected(Rejection::Incomplete)),
        include_str!("../../../test/fixtures/number-input-failed.hex"),
        decode_number_input_response,
    );
}
#[test]
fn snapshot_validation_preserves_transient_drafts_and_rejects_inconsistent_state() {
    let s = snapshot();
    assert_eq!(s.classification(), Draft::Invalid(DraftError::Syntax));
    assert!(s.is_valid());
    for bad in [
        Snapshot {
            revision: -1,
            ..s.clone()
        },
        Snapshot {
            selection: Selection { anchor: 1, head: 5 },
            ..s.clone()
        },
        Snapshot {
            selection: Selection {
                anchor: -1,
                head: 5,
            },
            ..s.clone()
        },
        Snapshot {
            selection: Selection { anchor: 6, head: 5 },
            ..s.clone()
        },
        Snapshot {
            composition: Some(Selection { anchor: 5, head: 2 }),
            ..s.clone()
        },
        Snapshot {
            composition: Some(Selection { anchor: 1, head: 5 }),
            ..s.clone()
        },
        Snapshot {
            committed: Value::Number(1.25),
            ..s.clone()
        },
        Snapshot {
            committed: Value::Number(9.),
            ..s.clone()
        },
        Snapshot {
            committed: Value::Number(f64::NAN),
            ..s.clone()
        },
        Snapshot {
            committed: Value::Number(f64::INFINITY),
            ..s.clone()
        },
        Snapshot {
            draft: "a".repeat(4097),
            ..s.clone()
        },
        Snapshot {
            draft: "a\nabc".into(),
            ..s.clone()
        },
    ] {
        assert!(!bad.is_valid());
        assert!(decode_number_input_event(&encode(&Event::Observed(bad.clone()))).is_err());
        assert!(decode_number_input_response(&encode(&Response::Applied(bad))).is_err());
    }
    for draft in ["", "-", "1e-", "1.", "1e999", "99", "é", "\t"] {
        let current = Snapshot {
            draft: draft.into(),
            selection: Selection { anchor: 0, head: 0 },
            composition: None,
            ..s.clone()
        };
        assert!(current.is_valid());
        assert_eq!(
            decode_number_input_event(&encode(&Event::Changed(current.clone()))),
            Ok(Event::Changed(current))
        );
    }
    // Initial observation can be zero; semantic changes cannot.
    let initial = Snapshot {
        revision: 0,
        ..settled()
    };
    assert!(Event::Observed(initial.clone()).is_valid());
    assert!(!Event::Changed(initial.clone()).is_valid());
    assert!(!Event::Committed(Source::Keyboard, initial.clone()).is_valid());
    assert!(!Event::Cancelled(CancelReason::Escape, initial).is_valid());
    // Commit/cancel must carry the final normalized value without active IME.
    for bad in [
        s,
        Snapshot {
            committed: Value::Number(3.),
            ..settled()
        },
        Snapshot {
            composition: Some(Selection { anchor: 0, head: 0 }),
            ..settled()
        },
    ] {
        assert!(
            decode_number_input_event(&encode(&Event::Committed(Source::Keyboard, bad.clone())))
                .is_err()
        );
        assert!(
            decode_number_input_event(&encode(&Event::Cancelled(CancelReason::Escape, bad)))
                .is_err()
        );
    }
}
#[test]
fn rejection_reasons_match_the_observed_draft_and_composition() {
    for (reason, draft, composing) in [
        (Rejection::EmptyRequired, "", false),
        (Rejection::Incomplete, "1e-", false),
        (Rejection::Syntax, "é", false),
        (Rejection::NonFinite, "1e999", false),
        (Rejection::Composing, "123", true),
    ] {
        let s = Snapshot {
            draft: draft.into(),
            selection: Selection { anchor: 0, head: 0 },
            composition: composing.then_some(Selection { anchor: 0, head: 0 }),
            ..snapshot()
        };
        let event = Event::Rejected(reason, s.clone());
        assert_eq!(decode_number_input_event(&encode(&event)), Ok(event));
        let response = Response::Failed(Error::Rejected(reason));
        assert_eq!(
            decode_number_input_response(&encode(&response)),
            Ok(response)
        );
        let inconsistent = Event::Rejected(
            reason,
            Snapshot {
                draft: "2.5".into(),
                composition: None,
                ..s
            },
        );
        assert!(decode_number_input_event(&encode(&inconsistent)).is_err());
    }
}
#[test]
fn every_command_response_and_source_roundtrips() {
    let mut commands = vec![
        Command::Select(Selection { anchor: 4, head: 1 }),
        Command::Focus,
        Command::Undo,
        Command::Redo,
        Command::Commit,
        Command::Cancel,
        Command::Step(Direction::Increase),
        Command::Step(Direction::Decrease),
        Command::ReadSnapshot,
    ];
    for selection in [
        SelectionPolicy::Start,
        SelectionPolicy::End,
        SelectionPolicy::Preserve,
        SelectionPolicy::Select(Selection { anchor: 2, head: 0 }),
    ] {
        for undo in [UndoPolicy::Record, UndoPolicy::Reset] {
            for if_revision in [None, Some(0), Some(i64::MAX)] {
                commands.push(Command::ReplaceDraft {
                    text: "é".into(),
                    selection,
                    undo,
                    if_revision,
                });
                commands.push(Command::ReplaceValue {
                    value: Value::Empty,
                    selection,
                    undo,
                    if_revision,
                });
            }
        }
    }
    for command in commands {
        assert_eq!(decode_number_input_command(&encode(&command)), Ok(command));
    }
    for error in [
        Error::NotMounted,
        Error::Closed,
        Error::StaleInput,
        Error::StaleRevision,
        Error::Composing,
        Error::InvalidSelection,
        Error::LimitExceeded,
        Error::Busy,
        Error::NativeFailure,
        Error::InvalidText,
        Error::InvalidValue,
        Error::FocusBlocked,
        Error::Disabled,
        Error::ReadOnly,
        Error::InvalidConfig,
    ] {
        let response = Response::Failed(error);
        assert_eq!(
            decode_number_input_response(&encode(&response)),
            Ok(response)
        );
    }
    let response = Response::Applied(settled());
    assert_eq!(
        decode_number_input_response(&encode(&response)),
        Ok(response)
    );
    for source in [
        Source::Keyboard,
        Source::Stepper,
        Source::Accessibility,
        Source::Programmatic,
    ] {
        let event = Event::Committed(source, settled());
        assert_eq!(decode_number_input_event(&encode(&event)), Ok(event));
    }
    for reason in [CancelReason::Escape, CancelReason::Programmatic] {
        let event = Event::Cancelled(reason, settled());
        assert_eq!(decode_number_input_event(&encode(&event)), Ok(event));
    }
    for step_controls in [
        StepControls::Hidden,
        StepControls::Sides,
        StepControls::Stacked,
    ] {
        let config = Config {
            step_controls,
            ..config()
        };
        assert_eq!(decode_number_input_config(&encode(&config)), Ok(config));
    }
}
#[test]
fn commands_validate_text_finite_values_selection_and_revision_before_admission() {
    let draft = |text, selection, if_revision| Command::ReplaceDraft {
        text,
        selection,
        undo: UndoPolicy::Record,
        if_revision,
    };
    for text in [
        "a\nb".into(),
        "a\rb".into(),
        "a\0b".into(),
        "a".repeat(4097),
    ] {
        assert!(
            decode_number_input_command(&encode(&draft(text, SelectionPolicy::End, None))).is_err()
        );
    }
    for command in [
        draft(
            "é".into(),
            SelectionPolicy::Select(Selection { anchor: 1, head: 2 }),
            None,
        ),
        draft("12".into(), SelectionPolicy::End, Some(-1)),
        Command::Select(Selection {
            anchor: 4097,
            head: 0,
        }),
        Command::ReplaceValue {
            value: Value::Number(f64::NAN),
            selection: SelectionPolicy::End,
            undo: UndoPolicy::Reset,
            if_revision: None,
        },
        Command::ReplaceValue {
            value: Value::Number(f64::NEG_INFINITY),
            selection: SelectionPolicy::End,
            undo: UndoPolicy::Reset,
            if_revision: None,
        },
        Command::ReplaceValue {
            value: Value::Empty,
            selection: SelectionPolicy::End,
            undo: UndoPolicy::Reset,
            if_revision: Some(-1),
        },
    ] {
        assert!(decode_number_input_command(&encode(&command)).is_err());
    }
    // Numeric grammar does not filter native draft text; malformed UTF8 still fails decoding.
    let command = draft("é".into(), SelectionPolicy::End, None);
    assert_eq!(decode_number_input_command(&encode(&command)), Ok(command));
    let mut bytes = encode(&draft("é".into(), SelectionPolicy::End, None));
    bytes[2] = 0xff;
    assert!(decode_number_input_command(&bytes).is_err());
    for bytes in [
        &[255][..],
        &[8, 2][..],
        &[0, 0, 255][..],
        &[0, 0, 1, 255][..],
        &[0, 0, 1, 0, 2][..],
    ] {
        assert!(decode_number_input_command(bytes).is_err());
    }
}
#[test]
fn admission_limits_accept_maximum_valid_payloads_and_reject_oversize_or_invalid_config() {
    let text = "a".repeat(MAX_DRAFT_BYTES);
    let maximum = Config {
        label: text.clone(),
        placeholder: text.clone(),
        increment_label: text.clone(),
        decrement_label: text.clone(),
        ..config()
    };
    assert_eq!(decode_number_input_config(&encode(&maximum)), Ok(maximum));
    let maximum = Snapshot {
        revision: i64::MAX,
        draft: text.clone(),
        selection: Selection {
            anchor: 4096,
            head: 4096,
        },
        composition: Some(Selection {
            anchor: 0,
            head: 4096,
        }),
        ..snapshot()
    };
    let event = Event::Observed(maximum.clone());
    assert_eq!(decode_number_input_event(&encode(&event)), Ok(event));
    let response = Response::Applied(maximum);
    assert_eq!(
        decode_number_input_response(&encode(&response)),
        Ok(response)
    );
    let command = Command::ReplaceDraft {
        text,
        selection: SelectionPolicy::Select(Selection {
            anchor: 4096,
            head: 4096,
        }),
        undo: UndoPolicy::Reset,
        if_revision: Some(i64::MAX),
    };
    assert_eq!(decode_number_input_command(&encode(&command)), Ok(command));
    assert_eq!(
        decode_number_input_config(&vec![0; MAX_CONFIG_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_number_input_event(&vec![0; MAX_EVENT_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_number_input_command(&vec![0; MAX_COMMAND_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_number_input_response(&vec![0; MAX_EVENT_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    for bad in [
        Config {
            label: "\t \u{b}".into(),
            ..config()
        },
        Config {
            placeholder: "bad\0".into(),
            ..config()
        },
        Config {
            increment_label: "".into(),
            ..config()
        },
        Config {
            decrement_label: "a".repeat(4097),
            ..config()
        },
    ] {
        assert!(decode_number_input_config(&encode(&bad)).is_err());
    }
    let mut bad = encode(&config());
    bad[62] = 2; // bool must be 0 or 1
    assert!(decode_number_input_config(&bad).is_err());
    let mut bad = encode(&config());
    bad[0..8].copy_from_slice(&f64::NAN.to_le_bytes());
    assert!(decode_number_input_config(&bad).is_err());
}
