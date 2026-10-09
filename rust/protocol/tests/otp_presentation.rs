use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, decode, otp_presentation::Appearance, v1::*};
fn config() -> Appearance {
    Appearance {
        groups: 2,
        cell_width: Some(40.),
        background: Some(0x11223344),
        focus_border: Some(0xffff_ffff),
        caret: Some(0),
        ..Appearance::default()
    }
}
fn message(appearance: Appearance) -> Message {
    let node = NodeId::from_parts(0, 1).unwrap();
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::SetOtpAppearance(node, Some(appearance)),
            Op::SetOtpAppearance(node, None),
        ],
    })
}
fn encode(value: &Message) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
#[test]
fn independent_appearance_fixture_and_strict_decoding() {
    let value = message(config());
    let bytes = encode(&value);
    assert_eq!(
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        include_str!("../../../test/fixtures/otp-presentation.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(value));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut extra = bytes;
    extra.push(0);
    assert!(decode(&extra).is_err());
    for bad in [f64::NAN, f64::INFINITY, -1., 4097.] {
        for appearance in [
            Appearance {
                cell_width: Some(bad),
                ..config()
            },
            Appearance {
                cell_gap: bad,
                ..config()
            },
            Appearance {
                group_gap: bad,
                ..config()
            },
            Appearance {
                radius: bad,
                ..config()
            },
        ] {
            assert!(decode(&encode(&message(appearance))).is_err());
        }
    }
    for appearance in [
        Appearance {
            groups: 0,
            ..config()
        },
        Appearance {
            groups: 33,
            ..config()
        },
        Appearance {
            cell_width: Some(0.5),
            ..config()
        },
        Appearance {
            border_width: 65.,
            ..config()
        },
        Appearance {
            caret: Some(-1),
            ..config()
        },
        Appearance {
            background: Some(0x1_0000_0000),
            ..config()
        },
    ] {
        assert!(decode(&encode(&message(appearance))).is_err());
    }
}
#[test]
fn groups_cover_all_cells_without_trailing_empty_space() {
    for length in 1usize..=32 {
        for groups in 1..=32 {
            let appearance = Appearance {
                groups,
                cell_gap: 3.,
                group_gap: 19.,
                ..Appearance::default()
            };
            let chunk = length.div_ceil((groups as usize).min(length));
            let mut expected = 0.;
            for index in 0..length {
                assert_eq!(appearance.cell_left(index, length, 40.), expected);
                expected += 40.;
                if index + 1 < length {
                    expected += if (index + 1) % chunk == 0 { 19. } else { 3. };
                }
            }
            assert_eq!(appearance.width(length, 40.), expected);
        }
    }
}
