use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, control_appearance::*, decode_control_appearance, v1::*};

fn encode(config: &Config) -> Vec<u8> {
    let mut bytes = vec![];
    config.binprot_write(&mut bytes).unwrap();
    bytes
}

#[test]
fn independent_geometry_and_part_state_fixtures_match_ocaml() {
    let cases = [
        (
            Config::default(),
            include_str!("../../../test/fixtures/control-appearance-default.hex"),
        ),
        (
            Config {
                size: 24.,
                switch_width: 48.,
                gap: 10.,
                label_position: LabelPosition::Before,
                indicator_style: vec![Style::State(
                    4,
                    vec![Field::Foreground(Color::Rgba(0x11223344))],
                )],
                mark_style: vec![Style::State(6, vec![Field::Opacity(0.5)])],
            },
            include_str!("../../../test/fixtures/control-appearance-states.hex"),
        ),
    ];
    for (config, fixture) in cases {
        let bytes = encode(&config);
        assert_eq!(
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            fixture.trim()
        );
        assert_eq!(decode_control_appearance(&bytes), Ok(config));
        for length in 0..bytes.len() {
            assert!(decode_control_appearance(&bytes[..length]).is_err());
        }
        let mut invalid = bytes.clone();
        invalid[24] = 2;
        assert!(decode_control_appearance(&invalid).is_err());
        let mut extra = bytes;
        extra.push(0);
        assert!(decode_control_appearance(&extra).is_err());
    }
}

#[test]
fn geometry_and_aggregate_style_limits_fail_before_admission() {
    for size in [f64::NAN, f64::INFINITY, -1., 7.99, 128.01] {
        assert!(
            decode_control_appearance(&encode(&Config {
                size,
                ..Config::default()
            }))
            .is_err()
        );
    }
    for switch_width in [f64::NAN, 17.99, 256.01] {
        assert!(
            decode_control_appearance(&encode(&Config {
                switch_width,
                ..Config::default()
            }))
            .is_err()
        );
    }
    for gap in [f64::NAN, -0.01, 128.01] {
        assert!(
            decode_control_appearance(&encode(&Config {
                gap,
                ..Config::default()
            }))
            .is_err()
        );
    }
    for state in [0, 1, 2, 3, 7, 99] {
        let config = Config {
            mark_style: vec![Style::State(state, vec![Field::Opacity(1.)])],
            ..Config::default()
        };
        assert!(decode_control_appearance(&encode(&config)).is_err());
    }
    let mut config = Config {
        indicator_style: vec![Style::Fields(vec![Field::Opacity(1.); 128])],
        ..Config::default()
    };
    assert!(decode_control_appearance(&encode(&config)).is_ok());
    config.mark_style = vec![Style::Fields(vec![Field::Opacity(1.)])];
    assert_eq!(
        decode_control_appearance(&encode(&config)),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_control_appearance(&vec![0; MAX_CONFIG_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
}

#[test]
fn appended_operation_reset_and_capability_match_independent_fixture() {
    use gpuio_protocol::{NodeId, WindowId, decode};
    let node = NodeId::from_parts(0, 1).unwrap();
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetControlAppearance(node, Some(Config::default())),
            Op::SetControlAppearance(node, None),
        ],
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/control-appearance-operation.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for (mask, expected) in [
        (CAP_CONTROL_APPEARANCE, "0003fc0000000000000010"),
        (CAPABILITIES, "0003fcffffffffffffff7f"),
    ] {
        let mut bytes = vec![];
        Message::Hello(VERSION, mask)
            .binprot_write(&mut bytes)
            .unwrap();
        assert_eq!(
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            expected
        );
    }
}
