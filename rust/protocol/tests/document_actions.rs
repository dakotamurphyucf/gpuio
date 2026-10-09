use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId,
    document::{Activation, ActivationSource},
    document_actions::*,
    v1::*,
};
fn bytes<T: BinProtWrite>(value: &T) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex<T: BinProtWrite>(value: &T) -> String {
    bytes(value).iter().map(|b| format!("{b:02x}")).collect()
}
fn config() -> Config {
    Config {
        epoch: 1,
        observe: true,
        copy_code: false,
        copy_table: true,
        code: vec![Action {
            id: "run".into(),
            label: "Run".into(),
            enabled: true,
        }],
        table: vec![],
    }
}
#[test]
fn document_actions_match_ocaml_and_reject_malformed_configs() {
    let node = NodeId::from_parts(0, 1).unwrap();
    let config = config();
    assert_eq!(
        hex(&Op::SetDocumentActions(node, config.clone())),
        "78000101010001010372756e0352756e0100"
    );
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetDocumentActions(node, config.clone())],
    });
    let packet = bytes(&message);
    assert_eq!(gpuio_protocol::decode(&packet), Ok(message.clone()));
    for end in 0..packet.len() {
        assert!(gpuio_protocol::decode(&packet[..end]).is_err());
    }
    let mut trailing = packet.clone();
    trailing.push(0);
    assert!(gpuio_protocol::decode(&trailing).is_err());
    let mut invalid = Vec::new();
    let mut c = config.clone();
    c.epoch = 0;
    invalid.push(c);
    let mut c = config.clone();
    c.observe = false;
    invalid.push(c);
    let mut c = config.clone();
    c.code.push(c.code[0].clone());
    invalid.push(c);
    let mut c = config.clone();
    c.code[0].id = "bad id".into();
    invalid.push(c);
    let mut c = config.clone();
    c.code[0].label = "x".repeat(257);
    invalid.push(c);
    let mut c = config.clone();
    c.code = (0..17)
        .map(|i| Action {
            id: format!("a{i}"),
            label: "A".into(),
            enabled: true,
        })
        .collect();
    invalid.push(c);
    for config in invalid {
        assert!(!config.is_valid());
        let mut bad = message.clone();
        let Message::Apply(tx) = &mut bad else {
            unreachable!()
        };
        tx.operations = vec![Op::SetDocumentActions(node, config)];
        assert!(gpuio_protocol::decode(&bytes(&bad)).is_err());
    }
    // Strict Boolean decoding, not truthiness.
    let offset = packet.len() - bytes(&Op::SetDocumentActions(node, config.clone())).len();
    for field in [4, 5, 6] {
        let mut bad = packet.clone();
        bad[offset + field] = 2;
        assert!(gpuio_protocol::decode(&bad).is_err());
    }
}
#[test]
fn document_action_payloads_have_paired_bytes_and_bounded_accounting() {
    let event = gpuio_protocol::document_actions::Event {
        config_epoch: 1,
        action: "run".into(),
        source_revision: 7,
        source_generation: 2,
        source_range: None,
        block: Block::Code(None, "ok".into()),
        activation: Activation {
            source: ActivationSource::Keyboard,
            modifiers: Default::default(),
        },
    };
    assert_eq!(hex(&event), "010372756e0702000000026f6b010000000000");
    assert!(event.is_valid());
    assert!(event.payload_bytes() >= bytes(&event).len());
    let mut invalid = event.clone();
    invalid.source_revision = 0;
    assert!(!invalid.is_valid());
    let mut invalid = event.clone();
    invalid.source_generation = 0;
    assert!(!invalid.is_valid());
    let mut invalid = event.clone();
    invalid.source_range = Some(SourceRange {
        start_byte: 9,
        end_byte: 8,
    });
    assert!(!invalid.is_valid());
    let mut invalid = event.clone();
    invalid.block = Block::Code(None, "x".repeat(65537));
    assert!(!invalid.is_valid());
    for block in [
        Block::Table(vec!["x".into(); 4097], vec![], String::new()),
        Block::Table(vec![], vec![vec![]; 4097], String::new()),
        Block::Table(vec!["x".repeat(MAX_TEXT)], vec![], "x".into()),
    ] {
        assert!(!block.is_valid());
    }
    let mut table = event.clone();
    table.block = Block::Table(
        vec!["世界".into()],
        vec![vec!["row".into()], vec![]],
        "| 世界 |\n| --- |\n| row |".into(),
    );
    assert!(table.is_valid());
    assert!(table.payload_bytes() >= bytes(&table).len());
    let mut cfg = config();
    assert!(cfg.allows("run", &event.block));
    assert!(!cfg.allows("run", &table.block));
    cfg.code[0].enabled = false;
    assert!(!cfg.allows("run", &event.block));
}
