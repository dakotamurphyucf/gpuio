use binprot::BinProtWrite;
use gpuio_protocol::{
    decode_navigation_stack,
    navigation_stack::{Config, Motion},
};

#[test]
fn independent_transaction_fixture_covers_appended_kind_and_operation() {
    use gpuio_protocol::{NodeId, WindowId, v1::*};
    let id = NodeId::from_parts(0, 1).unwrap();
    let request = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(id, Kind::NavigationStack, "Routes".into(), None),
            Op::SetNavigationStack(
                id,
                Config {
                    selected: None,
                    retain: true,
                    motion: Motion::Slide,
                    duration_ms: 200,
                },
            ),
            Op::SetRoot(Some(id)),
        ],
    });
    let mut bytes = Vec::new();
    request.binprot_write(&mut bytes).unwrap();
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex,
        include_str!("../../../test/fixtures/navigation-stack-request.hex").trim()
    );
    assert_eq!(gpuio_protocol::decode(&bytes), Ok(request));
    for length in 0..bytes.len() {
        assert!(gpuio_protocol::decode(&bytes[..length]).is_err());
    }
    bytes.push(0);
    assert!(gpuio_protocol::decode(&bytes).is_err());
}

#[test]
fn independent_config_fixture_and_malformed_inputs() {
    // Some index 2; retain true; slide tag 1; int16 200 milliseconds.
    let config = Config {
        selected: Some(2),
        retain: true,
        motion: Motion::Slide,
        duration_ms: 200,
    };
    let mut bytes = vec![];
    config.binprot_write(&mut bytes).unwrap();
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        hex,
        include_str!("../../../test/fixtures/navigation-stack-config.hex").trim()
    );
    assert_eq!(decode_navigation_stack(&bytes), Ok(config));
    for end in 0..bytes.len() {
        assert!(decode_navigation_stack(&bytes[..end]).is_err());
    }
    for (index, value) in [(0, 2), (2, 2), (3, 3)] {
        let mut bad = bytes.clone();
        bad[index] = value;
        assert!(decode_navigation_stack(&bad).is_err());
    }
    bytes.push(0);
    assert!(decode_navigation_stack(&bytes).is_err());
    for selected in [Some(-1), Some(128)] {
        let mut bad = vec![];
        Config { selected, ..config }
            .binprot_write(&mut bad)
            .unwrap();
        assert!(decode_navigation_stack(&bad).is_err());
    }
    for duration_ms in [-1, 10_001] {
        let mut bad = vec![];
        Config {
            duration_ms,
            ..config
        }
        .binprot_write(&mut bad)
        .unwrap();
        assert!(decode_navigation_stack(&bad).is_err());
    }
    assert!(!config.valid_children(2));
    assert!(config.valid_children(3));
    assert!(!config.valid_children(129));
    let empty = Config {
        selected: None,
        ..config
    };
    assert!(empty.valid_children(0));
    assert!(!empty.valid_children(1));
}
