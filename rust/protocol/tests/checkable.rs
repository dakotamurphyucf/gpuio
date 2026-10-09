use binprot::BinProtWrite;
use gpuio_protocol::{
    checkable::{Position, TabOrder},
    decode_radio_position, decode_tab_order,
};

fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = Vec::new();
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn fixture(value: &impl BinProtWrite, expected: &str) -> Vec<u8> {
    let bytes = encode(value);
    let hex = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    assert_eq!(hex, expected.trim());
    bytes
}

#[test]
fn tab_order_matches_independent_ocaml_bytes_and_rejects_invalid_boundaries() {
    let value = TabOrder {
        tab_stop: false,
        index: -2,
    };
    let bytes = fixture(
        &value,
        include_str!("../../../test/fixtures/checkable-tab-order.hex"),
    );
    assert_eq!(decode_tab_order(&bytes), Ok(value));
    for end in 0..bytes.len() {
        assert!(decode_tab_order(&bytes[..end]).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(decode_tab_order(&trailing).is_err());
    let mut invalid_bool = bytes;
    invalid_bool[0] = 2;
    assert!(decode_tab_order(&invalid_bool).is_err());
    for index in [i64::MIN, -1_000_001, 1_000_001, i64::MAX] {
        assert!(
            decode_tab_order(&encode(&TabOrder {
                tab_stop: true,
                index
            }))
            .is_err()
        );
    }
    for index in [-1_000_000, -1, 0, 1, 1_000_000] {
        for tab_stop in [false, true] {
            let value = TabOrder { tab_stop, index };
            assert_eq!(decode_tab_order(&encode(&value)), Ok(value));
        }
    }
    assert_eq!(
        TabOrder::default(),
        TabOrder {
            tab_stop: true,
            index: 0
        }
    );
}

#[test]
fn radio_position_matches_independent_bytes_and_requires_a_valid_pair() {
    let value = Position { index: 2, count: 5 };
    let bytes = fixture(
        &value,
        include_str!("../../../test/fixtures/checkable-position.hex"),
    );
    assert_eq!(decode_radio_position(&bytes), Ok(value));
    for end in 0..bytes.len() {
        assert!(decode_radio_position(&bytes[..end]).is_err());
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(decode_radio_position(&trailing).is_err());
    for (index, count) in [
        (-1, 5),
        (0, 0),
        (5, 5),
        (0, 100_001),
        (i64::MIN, 1),
        (0, i64::MAX),
        (i64::MAX, i64::MAX),
    ] {
        assert!(decode_radio_position(&encode(&Position { index, count })).is_err());
    }
    for count in [1, 5, 100_000] {
        for index in [0, count - 1] {
            let value = Position { index, count };
            assert_eq!(decode_radio_position(&encode(&value)), Ok(value));
        }
    }
}

#[test]
fn standalone_radio_transaction_matches_independent_bytes_and_validates_payloads() {
    use gpuio_protocol::{HandlerId, NodeId, WindowId, decode, v1::*};
    let node = NodeId::from_parts(0, 1).unwrap();
    let transaction = |position, order| {
        Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![
                Op::Create(
                    node,
                    Kind::Radio,
                    "Mode".into(),
                    Some(HandlerId::from_parts(0, 1).unwrap()),
                ),
                Op::SetControl(node, Control::Radio(false, position, false)),
                Op::SetTabOrder(node, order),
            ],
        })
    };
    let message = transaction(
        Some(Position { index: 2, count: 5 }),
        Some(TabOrder {
            tab_stop: false,
            index: -2,
        }),
    );
    let bytes = fixture(
        &message,
        include_str!("../../../test/fixtures/checkable-transaction.hex"),
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(decode(&trailing).is_err());
    for position in [
        Position {
            index: -1,
            count: 5,
        },
        Position { index: 5, count: 5 },
        Position {
            index: 0,
            count: 100_001,
        },
    ] {
        assert!(decode(&encode(&transaction(Some(position), None))).is_err());
    }
    for index in [i64::MIN, -1_000_001, 1_000_001, i64::MAX] {
        assert!(
            decode(&encode(&transaction(
                None,
                Some(TabOrder {
                    tab_stop: true,
                    index
                })
            )))
            .is_err()
        );
    }
    let reset = transaction(None, None);
    assert_eq!(decode(&encode(&reset)), Ok(reset));
}

#[test]
fn semantic_radio_group_is_container_metadata_with_validated_orientation() {
    use gpuio_protocol::{
        accessibility::{Config, Live, Orientation, Role},
        decode_accessibility,
        v1::Kind,
    };
    let config = Config {
        role: Some(Role::RadioGroup(Orientation::Vertical)),
        label: Some("Modes".into()),
        description: None,
        live: Live::Off,
        field: None,
        current: None,
    };
    let bytes = fixture(
        &config,
        include_str!("../../../test/fixtures/checkable-radio-group.hex"),
    );
    assert_eq!(decode_accessibility(&bytes), Ok(config.clone()));
    assert!(config.supports(Kind::Container));
    for kind in [Kind::Radio, Kind::Checkbox, Kind::RadioGroup, Kind::Text] {
        assert!(!config.supports(kind));
    }
    for end in 0..bytes.len() {
        assert!(decode_accessibility(&bytes[..end]).is_err());
    }
    let mut invalid = bytes;
    invalid[2] = 2;
    assert!(decode_accessibility(&invalid).is_err());
}

#[test]
fn capability_uses_the_last_positive_signed_bit_without_overflow() {
    use gpuio_protocol::{decode, v1::*};
    assert_eq!(CAP_CHECKABLE_NAVIGATION, 1_i64 << 62);
    assert_eq!(CAPABILITIES, i64::MAX);
    for (mask, expected) in [
        (CAP_CHECKABLE_NAVIGATION, "0003fc0000000000000040"),
        (CAPABILITIES, "0003fcffffffffffffff7f"),
    ] {
        let message = Message::Hello(VERSION, mask);
        let bytes = fixture(&message, expected);
        assert_eq!(decode(&bytes), Ok(message));
    }
}
