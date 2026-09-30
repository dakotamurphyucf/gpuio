use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId, command_binding::*, decode_command_binding_config,
    decode_command_binding_observation, v1::*,
};
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn fixture(text: &str) -> Vec<u8> {
    text.trim()
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn config() -> Config {
    Config {
        context: Context::Focused,
        targets: vec![
            Target::Command("run".into()),
            Target::NativeAction(NativeCommand::Copy),
        ],
    }
}
fn candidate() -> Candidate {
    Candidate {
        shortcut: Shortcut {
            key: "k".into(),
            modifiers: vec![ShortcutModifier::Primary],
            priority: ShortcutPriority::Override,
            text_input: ShortcutTextInput::ModifiedOnly,
            during_composition: false,
        },
        disposition: Disposition::Override,
    }
}
fn observation() -> Observation {
    Observation {
        epoch: 1,
        state: State::Ready(vec![
            Entry::Registry {
                enabled: true,
                candidates: vec![candidate()],
            },
            Entry::NativeBinding {
                strokes: vec![
                    Stroke {
                        key: "k".into(),
                        modifiers: 24,
                    },
                    Stroke {
                        key: "enter".into(),
                        modifiers: 0,
                    },
                ],
                disposition: Disposition::Widget,
            },
        ]),
    }
}
#[test]
fn independent_bytes_all_contexts_and_state_tags() {
    let config_bytes = fixture(include_str!(
        "../../../test/fixtures/command-binding-config.hex"
    ));
    let observation_bytes = fixture(include_str!(
        "../../../test/fixtures/command-binding-observation.hex"
    ));
    assert_eq!(bytes(&config()), config_bytes);
    assert_eq!(bytes(&observation()), observation_bytes);
    assert_eq!(
        decode_command_binding_config(&config_bytes).unwrap(),
        config()
    );
    assert_eq!(
        decode_command_binding_observation(&observation_bytes).unwrap(),
        observation()
    );
    assert!(observation().valid_for(&config()));
    let contexts = [
        (Context::Here, "0101000372756e"),
        (
            Context::Editor(
                WindowId::from_parts(0, 1).unwrap(),
                NodeId::from_parts(4, 7).unwrap(),
            ),
            "020001040701000372756e",
        ),
    ];
    for (context, expected) in contexts {
        let config = Config {
            context,
            targets: vec![Target::Command("run".into())],
        };
        assert_eq!(bytes(&config), fixture(expected));
        assert_eq!(
            decode_command_binding_config(&bytes(&config)).unwrap(),
            config
        );
    }
    let native = Config {
        context: Context::NativeContext("Input".into()),
        targets: vec![Target::NativeAction(NativeCommand::Copy)],
    };
    assert_eq!(bytes(&native), fixture("0305496e707574010100"));
    assert_eq!(
        decode_command_binding_config(&bytes(&native)).unwrap(),
        native
    );
    for (state, expected) in [
        (State::Suspended, "0101"),
        (State::ContextGone, "0102"),
        (State::InvalidContext, "0103"),
        (State::EpochExhausted, "0104"),
        (State::Capacity, "0105"),
    ] {
        let value = Observation { epoch: 1, state };
        assert_eq!(bytes(&value), fixture(expected));
        assert_eq!(
            decode_command_binding_observation(&bytes(&value)).unwrap(),
            value
        );
    }
    for (entry, expected) in [
        (Entry::MissingCommand, "01000100"),
        (Entry::NativeUnbound, "01000102"),
        (
            Entry::NativeUnsupported(Unsupported::SequenceTooLong),
            "0100010400",
        ),
        (
            Entry::NativeUnsupported(Unsupported::InvalidStroke),
            "0100010401",
        ),
    ] {
        let value = Observation {
            epoch: 1,
            state: State::Ready(vec![entry]),
        };
        assert_eq!(bytes(&value), fixture(expected));
        assert_eq!(
            decode_command_binding_observation(&bytes(&value)).unwrap(),
            value
        );
    }
}
#[test]
fn context_target_domains_and_duplicate_bounds() {
    for targets in [
        vec![],
        vec![Target::Command("run".into()); 2],
        vec![Target::NativeAction(NativeCommand::Copy); 2],
        (0..65).map(|i| Target::Command(i.to_string())).collect(),
    ] {
        let value = Config {
            context: Context::Focused,
            targets,
        };
        assert!(!value.is_valid());
        assert!(decode_command_binding_config(&bytes(&value)).is_err());
    }
    let exact = Config {
        context: Context::Focused,
        targets: (0..64).map(|i| Target::Command(i.to_string())).collect(),
    };
    assert!(exact.is_valid());
    assert_eq!(
        decode_command_binding_config(&bytes(&exact)).unwrap(),
        exact
    );
    for (context, target) in [
        (Context::Here, Target::NativeAction(NativeCommand::Copy)),
        (
            Context::NativeContext("Input".into()),
            Target::Command("run".into()),
        ),
    ] {
        let value = Config {
            context,
            targets: vec![target],
        };
        assert!(!value.is_valid());
        assert!(decode_command_binding_config(&bytes(&value)).is_err());
    }
    for context in [
        "".into(),
        "  ".into(),
        "bad\0context".into(),
        "x".repeat(1025),
    ] {
        let value = Config {
            context: Context::NativeContext(context),
            targets: vec![Target::NativeAction(NativeCommand::Copy)],
        };
        assert!(!value.is_valid());
        assert!(decode_command_binding_config(&bytes(&value)).is_err());
    }
}
#[test]
fn provenance_priority_disabled_conflicts_and_context_states_are_checked() {
    let commands = Config {
        context: Context::Focused,
        targets: vec![Target::Command("run".into())],
    };
    let natives = Config {
        context: Context::Focused,
        targets: vec![Target::NativeAction(NativeCommand::Copy)],
    };
    let ready = |entry| Observation {
        epoch: 1,
        state: State::Ready(vec![entry]),
    };
    assert!(ready(Entry::MissingCommand).valid_for(&commands));
    assert!(!ready(Entry::NativeUnbound).valid_for(&commands));
    assert!(!ready(Entry::MissingCommand).valid_for(&natives));
    assert!(ready(Entry::NativeUnsupported(Unsupported::InvalidStroke)).valid_for(&natives));
    for (enabled, disposition, valid) in [
        (true, Disposition::Override, true),
        (false, Disposition::Unavailable(Suppression::Disabled), true),
        (
            true,
            Disposition::Unavailable(Suppression::Conflict("other".into())),
            true,
        ),
        (true, Disposition::Declared, false),
        (true, Disposition::NativeFirst, false),
        (false, Disposition::Override, false),
        (true, Disposition::Widget, false),
        (true, Disposition::Unavailable(Suppression::Disabled), false),
        (
            true,
            Disposition::Unavailable(Suppression::Conflict("run".into())),
            false,
        ),
    ] {
        let value = ready(Entry::Registry {
            enabled,
            candidates: vec![Candidate {
                disposition,
                ..candidate()
            }],
        });
        assert_eq!(value.valid_for(&commands), valid);
    }
    let scoped = Config {
        context: Context::Here,
        ..commands.clone()
    };
    let value = ready(Entry::Registry {
        enabled: false,
        candidates: vec![Candidate {
            disposition: Disposition::Declared,
            ..candidate()
        }],
    });
    assert!(value.valid_for(&scoped));
    assert!(!value.valid_for(&commands));
    assert!(
        !Observation {
            epoch: 0,
            state: State::Suspended
        }
        .valid_for(&commands)
    );
    assert!(
        !Observation {
            epoch: 1,
            state: State::ContextGone
        }
        .valid_for(&commands)
    );
    assert!(
        !Observation {
            epoch: 1,
            state: State::InvalidContext
        }
        .valid_for(&natives)
    );
}
#[test]
fn bounded_readers_reject_truncation_trailing_lengths_tags_and_nested_payloads() {
    let config_bytes = bytes(&config());
    let observation_bytes = bytes(&observation());
    for end in 0..config_bytes.len() {
        assert!(decode_command_binding_config(&config_bytes[..end]).is_err());
    }
    for end in 0..observation_bytes.len() {
        assert!(decode_command_binding_observation(&observation_bytes[..end]).is_err());
    }
    assert!(decode_command_binding_config(&[config_bytes.as_slice(), &[0]].concat()).is_err());
    assert!(
        decode_command_binding_observation(&[observation_bytes.as_slice(), &[0]].concat()).is_err()
    );
    assert!(decode_command_binding_config(&vec![0; MAX_CONFIG_BYTES + 1]).is_err());
    assert!(decode_command_binding_observation(&vec![0; MAX_OBSERVATION_BYTES + 1]).is_err());
    for raw in [
        vec![0, 65],
        vec![255],
        vec![0, 1, 255],
        vec![3, 254, 255, 255],
    ] {
        assert!(decode_command_binding_config(&raw).is_err());
    }
    for raw in [
        vec![1, 0, 65],
        vec![1, 255],
        vec![1, 0, 1, 255],
        vec![1, 0, 1, 3, 255],
    ] {
        assert!(decode_command_binding_observation(&raw).is_err());
    }
    let native = |strokes| Observation {
        epoch: 1,
        state: State::Ready(vec![Entry::NativeBinding {
            strokes,
            disposition: Disposition::Widget,
        }]),
    };
    for modifiers in [-1, 32, i64::MAX] {
        assert!(
            decode_command_binding_observation(&bytes(&native(vec![Stroke {
                key: "k".into(),
                modifiers
            }])))
            .is_err()
        );
    }
    for key in [
        "".into(),
        "\u{1}".into(),
        "\u{7f}".into(),
        "\u{80}".into(),
        "bad\0key".into(),
        "x".repeat(257),
    ] {
        assert!(
            decode_command_binding_observation(&bytes(&native(vec![Stroke { key, modifiers: 0 }])))
                .is_err()
        );
    }
    for length in [0, 9] {
        assert!(
            decode_command_binding_observation(&bytes(&native(vec![
                Stroke {
                    key: "k".into(),
                    modifiers: 0
                };
                length
            ])))
            .is_err()
        );
    }
    let too_many = Observation {
        epoch: 1,
        state: State::Ready(vec![Entry::Registry {
            enabled: true,
            candidates: vec![candidate(); 5],
        }]),
    };
    assert!(decode_command_binding_observation(&bytes(&too_many)).is_err());
    let too_many = Observation {
        epoch: 1,
        state: State::Ready(vec![Entry::NativeUnbound; 65]),
    };
    assert!(decode_command_binding_observation(&bytes(&too_many)).is_err());
    for key in ["K".into(), "xx".into(), "\u{80}".into(), "k".repeat(257)] {
        let mut candidate = candidate();
        candidate.shortcut.key = key;
        assert!(
            decode_command_binding_observation(&bytes(&Observation {
                epoch: 1,
                state: State::Ready(vec![Entry::Registry {
                    enabled: true,
                    candidates: vec![candidate]
                }])
            }))
            .is_err()
        );
    }
}

#[test]
fn independent_dispositions_exact_limits_and_modifier_ordering() {
    for (disposition, suffix) in [
        (Disposition::Declared, "00"),
        (Disposition::Override, "01"),
        (Disposition::NativeFirst, "02"),
        (Disposition::Widget, "03"),
        (Disposition::Unavailable(Suppression::Disabled), "0400"),
        (Disposition::Unavailable(Suppression::ScopeBlocked), "0401"),
        (
            Disposition::Unavailable(Suppression::NativeUnavailable),
            "0402",
        ),
        (Disposition::Unavailable(Suppression::Composition), "0403"),
        (Disposition::Unavailable(Suppression::TextInput), "0404"),
        (
            Disposition::Unavailable(Suppression::NativeNavigation),
            "0405",
        ),
        (
            Disposition::Unavailable(Suppression::Conflict("other".into())),
            "0406056f74686572",
        ),
    ] {
        let value = Observation {
            epoch: 1,
            state: State::Ready(vec![Entry::NativeBinding {
                strokes: vec![Stroke {
                    key: "k".into(),
                    modifiers: 0,
                }],
                disposition,
            }]),
        };
        assert_eq!(bytes(&value), fixture(&format!("0100010301016b00{suffix}")));
        assert_eq!(
            decode_command_binding_observation(&bytes(&value)).unwrap(),
            value
        );
    }
    let exact = Observation {
        epoch: i64::MAX,
        state: State::Ready(vec![
            Entry::NativeBinding {
                strokes: vec![
                    Stroke {
                        key: "k".into(),
                        modifiers: 31
                    };
                    8
                ],
                disposition: Disposition::Widget,
            };
            64
        ]),
    };
    assert_eq!(
        decode_command_binding_observation(&bytes(&exact)).unwrap(),
        exact
    );
    for modifiers in [
        vec![ShortcutModifier::Primary, ShortcutModifier::Primary],
        vec![ShortcutModifier::Super, ShortcutModifier::Control],
    ] {
        let mut candidate = candidate();
        candidate.shortcut.modifiers = modifiers;
        let value = Observation {
            epoch: 1,
            state: State::Ready(vec![Entry::Registry {
                enabled: true,
                candidates: vec![candidate],
            }]),
        };
        assert!(decode_command_binding_observation(&bytes(&value)).is_err());
    }
    for (context, state) in [
        (Context::Focused, State::Capacity),
        (
            Context::Editor(
                WindowId::from_parts(0, 1).unwrap(),
                NodeId::from_parts(1, 1).unwrap(),
            ),
            State::ContextGone,
        ),
        (
            Context::NativeContext("Input mode=visible".into()),
            State::InvalidContext,
        ),
    ] {
        assert!(Observation { epoch: 1, state }.valid_for(&Config {
            context,
            targets: vec![Target::NativeAction(NativeCommand::Copy)],
        }));
    }
}

#[test]
fn mounted_envelopes_append_without_changing_existing_tags() {
    let node = NodeId::from_parts(0, 1).unwrap();
    let window = WindowId::from_parts(0, 1).unwrap();
    let handler = gpuio_protocol::HandlerId::from_parts(0, 1).unwrap();
    let op = Op::SetCommandBinding(node, Some(config()));
    assert_eq!(bytes(&op), fixture("3e0001010002000372756e0100"));
    assert_eq!(
        bytes(&Op::SetCommandBinding(node, None)),
        fixture("3e000100")
    );
    let request = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![op],
    });
    let golden = fixture("0300010001013e0001010002000372756e0100");
    assert_eq!(bytes(&request), golden);
    assert_eq!(gpuio_protocol::decode(&golden).unwrap(), request);
    let event = Event::CommandBindingObserved(
        window,
        node,
        handler,
        1,
        Observation {
            epoch: 1,
            state: State::Suspended,
        },
    );
    assert_eq!(bytes(&vec![event]), fixture("0142000100010001010101"));
}

#[test]
fn capability_bit_and_current_mask_have_independent_bytes() {
    assert_eq!(CAP_COMMAND_BINDINGS, 1_i64 << 52);
    assert_eq!(
        bytes(&Message::Hello(VERSION, CAP_COMMAND_BINDINGS)),
        fixture("0001fc0000000000001000")
    );
    assert_eq!(
        bytes(&Message::Hello(VERSION, CAPABILITIES)),
        fixture("0001fcffffffffffff1f00")
    );
}
