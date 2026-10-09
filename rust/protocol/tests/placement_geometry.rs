use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId, decode,
    placement_geometry::{Config, Corner, Point},
    v1::*,
};
#[test]
fn independent_corners_margins_reset_and_malformed_geometry() {
    let node = NodeId::from_parts(0, 1).unwrap();
    let mut operations = [
        Corner::TopLeft,
        Corner::TopRight,
        Corner::BottomLeft,
        Corner::BottomRight,
    ]
    .into_iter()
    .map(|corner| {
        Op::SetPlacementGeometry(
            node,
            Some(Config {
                viewport_margin: 24.5,
                point: Some(Point {
                    corner,
                    x: 125.5,
                    y: -12.25,
                }),
            }),
        )
    })
    .collect::<Vec<_>>();
    operations.extend([
        Op::SetPlacementGeometry(
            node,
            Some(Config {
                viewport_margin: 0.,
                point: None,
            }),
        ),
        Op::SetPlacementGeometry(node, None),
    ]);
    let message = Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations,
    });
    let mut bytes = vec![];
    message.binprot_write(&mut bytes).unwrap();
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/placement-geometry.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    for bad in [-1., 16385., f64::NAN, f64::INFINITY] {
        let mut invalid = bytes.clone();
        invalid[10..18].copy_from_slice(&bad.to_le_bytes());
        assert!(decode(&invalid).is_err());
    }
    for bad in [-1_000_001., 1_000_001., f64::NAN] {
        let mut invalid = bytes.clone();
        invalid[20..28].copy_from_slice(&bad.to_le_bytes());
        assert!(decode(&invalid).is_err());
    }
    let mut invalid = bytes.clone();
    invalid[19] = 4;
    assert!(decode(&invalid).is_err());
    bytes.push(0);
    assert!(decode(&bytes).is_err());
}
