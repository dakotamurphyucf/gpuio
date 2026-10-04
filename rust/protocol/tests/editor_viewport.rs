use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId, decode,
    editor_viewport::{Offset, Snapshot},
    v1::*,
};
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
fn viewport_requests_and_results_have_independent_bytes_and_bounded_offsets() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(1, 2).unwrap();
    let query = Message::EditorCommand(7, window, node, EditorCommand::ReadViewport);
    let offset = Offset { x: 12.5, y: 40. };
    let scroll = Message::EditorCommand(8, window, node, EditorCommand::ScrollViewport(offset));
    for (message, expected) in [
        (
            query,
            include_str!("../../../test/fixtures/editor-viewport-read.hex"),
        ),
        (
            scroll,
            include_str!("../../../test/fixtures/editor-viewport-scroll.hex"),
        ),
    ] {
        let mut data = bytes(&message);
        assert_eq!(data, fixture(expected));
        assert_eq!(decode(&data).unwrap(), message);
        for n in 0..data.len() {
            assert!(decode(&data[..n]).is_err());
        }
        data.push(0);
        assert!(decode(&data).is_err());
    }
    for invalid in [-1., f64::NAN, f64::INFINITY, 1e9 + 1.] {
        for offset in [Offset { x: invalid, y: 0. }, Offset { x: 0., y: invalid }] {
            assert!(
                decode(&bytes(&Message::EditorCommand(
                    8,
                    window,
                    node,
                    EditorCommand::ScrollViewport(offset)
                )))
                .is_err()
            );
        }
    }
    let geometry = Snapshot {
        offset,
        width: 200.,
        height: 100.,
        line_height: 20.,
        first_buffer_line: 2,
        buffer_line_limit: 7,
    };
    assert!(geometry.is_valid());
    let events = vec![
        Event::EditorResult(7, window, node, EditorResult::Viewport(None)),
        Event::EditorResult(8, window, node, EditorResult::Viewport(Some(geometry))),
        Event::EditorResult(9, window, node, EditorResult::ViewportScrollAccepted),
    ];
    assert_eq!(
        bytes(&events),
        fixture(include_str!(
            "../../../test/fixtures/editor-viewport-events.hex"
        ))
    );
}
