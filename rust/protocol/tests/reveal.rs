use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, animation::Spring, decode, reveal::Config, v1::*};
fn config() -> Config {
    Config {
        expanded: true,
        retain: false,
        spring: Spring {
            stiffness: 400.,
            damping: 40.,
            mass: 1.,
            epsilon: 0.1,
            max_duration_ms: 2000,
        },
    }
}
fn message(config: Config) -> Message {
    let node = NodeId::from_parts(0, 1).unwrap();
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetReveal(node, Some(config)),
            Op::SetReveal(
                node,
                Some(Config {
                    expanded: false,
                    retain: true,
                    ..config
                }),
            ),
            Op::SetReveal(node, None),
        ],
    })
}
fn encode(message: &Message) -> Vec<u8> {
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    bytes
}
#[test]
fn independent_fixture_and_strict_decode() {
    let message = message(config());
    let bytes = encode(&message);
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/reveal-operation.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(decode(&extra).is_err());
    for at in [9, 10, 11] {
        let mut invalid_bool = bytes.clone();
        invalid_bool[at] = 2;
        assert!(decode(&invalid_bool).is_err());
    }
}
#[test]
fn spring_domains_are_validated_before_native_admission() {
    let base = config();
    for spring in [
        Spring {
            stiffness: f64::NAN,
            ..base.spring
        },
        Spring {
            damping: -1.,
            ..base.spring
        },
        Spring {
            mass: 0.,
            ..base.spring
        },
        Spring {
            epsilon: f64::INFINITY,
            ..base.spring
        },
        Spring {
            max_duration_ms: 0,
            ..base.spring
        },
        Spring {
            max_duration_ms: 60_001,
            ..base.spring
        },
    ] {
        assert!(decode(&encode(&message(Config { spring, ..base }))).is_err());
    }
}
