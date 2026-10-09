use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId, decode,
    tab_content::{Config, Label},
    v1::*,
};
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn message(config: Option<Config>) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetTabContent(NodeId::from_parts(0, 1).unwrap(), config)],
    })
}
#[test]
fn paired_content_modes_reset_and_strict_decoder() {
    let config = Config {
        max_width: Some(160.),
        labels: vec![Label::Default, Label::Custom, Label::Hidden],
    };
    assert_eq!(
        bytes(&Op::SetTabContent(
            NodeId::from_parts(0, 1).unwrap(),
            Some(config.clone())
        ))
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>(),
        "6300010101000000000000644003000102"
    );
    for config in [
        None,
        Some(config),
        Some(Config {
            max_width: None,
            labels: vec![],
        }),
    ] {
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
    for width in [0., -1., 1e6 + 1., f64::NAN, f64::INFINITY] {
        assert!(
            decode(&bytes(&message(Some(Config {
                max_width: Some(width),
                labels: vec![]
            }))))
            .is_err()
        );
    }
    assert!(
        decode(&bytes(&message(Some(Config {
            max_width: None,
            labels: vec![Label::Default; 4097]
        }))))
        .is_err()
    );
}
