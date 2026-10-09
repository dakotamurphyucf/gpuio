use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, decode, editor_geometry::Snapshot, v1::*};
fn bytes<T: BinProtWrite>(value: &T) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn fixture(hex: &str) -> Vec<u8> {
    hex.trim()
        .as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn range_query_has_independent_bytes_and_rejects_bad_source_indices() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(1, 2).unwrap();
    let request = |revision, anchor, head| {
        Message::EditorCommand(
            7,
            window,
            node,
            EditorCommand::ReadRangeBounds(revision, EditorSelection { anchor, head }),
        )
    };
    let message = request(3, 5, 1);
    let expected = fixture(include_str!("../../../test/fixtures/editor-range-read.hex"));
    assert_eq!(bytes(&message), expected);
    assert_eq!(decode(&expected).unwrap(), message);
    for end in 0..expected.len() {
        assert!(decode(&expected[..end]).is_err());
    }
    let mut trailing = expected;
    trailing.push(0);
    assert!(decode(&trailing).is_err());
    for invalid in [request(-1, 5, 1), request(3, -1, 1), request(3, 5, 262145)] {
        assert!(decode(&bytes(&invalid)).is_err());
    }
}
#[test]
fn range_result_preserves_absence_and_signed_origin_with_bounded_dimensions() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(1, 2).unwrap();
    let geometry = Snapshot {
        revision: 3,
        x: -12.5,
        y: 40.,
        width: 0.,
        height: 20.,
    };
    assert!(geometry.is_valid());
    let events = vec![
        Event::EditorResult(7, window, node, EditorResult::RangeBounds(None)),
        Event::EditorResult(8, window, node, EditorResult::RangeBounds(Some(geometry))),
    ];
    assert_eq!(
        bytes(&events),
        fixture(include_str!(
            "../../../test/fixtures/editor-range-events.hex"
        ))
    );
    for invalid in [
        Snapshot {
            revision: -1,
            ..geometry
        },
        Snapshot {
            x: f64::NAN,
            ..geometry
        },
        Snapshot {
            y: f64::INFINITY,
            ..geometry
        },
        Snapshot {
            width: -1.,
            ..geometry
        },
        Snapshot {
            height: 0.,
            ..geometry
        },
        Snapshot {
            x: 1e9 + 1.,
            ..geometry
        },
    ] {
        assert!(!invalid.is_valid());
    }
}
