use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, link::Config, v1::*};

#[test]
fn op60_and_kind51_match_independent_ocaml_bytes() {
    let node = NodeId::from_parts(0, 1).unwrap();
    let config = Config {
        label: "Guide 世界".into(),
        disabled: false,
        tab_stop: false,
        tab_index: -2,
    };
    let packet = |config| {
        Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![Op::SetLink(node, config)],
        })
    };
    let encode = |message: &Message| {
        let mut bytes = vec![];
        message.binprot_write(&mut bytes).unwrap();
        bytes
    };
    let message = packet(config.clone());
    let bytes = encode(&message);
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        "0300010001013c00010c477569646520e4b896e7958c0000fffe"
    );
    assert_eq!(gpuio_protocol::decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(gpuio_protocol::decode(&bytes[..end]).is_err());
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(gpuio_protocol::decode(&trailing).is_err());
    assert!(
        gpuio_protocol::decode(&encode(&packet(Config {
            label: String::new(),
            ..config
        })))
        .is_err()
    );
    let create = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::Create(node, Kind::Link, String::new(), None)],
    });
    let bytes = encode(&create);
    assert_eq!(bytes, [3, 0, 1, 0, 1, 1, 0, 0, 1, 51, 0, 0]);
    assert_eq!(gpuio_protocol::decode(&bytes), Ok(create));
}
