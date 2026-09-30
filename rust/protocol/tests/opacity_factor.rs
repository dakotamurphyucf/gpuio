use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, animation::*, animation_program, v1::*};

fn config(value: f64) -> Config {
    Config {
        generation: 1,
        targets: vec![Target {
            property: Property::OpacityFactor,
            value,
        }],
        initial: None,
        duration_ms: 200,
        delay_ms: 0,
        easing: Easing::Linear,
        repeat: Repeat::Once,
    }
}
fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn message(config: Config) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetAnimation(NodeId::from_parts(0, 1).unwrap(), config)],
    })
}
#[test]
fn independent_factor_bytes_and_complete_decode() {
    for (value, expected) in [
        (0., "0b0000000000000000"),
        (0.5, "0b000000000000e03f"),
        (1., "0b000000000000f03f"),
    ] {
        let cfg = config(value);
        assert_eq!(
            encode(&cfg.targets[0])
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            expected
        );
        let msg = message(cfg);
        let bytes = encode(&msg);
        assert_eq!(gpuio_protocol::decode(&bytes), Ok(msg));
        for len in 0..bytes.len() {
            assert!(gpuio_protocol::decode(&bytes[..len]).is_err());
        }
        let mut trailing = bytes;
        trailing.push(0);
        assert!(gpuio_protocol::decode(&trailing).is_err());
    }
}
#[test]
fn both_animation_apis_reject_nonfinite_bounds_and_conflicting_opacity() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.1, 1.1] {
        let cfg = config(value);
        assert!(!cfg.is_valid());
        assert!(animation_program::Program::from_legacy(&cfg).is_none());
        assert!(gpuio_protocol::decode(&encode(&message(cfg))).is_err());
    }
    let mut cfg = config(0.5);
    cfg.targets.insert(
        0,
        Target {
            property: Property::Opacity,
            value: 0.4,
        },
    );
    assert!(!cfg.is_valid());
    let mut program = animation_program::Program::from_legacy(&config(0.5)).unwrap();
    program.stages[0].targets = cfg.targets.clone();
    assert!(!program.is_valid());
    assert!(gpuio_protocol::decode(&encode(&message(cfg))).is_err());
}
#[test]
fn factor_requires_matching_host() {
    assert_eq!(CAP_OPACITY_FACTOR, 1_i64 << 51);
    assert_eq!(CAPABILITIES, (1_i64 << 56) - 1);
    for (required, expected) in [
        (CAP_OPACITY_FACTOR, "0001fc0000000000000800"),
        (CAPABILITIES, "0001fcffffffffffffff00"),
    ] {
        let hello = Message::Hello(VERSION, required);
        let bytes = encode(&hello);
        assert_eq!(
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            expected
        );
        assert_eq!(gpuio_protocol::decode(&bytes), Ok(hello));
    }
}
