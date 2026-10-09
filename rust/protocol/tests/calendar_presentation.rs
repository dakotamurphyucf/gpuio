use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, calendar_presentation::Appearance, decode, v1::*};
fn config() -> Appearance {
    Appearance {
        months: 2,
        cell_height: 40.,
        cell_gap: 6.,
        month_gap: 20.,
        padding: 10.,
        cell_radius: 8.,
        outline_width: 2.,
        selected_background: Some(0x11223344),
        selected_foreground: Some(0xffff_ffff),
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
            Op::SetCalendarAppearance(n, Some(a)),
            Op::SetCalendarAppearance(n, None),
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
        include_str!("../../../test/fixtures/calendar-presentation.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(m));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(decode(&extra).is_err());
    for count in [i64::MIN, 0, 13, i64::MAX] {
        assert!(
            decode(&encode(&message(Appearance {
                months: count,
                ..config()
            })))
            .is_err()
        );
    }
    for n in [f64::NAN, f64::INFINITY, -1., 129.] {
        for a in [
            Appearance {
                cell_height: n,
                ..config()
            },
            Appearance {
                cell_gap: n,
                ..config()
            },
            Appearance {
                month_gap: n,
                ..config()
            },
            Appearance {
                padding: n,
                ..config()
            },
            Appearance {
                cell_radius: n,
                ..config()
            },
            Appearance {
                outline_width: n,
                ..config()
            },
        ] {
            assert!(decode(&encode(&message(a))).is_err());
        }
    }
    for a in [
        Appearance {
            cell_height: 15.,
            ..config()
        },
        Appearance {
            outline_width: 21.,
            ..config()
        },
        Appearance {
            focus_border: Some(-1),
            ..config()
        },
        Appearance {
            selected_background: Some(0x1_0000_0000),
            ..config()
        },
    ] {
        assert!(decode(&encode(&message(a))).is_err());
    }
}
