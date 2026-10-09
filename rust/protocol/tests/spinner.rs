use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, ResourceId,
    animation::Easing,
    decode_spinner_config,
    image::{ImageError, ImageFit, ImageSource},
    loading::Kind,
    spinner::{Config, MAX_CONFIG_BYTES},
};

fn config() -> Config {
    Config {
        label: "Loading".into(),
        animated: true,
        period_ms: 800,
        easing: Easing::EaseInOut,
        source: None,
    }
}
fn encode(config: &Config) -> Vec<u8> {
    let mut bytes = vec![];
    config.binprot_write(&mut bytes).unwrap();
    bytes
}
fn paired(config: Config, fixture: &str) {
    let bytes = encode(&config);
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        fixture.trim()
    );
    assert_eq!(decode_spinner_config(&bytes), Ok(config));
    for n in 0..bytes.len() {
        assert!(decode_spinner_config(&bytes[..n]).is_err(), "prefix {n}");
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(decode_spinner_config(&trailing).is_err());
}
#[test]
fn paired_configuration_and_decorative_image_contract() {
    paired(
        config(),
        include_str!("../../../test/fixtures/spinner-default.hex"),
    );
    let source = ImageSource::Reference(ResourceId::from_parts(7, 3).unwrap());
    let custom = Config {
        label: "Indexing".into(),
        animated: false,
        period_ms: 101,
        easing: Easing::CubicBezier(0.25, -2., 0.75, 3.),
        source: Some(source),
    };
    paired(
        custom.clone(),
        include_str!("../../../test/fixtures/spinner-custom.hex"),
    );
    let loading = custom.loading();
    assert_eq!(loading.kind, Kind::Spinner);
    assert_eq!(loading.label, custom.label);
    assert_eq!(loading.period_ms, 101);
    assert!(!loading.animated);
    let image = custom.image().unwrap();
    assert_eq!(image.source, source);
    assert_eq!(image.fit, ImageFit::Contain);
    assert_eq!(image.label, None);
    assert_eq!(
        custom.retained_bytes(),
        std::mem::size_of::<Config>() + custom.label.capacity()
    );
    assert!(config().image().is_none());
}
#[test]
fn validation_is_bounded_and_rejects_invalid_easing_labels_and_periods() {
    for label in ["".into(), " \t".into(), "a\0b".into(), "x".repeat(4097)] {
        let invalid = Config { label, ..config() };
        assert!(!invalid.is_valid());
        assert!(decode_spinner_config(&encode(&invalid)).is_err());
    }
    for period_ms in [i64::MIN, -1, 0, 99, 60001, i64::MAX] {
        assert!(
            decode_spinner_config(&encode(&Config {
                period_ms,
                ..config()
            }))
            .is_err()
        );
    }
    for period_ms in [100, 60000] {
        let valid = Config {
            period_ms,
            label: "界".repeat(1365) + "x",
            ..config()
        };
        assert_eq!(valid.label.len(), 4096);
        assert_eq!(decode_spinner_config(&encode(&valid)), Ok(valid));
    }
    for easing in [
        Easing::CubicBezier(-0.1, 0., 0.75, 1.),
        Easing::CubicBezier(0.25, 0., 1.1, 1.),
        Easing::CubicBezier(f64::NAN, 0., 1., 1.),
        Easing::CubicBezier(0., f64::INFINITY, 1., 1.),
        Easing::CubicBezier(0., 0., 1., f64::NEG_INFINITY),
    ] {
        let invalid = Config { easing, ..config() };
        assert!(!invalid.is_valid());
        assert!(decode_spinner_config(&encode(&invalid)).is_err());
    }
    for easing in [
        Easing::Linear,
        Easing::Ease,
        Easing::EaseIn,
        Easing::EaseOut,
        Easing::EaseInOut,
        Easing::CubicBezier(0., -f64::MAX, 1., f64::MAX),
    ] {
        let valid = Config { easing, ..config() };
        assert_eq!(decode_spinner_config(&encode(&valid)), Ok(valid));
    }
    assert_eq!(
        decode_spinner_config(&vec![0; MAX_CONFIG_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
}
#[test]
fn decoder_rejects_invalid_tags_utf8_and_resource_generations() {
    let original = encode(&config());
    // Label bytes, animated Boolean, easing tag, source option tag.
    for (offset, value) in [(1, 255), (8, 2), (12, 255), (13, 2)] {
        let mut bad = original.clone();
        bad[offset] = value;
        assert!(decode_spinner_config(&bad).is_err(), "offset {offset}");
    }
    let valid = Config {
        source: Some(ImageSource::Reference(
            ResourceId::from_parts(0, 1).unwrap(),
        )),
        ..config()
    };
    let mut bad = encode(&valid);
    *bad.last_mut().unwrap() = 0;
    assert!(decode_spinner_config(&bad).is_err());
    for error in [
        ImageError::WrongApplication,
        ImageError::Released,
        ImageError::InvalidData,
        ImageError::Unsupported,
        ImageError::ResourceLimit,
        ImageError::NativeFailure,
    ] {
        let valid = Config {
            source: Some(ImageSource::Unavailable(error)),
            ..config()
        };
        assert_eq!(decode_spinner_config(&encode(&valid)), Ok(valid));
    }
}

#[test]
fn atomic_spinner_operation_and_capability_match_ocaml() {
    use gpuio_protocol::{NodeId, WindowId, decode, v1::*};
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetSpinner(NodeId::from_parts(0, 1).unwrap(), config())],
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/spinner-operation.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for n in 0..bytes.len() {
        assert!(decode(&bytes[..n]).is_err());
    }
    let mut hello = vec![];
    Message::Hello(VERSION, CAP_SPINNER)
        .binprot_write(&mut hello)
        .unwrap();
    assert_eq!(
        hello.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        "0003fc0000000000000004"
    );
    assert_eq!(CAPABILITIES, i64::MAX);
}
