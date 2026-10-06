use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, chart_options::*, decode_chart_options};
fn bytes(value: &Options) -> Vec<u8> {
    let mut out = vec![];
    value.binprot_write(&mut out).unwrap();
    out
}
#[test]
fn default_options_match_independent_ocaml_fixture_and_reject_truncation() {
    let hex = "010101010500000000009a9999999999e93f0000000000000000000000000000000001040101666666666666e63f0000000000003040000000000000284003000601";
    let expected: Vec<u8> = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect();
    let value = Options::default();
    assert_eq!(bytes(&value), expected);
    assert_eq!(decode_chart_options(&expected), Ok(value));
    for end in 0..expected.len() {
        assert!(decode_chart_options(&expected[..end]).is_err());
    }
    let mut trailing = expected.clone();
    trailing.push(0);
    assert_eq!(decode_chart_options(&trailing), Err(DecodeError::Malformed));
    // Independently specified enum/boolean field offsets in the default frame.
    for offset in [1, 2, 3, 5, 6, 7, 8, 9, 34, 36, 37, 62, 63, 65] {
        let mut bad = expected.clone();
        bad[offset] = 255;
        assert!(decode_chart_options(&bad).is_err(), "offset {offset}");
    }
    assert_eq!(
        decode_chart_options(&[0; 257]),
        Err(DecodeError::LimitExceeded)
    );
}
#[test]
fn all_option_variants_roundtrip_and_invalid_records_fail_validation() {
    for curve in [Curve::Linear, Curve::Natural, Curve::StepAfter] {
        for orientation in [
            Orientation::Vertical,
            Orientation::Horizontal,
            Orientation::VerticalReversed,
            Orientation::HorizontalReversed,
        ] {
            for alignment in [
                Alignment::Left,
                Alignment::Right,
                Alignment::Center,
                Alignment::Justify,
            ] {
                for scale in [FlowScale::Linear, FlowScale::Sqrt] {
                    for format in [
                        NumberFormat::Compact,
                        NumberFormat::Fixed(6),
                        NumberFormat::Scientific(0),
                        NumberFormat::Percent(2),
                    ] {
                        let mut o = Options::default();
                        o.cartesian.curve = curve;
                        o.cartesian.orientation = orientation;
                        o.sankey.alignment = alignment;
                        o.sankey.scale = scale;
                        o.axes.x_format = format;
                        o.axes.y_format = format;
                        assert_eq!(decode_chart_options(&bytes(&o)), Ok(o));
                    }
                }
            }
        }
    }
    let reject = |o: Options| {
        assert!(!o.is_valid());
        assert_eq!(
            decode_chart_options(&bytes(&o)),
            Err(DecodeError::Malformed)
        );
    };
    for n in [f64::NAN, f64::INFINITY, -1., 1.01] {
        let mut o = Options::default();
        o.cartesian.bar_width = n;
        reject(o);
        let mut o = Options::default();
        o.pie.inner_radius = n;
        reject(o);
        let mut o = Options::default();
        o.pie.pad_angle = n;
        reject(o);
        let mut o = Options::default();
        o.candlestick.body_width = n;
        reject(o);
    }
    let mut o = Options::default();
    o.axes.x_format = NumberFormat::Fixed(7);
    reject(o);
    let mut o = Options::default();
    o.axes.ticks = 1;
    reject(o);
    let mut o = Options::default();
    o.radar.levels = 0;
    reject(o);
    let mut o = Options::default();
    o.sankey.node_width = 0.;
    reject(o);
    let mut o = Options::default();
    o.sankey.node_padding = f64::INFINITY;
    reject(o);
    let mut o = Options::default();
    o.sankey.iterations = 33;
    reject(o);
    reject(Options {
        version: 2,
        ..Options::default()
    });
}

#[test]
fn reversed_value_directions_append_tags_without_changing_existing_frames() {
    let original = bytes(&Options::default());
    for (tag, orientation) in [
        (2, Orientation::VerticalReversed),
        (3, Orientation::HorizontalReversed),
    ] {
        let mut expected = original.clone();
        expected[9] = tag;
        let mut options = Options::default();
        options.cartesian.orientation = orientation;
        assert_eq!(bytes(&options), expected);
        assert_eq!(decode_chart_options(&expected), Ok(options));
    }
    let mut unknown = original;
    unknown[9] = 4;
    assert_eq!(decode_chart_options(&unknown), Err(DecodeError::Malformed));
}
