use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, decode_highlight_config, highlight::*};

fn fixture() -> Config {
    Config(vec![Spec {
        query: Some(Query {
            text: "café".into(),
            case_sensitive: false,
            whole_word: true,
        }),
        ranges: vec![
            Range {
                start_byte: 0,
                end_byte: 5,
            },
            Range {
                start_byte: 6,
                end_byte: 9,
            },
        ],
        appearance: Appearance {
            color: 1,
            active_color: 2,
            radius: 2.0,
        },
        active_index: Some(130),
        match_index_offset: 129,
    }])
}
fn encode(c: &Config) -> Vec<u8> {
    let mut bytes = Vec::new();
    c.binprot_write(&mut bytes).unwrap();
    bytes
}
fn rejected(spec: Spec) {
    assert!(!spec.is_valid());
    assert!(decode_highlight_config(&encode(&Config(vec![spec]))).is_err());
}

#[test]
fn independent_ocaml_bytes_and_strict_decoding() {
    let c = fixture();
    let bytes = encode(&c);
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex,
        "010105636166c3a9000102000506090102000000000000004001fe8200fe8100"
    );
    assert_eq!(decode_highlight_config(&bytes), Ok(c));
    for n in 0..bytes.len() {
        assert!(decode_highlight_config(&bytes[..n]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(decode_highlight_config(&extra).is_err());
    assert_eq!(decode_highlight_config(&[0]), Ok(Config(vec![])));
    for index in [1, 8, 9, 25] {
        // option, booleans and active option tags
        let mut malformed = bytes.clone();
        malformed[index] = 2;
        assert!(
            decode_highlight_config(&malformed).is_err(),
            "tag at{index}"
        );
    }
    let mut malformed = bytes;
    malformed[3] = 0xff;
    assert!(decode_highlight_config(&malformed).is_err());
    assert_eq!(
        decode_highlight_config(&vec![0; MAX_CONFIG_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_highlight_config(&[17]),
        Err(DecodeError::LimitExceeded)
    );
    assert!(decode_highlight_config(&[1, 1, 0xfe, 1, 16]).is_err()); // query4097
    assert!(decode_highlight_config(&[1, 0, 0xfe, 1, 16]).is_err()); // ranges4097
}

#[test]
fn constructors_and_decoding_agree_on_invalid_values() {
    let base = fixture().0.remove(0);
    for text in [
        String::new(),
        "bad\0query".into(),
        "x".repeat(MAX_QUERY_BYTES + 1),
    ] {
        let mut spec = base.clone();
        spec.query.as_mut().unwrap().text = text;
        rejected(spec);
    }
    rejected(Spec {
        query: None,
        ranges: vec![],
        ..base.clone()
    });
    for (start_byte, end_byte) in [(-1, 1), (0, 0), (2, 1)] {
        rejected(Spec {
            ranges: vec![Range {
                start_byte,
                end_byte,
            }],
            ..base.clone()
        });
    }
    for color in [-1, 0x1_0000_0000] {
        rejected(Spec {
            appearance: Appearance {
                color,
                ..base.appearance
            },
            ..base.clone()
        });
        rejected(Spec {
            appearance: Appearance {
                active_color: color,
                ..base.appearance
            },
            ..base.clone()
        });
    }
    for radius in [-1., 64.01, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        rejected(Spec {
            appearance: Appearance {
                radius,
                ..base.appearance
            },
            ..base.clone()
        });
    }
    rejected(Spec {
        active_index: Some(-1),
        ..base.clone()
    });
    rejected(Spec {
        match_index_offset: -1,
        ..base
    });
}

#[test]
fn aggregate_ranges_and_worst_valid_encoding_are_bounded() {
    let mut base = fixture().0.remove(0);
    base.query.as_mut().unwrap().text = "x".repeat(MAX_QUERY_BYTES);
    base.ranges = vec![
        Range {
            start_byte: i64::MAX - 1,
            end_byte: i64::MAX
        };
        MAX_RANGES / MAX_SPECS
    ];
    base.appearance.color = 0xffff_ffff;
    base.appearance.active_color = 0xffff_ffff;
    base.active_index = Some(i64::MAX);
    base.match_index_offset = i64::MAX;
    let mut c = Config(vec![base; MAX_SPECS]);
    assert!(c.is_valid());
    let bytes = encode(&c);
    assert!(bytes.len() < MAX_CONFIG_BYTES);
    assert_eq!(decode_highlight_config(&bytes), Ok(c.clone()));
    c.0[0].ranges.push(Range {
        start_byte: 0,
        end_byte: 1,
    });
    assert!(!c.is_valid());
    assert_eq!(
        decode_highlight_config(&encode(&c)),
        Err(DecodeError::LimitExceeded)
    );
}

#[test]
fn cosmetic_changes_preserve_match_identity_and_indices_do_not_overflow() {
    let original = fixture();
    let mut c = original.clone();
    c.0[0].appearance.radius = 64.;
    c.0[0].appearance.color = 3;
    c.0[0].appearance.active_color = 4;
    c.0[0].active_index = Some(i64::MAX);
    c.0[0].match_index_offset = i64::MAX;
    assert!(c.same_matchers(&original));
    assert!(c.0[0].is_active(0));
    assert!(!c.0[0].is_active(1));
    assert!(!c.0[0].is_active(-1));
    c.0[0].active_index = Some(0);
    assert!(!c.0[0].is_active(0));
    c.0[0].query.as_mut().unwrap().case_sensitive = true;
    assert!(!c.same_matchers(&original));
    assert!(!Config(vec![]).same_matchers(&original));
    assert!(original.0[0].is_active(1));
    assert!(!original.0[0].is_active(0));
}
