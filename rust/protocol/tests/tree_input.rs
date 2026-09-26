use binprot::BinProtWrite;
use gpuio_protocol::{HandlerId, NodeId, WindowId, tree_input::*, v1::*};
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex(value: &impl BinProtWrite) -> String {
    bytes(value).iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn independent_request_event_and_opt_in_fixtures() {
    let requests = vec![
        Request::Navigate(Navigation::Next, Some(Selection::Range { extend: true })),
        Request::Select(42, Selection::Toggle),
        Request::Focus(42),
        Request::SetExpanded(42, true),
        Request::Activate(42),
        Request::SelectActive(Selection::Replace),
        Request::ActivateActive,
    ];
    assert_eq!(
        hex(&requests),
        include_str!("../../../test/fixtures/tree-input-requests.hex").trim()
    );
    assert!(requests.iter().all(Request::is_valid));
    assert!(!Request::Focus(0).is_valid());
    assert!(!Request::SetExpanded(-1, false).is_valid());
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let message = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![Op::SetTreeInput(node, true)],
    });
    let encoded = bytes(&message);
    assert_eq!(
        hex(&message),
        include_str!("../../../test/fixtures/tree-input-transaction.hex").trim()
    );
    assert_eq!(gpuio_protocol::decode(&encoded), Ok(message));
    for end in 0..encoded.len() {
        assert!(gpuio_protocol::decode(&encoded[..end]).is_err());
    }
    let mut invalid = encoded.clone();
    *invalid.last_mut().unwrap() = 2;
    assert!(gpuio_protocol::decode(&invalid).is_err());
    let event = Event::TreeInput(
        window,
        node,
        HandlerId::from_parts(0, 1).unwrap(),
        1,
        requests[0].clone(),
    );
    assert_eq!(
        hex(&event),
        include_str!("../../../test/fixtures/tree-input-event.hex").trim()
    );
}

#[test]
fn independent_tree_focus_transaction_and_strict_target_decode() {
    use gpuio_protocol::list::{ScrollRequest, ScrollTarget};
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::ScrollList(
            NodeId::from_parts(0, 1).unwrap(),
            ScrollRequest {
                serial: 7,
                target: ScrollTarget::FocusTreeRow(42),
            },
        )],
    });
    let encoded = bytes(&message);
    assert_eq!(
        hex(&message),
        include_str!("../../../test/fixtures/tree-focus-transaction.hex").trim()
    );
    assert_eq!(gpuio_protocol::decode(&encoded), Ok(message));
    for end in 0..encoded.len() {
        assert!(gpuio_protocol::decode(&encoded[..end]).is_err());
    }
    let mut invalid = encoded.clone();
    *invalid.last_mut().unwrap() = 0;
    assert!(gpuio_protocol::decode(&invalid).is_err());
    let mut trailing = encoded;
    trailing.push(0);
    assert!(gpuio_protocol::decode(&trailing).is_err());
}

#[test]
fn typeahead_unicode_fixture_and_text_bounds() {
    let request = Request::Typeahead {
        text: "e\u{301}".into(),
        reset: true,
        cycle: true,
    };
    assert!(request.is_valid());
    let event = Event::TreeInput(
        WindowId::from_parts(0, 1).unwrap(),
        NodeId::from_parts(0, 1).unwrap(),
        HandlerId::from_parts(0, 1).unwrap(),
        1,
        request,
    );
    assert_eq!(
        hex(&event),
        include_str!("../../../test/fixtures/tree-typeahead-event.hex").trim()
    );
    for text in [
        "".to_string(),
        "\0".into(),
        "\u{85}".into(),
        "a".repeat(257),
    ] {
        assert!(
            !Request::Typeahead {
                text,
                reset: true,
                cycle: true
            }
            .is_valid()
        );
    }
    assert!(valid_typeahead_text("👨‍👩‍👧‍👦"));
}

#[test]
fn exact_selection_fixture_preserves_desired_membership() {
    let event = Event::TreeInput(
        WindowId::from_parts(0, 1).unwrap(),
        NodeId::from_parts(0, 1).unwrap(),
        HandlerId::from_parts(0, 1).unwrap(),
        1,
        Request::SetSelected(42, false),
    );
    assert_eq!(
        hex(&event),
        include_str!("../../../test/fixtures/tree-selection-event.hex").trim()
    );
    assert_eq!(Request::SetSelected(42, false).target(), Some(42));
    assert!(!Request::SetSelected(0, true).is_valid());
    assert!(!Request::SetSelected(-1, false).is_valid());
}

#[test]
fn independent_move_fixtures_and_both_endpoint_validation() {
    use gpuio_protocol::tree_input::Placement;
    let request = Request::Move {
        source: 42,
        destination: 43,
        placement: Placement::Inside,
    };
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let event = Event::TreeInput(
        window,
        node,
        HandlerId::from_parts(0, 1).unwrap(),
        1,
        request.clone(),
    );
    assert_eq!(
        hex(&event),
        include_str!("../../../test/fixtures/tree-move-event.hex").trim()
    );
    assert_eq!(request.targets().collect::<Vec<_>>(), vec![43, 42]);
    assert!(request.is_valid());
    for (source, destination) in [(0, 43), (42, -1), (42, 42)] {
        assert!(
            !Request::Move {
                source,
                destination,
                placement: Placement::Before
            }
            .is_valid()
        );
    }
    let message = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![Op::SetTreeMoves(node, true)],
    });
    let encoded = bytes(&message);
    assert_eq!(
        hex(&message),
        include_str!("../../../test/fixtures/tree-move-transaction.hex").trim()
    );
    assert_eq!(gpuio_protocol::decode(&encoded), Ok(message));
    for end in 0..encoded.len() {
        assert!(gpuio_protocol::decode(&encoded[..end]).is_err());
    }
    let mut invalid = encoded.clone();
    *invalid.last_mut().unwrap() = 2;
    assert!(gpuio_protocol::decode(&invalid).is_err());
    invalid = encoded;
    invalid.push(0);
    assert!(gpuio_protocol::decode(&invalid).is_err());
}

#[test]
fn managed_trees_have_a_distinct_capability_from_retained_view_trees() {
    assert_eq!(CAPABILITIES & CAP_MANAGED_TREES, 1_i64 << 39);
    assert_ne!(CAP_MANAGED_TREES, CAP_TREE);
    let hello = Message::Hello(VERSION, CAP_MANAGED_TREES);
    assert_eq!(hex(&hello), "0001fc0000000080000000");
    assert_eq!(gpuio_protocol::decode(&bytes(&hello)), Ok(hello));
}
