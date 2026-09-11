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
