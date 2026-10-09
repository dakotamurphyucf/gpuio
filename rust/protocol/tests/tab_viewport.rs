use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId, decode,
    tab_viewport::{Config, Reveal},
    v1::*,
};
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut data = vec![];
    value.binprot_write(&mut data).unwrap();
    data
}
fn message(config: Option<Config>) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetTabViewport(
            NodeId::from_parts(0, 1).unwrap(),
            config,
        )],
    })
}
#[test]
fn paired_reveal_operation_and_strict_payload_validation() {
    let config = Config {
        reveal: Some(Reveal {
            serial: 1,
            target: "tab-7".into(),
        }),
    };
    assert_eq!(
        bytes(&Op::SetTabViewport(
            NodeId::from_parts(0, 1).unwrap(),
            Some(config.clone())
        ))
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>(),
        "640001010101057461622d37"
    );
    for config in [None, Some(Config::default()), Some(config)] {
        let message = message(config);
        let data = bytes(&message);
        assert_eq!(decode(&data), Ok(message));
        for n in 0..data.len() {
            assert!(decode(&data[..n]).is_err());
        }
        let mut extra = data;
        extra.push(0);
        assert!(decode(&extra).is_err());
    }
    for (serial, target) in [
        (0, "a".into()),
        (-1, "a".into()),
        (1, String::new()),
        (1, "a\0".into()),
        (1, "a".repeat(257)),
    ] {
        assert!(
            decode(&bytes(&message(Some(Config {
                reveal: Some(Reveal { serial, target })
            }))))
            .is_err()
        );
    }
}
