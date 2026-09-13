use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError,
    drag_drop::{CustomKind, File, Format, MAX_DATA_BYTES, Payload, Source, Target},
    file_path::FilePath,
};

fn unhex(value: &str) -> Vec<u8> {
    value
        .trim()
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}

fn file(path: &[u8], directory: Option<bool>) -> File {
    File {
        path: FilePath::new(path.to_vec()).unwrap(),
        is_directory: directory,
    }
}

#[test]
fn independent_source_and_target_fixtures_preserve_tags_and_raw_bytes() {
    let kind = CustomKind::new("k/v1".into()).unwrap();
    let sources = [
        Source::new(
            "S".into(),
            Payload::text("hi λ".into()).unwrap(),
            false,
            false,
        )
        .unwrap(),
        Source::new(
            "F".into(),
            Payload::files(vec![
                file(b"/a\xff", Some(false)),
                file(b"/dir", Some(true)),
            ])
            .unwrap(),
            false,
            true,
        )
        .unwrap(),
        Source::new(
            "C".into(),
            Payload::custom(kind.clone(), vec![0, 255, 128, 65]).unwrap(),
            true,
            false,
        )
        .unwrap(),
    ];
    let fixtures: Vec<_> = include_str!("../../../test/fixtures/drag-drop-v1-sources.hex")
        .lines()
        .map(unhex)
        .collect();
    assert_eq!(sources.len(), fixtures.len());
    for (source, bytes) in sources.iter().zip(fixtures) {
        assert_eq!(encode(source), bytes);
        assert_eq!(Source::decode(&bytes).unwrap(), *source);
        for length in 0..bytes.len() {
            assert!(Source::decode(&bytes[..length]).is_err());
        }
        let mut trailing = bytes;
        trailing.push(0);
        assert_eq!(Source::decode(&trailing), Err(DecodeError::Malformed));
    }
    let target = Target::new(
        "T".into(),
        vec![Format::Text, Format::Files, Format::Custom(kind)],
        false,
    )
    .unwrap();
    let bytes = unhex(include_str!(
        "../../../test/fixtures/drag-drop-v1-target.hex"
    ));
    assert_eq!(encode(&target), bytes);
    assert_eq!(Target::decode(&bytes).unwrap(), target);
    for length in 0..bytes.len() {
        assert!(Target::decode(&bytes[..length]).is_err());
    }
}

fn raw_source(payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![1, b'S'];
    bytes.extend(payload);
    bytes.extend([0, 0]);
    bytes
}

#[test]
fn reject_allocation_bombs_invalid_utf8_tags_metadata_and_formats() {
    let bomb = encode(&binprot::Nat0(100_000_000));
    assert_eq!(Source::decode(&bomb), Err(DecodeError::LimitExceeded));
    for prefix in [vec![0], vec![1], vec![1, 1], vec![2], vec![2, 1, b'k']] {
        let mut payload = prefix;
        payload.extend(&bomb);
        assert_eq!(
            Source::decode(&raw_source(&payload)),
            Err(DecodeError::LimitExceeded)
        );
    }
    for payload in [
        vec![0, 1, 255],           // text must be UTF-8
        vec![0, 1, 0],             // text must be NUL-free
        vec![1, 0],                // no empty file lists
        vec![1, 1, 1, b'a', 0],    // no relative paths
        vec![1, 1, 1, b'/', 2],    // invalid metadata option
        vec![1, 1, 1, b'/', 1, 2], // invalid metadata boolean
        vec![2, 1, b':', 0],       // invalid kind
        vec![3],                   // invalid payload tag
    ] {
        assert_eq!(
            Source::decode(&raw_source(&payload)),
            Err(DecodeError::Malformed)
        );
    }
    let mut unknown_metadata = raw_source(&[1, 1, 1, b'/', 0]);
    *unknown_metadata.last_mut().unwrap() = 1; // request desktop export
    assert_eq!(
        Source::decode(&unknown_metadata),
        Err(DecodeError::Malformed)
    );
    for bytes in [
        vec![1, b'T', 0, 0],
        vec![1, b'T', 2, 0, 0, 0],
        vec![1, b'T', 1, 3, 0],
        vec![1, b'T', 1, 0, 2],
    ] {
        assert_eq!(Target::decode(&bytes), Err(DecodeError::Malformed));
    }
    let mut bytes = vec![1, b'T'];
    bytes.extend(bomb);
    assert_eq!(Target::decode(&bytes), Err(DecodeError::LimitExceeded));
}

#[test]
fn readers_enforce_aggregate_file_bytes_and_inclusive_payload_boundaries() {
    let long = file(&vec![b'/'; 16_384], None);
    let mut payload = vec![1];
    payload.extend(encode(&vec![long.clone(); 16]));
    let source = Source::decode(&raw_source(&payload)).unwrap();
    assert_eq!(source.payload().data_bytes(), MAX_DATA_BYTES);
    let mut payload = vec![1];
    payload.extend(encode(&vec![long; 17]));
    assert_eq!(
        Source::decode(&raw_source(&payload)),
        Err(DecodeError::LimitExceeded)
    );
    for payload in [
        Payload::text("a".repeat(MAX_DATA_BYTES)).unwrap(),
        Payload::custom(
            CustomKind::new("k".into()).unwrap(),
            vec![255; MAX_DATA_BYTES],
        )
        .unwrap(),
    ] {
        let source = Source::new("S".into(), payload, false, false).unwrap();
        assert_eq!(Source::decode(&encode(&source)).unwrap(), source);
    }
}
