use binprot::BinProtWrite;
use gpuio_protocol::{
    decode_numeric_domain,
    numeric::{Direction::*, Domain, Draft, DraftError::*},
};

#[test]
fn independent_fixture_and_invalid_codec() {
    let domain = Domain::new(-1.5, 2.25, 0.125).unwrap();
    let mut bytes = Vec::new();
    domain.binprot_write(&mut bytes).unwrap();
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex,
        include_str!("../../../test/fixtures/numeric-domain.hex").trim()
    );
    assert_eq!(decode_numeric_domain(&bytes), Ok(domain));
    for n in 0..bytes.len() {
        assert!(decode_numeric_domain(&bytes[..n]).is_err());
    }
    bytes.push(0);
    assert!(decode_numeric_domain(&bytes).is_err());
    for (min, max, step) in [
        (f64::NAN, 1., 1.),
        (0., f64::INFINITY, 1.),
        (0., 1., f64::NAN),
        (1., 0., 1.),
        (0., 1., 0.),
        (0., 1., -1.),
        (0., 1., 1e-13),
        (1e16, 1e16 + 10., 1.),
        (-1e308, 1e308, 1e300),
    ] {
        assert!(Domain::new(min, max, step).is_none());
        let mut bad = Vec::new();
        for value in [min, max, step] {
            value.binprot_write(&mut bad).unwrap();
        }
        assert!(decode_numeric_domain(&bad).is_err());
    }
}

#[test]
fn min_anchored_grid_endpoints_ties_and_finite_rules() {
    let d = Domain::new(0.1, 1., 0.2).unwrap();
    let mut value = 0.1;
    for expected in [0.3, 0.5, 0.7, 0.9, 1., 1.] {
        value = d.advance(value, Increase).unwrap();
        assert!((value - expected).abs() < 1e-14);
    }
    for expected in [0.9, 0.7, 0.5, 0.3, 0.1, 0.1] {
        value = d.advance(value, Decrease).unwrap();
        assert!((value - expected).abs() < 1e-14);
    }
    let d = Domain::new(-1., 1., 0.5).unwrap();
    for (v, expected) in [(-0.75, -0.5), (-0.25, 0.), (0.25, 0.5), (0.75, 1.)] {
        assert_eq!(d.normalize(v), Some(expected));
    }
    for v in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(d.normalize(v), None);
        assert_eq!(d.advance(v, Increase), None);
    }
    let d = Domain::new(4., 4., 1e-100).unwrap();
    for v in [-100., 4., 100.] {
        assert_eq!(d.normalize(v), Some(4.));
        assert_eq!(d.advance(v, Decrease), Some(4.));
    }
    let d = Domain::new(-1., 1., f64::MAX).unwrap();
    assert_eq!(d.advance(-1., Increase), Some(1.));
    assert_eq!(d.advance(1., Decrease), Some(-1.));
}

#[test]
fn idempotence_and_adjacent_steps_across_scales() {
    for (min, max, step) in [
        (0., 1., 0.2),
        (-5., 5., 0.125),
        (0.1, 1., 0.2),
        (1e9, 1e9 + 0.01, 0.00001),
        (0., 1., 1e-12),
        (-1e200, 1e200, 1e195),
        (-1e-200, 1e-200, 1e-205),
    ] {
        let d = Domain::new(min, max, step).unwrap();
        for i in 0..=1024 {
            let value = d
                .normalize(min + (max - min) * f64::from(i) / 1024.)
                .unwrap();
            assert!(d.contains(value));
            assert_eq!(d.normalize(value), Some(value));
            let up = d.advance(value, Increase).unwrap();
            let down = d.advance(value, Decrease).unwrap();
            assert!(up >= value && down <= value);
            if value < max {
                assert!(up > value);
                assert_eq!(
                    d.advance(up, Decrease),
                    Some(value),
                    "{min} {max} {step}: {value} -> {up}"
                );
            }
            if value > min {
                assert!(down < value);
                assert_eq!(
                    d.advance(down, Increase),
                    Some(value),
                    "{min} {max} {step}: {value} -> {down}"
                );
            }
        }
    }
}

#[test]
fn draft_grammar_is_locale_independent_and_keeps_prefixes() {
    let d = Domain::new(-10., 10., 0.1).unwrap();
    for text in ["", " \t", "\u{b}\u{c}"] {
        assert_eq!(Draft::parse(d, text), Draft::Empty);
    }
    for text in ["-", "+.", ".", "1e", "1e-"] {
        assert_eq!(Draft::parse(d, text), Draft::Incomplete);
    }
    for (text, value) in [("1.", 1.), ("-.5", -0.5), (" +2e0 ", 2.)] {
        assert_eq!(Draft::parse(d, text), Draft::Valid(value));
    }
    for (text, value) in [("11", 11.), ("-11", -11.)] {
        assert_eq!(Draft::parse(d, text), Draft::OutOfRange(value));
    }
    assert_eq!(Draft::parse(d, "1e309"), Draft::Invalid(NonFinite));
    for text in ["nan", "inf", "0x10", "1_000", "1,2", "1e-x", "１２"] {
        assert_eq!(Draft::parse(d, text), Draft::Invalid(Syntax));
    }
    assert_eq!(Draft::parse(d, &"1".repeat(4097)), Draft::Invalid(TooLong));
}
