use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, calendar_input::*, decode_calendar_command, decode_calendar_config,
    decode_calendar_constraints, decode_calendar_event, decode_calendar_response,
    decode_calendar_selection,
};

#[test]
fn paired_events_commands_responses_and_semantic_guards() {
    let snapshot = Snapshot {
        revision: 8,
        mode: Mode::Range,
        selection: Selection::Range(Range::new(date(2024, 3, 4), date(2024, 3, 5)).unwrap()),
        selection_allowed: true,
        month: Month::new(2024, 3).unwrap(),
        focused_date: date(2024, 3, 5),
        presentation: Presentation::Days,
        focused: true,
    };
    let event = Event::Selected(snapshot.clone());
    let encoded = fixture(
        &event,
        include_str!("../../../test/fixtures/calendar-selected.hex"),
    );
    assert_eq!(decode_calendar_event(&encoded), Ok(event));
    for end in 0..encoded.len() {
        assert!(decode_calendar_event(&encoded[..end]).is_err());
    }
    let command = Command::Replace {
        selection: Selection::RangeStart(date(2024, 2, 28)),
        if_revision: Some(7),
    };
    let encoded = fixture(
        &command,
        include_str!("../../../test/fixtures/calendar-replace.hex"),
    );
    assert_eq!(decode_calendar_command(&encoded), Ok(command));
    for end in 0..encoded.len() {
        assert!(decode_calendar_command(&encoded[..end]).is_err());
    }
    for command in [
        Command::Clear { if_revision: None },
        Command::ShowMonth(Month::new(1, 1).unwrap()),
        Command::MoveMonths(-119987),
        Command::FocusDate(Date::MAX),
        Command::Focus,
        Command::SetPresentation(Presentation::Years),
        Command::ReadSnapshot,
    ] {
        assert_eq!(decode_calendar_command(&bytes(&command)), Ok(command));
    }
    for response in [
        Response::Applied(snapshot.clone()),
        Response::Failed(Error::StaleRevision),
        Response::Failed(Error::InvalidValue),
    ] {
        assert_eq!(decode_calendar_response(&bytes(&response)), Ok(response));
    }
    for event in [
        Event::Observed(snapshot.clone()),
        Event::Changed(snapshot.clone()),
        Event::Rejected(SelectionError::DisabledInterior, snapshot.clone()),
    ] {
        assert_eq!(decode_calendar_event(&bytes(&event)), Ok(event));
    }
    for invalid in [
        Snapshot {
            revision: -1,
            ..snapshot.clone()
        },
        Snapshot {
            mode: Mode::Single,
            ..snapshot.clone()
        },
        Snapshot {
            month: Month::new(2024, 2).unwrap(),
            ..snapshot.clone()
        },
        Snapshot {
            selection: Selection::Empty,
            selection_allowed: false,
            ..snapshot.clone()
        },
    ] {
        assert!(!invalid.is_valid());
        assert!(decode_calendar_event(&bytes(&Event::Observed(invalid))).is_err());
    }
    for invalid in [
        Event::Selected(Snapshot {
            selection: Selection::RangeStart(date(2024, 3, 5)),
            ..snapshot.clone()
        }),
        Event::Selected(Snapshot {
            selection_allowed: false,
            ..snapshot.clone()
        }),
        Event::Changed(Snapshot {
            revision: 0,
            ..snapshot.clone()
        }),
    ] {
        assert!(!invalid.is_valid());
        assert!(decode_calendar_event(&bytes(&invalid)).is_err());
    }
    for invalid in [
        Command::MoveMonths(i64::MIN),
        Command::MoveMonths(i64::MAX),
        Command::Clear {
            if_revision: Some(-1),
        },
    ] {
        assert!(decode_calendar_command(&bytes(&invalid)).is_err());
    }
    assert!(decode_calendar_command(&[8]).is_err());
    assert!(decode_calendar_event(&[4]).is_err());
    assert!(decode_calendar_response(&[1, 15]).is_err());
    assert_eq!(
        decode_calendar_response(&[1, 3]),
        Ok(Response::Failed(Error::StaleRevision))
    );
    assert!(decode_calendar_response(&[1, 3, 0]).is_err());
}

fn date(year: i64, month: i64, day: i64) -> Date {
    Date::from_ymd(year, month, day).unwrap()
}
fn constraints() -> Constraints {
    Constraints::new(
        date(2024, 1, 1),
        date(2030, 12, 31),
        vec![date(2024, 2, 29)],
        vec![Range::new(date(2024, 4, 10), date(2024, 4, 12)).unwrap()],
        vec![0, 6],
        RangePolicy::EveryDay,
    )
    .unwrap()
}
fn config() -> Config {
    Config {
        mode: Mode::Range,
        constraints: constraints(),
        first_weekday: 1,
        labels: Labels::english(),
        today: Some(date(2024, 3, 1)),
        label: "Review dates".into(),
        disabled: false,
        read_only: false,
        auto_focus: true,
    }
}
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut output = vec![];
    value.binprot_write(&mut output).unwrap();
    output
}
fn fixture(value: &impl BinProtWrite, expected: &str) -> Vec<u8> {
    let bytes = bytes(value);
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hex, expected.trim());
    bytes
}

#[test]
fn correlated_calendar_commands_match_independent_envelopes_and_validate_payloads() {
    use gpuio_protocol::{NodeId, WindowId, decode, v1};
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let command = Command::Replace {
        selection: Selection::RangeStart(date(2024, 2, 28)),
        if_revision: Some(7),
    };
    let request = v1::Message::CalendarCommand(9, window, node, command.clone());
    let encoded = fixture(
        &request,
        include_str!("../../../test/fixtures/calendar-command-request.hex"),
    );
    assert_eq!(decode(&encoded), Ok(request));
    for end in 0..encoded.len() {
        assert!(decode(&encoded[..end]).is_err());
    }
    let mut trailing = encoded;
    trailing.push(0);
    assert!(decode(&trailing).is_err());
    for correlation in [0, -1, i64::MIN] {
        assert!(
            decode(&bytes(&v1::Message::CalendarCommand(
                correlation,
                window,
                node,
                command.clone(),
            )))
            .is_err()
        );
    }
    for command in [
        Command::MoveMonths(i64::MIN),
        Command::MoveMonths(119988),
        Command::Clear {
            if_revision: Some(-1),
        },
    ] {
        assert!(
            decode(&bytes(&v1::Message::CalendarCommand(
                9, window, node, command
            )))
            .is_err()
        );
    }
    for (tag, value) in [(2, -1), (2, 119988), (4, -1), (4, 3652059)] {
        let mut raw = vec![17, 9, 0, 1, 0, 1, tag];
        (value as i64).binprot_write(&mut raw).unwrap();
        assert_eq!(decode(&raw), Err(DecodeError::Malformed));
    }
    fixture(
        &vec![v1::Event::CalendarResult(
            9,
            window,
            node,
            Response::Applied(Snapshot {
                revision: 8,
                mode: Mode::Range,
                selection: Selection::Range(
                    Range::new(date(2024, 3, 4), date(2024, 3, 5)).unwrap(),
                ),
                selection_allowed: true,
                month: Month::new(2024, 3).unwrap(),
                focused_date: date(2024, 3, 5),
                presentation: Presentation::Days,
                focused: true,
            }),
        )],
        include_str!("../../../test/fixtures/calendar-command-events.hex"),
    );
}

#[test]
fn independent_fixtures_full_consumption_and_all_truncated_prefixes() {
    let config = config();
    let bytes = fixture(
        &config,
        include_str!("../../../test/fixtures/calendar-config.hex"),
    );
    assert_eq!(decode_calendar_config(&bytes), Ok(config));
    for end in 0..bytes.len() {
        assert!(decode_calendar_config(&bytes[..end]).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(decode_calendar_config(&trailing).is_err());
    let constraints = constraints();
    let bytes = fixture(
        &constraints,
        include_str!("../../../test/fixtures/calendar-constraints.hex"),
    );
    assert_eq!(decode_calendar_constraints(&bytes), Ok(constraints));
    for end in 0..bytes.len() {
        assert!(decode_calendar_constraints(&bytes[..end]).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(decode_calendar_constraints(&trailing).is_err());
    let selection = Selection::Range(Range::new(date(2024, 2, 29), date(2024, 3, 1)).unwrap());
    let bytes = fixture(
        &selection,
        include_str!("../../../test/fixtures/calendar-selection.hex"),
    );
    assert_eq!(decode_calendar_selection(&bytes), Ok(selection));
    for end in 0..bytes.len() {
        assert!(decode_calendar_selection(&bytes[..end]).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(decode_calendar_selection(&trailing).is_err());
    for value in [
        Selection::Empty,
        Selection::Single(Date::MAX),
        Selection::RangeStart(Date::MIN),
    ] {
        assert_eq!(decode_calendar_selection(&self::bytes(&value)), Ok(value));
    }
}

#[test]
fn malformed_ordinals_ranges_tags_and_allocation_counts() {
    for tag in [1u8, 2] {
        for bad in [-1i64, 3652059, i64::MAX, i64::MIN] {
            let mut encoded = vec![tag];
            bad.binprot_write(&mut encoded).unwrap();
            assert_eq!(
                decode_calendar_selection(&encoded),
                Err(DecodeError::Malformed)
            );
        }
    }
    assert_eq!(
        decode_calendar_selection(&[3, 1, 0]),
        Err(DecodeError::Malformed)
    );
    assert_eq!(decode_calendar_selection(&[4]), Err(DecodeError::Malformed));
    // Each excessive count is rejected even when no declared elements follow.
    for (prefix, count) in [(vec![], 513), (vec![0], 129), (vec![0, 0], 8)] {
        let mut encoded = vec![0, 0]; // min and max day zero
        encoded.extend(prefix);
        binprot::Nat0(count).binprot_write(&mut encoded).unwrap();
        assert_eq!(
            decode_calendar_constraints(&encoded),
            Err(DecodeError::LimitExceeded)
        );
    }
    assert_eq!(
        decode_calendar_constraints(&[0, 0, 1]),
        Err(DecodeError::Malformed)
    );
    assert_eq!(
        decode_calendar_constraints(&[0, 0, 0, 0, 1, 7, 0]),
        Err(DecodeError::Malformed)
    );
    assert_eq!(
        decode_calendar_constraints(&[0, 0, 0, 0, 0, 2]),
        Err(DecodeError::Malformed)
    );
    assert_eq!(
        decode_calendar_constraints(&[1, 0, 0, 0, 0, 0]),
        Err(DecodeError::Malformed)
    );
    assert_eq!(
        decode_calendar_config(&vec![0; MAX_CONFIG_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    let mut config = bytes(&config());
    config[0] = 2;
    assert_eq!(decode_calendar_config(&config), Err(DecodeError::Malformed));
    config[0] = 1;
    *config.last_mut().unwrap() = 2;
    assert_eq!(decode_calendar_config(&config), Err(DecodeError::Malformed));
}

#[test]
fn label_shapes_utf8_and_controls_are_validated() {
    for text in ["", "  ", "bad\nlabel", "\t", "\u{7f}", "\0"] {
        let mut c = config();
        c.labels.choose_month = text.into();
        assert!(!c.is_valid());
        assert!(decode_calendar_config(&bytes(&c)).is_err());
    }
    for field in 0..3 {
        let mut c = config();
        match field {
            0 => c.labels.months.clear(),
            1 => c.labels.weekdays.clear(),
            _ => c.labels.short_weekdays.clear(),
        }
        assert!(!c.is_valid());
        assert!(decode_calendar_config(&bytes(&c)).is_err());
    }
    let mut c = config();
    c.labels.today = "x".repeat(129);
    assert_eq!(
        decode_calendar_config(&bytes(&c)),
        Err(DecodeError::LimitExceeded)
    );
    let mut c = config();
    c.label = "x".repeat(4097);
    assert_eq!(
        decode_calendar_config(&bytes(&c)),
        Err(DecodeError::LimitExceeded)
    );
    let mut raw = bytes(&config());
    let index = raw.windows(7).position(|s| s == b"January").unwrap();
    raw[index] = 0xff;
    assert_eq!(decode_calendar_config(&raw), Err(DecodeError::Malformed));
    let mut c = config();
    c.labels.months[2] = "März".into();
    c.labels.today = "Heute".into();
    c.today = None;
    c.first_weekday = 0;
    assert_eq!(decode_calendar_config(&bytes(&c)), Ok(c));
}

#[test]
fn maximum_valid_configuration_fits_envelope_and_raw_duplicates_canonicalize() {
    let mut c = config();
    c.constraints = Constraints::new(
        Date::MIN,
        Date::MAX,
        (0..512).map(|i| Date::MIN.shift(i).unwrap()).collect(),
        (0..128)
            .map(|i| {
                Range::new(
                    Date::MIN.shift(i * 3).unwrap(),
                    Date::MIN.shift(i * 3 + 1).unwrap(),
                )
                .unwrap()
            })
            .collect(),
        (0..7).collect(),
        RangePolicy::EveryDay,
    )
    .unwrap();
    c.label = "x".repeat(4096);
    for s in c
        .labels
        .months
        .iter_mut()
        .chain(&mut c.labels.weekdays)
        .chain(&mut c.labels.short_weekdays)
        .chain([
            &mut c.labels.previous,
            &mut c.labels.next,
            &mut c.labels.choose_month,
            &mut c.labels.choose_year,
            &mut c.labels.today,
            &mut c.labels.clear,
        ])
    {
        *s = "x".repeat(128);
    }
    let encoded = bytes(&c);
    assert!(encoded.len() < MAX_CONFIG_BYTES);
    assert_eq!(decode_calendar_config(&encoded), Ok(c));
    // min0/max10, dates2,1,2; ranges[3,4],[5,8]; weekdays6,0,6; EveryDay.
    let raw = [0, 10, 3, 2, 1, 2, 2, 3, 4, 5, 8, 3, 6, 0, 6, 0];
    let decoded = decode_calendar_constraints(&raw).unwrap();
    assert_eq!(
        decoded.disabled_dates(),
        &[
            Date::from_ordinal(1).unwrap(),
            Date::from_ordinal(2).unwrap()
        ]
    );
    assert_eq!(
        decoded.disabled_ranges(),
        &[Range::new(
            Date::from_ordinal(3).unwrap(),
            Date::from_ordinal(8).unwrap()
        )
        .unwrap()]
    );
    assert_eq!(decoded.disabled_weekdays(), &[0, 6]);
}

#[test]
fn retained_calendar_envelopes_match_independent_fixtures_and_bound_decoding() {
    use gpuio_protocol::{HandlerId, NodeId, WindowId, decode, v1};
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let request = v1::Message::Apply(v1::Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![
            v1::Op::Create(node, v1::Kind::Calendar, "".into(), Some(handler)),
            v1::Op::SetCalendar(
                node,
                Box::new(config()),
                Selection::Empty,
                Month::new(2024, 2).unwrap(),
            ),
            v1::Op::SetRoot(Some(node)),
        ],
    });
    // Envelopes assembled with explicit bin_prot tags/integers in Python, using
    // independently assembled civil-date/configuration payload fixtures.
    let encoded = fixture(
        &request,
        include_str!("../../../test/fixtures/calendar-request.hex"),
    );
    assert_eq!(decode(&encoded), Ok(request));
    for end in 0..encoded.len() {
        assert!(decode(&encoded[..end]).is_err());
    }
    let mut trailing = encoded;
    trailing.push(0);
    assert!(decode(&trailing).is_err());
    let selected = Snapshot {
        revision: 8,
        mode: Mode::Range,
        selection: Selection::Range(Range::new(date(2024, 3, 4), date(2024, 3, 5)).unwrap()),
        selection_allowed: true,
        month: Month::new(2024, 3).unwrap(),
        focused_date: date(2024, 3, 5),
        presentation: Presentation::Days,
        focused: true,
    };
    fixture(
        &vec![v1::Event::CalendarEvent(
            window,
            node,
            handler,
            1,
            Event::Selected(selected),
        )],
        include_str!("../../../test/fixtures/calendar-events.hex"),
    );
    let single_op = |config, selection, month| {
        v1::Message::Apply(v1::Transaction {
            window,
            base: 0,
            revision: 1,
            operations: vec![v1::Op::SetCalendar(
                node,
                Box::new(config),
                selection,
                month,
            )],
        })
    };
    let valid = single_op(config(), Selection::Empty, Month::new(2024, 2).unwrap());
    let mut invalid_month = bytes(&valid);
    invalid_month.truncate(invalid_month.len() - 3);
    (-1_i64).binprot_write(&mut invalid_month).unwrap();
    assert_eq!(decode(&invalid_month), Err(DecodeError::Malformed));
    let invalid_config = single_op(
        Config {
            label: "".into(),
            ..config()
        },
        Selection::Empty,
        Month::new(2024, 2).unwrap(),
    );
    assert_eq!(decode(&bytes(&invalid_config)), Err(DecodeError::Malformed));
    // The operation's nested date remains checked by the bounded selection reader.
    let mut invalid_selection = bytes(&valid);
    invalid_selection.truncate(invalid_selection.len() - 4);
    invalid_selection.push(1); // Single
    (-1_i64).binprot_write(&mut invalid_selection).unwrap();
    Month::new(2024, 2)
        .unwrap()
        .binprot_write(&mut invalid_selection)
        .unwrap();
    assert_eq!(decode(&invalid_selection), Err(DecodeError::Malformed));
}

#[test]
fn calendar_capability_uses_the_shared_64_bit_handshake() {
    use gpuio_protocol::{decode, v1::*};
    assert_eq!(CAPABILITIES & CAP_CALENDARS, 1_i64 << 36);
    let hello = Message::Hello(VERSION, CAPABILITIES);
    let mut bytes = Vec::new();
    hello.binprot_write(&mut bytes).unwrap();
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(hex, "0001fcffffffffffff7f00");
    assert_eq!(decode(&bytes), Ok(hello));
}
