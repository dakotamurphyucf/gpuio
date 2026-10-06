use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, chart_style::Style, decode_chart_style};
fn bytes(style: &Style) -> Vec<u8> {
    let mut out = vec![];
    style.binprot_write(&mut out).unwrap();
    out
}
#[test]
fn paired_style_fixture_and_decoder_bounds() {
    let value = Style {
        version: -2,
        palette: vec![1, 2],
        axis_color: 3,
        grid_color: 4,
        label_color: 5,
        selection_color: 6,
        gradient_end: Some(7),
        stroke_width: 2.,
        point_radius: 3.,
        bar_radius: 4.,
        area_opacity: 0.5,
        ordinal: None,
        inspection: Default::default(),
        node_labels: vec![],
    };
    let hex = "fffe020102030405060107000000000000004000000000000008400000000000001040000000000000e03f000101010000000000008071400000000000002040000000000000204000000000000018400000000000002840000000000000314000000000000000000000000000000000000000f03f00010100000000000030400000000000000000000000";
    let expected: Vec<u8> = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(bytes(&value), expected);
    assert_eq!(decode_chart_style(&expected), Ok(value));
    for end in 0..expected.len() {
        assert!(decode_chart_style(&expected[..end]).is_err());
    }
    let mut trailing = expected;
    trailing.push(0);
    assert_eq!(decode_chart_style(&trailing), Err(DecodeError::Malformed));
    assert_eq!(
        decode_chart_style(&[0; 64 * 1024 + 1]),
        Err(DecodeError::LimitExceeded)
    );
    assert!(Style::default().is_valid());
    assert_eq!(
        decode_chart_style(&bytes(&Style::default())),
        Ok(Style::default())
    );
}
#[test]
fn invalid_raw_styles_cannot_reach_paint() {
    for value in [
        Style {
            palette: vec![],
            ..Style::default()
        },
        Style {
            palette: vec![0; 33],
            ..Style::default()
        },
        Style {
            palette: vec![-1],
            ..Style::default()
        },
        Style {
            axis_color: 0x100000000,
            ..Style::default()
        },
        Style {
            gradient_end: Some(-1),
            ..Style::default()
        },
        Style {
            stroke_width: f64::NAN,
            ..Style::default()
        },
        Style {
            point_radius: 0.,
            ..Style::default()
        },
        Style {
            bar_radius: 33.,
            ..Style::default()
        },
        Style {
            area_opacity: 1.01,
            ..Style::default()
        },
    ] {
        assert!(!value.is_valid());
        assert!(decode_chart_style(&bytes(&value)).is_err());
    }
}

#[test]
fn explicit_ordinal_namespaces_match_independent_bytes_and_reject_legacy_style() {
    use gpuio_protocol::chart_style::{Key, Ordinal};
    let mut style = Style {
        version: -2,
        palette: vec![1, 2],
        axis_color: 3,
        grid_color: 4,
        label_color: 5,
        selection_color: 6,
        gradient_end: Some(7),
        stroke_width: 2.,
        point_radius: 3.,
        bar_radius: 4.,
        area_opacity: 0.5,
        ordinal: Some(Ordinal {
            domain: vec![
                Key::Series(9),
                Key::Slice(9),
                Key::Node(9),
                Key::Rising,
                Key::Falling,
            ],
            range: vec![10, 20],
            unknown: Some(30),
        }),
        inspection: Default::default(),
        node_labels: vec![],
    };
    let hex = "fffe020102030405060107000000000000004000000000000008400000000000001040000000000000e03f01050009010902090304020a14011e0101010000000000008071400000000000002040000000000000204000000000000018400000000000002840000000000000314000000000000000000000000000000000000000f03f00010100000000000030400000000000000000000000";
    let expected = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(bytes(&style), expected);
    assert_eq!(decode_chart_style(&expected), Ok(style.clone()));
    for end in 0..expected.len() {
        assert!(decode_chart_style(&expected[..end]).is_err());
    }
    let mut unknown_tag = expected.clone();
    // -2 schema (2), palette (3), four colors (4), gradient (2), floats (32).
    let ordinal_start = 43;
    unknown_tag[ordinal_start + 2] = 5;
    assert!(decode_chart_style(&unknown_tag).is_err());
    style.ordinal = None;
    let mut previous = bytes(&style);
    previous.drain(..2);
    previous.insert(0, 0);
    previous.truncate(previous.len() - 94);
    assert_eq!(decode_chart_style(&previous), Err(DecodeError::Malformed));
    let mut legacy = previous;
    legacy.remove(0);
    legacy.pop();
    assert_eq!(decode_chart_style(&legacy), Err(DecodeError::Malformed));
    let legacy_view = include_str!("../../../test/fixtures/chart-v3-view.hex").trim();
    let legacy_view = legacy_view
        .as_bytes()
        .chunks_exact(2)
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        gpuio_protocol::decode_chart_view_config(&legacy_view),
        Err(DecodeError::Malformed)
    );
}

#[test]
fn ordinal_validation_and_capacity_accounting_cover_large_domains() {
    use gpuio_protocol::chart_style::{Key, MAX_COLOR_DOMAIN, Ordinal};
    let ordinal = Ordinal {
        domain: (1..=MAX_COLOR_DOMAIN as i64).map(Key::Series).collect(),
        range: vec![0x112233ff; 32],
        unknown: Some(0x445566ff),
    };
    let base = Style::default();
    let style = Style {
        ordinal: Some(ordinal.clone()),
        ..base.clone()
    };
    assert!(style.is_valid());
    assert_eq!(decode_chart_style(&bytes(&style)), Ok(style.clone()));
    assert_eq!(style.heap_bytes() - base.heap_bytes(), ordinal.heap_bytes());
    assert!(style.heap_bytes() >= MAX_COLOR_DOMAIN * std::mem::size_of::<Key>());
    let mut too_many = ordinal.clone();
    too_many.domain.push(Key::Slice(1));
    let mut duplicate = ordinal.clone();
    duplicate.domain[1] = duplicate.domain[0];
    for invalid in [
        too_many,
        duplicate,
        Ordinal {
            domain: vec![Key::Node(0)],
            ..ordinal.clone()
        },
        Ordinal {
            range: vec![],
            ..ordinal.clone()
        },
        Ordinal {
            range: vec![0; 33],
            ..ordinal.clone()
        },
        Ordinal {
            range: vec![-1],
            ..ordinal.clone()
        },
        Ordinal {
            unknown: Some(0x1_0000_0000),
            ..ordinal.clone()
        },
    ] {
        assert!(!invalid.is_valid());
        assert!(
            decode_chart_style(&bytes(&Style {
                ordinal: Some(invalid),
                ..base.clone()
            }))
            .is_err()
        );
    }
    let empty_domain = Style {
        ordinal: Some(Ordinal {
            domain: vec![],
            range: vec![1],
            unknown: None,
        }),
        ..base.clone()
    };
    assert_eq!(decode_chart_style(&bytes(&empty_domain)), Ok(empty_domain));
    assert!(!Style { version: 1, ..base }.is_valid());
}
