use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, decode, toast_motion::Config, v1::*};
fn message(config: Config) -> Message {
    let id = NodeId::from_parts(0, 1).unwrap();
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetToastMotion(id, Some(config)),
            Op::SetToastMotion(id, None),
        ],
    })
}
#[test]
fn independent_motion_fixture_and_checked_numeric_domain() {
    let valid = message(Config::default());
    let mut bytes = vec![];
    valid.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/toast-motion.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(valid));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    for bad in [
        Config {
            enter_ms: -1,
            ..Default::default()
        },
        Config {
            exit_ms: 60_001,
            ..Default::default()
        },
        Config {
            offset: f64::NAN,
            ..Default::default()
        },
        Config {
            offset: f64::INFINITY,
            ..Default::default()
        },
        Config {
            offset: -1.,
            ..Default::default()
        },
        Config {
            offset: 16385.,
            ..Default::default()
        },
        Config {
            spring: gpuio_protocol::animation::Spring {
                mass: 0.,
                ..Config::default().spring
            },
            ..Default::default()
        },
    ] {
        let mut invalid = vec![];
        message(bad).binprot_write(&mut invalid).unwrap();
        assert!(decode(&invalid).is_err());
    }
    bytes.push(0);
    assert!(decode(&bytes).is_err());
}
