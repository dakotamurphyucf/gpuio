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
    };
    let hex = "020102030405060107000000000000004000000000000008400000000000001040000000000000e03f";
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
        decode_chart_style(&[0; 513]),
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
