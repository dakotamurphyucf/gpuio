mod common;
use binprot::BinProtWrite;

fn bytes(hex: &str) -> Vec<u8> {
    hex.trim()
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn independent_ocaml_rust_request_and_event_fixtures() {
    let request = bytes(include_str!("../../../test/fixtures/bridge-v1-request.hex"));
    let events = bytes(include_str!("../../../test/fixtures/bridge-v1-events.hex"));
    let mut actual = Vec::new();
    common::request().binprot_write(&mut actual).unwrap();
    assert_eq!(actual, request);
    assert_eq!(gpuio_protocol::decode(&request).unwrap(), common::request());
    actual.clear();
    common::events().binprot_write(&mut actual).unwrap();
    assert_eq!(actual, events);
}

#[path = "common/choice_fixture.rs"]
mod choice_fixture;
#[path = "common/control_fixture.rs"]
mod control_fixture;

#[test]
fn stable_choice_requests_and_events_match_ocaml() {
    let expected = bytes(include_str!("../../../test/fixtures/choice-v1-request.hex"));
    let request = choice_fixture::request();
    let mut actual = Vec::new();
    request.binprot_write(&mut actual).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(gpuio_protocol::decode(&expected).unwrap(), request);
    for length in 0..expected.len() {
        assert!(gpuio_protocol::decode(&expected[..length]).is_err());
    }
    actual.push(0);
    assert!(gpuio_protocol::decode(&actual).is_err());
    actual.clear();
    choice_fixture::events().binprot_write(&mut actual).unwrap();
    assert_eq!(
        actual,
        bytes(include_str!("../../../test/fixtures/choice-v1-events.hex"))
    );
}
#[path = "common/editor_fixture.rs"]
mod editor_fixture;
#[path = "common/style_fixture.rs"]
mod style_fixture;

#[test]
fn control_configuration_matches_ocaml_and_rejects_malformed_tags() {
    let expected = bytes(include_str!("../../../test/fixtures/controls-v1.hex"));
    let message = control_fixture::request();
    let mut actual = Vec::new();
    message.binprot_write(&mut actual).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(gpuio_protocol::decode(&expected).unwrap(), message);
    for end in 0..expected.len() {
        assert!(gpuio_protocol::decode(&expected[..end]).is_err());
    }
    actual.push(0);
    assert!(gpuio_protocol::decode(&actual).is_err());
    // Apply(window, base, revision, one SetControl(node, checkbox(state, disabled))).
    let valid = [3, 0, 1, 0, 1, 1, 8, 0, 1, 1, 2, 0];
    assert!(gpuio_protocol::decode(&valid).is_ok());
    for index in [9, 10, 11] {
        let mut invalid = valid;
        invalid[index] = 3;
        assert!(gpuio_protocol::decode(&invalid).is_err());
    }
}
#[test]
fn editor_tags_match_ocaml_and_messages_reject_truncation() {
    let requests = editor_fixture::requests();
    let mut actual = Vec::new();
    requests.binprot_write(&mut actual).unwrap();
    assert_eq!(
        actual,
        bytes(include_str!(
            "../../../test/fixtures/editor-v1-requests.hex"
        ))
    );
    actual.clear();
    editor_fixture::events().binprot_write(&mut actual).unwrap();
    assert_eq!(
        actual,
        bytes(include_str!("../../../test/fixtures/editor-v1-events.hex"))
    );
    for request in requests {
        actual.clear();
        request.binprot_write(&mut actual).unwrap();
        assert_eq!(gpuio_protocol::decode(&actual).unwrap(), request);
        for end in 0..actual.len() {
            assert!(gpuio_protocol::decode(&actual[..end]).is_err());
        }
        actual.push(0);
        assert!(gpuio_protocol::decode(&actual).is_err());
    }
}
#[test]
fn every_extended_style_field_matches_the_independent_ocaml_fixture() {
    let expected = bytes(include_str!("../../../test/fixtures/style-v1.hex"));
    let message = style_fixture::request();
    let mut actual = Vec::new();
    message.binprot_write(&mut actual).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(gpuio_protocol::decode(&expected).unwrap(), message);
    for end in 0..expected.len() {
        assert!(gpuio_protocol::decode(&expected[..end]).is_err());
    }
}

#[path = "common/combobox_fixture.rs"]
mod combobox_fixture;
#[test]
fn editable_choice_request_and_exact_snapshot_fixture_match_ocaml() {
    let request = bytes(include_str!(
        "../../../test/fixtures/combobox-v1-request.hex"
    ));
    let events = bytes(include_str!(
        "../../../test/fixtures/combobox-v1-events.hex"
    ));
    let mut actual = Vec::new();
    combobox_fixture::request()
        .binprot_write(&mut actual)
        .unwrap();
    assert_eq!(actual, request);
    assert_eq!(
        gpuio_protocol::decode(&request).unwrap(),
        combobox_fixture::request()
    );
    for length in 0..request.len() {
        assert!(gpuio_protocol::decode(&request[..length]).is_err());
    }
    *actual.last_mut().unwrap() = 2;
    assert!(gpuio_protocol::decode(&actual).is_err());
    actual.clear();
    combobox_fixture::events()
        .binprot_write(&mut actual)
        .unwrap();
    assert_eq!(actual, events);
}

#[test]
fn focus_scope_and_focus_denial_match_ocaml() {
    use gpuio_protocol::{NodeId, WindowId, v1::*};
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let request = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(node, Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                node,
                FocusScopeConfig {
                    trap: true,
                    auto_focus: false,
                    restore_focus: true,
                },
            ),
            Op::SetRoot(Some(node)),
        ],
    });
    let expected = bytes(include_str!("../../../test/fixtures/focus-v1-request.hex"));
    let mut actual = Vec::new();
    request.binprot_write(&mut actual).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(gpuio_protocol::decode(&expected).unwrap(), request);
    for length in 0..expected.len() {
        assert!(gpuio_protocol::decode(&expected[..length]).is_err());
    }
    for index in [15, 16, 17] {
        let mut invalid = expected.clone();
        invalid[index] = 2;
        assert!(gpuio_protocol::decode(&invalid).is_err());
    }
    actual.clear();
    vec![Event::EditorResult(
        7,
        window,
        node,
        EditorResult::Failed(EditorError::FocusBlocked),
    )]
    .binprot_write(&mut actual)
    .unwrap();
    assert_eq!(
        actual,
        bytes(include_str!("../../../test/fixtures/focus-v1-events.hex"))
    );
}

#[test]
fn overlays_match_ocaml_and_reject_malformed_envelopes() {
    use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let request = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(node, Kind::FocusScope, "".into(), Some(handler)),
            Op::SetFocusScope(
                node,
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::SetOverlay(
                node,
                Some(OverlayConfig {
                    kind: OverlayKind::Dialog,
                    label: "Settings".into(),
                    width: 220.,
                    dismiss_on_escape: true,
                    dismiss_on_outside_pointer: false,
                }),
            ),
            Op::SetRoot(Some(node)),
        ],
    });
    let expected = bytes(include_str!(
        "../../../test/fixtures/overlay-v1-request.hex"
    ));
    let mut actual = Vec::new();
    request.binprot_write(&mut actual).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(gpuio_protocol::decode(&expected).unwrap(), request);
    for length in 0..expected.len() {
        assert!(gpuio_protocol::decode(&expected[..length]).is_err());
    }
    for index in [24, expected.len() - 6, expected.len() - 5] {
        let mut malformed = expected.clone();
        malformed[index] = 2;
        assert!(gpuio_protocol::decode(&malformed).is_err());
    }
    actual.clear();
    vec![Event::OverlayDismissed(
        window,
        node,
        handler,
        1,
        Dismissal::Escape,
    )]
    .binprot_write(&mut actual)
    .unwrap();
    assert_eq!(
        actual,
        bytes(include_str!("../../../test/fixtures/overlay-v1-events.hex"))
    );
}

#[test]
fn placement_update_matches_ocaml_without_changing_overlay_records() {
    use gpuio_protocol::{NodeId, WindowId, v1::*};
    let expected = bytes(include_str!(
        "../../../test/fixtures/placement-v1-request.hex"
    ));
    let request = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 1,
        revision: 2,
        operations: vec![Op::SetPlacement(
            NodeId::from_parts(0, 1).unwrap(),
            Some(Placement {
                side: Side::Left,
                align: Align::End,
                offset: -3.5,
            }),
        )],
    });
    let mut actual = Vec::new();
    request.binprot_write(&mut actual).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(gpuio_protocol::decode(&expected).unwrap(), request);
    for length in 0..expected.len() {
        assert!(gpuio_protocol::decode(&expected[..length]).is_err());
    }
    for (index, value) in [(9, 2), (10, 4), (11, 3)] {
        let mut malformed = expected.clone();
        malformed[index] = value;
        assert!(gpuio_protocol::decode(&malformed).is_err());
    }
}

#[test]
fn tooltip_ownership_modes_and_events_match_ocaml() {
    use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let config = TooltipConfig {
        label: "Details".into(),
        width: 200.,
        open_state: TooltipOpenState::Managed(false),
        disabled: false,
        hoverable: true,
        show_delay_ns: 250_000_000,
        hide_delay_ns: 80_000_000,
        skip_delay_ns: 300_000_000,
    };
    let request = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(node, Kind::Tooltip, "".into(), Some(handler)),
            Op::SetTooltip(node, config.clone()),
            Op::SetTooltip(
                node,
                TooltipConfig {
                    open_state: TooltipOpenState::Controlled(true),
                    ..config
                },
            ),
            Op::SetPlacement(
                node,
                Some(Placement {
                    side: Side::Top,
                    align: Align::Center,
                    offset: 6.,
                }),
            ),
        ],
    });
    let expected = bytes(include_str!(
        "../../../test/fixtures/tooltip-v1-request.hex"
    ));
    let mut actual = Vec::new();
    request.binprot_write(&mut actual).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(gpuio_protocol::decode(&actual).unwrap(), request);
    for end in 0..actual.len() {
        assert!(gpuio_protocol::decode(&actual[..end]).is_err());
    }
    actual.push(0);
    assert!(gpuio_protocol::decode(&actual).is_err());
    let events = [true, false]
        .into_iter()
        .map(|open| Event::TooltipOpenChanged(window, node, handler, 1, open))
        .collect::<Vec<_>>();
    actual.clear();
    events.binprot_write(&mut actual).unwrap();
    assert_eq!(
        actual,
        bytes(include_str!("../../../test/fixtures/tooltip-v1-events.hex"))
    );
}

#[path = "common/command_fixture.rs"]
mod command_fixture;
#[test]
fn commands_and_native_targets_match_ocaml() {
    let expected = bytes(include_str!(
        "../../../test/fixtures/commands-v1-request.hex"
    ));
    let request = command_fixture::request();
    let mut actual = Vec::new();
    request.binprot_write(&mut actual).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(gpuio_protocol::decode(&actual).unwrap(), request);
    for end in 0..actual.len() {
        assert!(gpuio_protocol::decode(&actual[..end]).is_err());
    }
    actual.push(0);
    assert!(gpuio_protocol::decode(&actual).is_err());
    actual.clear();
    command_fixture::events()
        .binprot_write(&mut actual)
        .unwrap();
    assert_eq!(
        actual,
        bytes(include_str!(
            "../../../test/fixtures/commands-v1-events.hex"
        ))
    );
}

#[path = "common/menu_fixture.rs"]
mod menu_fixture;
#[test]
fn menu_presentations_nested_definitions_and_bounded_decoding_match_ocaml() {
    let expected = bytes(include_str!("../../../test/fixtures/menus-v1-request.hex"));
    let request = menu_fixture::request();
    let mut actual = Vec::new();
    request.binprot_write(&mut actual).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(gpuio_protocol::decode(&expected).unwrap(), request);
    for end in 0..expected.len() {
        assert!(gpuio_protocol::decode(&expected[..end]).is_err());
    }
    actual.push(0);
    assert!(gpuio_protocol::decode(&actual).is_err());
    use gpuio_protocol::{NodeId, WindowId, v1::*};
    let mut deep = menu_fixture::definition();
    for _ in 0..8 {
        deep = MenuDefinition {
            label: "Nested".into(),
            disabled: false,
            items: vec![MenuItem::Submenu(deep)],
        };
    }
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetMenu(
            NodeId::from_parts(0, 1).unwrap(),
            MenuConfig {
                presentation: MenuPresentation::Button,
                menus: vec![deep],
            },
        )],
    });
    let mut bytes = Vec::new();
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        gpuio_protocol::decode(&bytes),
        Err(gpuio_protocol::DecodeError::LimitExceeded)
    );
}

#[path = "common/palette_fixture.rs"]
mod palette_fixture;
#[test]
fn palette_requests_and_dismissals_match_ocaml() {
    let expected = bytes(include_str!(
        "../../../test/fixtures/palette-v1-request.hex"
    ));
    let mut actual = Vec::new();
    palette_fixture::request()
        .binprot_write(&mut actual)
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(
        gpuio_protocol::decode(&actual).unwrap(),
        palette_fixture::request()
    );
    for length in 0..actual.len() {
        assert!(gpuio_protocol::decode(&actual[..length]).is_err());
    }
    actual.push(0);
    assert!(gpuio_protocol::decode(&actual).is_err());
    actual.clear();
    palette_fixture::events()
        .binprot_write(&mut actual)
        .unwrap();
    assert_eq!(
        actual,
        bytes(include_str!("../../../test/fixtures/palette-v1-events.hex"))
    );
}

#[path = "common/progress_fixture.rs"]
mod progress_fixture;
#[test]
fn progress_request_matches_ocaml_and_decodes() {
    let expected = bytes(include_str!(
        "../../../test/fixtures/progress-v1-request.hex"
    ));
    let mut actual = vec![];
    progress_fixture::request()
        .binprot_write(&mut actual)
        .unwrap();
    assert_eq!(expected, actual);
    assert_eq!(
        gpuio_protocol::decode(&actual).unwrap(),
        progress_fixture::request()
    );
}

#[path = "common/toast_fixture.rs"]
mod toast_fixture;
#[test]
fn toast_ownership_and_dismissal_fixtures_match_ocaml() {
    let request = bytes(include_str!("../../../test/fixtures/toast-v1-request.hex"));
    let events = bytes(include_str!("../../../test/fixtures/toast-v1-events.hex"));
    let mut actual = vec![];
    toast_fixture::request().binprot_write(&mut actual).unwrap();
    assert_eq!(actual, request);
    assert_eq!(
        gpuio_protocol::decode(&actual).unwrap(),
        toast_fixture::request()
    );
    actual.clear();
    toast_fixture::events().binprot_write(&mut actual).unwrap();
    assert_eq!(actual, events);
}
