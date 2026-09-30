use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, color_input::*, color_value::*, decode_color_command, decode_color_config,
    decode_color_event, decode_color_response,
};

fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut output = Vec::new();
    value.binprot_write(&mut output).unwrap();
    output
}
fn fixture(value: &impl BinProtWrite, expected: &str) -> Vec<u8> {
    let output = bytes(value);
    let hex: String = output.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hex, expected.trim());
    output
}
fn color(value: i64) -> Value {
    Value::Color(Rgba::from_packed(value).unwrap())
}
fn config() -> Config {
    Config {
        labels: Labels {
            control: "Accent".into(),
            hue: "Hue".into(),
            saturation: "Saturation".into(),
            lightness: "Lightness".into(),
            alpha: "Opacity".into(),
            hex: "Hex color".into(),
            clear: "Clear color".into(),
        },
        palette: vec![
            PaletteEntry {
                color: Rgba::new(255, 0, 0, 128),
                label: "Half red".into(),
            },
            PaletteEntry {
                color: Rgba::new(17, 34, 51, 255),
                label: "Slate".into(),
            },
        ],
        alpha_policy: AlphaPolicy::AllowAlpha,
        allow_empty: true,
        disabled: false,
        read_only: true,
    }
}
fn snapshot() -> Snapshot {
    Snapshot {
        revision: 8,
        value: color(0xff000080),
        committed: color(0x00ff00ff),
        channels: Hsla::new(0., 1., 0.5, 0.5).unwrap(),
        interaction: Some(Interaction {
            id: 5,
            kind: InteractionKind::Text(Field::Hex),
        }),
        draft: Some(Draft {
            text: "#ff000080".into(),
            composing: false,
            status: DraftStatus::Valid,
        }),
        value_allowed: true,
        committed_allowed: true,
    }
}

#[test]
fn independently_assembled_config_event_command_response_fixtures() {
    let c = config();
    let encoded = fixture(&c, include_str!("../../../test/fixtures/color-config.hex"));
    assert_eq!(encoded.len(), 96);
    assert_eq!(decode_color_config(&encoded), Ok(c));
    for end in 0..encoded.len() {
        assert!(decode_color_config(&encoded[..end]).is_err());
    }
    let e = Event::Preview(snapshot());
    let encoded = fixture(&e, include_str!("../../../test/fixtures/color-preview.hex"));
    assert_eq!(encoded.len(), 69);
    assert_eq!(decode_color_event(&encoded), Ok(e));
    for end in 0..encoded.len() {
        assert!(decode_color_event(&encoded[..end]).is_err());
    }
    let cmd = Command::Set {
        value: color(0x11223344),
        if_revision: Some(7),
    };
    let encoded = fixture(&cmd, include_str!("../../../test/fixtures/color-set.hex"));
    assert_eq!(decode_color_command(&encoded), Ok(cmd));
    for end in 0..encoded.len() {
        assert!(decode_color_command(&encoded[..end]).is_err());
    }
    let response = Response::Failed(Error::StaleInteraction);
    let encoded = fixture(
        &response,
        include_str!("../../../test/fixtures/color-failed.hex"),
    );
    assert_eq!(decode_color_response(&encoded), Ok(response));
    for end in 0..encoded.len() {
        assert!(decode_color_response(&encoded[..end]).is_err());
    }
}

#[test]
fn tags_full_consumption_and_semantic_guards() {
    let mut c = config();
    c.alpha_policy = AlphaPolicy::OpaqueOnly;
    c.labels.control = "色 / Couleur".into();
    assert_eq!(decode_color_config(&bytes(&c)), Ok(c));
    let mut invalid_utf8 = bytes(&config());
    invalid_utf8[1] = 0xff;
    assert_eq!(
        decode_color_config(&invalid_utf8),
        Err(DecodeError::Malformed)
    );
    let mut invalid_bool = bytes(&config());
    *invalid_bool.last_mut().unwrap() = 2;
    assert_eq!(
        decode_color_config(&invalid_bool),
        Err(DecodeError::Malformed)
    );
    let mut trailing = bytes(&config());
    trailing.push(0);
    assert_eq!(decode_color_config(&trailing), Err(DecodeError::Malformed));
    for command in [
        Command::Set {
            value: Value::Empty,
            if_revision: None,
        },
        Command::Reset {
            if_revision: Some(8),
        },
        Command::Cancel,
        Command::Focus(Field::Hex),
        Command::Focus(Field::Channel(Channel::Hue)),
        Command::Focus(Field::Channel(Channel::Saturation)),
        Command::Focus(Field::Channel(Channel::Lightness)),
        Command::Focus(Field::Channel(Channel::Alpha)),
        Command::ReadSnapshot,
    ] {
        let mut b = bytes(&command);
        assert_eq!(decode_color_command(&b), Ok(command));
        b.push(0);
        assert!(decode_color_command(&b).is_err());
    }
    let idle = Snapshot {
        value: snapshot().value,
        committed: snapshot().value,
        interaction: None,
        draft: None,
        ..snapshot()
    };
    let started = Snapshot {
        committed: snapshot().value,
        interaction: Some(Interaction {
            id: 8,
            kind: InteractionKind::Text(Field::Hex),
        }),
        ..snapshot()
    };
    let mut events = vec![
        Event::Observed(snapshot()),
        Event::Started(started),
        Event::Preview(snapshot()),
    ];
    for channel in [
        Channel::Hue,
        Channel::Saturation,
        Channel::Lightness,
        Channel::Alpha,
    ] {
        events.push(Event::Preview(Snapshot {
            interaction: Some(Interaction {
                id: 5,
                kind: InteractionKind::Drag(channel),
            }),
            draft: None,
            ..snapshot()
        }));
        for (text, status) in [
            ("", DraftStatus::Empty),
            ("-", DraftStatus::Incomplete),
            ("nan", DraftStatus::Invalid),
            ("401", DraftStatus::OutOfRange),
            ("50", DraftStatus::Valid),
            ("50", DraftStatus::Forbidden),
        ] {
            events.push(Event::Preview(Snapshot {
                interaction: Some(Interaction {
                    id: 5,
                    kind: InteractionKind::Text(Field::Channel(channel)),
                }),
                draft: Some(Draft {
                    text: text.into(),
                    composing: true,
                    status,
                }),
                ..snapshot()
            }));
        }
    }
    for source in [
        Source::Pointer,
        Source::Keyboard,
        Source::Accessibility,
        Source::Text,
        Source::Palette,
        Source::Clear,
    ] {
        events.push(Event::Committed(source, idle.clone()));
    }
    for reason in [
        CancelReason::Escape,
        CancelReason::ConfigurationChanged,
        CancelReason::Disabled,
        CancelReason::ReadOnly,
        CancelReason::Hidden,
        CancelReason::Modal,
        CancelReason::WindowInactive,
        CancelReason::Unmounted,
        CancelReason::Programmatic,
        CancelReason::Interrupted,
    ] {
        events.push(Event::Cancelled(reason, idle.clone()));
    }
    for event in events {
        let mut b = bytes(&event);
        assert_eq!(decode_color_event(&b), Ok(event));
        b.push(0);
        assert!(decode_color_event(&b).is_err());
    }
    for error in [
        Error::NotMounted,
        Error::Closed,
        Error::StaleColorInput,
        Error::StaleRevision,
        Error::StaleInteraction,
        Error::Disabled,
        Error::ReadOnly,
        Error::FocusBlocked,
        Error::Busy,
        Error::InvalidValue,
        Error::InvalidConfig,
        Error::InvalidDraft,
        Error::Composing,
        Error::LimitExceeded,
        Error::NativeFailure,
    ] {
        let r = Response::Failed(error);
        assert_eq!(decode_color_response(&bytes(&r)), Ok(r));
    }
    let applied = Response::Applied(idle.clone());
    assert_eq!(decode_color_response(&bytes(&applied)), Ok(applied));
    for invalid in [
        Snapshot {
            revision: -1,
            ..snapshot()
        },
        Snapshot {
            value: color(0xff0000ff),
            ..snapshot()
        },
        Snapshot {
            interaction: Some(Interaction {
                id: 9,
                kind: InteractionKind::Text(Field::Hex),
            }),
            ..snapshot()
        },
        Snapshot {
            interaction: Some(Interaction {
                id: 0,
                kind: InteractionKind::Text(Field::Hex),
            }),
            ..snapshot()
        },
        Snapshot {
            interaction: Some(Interaction {
                id: 5,
                kind: InteractionKind::Drag(Channel::Hue),
            }),
            ..snapshot()
        },
        Snapshot {
            interaction: None,
            ..snapshot()
        },
        Snapshot {
            draft: None,
            ..snapshot()
        },
        Snapshot {
            draft: Some(Draft {
                text: "a\0b".into(),
                composing: false,
                status: DraftStatus::Invalid,
            }),
            ..snapshot()
        },
        Snapshot {
            committed_allowed: false,
            ..idle.clone()
        },
    ] {
        assert!(!invalid.is_valid());
        assert!(decode_color_event(&bytes(&Event::Observed(invalid))).is_err());
    }
    for invalid in [
        Event::Started(snapshot()),
        Event::Preview(idle.clone()),
        Event::Committed(Source::Text, snapshot()),
        Event::Cancelled(CancelReason::Escape, snapshot()),
    ] {
        assert!(decode_color_event(&bytes(&invalid)).is_err());
    }
    for c in [
        Command::Reset {
            if_revision: Some(-1),
        },
        Command::Set {
            value: Value::Empty,
            if_revision: Some(-1),
        },
    ] {
        assert!(decode_color_command(&bytes(&c)).is_err());
    }
    assert!(decode_color_event(&[5]).is_err());
    assert!(decode_color_response(&[1, 15]).is_err());
    assert!(decode_color_command(&[3, 1, 4]).is_err());
    for value in [-1i64, 0x1_0000_0000, i64::MAX] {
        let mut b = vec![0, 1];
        value.binprot_write(&mut b).unwrap();
        b.push(0);
        assert!(decode_color_command(&b).is_err());
    }
    // Rewrite each HSLA field in an otherwise valid independently-sized event.
    let good = bytes(&Event::Preview(snapshot()));
    let hsl_offset = 1
        + bytes(&8i64).len()
        + bytes(&snapshot().value).len()
        + bytes(&snapshot().committed).len();
    for channel in 0..4 {
        for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1., 361.] {
            let mut b = good.clone();
            b[hsl_offset + 8 * channel..hsl_offset + 8 * (channel + 1)]
                .copy_from_slice(&invalid.to_le_bytes());
            assert!(decode_color_event(&b).is_err());
        }
    }
}

#[test]
fn maximum_configs_and_drafts_fit_bounded_envelopes_and_excess_lengths_reject() {
    let mut c = config();
    let label = "L".repeat(MAX_LABEL_BYTES);
    c.labels = Labels {
        control: label.clone(),
        hue: label.clone(),
        saturation: label.clone(),
        lightness: label.clone(),
        alpha: label.clone(),
        hex: label.clone(),
        clear: label,
    };
    c.palette = vec![
        PaletteEntry {
            color: Rgba::new(255, 255, 255, 255),
            label: "P".repeat(MAX_PALETTE_LABEL_BYTES)
        };
        MAX_PALETTE_ENTRIES
    ];
    let encoded = bytes(&c);
    assert_eq!(encoded.len(), 97308);
    assert!(encoded.len() <= MAX_CONFIG_BYTES);
    assert_eq!(decode_color_config(&encoded), Ok(c.clone()));
    c.palette.push(c.palette[0].clone());
    assert_eq!(
        decode_color_config(&bytes(&c)),
        Err(DecodeError::LimitExceeded)
    );
    let mut c = config();
    c.labels.hue = "x".repeat(4097);
    assert_eq!(
        decode_color_config(&bytes(&c)),
        Err(DecodeError::LimitExceeded)
    );
    let mut c = config();
    c.palette[0].label = "x".repeat(257);
    assert_eq!(
        decode_color_config(&bytes(&c)),
        Err(DecodeError::LimitExceeded)
    );
    let mut c = config();
    c.labels.hue = "\0".into();
    assert_eq!(decode_color_config(&bytes(&c)), Err(DecodeError::Malformed));
    let mut c = config();
    c.labels.hue = " \t".into();
    assert_eq!(decode_color_config(&bytes(&c)), Err(DecodeError::Malformed));
    for (text, status) in [
        ("".to_owned(), DraftStatus::Empty),
        ("#".to_owned(), DraftStatus::Incomplete),
        ("x".repeat(4096), DraftStatus::Invalid),
        ("#0000".to_owned(), DraftStatus::Forbidden),
        ("#1234".to_owned(), DraftStatus::Valid),
    ] {
        let event = Event::Preview(Snapshot {
            draft: Some(Draft {
                text,
                composing: true,
                status,
            }),
            ..snapshot()
        });
        assert!(bytes(&event).len() <= MAX_EVENT_BYTES);
        assert_eq!(decode_color_event(&bytes(&event)), Ok(event));
    }
    let event = Event::Preview(Snapshot {
        draft: Some(Draft {
            text: "x".repeat(4097),
            composing: false,
            status: DraftStatus::Invalid,
        }),
        ..snapshot()
    });
    assert_eq!(
        decode_color_event(&bytes(&event)),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_color_config(&vec![0; MAX_CONFIG_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_color_event(&vec![0; MAX_EVENT_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_color_command(&[0; MAX_COMMAND_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    // Nat0 length declaration is rejected before text allocation, even in tiny input.
    let mut huge = vec![0xfc];
    huge.extend_from_slice(&u64::MAX.to_le_bytes());
    assert_eq!(decode_color_config(&huge), Err(DecodeError::LimitExceeded));
}

#[test]
fn correlated_commands_use_appended_tags_and_strict_admission() {
    use gpuio_protocol::{NodeId, WindowId, decode, v1};
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let command = Command::Set {
        value: color(0x11223344),
        if_revision: Some(7),
    };
    let message = v1::Message::ColorInputCommand(9, window, node, command);
    let encoded = fixture(
        &message,
        include_str!("../../../test/fixtures/color-command-request.hex"),
    );
    assert_eq!(decode(&encoded), Ok(message));
    for length in 0..encoded.len() {
        assert!(decode(&encoded[..length]).is_err());
    }
    let mut trailing = encoded;
    trailing.push(0);
    assert!(decode(&trailing).is_err());
    for correlation in [0, -1, i64::MIN] {
        assert!(
            decode(&bytes(&v1::Message::ColorInputCommand(
                correlation,
                window,
                node,
                Command::ReadSnapshot
            )))
            .is_err()
        );
    }
    for command in [
        Command::Set {
            value: color(0x11223344),
            if_revision: Some(-1),
        },
        Command::Reset {
            if_revision: Some(-1),
        },
    ] {
        assert!(
            decode(&bytes(&v1::Message::ColorInputCommand(
                9, window, node, command
            )))
            .is_err()
        );
    }
    fixture(
        &vec![v1::Event::ColorInputResult(
            9,
            window,
            node,
            Response::Applied(snapshot()),
        )],
        include_str!("../../../test/fixtures/color-command-events.hex"),
    );
}

#[test]
fn color_capability_uses_the_shared_64_bit_handshake() {
    use gpuio_protocol::{decode, v1::*};
    assert_eq!(CAPABILITIES & CAP_COLOR_INPUTS, 1_i64 << 37);
    let hello = Message::Hello(VERSION, CAPABILITIES);
    let encoded = fixture(&hello, "0001fcffffffffffff1f00");
    assert_eq!(decode(&encoded), Ok(hello));
}
