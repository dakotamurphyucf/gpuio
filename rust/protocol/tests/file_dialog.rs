#[path = "common/file_dialog_fixture.rs"]
mod fixture;
use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, decode, v1::*};

fn bytes(hex: &str) -> Vec<u8> {
    hex.trim()
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
fn encode(value: impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
#[test]
fn independent_file_dialog_fixtures_and_bounded_request_decoding() {
    let expected = include_str!("../../../test/fixtures/file-dialog-v1-requests.hex")
        .lines()
        .map(bytes)
        .collect::<Vec<_>>();
    let requests = fixture::requests();
    assert_eq!(expected.len(), requests.len());
    for (expected, request) in expected.iter().zip(&requests) {
        assert_eq!(encode(request.clone()), *expected);
        assert_eq!(decode(expected).unwrap(), *request);
        for length in 0..expected.len() {
            assert!(decode(&expected[..length]).is_err());
        }
        let mut trailing = expected.clone();
        trailing.push(0);
        assert_eq!(decode(&trailing), Err(DecodeError::Malformed));
    }
    assert_eq!(
        encode(fixture::events()),
        bytes(include_str!(
            "../../../test/fixtures/file-dialog-v1-events.hex"
        ))
    );
    let Message::FileDialog(_, window, config) = requests[0].clone() else {
        panic!()
    };
    assert_eq!(
        decode(&encode(Message::FileDialog(0, window, config))),
        Err(DecodeError::Malformed)
    );
    let Message::FileDialog(correlation, window, FileDialogConfig::Save(mut config)) =
        requests[1].clone()
    else {
        panic!()
    };
    config.suggested_name = "x".repeat(256);
    assert_eq!(
        decode(&encode(Message::FileDialog(
            correlation,
            window,
            FileDialogConfig::Save(config)
        ))),
        Err(DecodeError::LimitExceeded)
    );
    let mut invalid = expected[0].clone();
    let position = invalid
        .windows(5)
        .position(|part| part == b"/tmp/")
        .unwrap();
    invalid[position] = b'x';
    assert_eq!(decode(&invalid), Err(DecodeError::Malformed));
    invalid[position] = b'/';
    invalid[position + 1] = 0;
    assert_eq!(decode(&invalid), Err(DecodeError::Malformed));
}
