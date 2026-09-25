use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, container_query::*, decode_container_query};

fn config() -> Config {
    Config {
        generation: 42,
        branches: vec!["compact".into(), "wide".into(), "tall".into()],
        default: 0,
        rules: vec![
            Rule {
                condition: Predicate {
                    width: Range {
                        minimum: 480.25,
                        maximum: None,
                    },
                    height: Range {
                        minimum: 200.,
                        maximum: None,
                    },
                },
                branch: 1,
            },
            Rule {
                condition: Predicate {
                    width: Range::ALL,
                    height: Range {
                        minimum: 600.,
                        maximum: None,
                    },
                },
                branch: 2,
            },
        ],
    }
}
fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
#[test]
fn fixture_and_bounded_decoder_agree_with_independent_ocaml_construction() {
    let config = config();
    let bytes = encode(&config);
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex,
        include_str!("../../../test/fixtures/container-query.hex").trim()
    );
    assert_eq!(decode_container_query(&bytes), Ok(config.clone()));
    for end in 0..bytes.len() {
        assert!(decode_container_query(&bytes[..end]).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(
        decode_container_query(&trailing),
        Err(DecodeError::Malformed)
    );
    // generation followed by branch count. Bounds precede any string allocation.
    assert_eq!(
        decode_container_query(&[42, 17]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_container_query(&[42, 1, 254, 129, 0]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_container_query(&[42, 1, 1, 0xff]),
        Err(DecodeError::Malformed)
    );
    let empty = Config {
        rules: vec![],
        ..config.clone()
    };
    let mut excess_rules = encode(&empty);
    *excess_rules.last_mut().unwrap() = 33;
    assert_eq!(
        decode_container_query(&excess_rules),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_container_query(&vec![0; MAX_CONFIG_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    assert!(config.retained_bytes() >= std::mem::size_of::<Config>() + 15);
}
#[test]
fn selection_has_half_open_boundaries_and_first_match_precedence() {
    let mut config = config();
    assert!(config.is_valid());
    for (width, height, selected) in [
        (480.24, 200., 0),
        (480.25, 200., 1),
        (480.26, 200., 1),
        (900., 199.99, 0),
        (100., 600., 2),
        (900., 600., 1),
    ] {
        assert_eq!(config.select(width, height), Some(selected));
    }
    config.rules[0].condition.width.maximum = Some(900.5);
    assert_eq!(config.select(900.49, 600.), Some(1));
    assert_eq!(config.select(900.5, 600.), Some(2));
    assert_eq!(config.select(901., 300.), Some(0));
    config.rules.swap(0, 1);
    assert_eq!(
        config.select(700., 600.),
        Some(2),
        "rule order controls overlap"
    );
    config.rules.clear();
    assert_eq!(config.select(0., 0.), Some(0));
    assert_eq!(config.select(f64::MAX, f64::MAX), Some(0));
    for bad in [-1., f64::NAN, f64::INFINITY] {
        assert_eq!(config.select(bad, 10.), None);
        assert_eq!(config.select(10., bad), None);
    }
}
#[test]
fn all_admission_invariants_are_checked() {
    let base = config();
    let mut invalid = Vec::new();
    for generation in [0, -1] {
        invalid.push(Config {
            generation,
            ..base.clone()
        });
    }
    for default in [-1, 3] {
        invalid.push(Config {
            default,
            ..base.clone()
        });
    }
    for branches in [
        vec![],
        vec!["".into()],
        vec!["a".into(), "a".into()],
        vec!["a\0b".into()],
        vec!["a".repeat(129)],
        (0..17).map(|i| i.to_string()).collect(),
    ] {
        invalid.push(Config {
            branches,
            ..base.clone()
        });
    }
    invalid.push(Config {
        rules: vec![base.rules[0].clone(); 33],
        ..base.clone()
    });
    for branch in [-1, 3] {
        let mut next = base.clone();
        next.rules[0].branch = branch;
        invalid.push(next);
    }
    for (minimum, maximum) in [
        (-1., None),
        (f64::NAN, None),
        (f64::INFINITY, None),
        (2., Some(2.)),
        (2., Some(1.)),
        (0., Some(f64::INFINITY)),
        (0., Some(f64::NAN)),
    ] {
        let mut next = base.clone();
        next.rules[0].condition.width = Range { minimum, maximum };
        invalid.push(next);
    }
    for value in invalid {
        assert!(!value.is_valid(), "{value:?}");
        assert!(decode_container_query(&encode(&value)).is_err());
    }
    let mut maximum = base;
    maximum.branches = (0..16).map(|i| format!("{i:0128}")).collect();
    maximum.rules = (0..32)
        .map(|i| Rule {
            condition: Predicate {
                width: Range {
                    minimum: i as f64,
                    maximum: Some(i as f64 + 0.5),
                },
                height: Range::ALL,
            },
            branch: i % 16,
        })
        .collect();
    assert!(maximum.is_valid());
    assert!(encode(&maximum).len() < MAX_CONFIG_BYTES);
    assert_eq!(decode_container_query(&encode(&maximum)), Ok(maximum));
}
