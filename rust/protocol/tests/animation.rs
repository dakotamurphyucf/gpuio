use binprot::BinProtWrite;
use gpuio_protocol::animation::*;
#[test]
fn animation_configuration_matches_independent_ocaml_fixture() {
    let config = Config {
        generation: 42,
        targets: vec![
            Target {
                property: Property::Width,
                value: 240.,
            },
            Target {
                property: Property::Opacity,
                value: 0.5,
            },
        ],
        initial: Some(vec![
            Target {
                property: Property::Width,
                value: 0.,
            },
            Target {
                property: Property::Opacity,
                value: 0.,
            },
        ]),
        duration_ms: 100,
        delay_ms: 10,
        easing: Easing::CubicBezier(0.25, 0., 0.75, 1.),
        repeat: Repeat::Alternate,
    };
    assert!(config.is_valid());
    let mut bytes = Vec::new();
    config.binprot_write(&mut bytes).unwrap();
    let encoded: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        encoded,
        include_str!("../../../test/fixtures/animation-v1-config.hex").trim()
    );
}

#[test]
fn animation_messages_have_bounded_validated_decoding() {
    use gpuio_protocol::{DecodeError, NodeId, WindowId, decode, v1::*};
    fn encode(value: &impl BinProtWrite) -> Vec<u8> {
        let mut bytes = vec![];
        value.binprot_write(&mut bytes).unwrap();
        bytes
    }
    let node = NodeId::from_parts(0, 1).unwrap();
    let config = Config {
        generation: 1,
        targets: vec![Target {
            property: Property::Width,
            value: 240.,
        }],
        initial: None,
        duration_ms: 100,
        delay_ms: 0,
        easing: Easing::Linear,
        repeat: Repeat::Once,
    };
    let message = |config| {
        Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![
                Op::Create(node, Kind::Animated, "".into(), None),
                Op::SetAnimation(node, config),
                Op::SetRoot(Some(node)),
            ],
        })
    };
    let valid = message(config.clone());
    assert_eq!(decode(&encode(&valid)), Ok(valid));
    for value in [f64::NAN, f64::INFINITY, -1., 1_000_001.] {
        let mut invalid = config.clone();
        invalid.targets[0].value = value;
        assert_eq!(
            decode(&encode(&message(invalid))),
            Err(DecodeError::Malformed)
        );
    }
    let mut invalid = config.clone();
    invalid.generation = 0;
    assert_eq!(
        decode(&encode(&message(invalid))),
        Err(DecodeError::Malformed)
    );
    let mut invalid = config.clone();
    invalid.initial = Some(vec![Target {
        property: Property::Height,
        value: 0.,
    }]);
    assert_eq!(
        decode(&encode(&message(invalid))),
        Err(DecodeError::Malformed)
    );
    let mut invalid = config.clone();
    invalid.targets = vec![invalid.targets[0]; 12];
    assert_eq!(
        decode(&encode(&message(invalid))),
        Err(DecodeError::LimitExceeded)
    );
    let mut invalid = config.clone();
    invalid.repeat = Repeat::Loop;
    assert_eq!(
        decode(&encode(&message(invalid))),
        Err(DecodeError::Malformed)
    );
    let mut invalid = config.clone();
    invalid.easing = Easing::CubicBezier(2., 0., 1., 1.);
    assert_eq!(
        decode(&encode(&message(invalid))),
        Err(DecodeError::Malformed)
    );
    let mut bytes = encode(&message(config));
    bytes.push(0);
    assert_eq!(decode(&bytes), Err(DecodeError::Malformed));
}

#[test]
fn application_motion_tags_are_stable_and_invalid_values_rejected() {
    use gpuio_protocol::{decode, v1::Message};
    for (tag, preference) in [
        (0, Preference::System),
        (1, Preference::Reduce),
        (2, Preference::Full),
    ] {
        let expected = Message::SetMotion(preference);
        let mut bytes = vec![];
        expected.binprot_write(&mut bytes).unwrap();
        assert_eq!(bytes, [9, tag]);
        assert_eq!(decode(&bytes).unwrap(), expected);
    }
    assert!(decode(&[9, 3]).is_err());
    assert!(decode(&[9]).is_err());
    assert!(decode(&[9, 1, 0]).is_err());
}
