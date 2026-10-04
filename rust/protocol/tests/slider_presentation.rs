use binprot::BinProtWrite;
use gpuio_protocol::{
    NodeId, WindowId, decode,
    slider_presentation::{Appearance, Fill},
    v1::*,
};
fn config() -> Appearance {
    Appearance {
        fill: Fill::Remaining,
        track_thickness: 6.,
        track_radius: 3.,
        thumb_size: 16.,
        target_size: 28.,
        track_color: Some(0x11223344),
        fill_color: Some(0xffff_ffff),
        thumb_color: Some(0),
        ..Appearance::default()
    }
}
fn message(a: Appearance) -> Message {
    let n = NodeId::from_parts(0, 1).unwrap();
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetSliderAppearance(n, Some(a)),
            Op::SetSliderAppearance(n, None),
        ],
    })
}
fn encode(m: &Message) -> Vec<u8> {
    let mut b = vec![];
    m.binprot_write(&mut b).unwrap();
    b
}
#[test]
fn independent_fixture_and_strict_bounds() {
    let m = message(config());
    let bytes = encode(&m);
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/slider-presentation.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(m));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(decode(&extra).is_err());
    let mut bad_tag = bytes;
    bad_tag[10] = 2;
    assert!(decode(&bad_tag).is_err());
    for n in [f64::NAN, f64::INFINITY, -1., 257.] {
        for a in [
            Appearance {
                track_thickness: n,
                ..config()
            },
            Appearance {
                track_radius: n,
                ..config()
            },
            Appearance {
                thumb_size: n,
                ..config()
            },
            Appearance {
                target_size: n,
                ..config()
            },
            Appearance {
                ring_width: n,
                ..config()
            },
        ] {
            assert!(decode(&encode(&message(a))).is_err());
        }
    }
    for a in [
        Appearance {
            thumb_size: 29.,
            ..config()
        },
        Appearance {
            track_thickness: 29.,
            ..config()
        },
        Appearance {
            ring_width: 15.,
            ..config()
        },
        Appearance {
            track_color: Some(-1),
            ..config()
        },
        Appearance {
            ring_color: Some(0x1_0000_0000),
            ..config()
        },
    ] {
        assert!(decode(&encode(&message(a))).is_err());
    }
}
