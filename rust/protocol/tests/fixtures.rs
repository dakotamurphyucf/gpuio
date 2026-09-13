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

#[path = "common/control_fixture.rs"]
mod control_fixture;
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
