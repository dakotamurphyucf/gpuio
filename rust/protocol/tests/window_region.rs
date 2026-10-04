use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId, decode,
    v1::*,
    window_region::{Edge, Region},
};

#[test]
fn window_regions_match_independent_ocaml_bytes() {
    let cases = [
        (None, "0300010001017a000100"),
        (Some(Region::TitleBar), "0300010001017a00010100"),
        (Some(Region::Exclude), "0300010001017a00010101"),
        (Some(Region::Resize(Edge::Top)), "0300010001017a0001010200"),
        (
            Some(Region::Resize(Edge::Bottom)),
            "0300010001017a0001010201",
        ),
        (Some(Region::Resize(Edge::Left)), "0300010001017a0001010202"),
        (
            Some(Region::Resize(Edge::Right)),
            "0300010001017a0001010203",
        ),
        (
            Some(Region::Resize(Edge::TopLeft)),
            "0300010001017a0001010204",
        ),
        (
            Some(Region::Resize(Edge::TopRight)),
            "0300010001017a0001010205",
        ),
        (
            Some(Region::Resize(Edge::BottomLeft)),
            "0300010001017a0001010206",
        ),
        (
            Some(Region::Resize(Edge::BottomRight)),
            "0300010001017a0001010207",
        ),
    ];
    for (region, expected) in cases {
        let message = Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![Op::SetWindowRegion(
                NodeId::from_parts(0, 1).unwrap(),
                region,
            )],
        });
        let mut bytes = vec![];
        message.binprot_write(&mut bytes).unwrap();
        assert_eq!(
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            expected
        );
        assert_eq!(decode(&bytes), Ok(message));
        for end in 0..bytes.len() {
            assert!(decode(&bytes[..end]).is_err());
        }
        if region.is_some() {
            let mut bad = bytes.clone();
            bad[9] = 3;
            assert!(decode(&bad).is_err());
        }
        if matches!(region, Some(Region::Resize(_))) {
            let mut bad = bytes.clone();
            bad[10] = 8;
            assert!(decode(&bad).is_err());
        }
        bytes.push(0);
        assert!(decode(&bytes).is_err());
    }
}
