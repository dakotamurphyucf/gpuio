#[path = "common/input_fixture.rs"]
mod fixture;
use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, decode_input_config, decode_input_event, input::*, pointer::PointerButton,
};
fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn independent_bytes_all_payload_kinds_and_strict_bounded_decoding() {
    let c = fixture::config();
    let events = fixture::events();
    let lines: Vec<_> = std::iter::once(hex(&encode(&c)))
        .chain(events.iter().map(|e| hex(&encode(e))))
        .collect();
    assert_eq!(
        lines.join("\n"),
        include_str!("../../../test/fixtures/input-values.hex").trim()
    );
    let bytes = encode(&c);
    assert_eq!(decode_input_config(&bytes), Ok(c));
    for length in 0..bytes.len() {
        assert!(decode_input_config(&bytes[..length]).is_err());
    }
    let mut extra = bytes;
    extra.push(0);
    assert!(decode_input_config(&extra).is_err());
    for event in events {
        assert!(event.is_valid());
        let bytes = encode(&event);
        assert_eq!(decode_input_event(&bytes), Ok(event));
        for length in 0..bytes.len() {
            assert!(decode_input_event(&bytes[..length]).is_err());
        }
        let mut extra = bytes;
        extra.push(0);
        assert!(decode_input_event(&extra).is_err());
    }
    assert_eq!(
        decode_input_config(&vec![0; MAX_CONFIG_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_input_event(&vec![0; MAX_EVENT_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
}
#[test]
fn subscriptions_require_unique_canonical_kinds_and_real_native_policies() {
    let all = fixture::config();
    for subscription in &all.subscriptions {
        let derived = matches!(
            subscription.kind,
            Kind::Click
                | Kind::AuxiliaryClick
                | Kind::MouseEnter
                | Kind::MouseLeave
                | Kind::MouseDownOutside
                | Kind::Focus
                | Kind::Blur
        );
        for phase in [Phase::Capture, Phase::Bubble] {
            for policy in [
                Policy::Observe,
                Policy::StopPropagation,
                Policy::PreventDefault,
                Policy::PreventAndStop,
            ] {
                let mut c = all.clone();
                c.subscriptions = vec![Subscription {
                    phase,
                    policy,
                    ..*subscription
                }];
                assert_eq!(
                    decode_input_config(&encode(&c)).is_ok(),
                    !derived || (phase == Phase::Bubble && policy == Policy::Observe)
                );
            }
        }
    }
    let mut invalid = all.clone();
    invalid.subscriptions.reverse();
    assert!(decode_input_config(&encode(&invalid)).is_err());
    invalid = all.clone();
    invalid.subscriptions[1] = invalid.subscriptions[0];
    assert!(decode_input_config(&encode(&invalid)).is_err());
    invalid = all.clone();
    invalid.subscriptions.clear();
    assert!(decode_input_config(&encode(&invalid)).is_err());
    invalid = all.clone();
    invalid.focus = Focus::None;
    assert!(decode_input_config(&encode(&invalid)).is_err());
    for label in [
        "".to_string(),
        " \t\u{b}".into(),
        "bad\0name".into(),
        "a".repeat(4097),
    ] {
        invalid = all.clone();
        invalid.label = label;
        assert!(decode_input_config(&encode(&invalid)).is_err());
    }
    for label in ["\u{a0}".to_string(), "a".repeat(4096)] {
        let mut valid = all.clone();
        valid.label = label;
        assert!(decode_input_config(&encode(&valid)).is_ok());
    }
}
#[test]
fn malformed_samples_and_tags_do_not_enter_domain_state() {
    let mouse = Mouse {
        location: fixture::location(),
        button: PointerButton::Left,
        click_count: 1,
    };
    for count in [-1, 0, u32::MAX as i64 + 1] {
        assert!(
            decode_input_event(&encode(&Event::MouseDown(Mouse {
                click_count: count,
                ..mouse
            })))
            .is_err()
        );
    }
    assert!(decode_input_event(&encode(&Event::AuxiliaryClick(mouse))).is_err());
    assert!(
        decode_input_event(&encode(&Event::Click(Mouse {
            button: PointerButton::Right,
            ..mouse
        })))
        .is_err()
    );
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut m = mouse;
        m.location.local.x = value;
        assert!(decode_input_event(&encode(&Event::MouseUp(m))).is_err());
        assert!(
            decode_input_event(&encode(&Event::Scroll(Scroll {
                location: fixture::location(),
                delta: Delta::Pixels(Position { x: 0., y: value }),
                phase: TouchPhase::Moved
            })))
            .is_err()
        );
    }
    for text in ["".to_string(), "a\0b".into(), "x".repeat(257)] {
        let key = Key {
            key: text.clone(),
            character: None,
            modifiers: Default::default(),
        };
        assert!(decode_input_event(&encode(&Event::KeyUp(key))).is_err());
        let key = Key {
            key: "x".into(),
            character: Some(text),
            modifiers: Default::default(),
        };
        assert!(decode_input_event(&encode(&Event::KeyDown(key, false))).is_err());
    }
    for tag in 13..=255 {
        assert_eq!(decode_input_event(&[tag]), Err(DecodeError::Malformed));
    }
    // A hostile declared count is rejected before reserving a vector or string.
    assert!(
        decode_input_config(&[
            1, b'x', 0, 0, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff
        ])
        .is_err()
    );
    // Invalid UTF-8 in an otherwise well-shaped key-up sample.
    assert!(decode_input_event(&[9, 1, 0xff, 0, 0, 0, 0, 0, 0]).is_err());
    let mut bytes = encode(&Event::MouseDown(mouse));
    bytes[38] = 5;
    assert!(decode_input_event(&bytes).is_err());
}

#[test]
fn independent_policy_phase_focus_and_disabled_bytes() {
    let configs = fixture::policy_configs();
    let lines: Vec<_> = configs
        .iter()
        .map(|config| {
            let bytes = encode(config);
            assert_eq!(decode_input_config(&bytes).as_ref(), Ok(config));
            hex(&bytes)
        })
        .collect();
    assert_eq!(
        lines.join("\n"),
        include_str!("../../../test/fixtures/input-policies.hex").trim()
    );
}
