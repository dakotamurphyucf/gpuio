use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, ResourceId,
    asset::*,
    decode,
    v1::{Event, Message},
};
fn id() -> ResourceId {
    ResourceId::from_parts(2, 3).unwrap()
}
fn unhex(hex: &str) -> Vec<u8> {
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
fn independent_asset_commands_and_responses_match_ocaml() {
    let formats = [
        Format::Png,
        Format::Jpeg,
        Format::Webp,
        Format::Gif,
        Format::Svg,
        Format::Bmp,
        Format::Tiff,
        Format::Ico,
        Format::Pnm,
    ];
    let mut requests: Vec<_> = formats
        .into_iter()
        .enumerate()
        .map(|(i, format)| {
            Message::Asset(
                128 + i as i64,
                Request::Begin(format, MAX_ENCODED_BYTES as i64),
            )
        })
        .collect();
    requests.extend([
        Message::Asset(
            140,
            Request::Append(id(), 128, Chunk::new(vec![0, 255, 128, b'A']).unwrap()),
        ),
        Message::Asset(141, Request::Finish(id())),
        Message::Asset(142, Request::Release(id())),
    ]);
    let fixtures: Vec<_> = include_str!("../../../test/fixtures/assets-v1-requests.hex")
        .lines()
        .map(unhex)
        .collect();
    assert_eq!(requests.len(), fixtures.len());
    for (request, fixture) in requests.into_iter().zip(&fixtures) {
        assert_eq!(&encode(request.clone()), fixture);
        assert_eq!(decode(fixture).unwrap(), request);
        for length in 0..fixture.len() {
            assert!(decode(&fixture[..length]).is_err());
        }
        let mut trailing = fixture.clone();
        trailing.push(0);
        assert_eq!(decode(&trailing), Err(DecodeError::Malformed));
    }
    let mut events = vec![
        Event::AssetResponse(128, Response::Begun(id())),
        Event::AssetResponse(140, Response::Ack),
    ];
    events.extend(
        [
            Error::Closed,
            Error::InvalidSize,
            Error::ResourceLimit,
            Error::StaleHandle,
            Error::NotUploading,
            Error::InvalidChunk,
            Error::Incomplete,
            Error::NotReady,
            Error::NativeFailure,
        ]
        .into_iter()
        .enumerate()
        .map(|(i, error)| Event::AssetResponse(150 + i as i64, Response::Failed(error))),
    );
    assert_eq!(
        encode(events),
        unhex(include_str!("../../../test/fixtures/assets-v1-events.hex"))
    );
    let mut invalid_format = fixtures[0].clone();
    invalid_format[5] = 9;
    assert_eq!(decode(&invalid_format), Err(DecodeError::Malformed));
    assert_eq!(
        decode(&encode(Message::Asset(0, Request::Finish(id())))),
        Err(DecodeError::Malformed)
    );
}
#[test]
fn byte_lengths_are_bounded_before_chunk_allocation() {
    let mut header = vec![8, 1, 1, 2, 3, 0]; // Asset(1, Append(id, offset=0, ...))
    binprot::Nat0((MAX_CHUNK_BYTES + 1) as u64)
        .binprot_write(&mut header)
        .unwrap();
    assert_eq!(decode(&header), Err(DecodeError::LimitExceeded));
    let mut short = vec![8, 1, 1, 2, 3, 0, 4];
    short.extend([0, 255]);
    assert_eq!(decode(&short), Err(DecodeError::Malformed));
    let request = Message::Asset(
        1,
        Request::Append(id(), 0, Chunk::new(vec![255; MAX_CHUNK_BYTES]).unwrap()),
    );
    let bytes = encode(request.clone());
    assert!(bytes.len() < gpuio_protocol::v1::MAX_MESSAGE_BYTES);
    assert_eq!(decode(&bytes).unwrap(), request);
    assert!(Chunk::new(vec![0; MAX_CHUNK_BYTES + 1]).is_err());
}
