use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, choice_picker::*, decode_choice_picker_config, decode_choice_picker_event,
};
fn item(id: &str, label: &str, disabled: bool) -> Item {
    Item {
        id: id.into(),
        label: label.into(),
        disabled,
    }
}
fn fixture() -> Config {
    Config {
        label: "Pick".into(),
        options: Collection::Grouped(vec![Group {
            id: "g".into(),
            label: "Group".into(),
            items: vec![item("a", "Alpha", false), item("b", "β", true)],
        }]),
        selected: Selection::Multiple(vec!["b".into(), "a".into()]),
        disabled: false,
        search: Search::Substring,
        clearable: true,
        open_state: OpenState::Controlled(true),
        placeholder: "Choose".into(),
        search_placeholder: "Find".into(),
    }
}
fn encode(config: &Config) -> Vec<u8> {
    let mut bytes = vec![];
    config.binprot_write(&mut bytes).unwrap();
    bytes
}
#[test]
fn picker_config_matches_independent_bytes_and_rejects_truncation_and_trailing_data() {
    let empty = Config {
        options: Collection::Flat(vec![]),
        selected: Selection::Single(None),
        disabled: true,
        search: Search::Application,
        clearable: false,
        open_state: OpenState::Managed(true),
        placeholder: String::new(),
        search_placeholder: String::new(),
        ..fixture()
    };
    for (value, hex) in [
        (
            fixture(),
            include_str!("../../../test/fixtures/choice-picker-grouped.hex"),
        ),
        (
            empty,
            include_str!("../../../test/fixtures/choice-picker-empty.hex"),
        ),
    ] {
        let bytes = encode(&value);
        assert_eq!(
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            hex.trim()
        );
        assert!(value.is_valid());
        assert_eq!(decode_choice_picker_config(&bytes), Ok(value));
        for n in 0..bytes.len() {
            assert!(decode_choice_picker_config(&bytes[..n]).is_err());
        }
        let mut extra = bytes;
        extra.push(0);
        assert_eq!(
            decode_choice_picker_config(&extra),
            Err(DecodeError::Malformed)
        );
    }
    let mut bytes = encode(&fixture());
    bytes[1] = 0xff;
    assert_eq!(
        decode_choice_picker_config(&bytes),
        Err(DecodeError::Malformed)
    );
    let mut bytes = encode(&fixture());
    bytes[5] = 2;
    assert_eq!(
        decode_choice_picker_config(&bytes),
        Err(DecodeError::Malformed)
    );
    // Independent fixture offsets: item flags, root disabled/clearable and open flag.
    for offset in [24, 30, 37, 39, 41] {
        let mut bytes = encode(&fixture());
        bytes[offset] = 2;
        assert_eq!(
            decode_choice_picker_config(&bytes),
            Err(DecodeError::Malformed)
        );
    }
    for (offset, tag) in [(31, 2), (38, 3), (40, 2)] {
        let mut bytes = encode(&fixture());
        bytes[offset] = tag;
        assert_eq!(
            decode_choice_picker_config(&bytes),
            Err(DecodeError::Malformed)
        );
    }
}
#[test]
fn picker_semantics_reject_duplicate_groups_items_selections_and_missing_values() {
    let mut cases = vec![];
    let Collection::Grouped(groups) = fixture().options else {
        unreachable!()
    };
    cases.push(Config {
        options: Collection::Grouped(vec![groups[0].clone(), groups[0].clone()]),
        ..fixture()
    });
    let mut duplicate = groups[0].clone();
    duplicate.id = "other".into();
    cases.push(Config {
        options: Collection::Grouped(vec![groups[0].clone(), duplicate]),
        ..fixture()
    });
    for ids in [vec!["a", "a"], vec!["gone"]] {
        cases.push(Config {
            selected: Selection::Multiple(ids.into_iter().map(str::to_string).collect()),
            ..fixture()
        });
    }
    cases.push(Config {
        label: "bad\0label".into(),
        ..fixture()
    });
    cases.push(Config {
        selected: Selection::Single(Some("gone".into())),
        ..fixture()
    });
    for config in cases {
        assert!(!config.is_valid());
        assert!(decode_choice_picker_config(&encode(&config)).is_err());
    }
    for search in [Search::None, Search::Substring, Search::Application] {
        for open_state in [
            OpenState::Managed(false),
            OpenState::Managed(true),
            OpenState::Controlled(false),
            OpenState::Controlled(true),
        ] {
            let config = Config {
                search,
                open_state,
                ..fixture()
            };
            assert_eq!(decode_choice_picker_config(&encode(&config)), Ok(config));
        }
    }
}
#[test]
fn picker_decoder_limits_nested_items_groups_catalog_and_selection_before_allocation() {
    for selected in [
        Selection::Multiple(vec!["a".into(); MAX_ITEMS + 1]),
        Selection::Multiple(vec!["x".repeat(256); MAX_TEXT_BYTES / 256 + 1]),
    ] {
        let oversized = Config {
            selected,
            ..fixture()
        };
        assert_eq!(
            decode_choice_picker_config(&encode(&oversized)),
            Err(DecodeError::LimitExceeded)
        );
    }
    let declared = |tag: u8, count: u64| {
        let mut bytes = vec![0, tag]; // empty label is later malformed; count must reject first.
        binprot::Nat0(count).binprot_write(&mut bytes).unwrap();
        bytes
    };
    assert_eq!(
        decode_choice_picker_config(&declared(0, 4097)),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_choice_picker_config(&declared(1, 257)),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_choice_picker_config(&vec![0; MAX_CONFIG_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    let groups: Vec<_> = (0..256)
        .map(|i| Group {
            id: i.to_string(),
            label: "x".repeat(1024),
            items: vec![],
        })
        .collect();
    let oversized = Config {
        options: Collection::Grouped(groups),
        selected: Selection::Multiple(vec![]),
        ..fixture()
    };
    assert!(!oversized.is_valid());
    assert_eq!(
        decode_choice_picker_config(&encode(&oversized)),
        Err(DecodeError::LimitExceeded)
    );
    let items: Vec<_> = (0..MAX_ITEMS)
        .map(|i| item(&i.to_string(), "Value", false))
        .collect();
    let maximum = Config {
        selected: Selection::Multiple(items.iter().map(|i| i.id.clone()).collect()),
        options: Collection::Flat(items.clone()),
        ..fixture()
    };
    assert_eq!(decode_choice_picker_config(&encode(&maximum)), Ok(maximum));
    let oversized = Config {
        options: Collection::Grouped(vec![
            Group {
                id: "one".into(),
                label: "One".into(),
                items,
            },
            Group {
                id: "two".into(),
                label: "Two".into(),
                items: vec![item("extra", "Value", false)],
            },
        ]),
        selected: Selection::Multiple(vec![]),
        ..fixture()
    };
    assert!(!oversized.is_valid());
    assert_eq!(
        decode_choice_picker_config(&encode(&oversized)),
        Err(DecodeError::LimitExceeded)
    );
}

#[test]
fn picker_events_match_independent_bytes_with_direction_and_identity_validation() {
    let events = [
        Event::SelectionRequested(Request::Select("α".into()), None),
        Event::SelectionRequested(Request::Toggle("b".into()), None),
        Event::SelectionRequested(Request::Clear, None),
        Event::OpenRequested(true, OpenReason::Trigger),
        Event::OpenRequested(true, OpenReason::Keyboard),
        Event::OpenRequested(false, OpenReason::Escape),
        Event::OpenRequested(false, OpenReason::OutsidePointer),
        Event::OpenRequested(false, OpenReason::FocusLeft),
        Event::OpenRequested(false, OpenReason::Selection),
        Event::Visibility(Visibility::Snapshot(false)),
        Event::Visibility(Visibility::Snapshot(true)),
        Event::Visibility(Visibility::Changed(
            true,
            VisibilityReason::Interaction(OpenReason::Trigger),
        )),
        Event::Visibility(Visibility::Changed(
            false,
            VisibilityReason::Interaction(OpenReason::Escape),
        )),
        Event::Visibility(Visibility::Changed(true, VisibilityReason::Application)),
        Event::Visibility(Visibility::Changed(false, VisibilityReason::Unavailable)),
        Event::Visibility(Visibility::Changed(false, VisibilityReason::Application)),
    ];
    let lines: Vec<_> = include_str!("../../../test/fixtures/choice-picker-events.hex")
        .lines()
        .collect();
    assert_eq!(lines.len(), events.len());
    for (event, hex) in events.into_iter().zip(lines) {
        let mut bytes = vec![];
        event.binprot_write(&mut bytes).unwrap();
        assert_eq!(
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            hex
        );
        assert!(event.is_valid());
        assert_eq!(decode_choice_picker_event(&bytes), Ok(event));
        for n in 0..bytes.len() {
            assert!(decode_choice_picker_event(&bytes[..n]).is_err());
        }
        bytes.push(0);
        assert_eq!(
            decode_choice_picker_event(&bytes),
            Err(DecodeError::Malformed)
        );
    }
    for bytes in [
        vec![3],
        vec![0, 3],
        vec![1, 2, 0],
        vec![1, 1, 6],
        vec![2, 2],
        vec![2, 0, 2],
        vec![2, 1, 2, 1],
        vec![2, 1, 0, 3],
        vec![2, 1, 0, 0, 6],
        vec![0, 0, 1, 0xff],
        vec![0, 1, 1, 0xff],
    ] {
        assert!(decode_choice_picker_event(&bytes).is_err(), "{bytes:?}");
    }
    for event in [
        Event::SelectionRequested(Request::Select(String::new()), None),
        Event::SelectionRequested(Request::Toggle("a\0b".into()), None),
        Event::SelectionRequested(Request::Select("x".repeat(257)), None),
        Event::OpenRequested(false, OpenReason::Keyboard),
        Event::OpenRequested(true, OpenReason::Escape),
        Event::OpenRequested(true, OpenReason::OutsidePointer),
        Event::OpenRequested(true, OpenReason::FocusLeft),
        Event::OpenRequested(true, OpenReason::Selection),
        Event::Visibility(Visibility::Changed(true, VisibilityReason::Unavailable)),
        Event::Visibility(Visibility::Changed(
            false,
            VisibilityReason::Interaction(OpenReason::Keyboard),
        )),
    ] {
        assert!(!event.is_valid());
        let mut bytes = vec![];
        event.binprot_write(&mut bytes).unwrap();
        assert!(decode_choice_picker_event(&bytes).is_err());
    }
    assert_eq!(
        decode_choice_picker_event(&vec![0; MAX_EVENT_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    let max_id = Event::SelectionRequested(Request::Toggle("x".repeat(256)), None);
    let mut bytes = vec![];
    max_id.binprot_write(&mut bytes).unwrap();
    assert_eq!(decode_choice_picker_event(&bytes), Ok(max_id));
}

#[test]
fn query_snapshots_match_independent_bytes_and_validate_utf8_ranges_and_composition() {
    use gpuio_protocol::{
        NodeId,
        v1::{EditorSelection, EditorSnapshot},
    };
    let query = Query {
        node: NodeId::from_parts(7, 2).unwrap(),
        snapshot: EditorSnapshot {
            revision: 9,
            text: "λx".into(),
            selection: EditorSelection { anchor: 3, head: 3 },
            composition: None,
            focused: true,
        },
    };
    let mut composing = query.clone();
    composing.snapshot.composition = Some(EditorSelection { anchor: 0, head: 2 });
    let events = [
        Event::SelectionRequested(Request::Select("α".into()), Some(query.clone())),
        Event::QueryChanged(query.clone()),
        Event::QueryChanged(composing.clone()),
        Event::SelectionRequested(Request::Clear, Some(query.clone())),
    ];
    let lines: Vec<_> = include_str!("../../../test/fixtures/choice-picker-query-events.hex")
        .lines()
        .collect();
    assert_eq!(events.len(), lines.len());
    for (event, hex) in events.into_iter().zip(lines) {
        let mut bytes = vec![];
        event.binprot_write(&mut bytes).unwrap();
        assert!(event.is_valid());
        assert_eq!(
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            hex
        );
        assert_eq!(decode_choice_picker_event(&bytes), Ok(event));
        for n in 0..bytes.len() {
            assert!(decode_choice_picker_event(&bytes[..n]).is_err());
        }
        bytes.push(0);
        assert!(decode_choice_picker_event(&bytes).is_err());
    }
    assert!(!Event::SelectionRequested(Request::Clear, Some(composing)).is_valid());
    let mut invalid = Vec::new();
    for text in ["a\nb", "a\rb", "a\0b"] {
        let mut q = query.clone();
        q.snapshot.text = text.into();
        invalid.push(q);
    }
    let mut q = query.clone();
    q.snapshot.revision = -1;
    invalid.push(q);
    for offset in [-1, 1, 4, i64::MAX] {
        let mut q = query.clone();
        q.snapshot.selection.head = offset;
        invalid.push(q);
    }
    let mut q = query.clone();
    q.snapshot.composition = Some(EditorSelection { anchor: 2, head: 0 });
    invalid.push(q);
    let mut q = query.clone();
    q.snapshot.composition = Some(EditorSelection { anchor: 0, head: 1 });
    invalid.push(q);
    let mut q = query.clone();
    q.snapshot.text = "x".repeat(MAX_QUERY_BYTES + 1);
    invalid.push(q);
    for query in invalid {
        let event = Event::QueryChanged(query);
        assert!(!event.is_valid());
        let mut bytes = vec![];
        event.binprot_write(&mut bytes).unwrap();
        assert!(decode_choice_picker_event(&bytes).is_err());
    }
    let mut bytes = vec![];
    Event::QueryChanged(query.clone())
        .binprot_write(&mut bytes)
        .unwrap();
    for (offset, invalid) in [(2, 0), (5, 0xff), (10, 2), (11, 2)] {
        let mut bad = bytes.clone();
        bad[offset] = invalid;
        assert!(decode_choice_picker_event(&bad).is_err(), "{bad:?}");
    }
    let mut maximum = query;
    maximum.snapshot.text = "x".repeat(MAX_QUERY_BYTES);
    maximum.snapshot.selection = EditorSelection {
        anchor: MAX_QUERY_BYTES as i64,
        head: 0,
    };
    let event = Event::SelectionRequested(Request::Toggle("x".repeat(256)), Some(maximum));
    let mut bytes = vec![];
    event.binprot_write(&mut bytes).unwrap();
    assert!(bytes.len() <= MAX_EVENT_BYTES);
    assert_eq!(decode_choice_picker_event(&bytes), Ok(event));
}

fn presentation() -> Presentation {
    use gpuio_protocol::v1::{Field, Style};
    Presentation {
        config: fixture(),
        popup_width: 320.,
        max_height: 240.,
        estimated_row_height: 32.,
        overscan: 64.,
        empty_label: "No matches".into(),
        popup_style: vec![Style::Fields(vec![Field::Opacity(0.8)])],
        option_style: vec![Style::State(7, vec![Field::Opacity(0.9)])],
        header_style: vec![Style::Fields(vec![Field::Opacity(0.6)])],
        empty_style: vec![Style::Fields(vec![Field::Opacity(0.5)])],
        slots: vec![
            Slot::Trigger,
            Slot::Query,
            Slot::Empty,
            Slot::Footer,
            Slot::Group("g".into()),
            Slot::Option("a".into(), Checkmark::Custom),
            Slot::Option("b".into(), Checkmark::Native),
        ],
    }
}
#[test]
fn presentation_matches_independent_geometry_style_and_slot_bytes() {
    use gpuio_protocol::decode_choice_picker_presentation as decode;
    let value = presentation();
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/choice-picker-presentation.hex").trim()
    );
    assert!(value.has_valid_shape());
    assert_eq!(decode(&bytes), Ok(value));
    for n in 0..bytes.len() {
        assert!(decode(&bytes[..n]).is_err());
    }
    bytes.push(0);
    assert!(decode(&bytes).is_err());
    assert_eq!(
        decode(&vec![0; MAX_PRESENTATION_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
}
#[test]
fn presentation_rejects_invalid_geometry_roles_membership_and_allocation_budgets() {
    use gpuio_protocol::{
        decode_choice_picker_presentation as decode,
        v1::{Field, Style},
    };
    let reject = |value: Presentation| {
        let mut bytes = vec![];
        value.binprot_write(&mut bytes).unwrap();
        assert!(decode(&bytes).is_err());
    };
    for slots in [
        vec![],
        vec![Slot::Query, Slot::Query],
        vec![Slot::Query, Slot::Group("missing".into())],
        vec![
            Slot::Query,
            Slot::Option("missing".into(), Checkmark::Native),
        ],
        vec![
            Slot::Query,
            Slot::Option("a".into(), Checkmark::Native),
            Slot::Option("a".into(), Checkmark::Custom),
        ],
        vec![
            Slot::Query,
            Slot::Group("g".into()),
            Slot::Group("g".into()),
        ],
        vec![Slot::Query; MAX_SLOTS + 1],
    ] {
        reject(Presentation {
            slots,
            ..presentation()
        });
    }
    for bad in [0., -1., 1_000_001., f64::NAN, f64::INFINITY] {
        reject(Presentation {
            popup_width: bad,
            ..presentation()
        });
        reject(Presentation {
            max_height: bad,
            ..presentation()
        });
        reject(Presentation {
            estimated_row_height: bad,
            ..presentation()
        });
    }
    reject(Presentation {
        estimated_row_height: 0.5,
        ..presentation()
    });
    reject(Presentation {
        overscan: 4097.,
        ..presentation()
    });
    reject(Presentation {
        empty_label: String::new(),
        ..presentation()
    });
    reject(Presentation {
        header_style: vec![Style::Fields(vec![Field::Opacity(0.5); 129])],
        ..presentation()
    });
    reject(Presentation {
        header_style: vec![Style::Fields(vec![]); 129],
        ..presentation()
    });
    let mut disabled_search = presentation();
    disabled_search.config.search = Search::None;
    reject(disabled_search);
    // Slot identity namespaces are separate, even when group/item text is equal.
    let mut valid = presentation();
    let Collection::Grouped(groups) = &mut valid.config.options else {
        unreachable!()
    };
    groups[0].id = "a".into();
    valid.slots[4] = Slot::Group("a".into());
    assert!(valid.has_valid_shape());
}

#[test]
fn picker_operation_and_kind_match_independent_envelope() {
    use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
    let node = NodeId::from_parts(1, 2).unwrap();
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(
                node,
                Kind::ChoicePicker,
                String::new(),
                Some(HandlerId::from_parts(3, 4).unwrap()),
            ),
            Op::SetChoicePicker(node, Box::new(presentation())),
        ],
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/choice-picker-operation.hex").trim()
    );
    assert_eq!(gpuio_protocol::decode(&bytes), Ok(message));
    for length in 0..bytes.len() {
        assert!(gpuio_protocol::decode(&bytes[..length]).is_err());
    }
    bytes.push(0);
    assert!(gpuio_protocol::decode(&bytes).is_err());
}

#[test]
fn picker_bridge_envelopes_match_independent_event_identity_and_snapshot_bytes() {
    use gpuio_protocol::{HandlerId, NodeId, WindowId, v1};
    let query = Query {
        node: NodeId::from_parts(7, 2).unwrap(),
        snapshot: v1::EditorSnapshot {
            revision: 9,
            text: "λx".into(),
            selection: v1::EditorSelection { anchor: 3, head: 3 },
            composition: None,
            focused: true,
        },
    };
    let mut composing = query.clone();
    composing.snapshot.composition = Some(v1::EditorSelection { anchor: 0, head: 2 });
    let events = [
        Event::SelectionRequested(Request::Select("α".into()), Some(query.clone())),
        Event::QueryChanged(query.clone()),
        Event::QueryChanged(composing),
        Event::SelectionRequested(Request::Clear, Some(query)),
    ];
    let expected: Vec<_> = include_str!("../../../test/fixtures/choice-picker-envelope.hex")
        .lines()
        .collect();
    assert_eq!(events.len(), expected.len());
    for (event, hex) in events.into_iter().zip(expected) {
        let event = v1::Event::ChoicePickerEvent(
            WindowId::from_parts(0, 1).unwrap(),
            NodeId::from_parts(1, 2).unwrap(),
            HandlerId::from_parts(3, 4).unwrap(),
            5,
            event,
        );
        let mut bytes = vec![];
        vec![event].binprot_write(&mut bytes).unwrap();
        assert_eq!(
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            hex
        );
    }
}
