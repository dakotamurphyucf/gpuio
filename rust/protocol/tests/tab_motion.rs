use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, decode, tab_motion::Config, v1::*};
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut out = vec![];
    value.binprot_write(&mut out).unwrap();
    out
}
fn message(config: Option<Config>) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetTabMotion(NodeId::from_parts(0, 1).unwrap(), config)],
    })
}
#[test]
fn paired_tab_motion_payload_and_strict_validation() {
    let config = Config::default();
    let data = bytes(&Op::SetTabMotion(
        NodeId::from_parts(0, 1).unwrap(),
        Some(config),
    ));
    assert_eq!(
        data.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        "6700010100000000000079400000000000004440000000000000f03f7b14ae47e17a843ffeD007feC800"
            .to_lowercase()
    );
    for config in [
        None,
        Some(config),
        Some(Config {
            color_duration_ms: 0,
            ..config
        }),
        Some(Config {
            color_duration_ms: 60000,
            ..config
        }),
    ] {
        let msg = message(config);
        let data = bytes(&msg);
        assert_eq!(decode(&data), Ok(msg));
        for n in 0..data.len() {
            assert!(decode(&data[..n]).is_err());
        }
        let mut trailing = data;
        trailing.push(0);
        assert!(decode(&trailing).is_err());
    }
    for duration in [-1, 60001, i64::MAX] {
        assert!(
            decode(&bytes(&message(Some(Config {
                color_duration_ms: duration,
                ..config
            }))))
            .is_err()
        );
    }
    for bad in [f64::NAN, f64::INFINITY, -1., 0.] {
        let mut config = config;
        config.spring.mass = bad;
        assert!(decode(&bytes(&message(Some(config)))).is_err());
    }
}
