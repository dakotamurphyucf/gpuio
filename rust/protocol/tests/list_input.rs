use binprot::BinProtWrite;
use gpuio_protocol::{HandlerId, NodeId, WindowId, accessibility as ax, list_input::*, v1::*};
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn fixture(value: &impl BinProtWrite, expected: &str) {
    assert_eq!(
        bytes(value)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        expected.trim()
    );
}
fn config() -> Config {
    Config {
        generation: 7,
        cursor: Some(42),
        query: Some(NodeId::from_parts(2, 1).unwrap()),
        selection_on_navigation: true,
        disabled: false,
        busy: true,
    }
}
fn requests() -> Vec<Request> {
    vec![
        Request::Navigate(Navigation::Next, Some(Gesture::Range { extend: true })),
        Request::Select(42, Gesture::Toggle),
        Request::Focus(42),
        Request::SelectActive(Gesture::Replace),
        Request::Confirm(42, Confirmation::Secondary),
        Request::ConfirmActive(Confirmation::Primary),
        Request::Context(42),
        Request::ContextActive,
        Request::SetSelected(42, false),
        Request::Cancel,
        Request::Navigate(Navigation::First, None),
        Request::ConfirmActive(Confirmation::Secondary),
    ]
}
#[test]
fn paired_fixed_size_requests_event_and_operation_preserve_prior_tags() {
    fixture(
        &requests(),
        include_str!("../../../test/fixtures/list-input-requests.hex"),
    );
    assert!(requests().into_iter().all(Request::is_valid));
    let node = NodeId::from_parts(0, 1).unwrap();
    let window = WindowId::from_parts(0, 1).unwrap();
    let event = Event::ListInput(
        window,
        node,
        HandlerId::from_parts(0, 1).unwrap(),
        1,
        7,
        requests()[0],
    );
    fixture(
        &event,
        include_str!("../../../test/fixtures/list-input-event.hex"),
    );
    let message = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetListInput(node, Some(config())),
            Op::SetListInput(node, None),
        ],
    });
    fixture(
        &message,
        include_str!("../../../test/fixtures/list-input-transaction.hex"),
    );
    let data = bytes(&message);
    assert_eq!(gpuio_protocol::decode(&data), Ok(message));
    for length in 0..data.len() {
        assert!(gpuio_protocol::decode(&data[..length]).is_err());
    }
    let mut trailing = data.clone();
    trailing.push(0);
    assert!(gpuio_protocol::decode(&trailing).is_err());
    // Option/config generation/row/query handle/bool bytes in the independently authored fixture.
    for (offset, value) in [(9, 2), (10, 0), (12, 0), (15, 0), (16, 2), (17, 2), (18, 2)] {
        let mut invalid = data.clone();
        invalid[offset] = value;
        assert!(gpuio_protocol::decode(&invalid).is_err(), "offset {offset}");
    }
}
#[test]
fn interaction_epoch_is_independent_from_cursor_and_busy() {
    let old = config();
    assert!(
        Config {
            cursor: None,
            busy: false,
            ..old
        }
        .can_replace(old)
    );
    for config in [
        Config {
            generation: 6,
            ..old
        },
        Config { query: None, ..old },
        Config {
            disabled: true,
            ..old
        },
        Config {
            selection_on_navigation: false,
            ..old
        },
    ] {
        assert!(!config.can_replace(old));
        assert!(
            Config {
                generation: 8,
                ..config
            }
            .can_replace(old)
        );
    }
    assert!(
        !Config {
            generation: 0,
            ..old
        }
        .is_valid()
    );
    assert!(
        !Config {
            cursor: Some(-1),
            ..old
        }
        .is_valid()
    );
    for id in [0, -1] {
        for request in [
            Request::Select(id, Gesture::Replace),
            Request::Focus(id),
            Request::Confirm(id, Confirmation::Primary),
            Request::Context(id),
            Request::SetSelected(id, true),
        ] {
            assert!(!request.is_valid());
        }
    }
}
#[test]
fn listbox_option_semantics_preserve_unknown_total_and_reject_invalid_bounds() {
    let metadata = |role, label: &str| ax::Config {
        role: Some(role),
        label: Some(label.into()),
        description: None,
        live: ax::Live::Off,
        field: None,
        current: None,
    };
    let list = metadata(ax::Role::ListBox(true), "Pick");
    let item = ax::OptionItem {
        index: 4,
        count: Some(9),
        selected: true,
        disabled: false,
    };
    let option = metadata(ax::Role::OptionItem(item), "Item");
    fixture(
        &list,
        include_str!("../../../test/fixtures/accessibility-listbox.hex"),
    );
    fixture(
        &option,
        include_str!("../../../test/fixtures/accessibility-option.hex"),
    );
    for config in [list.clone(), option.clone()] {
        let encoded = bytes(&config);
        assert_eq!(gpuio_protocol::decode_accessibility(&encoded), Ok(config));
        for end in 0..encoded.len() {
            assert!(gpuio_protocol::decode_accessibility(&encoded[..end]).is_err());
        }
        let mut invalid = encoded.clone();
        invalid[1] = 19;
        assert!(gpuio_protocol::decode_accessibility(&invalid).is_err());
        let mut extra = encoded;
        extra.push(0);
        assert!(gpuio_protocol::decode_accessibility(&extra).is_err());
    }
    assert!(list.supports(Kind::VirtualList));
    assert!(!list.supports(Kind::Container));
    assert!(option.supports(Kind::Container));
    assert!(!option.supports(Kind::VirtualList));
    for item in [
        ax::OptionItem { index: -1, ..item },
        ax::OptionItem {
            index: 1_000_000,
            ..item
        },
        ax::OptionItem {
            count: Some(4),
            ..item
        },
        ax::OptionItem {
            count: Some(1_000_001),
            ..item
        },
    ] {
        assert!(!item.is_valid());
        assert!(
            gpuio_protocol::decode_accessibility(&bytes(&metadata(
                ax::Role::OptionItem(item),
                "Item"
            )))
            .is_err()
        );
    }
    assert!(
        ax::OptionItem {
            index: 999_999,
            count: None,
            ..item
        }
        .is_valid()
    );
}
