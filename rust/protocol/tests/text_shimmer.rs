use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, decode_text_shimmer_config as decode, text_shimmer::*};

fn minimal() -> Config {
    Config {
        duration_ms: 1,
        spread: Spread::Relative(0.5),
        direction: Direction::LeftToRight,
        repeat: Repeat::Loop,
        animated: true,
        highlight: None,
        appearance: None,
    }
}

fn explicit() -> Config {
    Config {
        duration_ms: 60_000,
        spread: Spread::Pixels(128.5),
        direction: Direction::RightToLeft,
        repeat: Repeat::Once,
        animated: false,
        highlight: Some(0xffff_ffff),
        appearance: None,
    }
}

fn encode(config: &Config) -> Vec<u8> {
    let mut bytes = vec![];
    config.binprot_write(&mut bytes).unwrap();
    bytes
}

#[test]
fn independent_ocaml_fixtures_and_strict_tags() {
    for (config, expected) in [
        (minimal(), "0100000000000000e03f0001010000"),
        (
            explicit(),
            "fd60ea000001000000000010604001000001fcffffffff0000000000",
        ),
    ] {
        let bytes = encode(&config);
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, expected);
        assert_eq!(decode(&bytes), Ok(config));
        for n in 0..bytes.len() {
            assert!(decode(&bytes[..n]).is_err(), "truncation at {n}");
        }
        let mut extra = bytes;
        extra.push(0);
        assert_eq!(decode(&extra), Err(DecodeError::Malformed));
    }
    for index in [1, 10, 11, 12, 13] {
        for tag in [2, 255] {
            let mut malformed = encode(&minimal());
            malformed[index] = tag;
            assert_eq!(decode(&malformed), Err(DecodeError::Malformed));
        }
    }
    assert_eq!(
        decode(&[0; MAX_CONFIG_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
}

#[test]
fn native_admission_and_decode_reject_invalid_values() {
    let reject = |config: Config| {
        assert!(!config.is_valid());
        assert!(decode(&encode(&config)).is_err());
    };
    for duration_ms in [i64::MIN, -1, 0, 60_001, i64::MAX] {
        reject(Config {
            duration_ms,
            ..minimal()
        });
    }
    for color in [-1, 0x1_0000_0000, i64::MAX] {
        reject(Config {
            highlight: Some(color),
            ..minimal()
        });
    }
    for value in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.,
        0.,
        0.049,
        1.001,
    ] {
        reject(Config {
            spread: Spread::Relative(value),
            ..minimal()
        });
    }
    for value in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.,
        0.,
        0.999,
        1_000_001.,
    ] {
        reject(Config {
            spread: Spread::Pixels(value),
            ..minimal()
        });
    }
}

#[test]
fn configuration_cross_product_has_bounded_wire_and_retained_size() {
    let mut count = 0;
    for duration_ms in [1, 127, 128, 32_767, 32_768, 60_000] {
        for spread in [
            Spread::Relative(0.05),
            Spread::Relative(1.),
            Spread::Pixels(1.),
            Spread::Pixels(1_000_000.),
        ] {
            for direction in [Direction::LeftToRight, Direction::RightToLeft] {
                for repeat in [Repeat::Once, Repeat::Loop] {
                    for animated in [false, true] {
                        for highlight in [None, Some(0), Some(0xffff_ffff)] {
                            let config = Config {
                                duration_ms,
                                spread,
                                direction,
                                repeat,
                                animated,
                                highlight,
                                appearance: None,
                            };
                            assert!(config.is_valid());
                            let bytes = encode(&config);
                            assert!(bytes.len() <= MAX_CONFIG_BYTES);
                            assert_eq!(decode(&bytes), Ok(config));
                            assert_eq!(config.retained_bytes(), std::mem::size_of::<Config>());
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(count, 576);
}

#[test]
fn op61_set_clear_match_ocaml_and_reject_malformed_live_configs() {
    use gpuio_protocol::{NodeId, WindowId, v1::*};
    let packet = |config| {
        Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![Op::SetTextShimmer(
                NodeId::from_parts(0, 1).unwrap(),
                config,
            )],
        })
    };
    let bytes = |packet: &Message| {
        let mut bytes = vec![];
        packet.binprot_write(&mut bytes).unwrap();
        bytes
    };
    for (config, expected) in [
        (
            Some(minimal()),
            "0300010001013d0001010100000000000000e03f0001010000",
        ),
        (None, "0300010001013d000100"),
    ] {
        let message = packet(config);
        let encoded = bytes(&message);
        assert_eq!(
            encoded
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            expected
        );
        assert_eq!(gpuio_protocol::decode(&encoded), Ok(message));
        for end in 0..encoded.len() {
            assert!(gpuio_protocol::decode(&encoded[..end]).is_err());
        }
        let mut trailing = encoded;
        trailing.push(0);
        assert!(gpuio_protocol::decode(&trailing).is_err());
    }
    // Operation option, then configuration spread/direction/repeat/Boolean/color option.
    for index in [9, 11, 20, 21, 22, 23] {
        let mut malformed = bytes(&packet(Some(minimal())));
        malformed[index] = 2;
        assert!(
            gpuio_protocol::decode(&malformed).is_err(),
            "tag at {index}"
        );
    }
    for config in [
        Config {
            duration_ms: 0,
            ..minimal()
        },
        Config {
            spread: Spread::Relative(f64::NAN),
            ..minimal()
        },
        Config {
            highlight: Some(-1),
            ..minimal()
        },
    ] {
        assert!(gpuio_protocol::decode(&bytes(&packet(Some(config)))).is_err());
    }
}

#[test]
fn explicit_appearance_matches_ocaml_and_validates_both_colors_and_tags() {
    let config = Config {
        appearance: Some(Appearance {
            foreground: 1,
            background: 2,
            dark: true,
        }),
        ..minimal()
    };
    let bytes = encode(&config);
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        "0100000000000000e03f0001010001010201"
    );
    assert_eq!(decode(&bytes), Ok(config));
    for index in [14, 17] {
        let mut bad = bytes.clone();
        bad[index] = 2;
        assert!(decode(&bad).is_err());
    }
    for color in [-1, 0x1_0000_0000] {
        for appearance in [
            Appearance {
                foreground: color,
                background: 0,
                dark: true,
            },
            Appearance {
                foreground: 0,
                background: color,
                dark: false,
            },
        ] {
            let config = Config {
                appearance: Some(appearance),
                ..minimal()
            };
            assert!(!config.is_valid());
            assert!(decode(&encode(&config)).is_err());
        }
    }
}
