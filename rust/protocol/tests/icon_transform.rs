use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, decode, icon_transform::Transform, v1::*};

fn message(transform: Option<Transform>) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetIconTransform(
            NodeId::from_parts(1, 1).unwrap(),
            transform,
        )],
    })
}
fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn from_hex(hex: &str) -> Vec<u8> {
    let hex = hex.trim();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

#[test]
fn independent_srt_and_reset_bytes_truncation_and_trailing_data() {
    let transform = Transform {
        scale_x: 1.5,
        scale_y: -0.5,
        rotation_degrees: 90.,
        translate_x: 3.,
        translate_y: -4.,
    };
    for (value, hex) in [
        (
            Some(transform),
            include_str!("../../../test/fixtures/icon-transform.hex"),
        ),
        (
            None,
            include_str!("../../../test/fixtures/icon-transform-clear.hex"),
        ),
    ] {
        let expected = from_hex(hex);
        assert_eq!(encode(&message(value)), expected);
        assert_eq!(decode(&expected), Ok(message(value)));
        for end in 0..expected.len() {
            assert!(decode(&expected[..end]).is_err());
        }
        let mut extra = expected;
        extra.push(0);
        assert!(decode(&extra).is_err());
    }
}

#[test]
fn finite_bounds_reject_untrusted_values_without_clamping() {
    for (field, bound) in [(0, 64.), (1, 64.), (2, 360.), (3, 16384.), (4, 16384.)] {
        for value in [
            0.,
            bound,
            -bound,
            bound + 1.,
            -bound - 1.,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            let mut transform = Transform::default();
            *match field {
                0 => &mut transform.scale_x,
                1 => &mut transform.scale_y,
                2 => &mut transform.rotation_degrees,
                3 => &mut transform.translate_x,
                _ => &mut transform.translate_y,
            } = value;
            let valid = value.is_finite() && value.abs() <= bound;
            assert_eq!(transform.is_valid(), valid);
            assert_eq!(decode(&encode(&message(Some(transform)))).is_ok(), valid);
        }
    }
}
