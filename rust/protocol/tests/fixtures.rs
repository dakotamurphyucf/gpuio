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
