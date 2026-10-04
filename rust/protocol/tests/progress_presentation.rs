use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError,
    animation::Easing,
    decode_progress_presentation,
    progress::ProgressConfig,
    progress_presentation::{Config, MAX_CONFIG_BYTES, Shape, Transition},
};

fn config() -> Config {
    Config {
        progress: ProgressConfig {
            label: "Upload".into(),
            fraction: Some(0.25),
        },
        shape: Shape::Linear,
        transition: Transition::Immediate,
    }
}
fn encode(config: &Config) -> Vec<u8> {
    let mut bytes = Vec::new();
    config.binprot_write(&mut bytes).unwrap();
    bytes
}
fn paired(config: Config, fixture: &str) {
    let bytes = encode(&config);
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        fixture.trim()
    );
    assert_eq!(decode_progress_presentation(&bytes), Ok(config));
    for n in 0..bytes.len() {
        assert!(
            decode_progress_presentation(&bytes[..n]).is_err(),
            "prefix {n}"
        );
    }
    let mut bytes = bytes;
    bytes.push(0);
    assert!(decode_progress_presentation(&bytes).is_err());
}

#[test]
fn independent_ocaml_fixtures_keep_the_semantic_payload_and_all_field_order() {
    paired(
        config(),
        include_str!("../../../test/fixtures/progress-presentation-linear.hex"),
    );
    paired(
        Config {
            progress: ProgressConfig {
                label: "Indexing".into(),
                fraction: None,
            },
            shape: Shape::Circle,
            transition: Transition::Tween {
                duration_ms: 200,
                easing: Easing::EaseOut,
            },
        },
        include_str!("../../../test/fixtures/progress-presentation-circle.hex"),
    );
    let config = config();
    let mut legacy = Vec::new();
    config.progress.binprot_write(&mut legacy).unwrap();
    legacy.extend([0, 0]);
    assert_eq!(encode(&config), legacy);
    assert_eq!(
        config.retained_bytes(),
        std::mem::size_of::<Config>() + config.progress.label.capacity()
    );
}

#[test]
fn semantic_bounds_and_transition_validation_reject_malformed_configuration() {
    for label in ["".into(), " \t".into(), "a\0b".into(), "x".repeat(4097)] {
        let mut invalid = config();
        invalid.progress.label = label;
        assert!(!invalid.is_valid());
        assert!(decode_progress_presentation(&encode(&invalid)).is_err());
    }
    for fraction in [-0.1, 1.1, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut invalid = config();
        invalid.progress.fraction = Some(fraction);
        assert!(!invalid.is_valid());
        assert!(decode_progress_presentation(&encode(&invalid)).is_err());
    }
    for duration_ms in [i64::MIN, -1, 0, 60001, i64::MAX] {
        let invalid = Config {
            transition: Transition::Tween {
                duration_ms,
                easing: Easing::Linear,
            },
            ..config()
        };
        assert!(!invalid.is_valid());
        assert!(decode_progress_presentation(&encode(&invalid)).is_err());
    }
    for easing in [
        Easing::CubicBezier(-0.1, 0., 1., 1.),
        Easing::CubicBezier(0., 0., 1.1, 1.),
        Easing::CubicBezier(f64::NAN, 0., 1., 1.),
        Easing::CubicBezier(0., f64::INFINITY, 1., 1.),
    ] {
        let invalid = Config {
            transition: Transition::Tween {
                duration_ms: 200,
                easing,
            },
            ..config()
        };
        assert!(!invalid.is_valid());
        assert!(decode_progress_presentation(&encode(&invalid)).is_err());
    }
}

#[test]
fn valid_boundaries_and_all_easing_presets_decode_without_changing_semantics() {
    for shape in [Shape::Linear, Shape::Circle] {
        for fraction in [None, Some(0.), Some(1.)] {
            for duration_ms in [1, 60000] {
                for easing in [
                    Easing::Linear,
                    Easing::Ease,
                    Easing::EaseIn,
                    Easing::EaseOut,
                    Easing::EaseInOut,
                    Easing::CubicBezier(0., -f64::MAX, 1., f64::MAX),
                ] {
                    let valid = Config {
                        progress: ProgressConfig {
                            label: "界".repeat(1365) + "x",
                            fraction,
                        },
                        shape,
                        transition: Transition::Tween {
                            duration_ms,
                            easing,
                        },
                    };
                    assert_eq!(valid.progress.label.len(), 4096);
                    assert_eq!(decode_progress_presentation(&encode(&valid)), Ok(valid));
                }
            }
        }
    }
}

#[test]
fn decoder_rejects_tags_utf8_and_oversized_input_before_admission() {
    let bytes = encode(&config());
    // Independent positions: UTF-8 label, option tag, shape, transition.
    for (offset, tag) in [(1, 255), (7, 2), (16, 2), (17, 2)] {
        let mut invalid = bytes.clone();
        invalid[offset] = tag;
        assert!(
            decode_progress_presentation(&invalid).is_err(),
            "offset {offset}"
        );
    }
    let mut tween = encode(&Config {
        transition: Transition::Tween {
            duration_ms: 1,
            easing: Easing::Linear,
        },
        ..config()
    });
    *tween.last_mut().unwrap() = 6;
    assert!(decode_progress_presentation(&tween).is_err());
    assert_eq!(
        decode_progress_presentation(&vec![0; MAX_CONFIG_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
}

#[test]
fn atomic_operation_and_capability_match_independent_ocaml_fixture() {
    use gpuio_protocol::{NodeId, WindowId, decode, v1::*};
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetProgressPresentation(
            NodeId::from_parts(0, 1).unwrap(),
            config(),
        )],
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/progress-presentation-operation.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for n in 0..bytes.len() {
        assert!(decode(&bytes[..n]).is_err());
    }
    let mut hello = vec![];
    Message::Hello(VERSION, CAP_PROGRESS_PRESENTATION)
        .binprot_write(&mut hello)
        .unwrap();
    assert_eq!(hello, vec![0, 3, 252, 0, 0, 0, 0, 0, 0, 0, 8]);
    assert_eq!(CAPABILITIES, i64::MAX);
}
