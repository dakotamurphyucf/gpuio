use binprot::BinProtWrite;
use gpuio_protocol::{
    decode_slider_command, decode_slider_config, decode_slider_event, numeric::Domain, slider::*,
};
fn config() -> Config {
    Config {
        domain: Domain::new(-2., 8., 0.5).unwrap(),
        label: "Temperature".into(),
        lower_label: "Minimum".into(),
        upper_label: "Maximum".into(),
        axis: Axis::Vertical,
        scale: Scale::Linear,
        disabled: false,
        read_only: false,
    }
}
fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn snapshot() -> Snapshot {
    Snapshot {
        revision: 7,
        value: Value::Range {
            lower: 1.5,
            upper: 7.,
        },
        committed: Value::Range {
            lower: 2.,
            upper: 7.,
        },
        dragging: Some(Thumb::Lower),
    }
}
#[test]
fn independent_ocaml_rust_fixtures_and_bounded_decoding() {
    let c = config();
    let event = Event::Preview(snapshot());
    let command = Command::Replace {
        value: Value::Range {
            lower: 0.,
            upper: 6.,
        },
        if_revision: Some(7),
    };
    let bytes = encode(&c);
    assert_eq!(
        hex(&bytes),
        include_str!("../../../test/fixtures/slider-config.hex").trim()
    );
    assert_eq!(decode_slider_config(&bytes), Ok(c));
    for n in 0..bytes.len() {
        assert!(decode_slider_config(&bytes[..n]).is_err());
    }
    let mut extra = bytes;
    extra.push(0);
    assert!(decode_slider_config(&extra).is_err());
    let bytes = encode(&event);
    assert_eq!(
        hex(&bytes),
        include_str!("../../../test/fixtures/slider-preview.hex").trim()
    );
    assert_eq!(decode_slider_event(&bytes), Ok(event));
    for n in 0..bytes.len() {
        assert!(decode_slider_event(&bytes[..n]).is_err());
    }
    let mut extra = bytes;
    extra.push(0);
    assert!(decode_slider_event(&extra).is_err());
    let bytes = encode(&command);
    assert_eq!(
        hex(&bytes),
        include_str!("../../../test/fixtures/slider-replace.hex").trim()
    );
    assert_eq!(decode_slider_command(&bytes), Ok(command));
    for n in 0..bytes.len() {
        assert!(decode_slider_command(&bytes[..n]).is_err());
    }
    let mut extra = bytes;
    extra.push(0);
    assert!(decode_slider_command(&extra).is_err());
    assert!(decode_slider_config(&vec![0; MAX_CONFIG_BYTES + 1]).is_err());
    assert!(decode_slider_event(&[0; 65]).is_err());
    assert!(decode_slider_command(&[0; 33]).is_err());
}
#[test]
fn malformed_values_states_and_tags_are_rejected() {
    let mut c = config();
    c.scale = Scale::Logarithmic;
    assert!(decode_slider_config(&encode(&c)).is_err());
    c = config();
    c.label = " \t\u{b}".into();
    assert!(decode_slider_config(&encode(&c)).is_err());
    c = config();
    c.upper_label = "a".repeat(4097);
    assert!(decode_slider_config(&encode(&c)).is_err());
    for bad in [
        Value::Single(f64::NAN),
        Value::Range {
            lower: 5.,
            upper: 1.,
        },
        Value::Single(f64::INFINITY),
    ] {
        for thumb in [Thumb::Single, Thumb::Lower, Thumb::Upper] {
            assert_eq!(bad.set(thumb, 3., c.domain), None);
        }
        assert!(
            decode_slider_command(&encode(&Command::Replace {
                value: bad,
                if_revision: None
            }))
            .is_err()
        );
    }
    assert!(
        decode_slider_command(&encode(&Command::Replace {
            value: Value::Single(1.),
            if_revision: Some(-1)
        }))
        .is_err()
    );
    let s = snapshot();
    for bad in [
        Snapshot { revision: -1, ..s },
        Snapshot { revision: 0, ..s },
        Snapshot {
            dragging: None,
            ..s
        },
        Snapshot {
            dragging: Some(Thumb::Single),
            ..s
        },
        Snapshot {
            value: Value::Range {
                lower: 1.5,
                upper: 6.,
            },
            ..s
        },
        Snapshot {
            committed: Value::Single(1.),
            ..s
        },
    ] {
        assert!(decode_slider_event(&encode(&Event::Observed(bad))).is_err());
    }
    assert!(decode_slider_event(&encode(&Event::DragStarted(s))).is_err());
    assert!(decode_slider_event(&encode(&Event::Committed(Source::Pointer, s))).is_err());
    assert!(decode_slider_event(&encode(&Event::Cancelled(CancelReason::Escape, s))).is_err());
    for bytes in [&[255][..], &[2, 255][..]] {
        assert!(decode_slider_command(bytes).is_err());
    }
}
#[test]
fn every_lifecycle_variant_roundtrips() {
    let s = snapshot();
    let idle = Snapshot {
        value: s.committed,
        dragging: None,
        ..s
    };
    let start = Snapshot {
        value: s.committed,
        ..s
    };
    let mut events = vec![
        Event::Observed(idle),
        Event::Observed(s),
        Event::DragStarted(start),
        Event::Preview(s),
    ];
    for source in [Source::Pointer, Source::Keyboard, Source::Accessibility] {
        events.push(Event::Committed(source, idle));
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
        events.push(Event::Cancelled(reason, idle));
    }
    for event in events {
        assert_eq!(decode_slider_event(&encode(&event)), Ok(event));
    }
    for command in [
        Command::CancelDrag,
        Command::ReadSnapshot,
        Command::Focus(Thumb::Single),
        Command::Focus(Thumb::Lower),
        Command::Focus(Thumb::Upper),
    ] {
        assert_eq!(decode_slider_command(&encode(&command)), Ok(command));
    }
}
#[test]
fn bounded_linear_log_mapping_and_fixed_domain() {
    let mut c = config();
    assert_eq!(c.from_fraction(0.), Some(-2.));
    assert_eq!(c.from_fraction(1.), Some(8.));
    assert_eq!(c.from_fraction(0.5), Some(3.));
    assert_eq!(c.fraction(3.), Some(0.5));
    for value in [f64::NAN, f64::INFINITY] {
        assert_eq!(c.from_fraction(value), None);
        assert_eq!(c.fraction(value), None);
    }
    c.domain = Domain::new(1., 1000., 1.).unwrap();
    c.scale = Scale::Logarithmic;
    assert_eq!(c.from_fraction(1. / 3.), Some(10.));
    assert_eq!(c.from_fraction(2. / 3.), Some(100.));
    assert!((c.fraction(10.).unwrap() - 1. / 3.).abs() < 1e-12);
    // Positive narrow spans must not disappear after subtracting two rounded logs.
    c.domain = Domain::new(1e12, 1e12 + 1., 0.01).unwrap();
    assert!((c.fraction(1e12 + 0.5).unwrap() - 0.5).abs() < 1e-10);
    assert!((c.from_fraction(0.5).unwrap() - (1e12 + 0.5)).abs() < 0.01);
    // The max/min ratio overflows; the logarithmic fallback stays finite.
    c.domain = Domain::new(1e-300, 1e300, 1e289).unwrap();
    for i in 0..=100 {
        let v = c.from_fraction(f64::from(i) / 100.).unwrap();
        assert!(c.domain.contains(v));
        assert!(c.fraction(v).unwrap().is_finite());
    }
    c.domain = Domain::new(5., 5., 1.).unwrap();
    assert_eq!(c.from_fraction(0.5), Some(5.));
    assert_eq!(c.fraction(5.), Some(0.));
}
