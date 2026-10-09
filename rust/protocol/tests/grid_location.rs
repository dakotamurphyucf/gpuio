use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, grid_location::*, v1::*};

fn location(edges: [Edge; 4]) -> Location {
    Location {
        column: Axis {
            start: edges[0],
            end: edges[1],
        },
        row: Axis {
            start: edges[2],
            end: edges[3],
        },
    }
}
fn message(value: Location) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![Op::SetStyle(
            NodeId::from_parts(0, 1).unwrap(),
            vec![Style::Fields(vec![Field::GridLocation(value)])],
        )],
    })
}
fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn independent_ocaml_fixtures_and_complete_message_boundaries() {
    use Edge::*;
    for (edges, expected) in [
        ([Auto, Auto, Auto, Auto], "4600000000"),
        ([Line(1), Line(-1), Auto, Auto], "46010101ffff0000"),
        ([Span(2), Span(2), Line(2), Line(4)], "460202020201020104"),
    ] {
        let value = location(edges);
        assert_eq!(hex(&encode(&Field::GridLocation(value))), expected);
        let message = message(value);
        let mut bytes = encode(&message);
        assert_eq!(gpuio_protocol::decode(&bytes), Ok(message));
        for end in 0..bytes.len() {
            assert!(gpuio_protocol::decode(&bytes[..end]).is_err());
        }
        bytes.push(0);
        assert!(gpuio_protocol::decode(&bytes).is_err());
    }
}

#[test]
fn every_edge_rejects_zero_overflow_and_unknown_tags() {
    use Edge::*;
    for index in 0..4 {
        for edge in [
            Line(-1025),
            Line(-1),
            Line(1),
            Line(1025),
            Span(1),
            Span(1024),
        ] {
            let mut edges = [Auto; 4];
            edges[index] = edge;
            let message = message(location(edges));
            assert_eq!(gpuio_protocol::decode(&encode(&message)), Ok(message));
        }
        for edge in [
            Line(0),
            Line(-1026),
            Line(1026),
            Line(i64::MIN),
            Line(i64::MAX),
            Span(-1),
            Span(0),
            Span(1025),
            Span(i64::MIN),
            Span(i64::MAX),
        ] {
            let mut edges = [Auto; 4];
            edges[index] = edge;
            let value = location(edges);
            assert!(!value.valid());
            assert!(gpuio_protocol::decode(&encode(&message(value))).is_err());
        }
        // Four Auto endpoints are the final four bytes of this independent packet.
        let mut bytes = encode(&message(location([Auto; 4])));
        let offset = bytes.len() - 4 + index;
        bytes[offset] = 3;
        assert!(gpuio_protocol::decode(&bytes).is_err());
    }
}

#[test]
fn grid_location_capability_matches_ocaml() {
    assert_eq!(CAP_GRID_LOCATION, 1_i64 << 55);
    assert_eq!(CAPABILITIES, i64::MAX);
    for (mask, expected) in [
        (CAP_GRID_LOCATION, "0003fc0000000000008000"),
        (CAPABILITIES, "0003fcffffffffffffff7f"),
    ] {
        assert_eq!(hex(&encode(&Message::Hello(VERSION, mask))), expected);
    }
}
