use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, chart_options::*, decode_chart_options};
fn bytes(value: &Options) -> Vec<u8> {
    let mut out = vec![];
    value.binprot_write(&mut out).unwrap();
    out
}
#[test]
fn default_options_match_independent_ocaml_fixture_and_reject_truncation() {
    let hex = "090101010500000000009a9999999999e93f000000000000000000000000000000000000010000000000000000002e4004010100000000000000000000666666666666e63f0000000000003040000000000000284003000601000000000000f03f000000000000e03f000000000000000000000000000018400000";
    let expected: Vec<u8> = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect();
    let value = Options::default();
    assert_eq!(bytes(&value), expected);
    assert_eq!(decode_chart_options(&expected), Ok(value.clone()));
    for end in 0..expected.len() {
        assert!(decode_chart_options(&expected[..end]).is_err());
    }
    let mut trailing = expected.clone();
    trailing.push(0);
    assert_eq!(decode_chart_options(&trailing), Err(DecodeError::Malformed));
    // Independently specified enum/boolean field offsets in the default frame.
    for offset in [
        1, 2, 3, 5, 6, 7, 8, 9, 18, 19, 36, 37, 39, 49, 50, 51, 52, 85, 86, 88,
    ] {
        let mut bad = expected.clone();
        bad[offset] = 255;
        assert!(decode_chart_options(&bad).is_err(), "offset {offset}");
    }
    assert_eq!(
        decode_chart_options(&[0; 16 * 1024 + 1]),
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
        version: 1,
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

#[test]
fn categorical_layout_tags_and_padding_bounds_are_paired_and_versioned() {
    for layout in [
        CategoryLayout::Auto,
        CategoryLayout::Point(0.),
        CategoryLayout::Point(1.),
        CategoryLayout::Band {
            inner: 0.,
            outer: 1.,
        },
        CategoryLayout::Band {
            inner: 0.2,
            outer: 0.1,
        },
    ] {
        let mut o = Options::default();
        o.cartesian.category_layout = layout;
        assert_eq!(decode_chart_options(&bytes(&o)), Ok(o));
    }
    for layout in [
        CategoryLayout::Point(f64::NAN),
        CategoryLayout::Point(-0.1),
        CategoryLayout::Point(1.1),
        CategoryLayout::Band {
            inner: 1.,
            outer: 0.,
        },
        CategoryLayout::Band {
            inner: 0.,
            outer: f64::INFINITY,
        },
    ] {
        let mut o = Options::default();
        o.cartesian.category_layout = layout;
        assert!(!o.is_valid());
        assert!(decode_chart_options(&bytes(&o)).is_err());
    }
    let mut o = Options::default();
    o.cartesian.category_layout = CategoryLayout::Point(0.5);
    let mut expected = bytes(&Options::default());
    expected.splice(18..19, [1, 0, 0, 0, 0, 0, 0, 224, 63]);
    assert_eq!(bytes(&o), expected);
    let old = include_str!("../../../test/fixtures/chart-v1-view.hex").trim();
    let old = (0..old.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&old[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    assert!(gpuio_protocol::decode_chart_view_config(&old).is_err());
}

#[test]
fn stacking_is_explicit_paired_and_old_layouts_are_rejected() {
    let mut options = Options::default();
    let mut expected = bytes(&options);
    assert_eq!(expected[0], 9);
    assert_eq!(expected[19], 0);
    options.cartesian.stacking = Stacking::Stacked;
    expected[19] = 1;
    assert_eq!(bytes(&options), expected);
    assert_eq!(decode_chart_options(&expected), Ok(options));
    expected[19] = 2;
    assert_eq!(decode_chart_options(&expected), Err(DecodeError::Malformed));
    for fixture in [
        include_str!("../../../test/fixtures/chart-v1-view.hex"),
        include_str!("../../../test/fixtures/chart-v2-view.hex"),
        include_str!("../../../test/fixtures/chart-v3-style-inspection-view.hex"),
    ] {
        let frame = fixture
            .trim()
            .as_bytes()
            .chunks_exact(2)
            .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            gpuio_protocol::decode_chart_view_config(&frame),
            Err(DecodeError::Malformed)
        );
    }
}

#[test]
fn sankey_presentation_bounds_apply_to_native_records_and_wire() {
    for n in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1., 65.] {
        for field in 0..4 {
            let mut options = Options::default();
            match field {
                0 => options.sankey.node_corner_radius = n,
                1 => options.sankey.link_opacity = n,
                2 => options.sankey.min_link_width = n,
                _ => options.sankey.label_gap = n,
            }
            assert!(!options.is_valid());
            assert_eq!(
                decode_chart_options(&bytes(&options)),
                Err(DecodeError::Malformed)
            );
        }
    }
    for (radius, opacity, minimum, gap) in [(0., 0., 0., 0.), (32., 1., 64., 64.)] {
        let mut options = Options::default();
        options.sankey.node_corner_radius = radius;
        options.sankey.link_opacity = opacity;
        options.sankey.min_link_width = minimum;
        options.sankey.label_gap = gap;
        assert_eq!(decode_chart_options(&bytes(&options)), Ok(options));
    }
}

#[test]
fn sankey_link_color_tags_preserve_default_and_reject_unknown_variants() {
    for (mode, tag) in [
        (LinkColor::Source, 0),
        (LinkColor::Target, 1),
        (LinkColor::Gradient, 2),
    ] {
        let mut value = Options::default();
        value.sankey.link_color = mode;
        let encoded = bytes(&value);
        assert_eq!(encoded.len(), 123);
        assert_eq!(encoded[121], tag);
        assert_eq!(decode_chart_options(&encoded), Ok(value.clone()));
    }
    let mut encoded = bytes(&Options::default());
    assert_eq!(encoded[121], 0);
    encoded[121] = 3;
    assert_eq!(decode_chart_options(&encoded), Err(DecodeError::Malformed));
    encoded[121] = 0;
    encoded[0] = 4;
    assert_eq!(decode_chart_options(&encoded), Err(DecodeError::Malformed));
}

#[test]
fn measured_label_placement_is_explicit_and_version_nine_rejects_old_options() {
    let mut value = Options::default();
    let original = bytes(&value);
    assert_eq!(original.len(), 123);
    assert_eq!(original[0], 9);
    assert_eq!(original[122], 0);
    value.sankey.label_placement = LabelPlacement::Outside;
    let mut expected = original.clone();
    expected[122] = 1;
    assert_eq!(bytes(&value), expected);
    assert_eq!(decode_chart_options(&expected), Ok(value.clone()));
    expected[122] = 2;
    assert_eq!(decode_chart_options(&expected), Err(DecodeError::Malformed));
    for version in 0..9 {
        let mut old = original.clone();
        old[0] = version;
        assert_eq!(decode_chart_options(&old), Err(DecodeError::Malformed));
    }
}

#[test]
fn radar_scale_radius_gap_tags_and_invalid_wire_are_paired() {
    let mut value = Options::default();
    value.radar.scale = RadarScale::DataMax;
    value.radar.radius = RadarRadius::Pixels(80.);
    value.radar.label_gap = 10.;
    let mut expected = bytes(&Options::default());
    expected.splice(
        51..61,
        [1, 1, 0, 0, 0, 0, 0, 0, 84, 64, 0, 0, 0, 0, 0, 0, 36, 64],
    );
    assert_eq!(bytes(&value), expected);
    assert_eq!(decode_chart_options(&expected), Ok(value.clone()));
    for maximum in [f64::from_bits(1), 1., 1e100] {
        value.radar.scale = RadarScale::Maximum(maximum);
        assert_eq!(decode_chart_options(&bytes(&value)), Ok(value.clone()));
    }
    for n in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1., 0., 1.1e100] {
        value.radar.scale = RadarScale::Maximum(n);
        assert_eq!(
            decode_chart_options(&bytes(&value)),
            Err(DecodeError::Malformed)
        );
    }
    value = Options::default();
    for n in [f64::NAN, f64::INFINITY, -1., 0., 32768.01] {
        value.radar.radius = RadarRadius::Pixels(n);
        assert_eq!(
            decode_chart_options(&bytes(&value)),
            Err(DecodeError::Malformed)
        );
    }
    value.radar.radius = RadarRadius::Pixels(32768.);
    value.radar.label_gap = 64.;
    assert_eq!(decode_chart_options(&bytes(&value)), Ok(value.clone()));
    for n in [f64::NAN, f64::INFINITY, -1., 64.01] {
        value.radar.label_gap = n;
        assert_eq!(
            decode_chart_options(&bytes(&value)),
            Err(DecodeError::Malformed)
        );
    }
    for (offset, tag) in [(51, 3), (52, 2)] {
        let mut invalid = bytes(&Options::default());
        invalid[offset] = tag;
        assert_eq!(decode_chart_options(&invalid), Err(DecodeError::Malformed));
    }
    let old = include_str!("../../../test/fixtures/chart-v6-label-placement-view.hex").trim();
    let old: Vec<_> = old
        .as_bytes()
        .chunks_exact(2)
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(
        gpuio_protocol::decode_chart_view_config(&old),
        Err(DecodeError::Malformed)
    );
}

#[test]
fn pie_radii_have_independent_tags_and_id_keyed_pairs() {
    let mut options = Options::default();
    options.pie.radius = PieRadius::Pixels(80.);
    options.pie.slice_radii = vec![
        SliceRadii {
            slice: 7,
            inner: 20.,
            outer: 60.,
        },
        SliceRadii {
            slice: 9,
            inner: 0.,
            outer: 0.,
        },
    ];
    // Radius tag+80px, two overrides: ID 7 at [20,60], ID 9 at [0,0].
    // Specified independently of BinProtWrite; OCaml asserts the same hex.
    let hex =
        "010000000000005440020700000000000034400000000000004e400900000000000000000000000000000000";
    let fields = hex
        .as_bytes()
        .chunks_exact(2)
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    let mut expected = bytes(&Options::default());
    expected.splice(37..39, fields);
    assert_eq!(bytes(&options), expected);
    assert_eq!(decode_chart_options(&expected), Ok(options));
    for end in 0..expected.len() {
        assert!(decode_chart_options(&expected[..end]).is_err());
    }
    let mut trailing = expected;
    trailing.push(0);
    assert_eq!(decode_chart_options(&trailing), Err(DecodeError::Malformed));
}

#[test]
fn pie_radii_bound_untrusted_frames_and_native_records() {
    let mut options = Options::default();
    for radius in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.,
        0.,
        32768.01,
    ] {
        options.pie.radius = PieRadius::Pixels(radius);
        assert!(!options.is_valid());
        assert_eq!(
            decode_chart_options(&bytes(&options)),
            Err(DecodeError::Malformed)
        );
    }
    for radius in [f64::from_bits(1), 1., 32768.] {
        options.pie.radius = PieRadius::Pixels(radius);
        assert_eq!(decode_chart_options(&bytes(&options)), Ok(options.clone()));
    }
    options.pie.radius = PieRadius::Fit;
    for (inner, outer) in [(0., 0.), (10., 10.), (0., 32768.), (f64::from_bits(1), 1.)] {
        options.pie.slice_radii = vec![SliceRadii {
            slice: i64::MAX,
            inner,
            outer,
        }];
        assert_eq!(decode_chart_options(&bytes(&options)), Ok(options.clone()));
    }
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1., 32768.01] {
        for (inner, outer) in [(invalid, 10.), (0., invalid), (20., 10.)] {
            options.pie.slice_radii = vec![SliceRadii {
                slice: 1,
                inner,
                outer,
            }];
            assert!(!options.is_valid());
            assert_eq!(
                decode_chart_options(&bytes(&options)),
                Err(DecodeError::Malformed)
            );
        }
    }
    for slice in [0, -1] {
        options.pie.slice_radii = vec![SliceRadii {
            slice,
            inner: 0.,
            outer: 1.,
        }];
        assert!(!options.is_valid());
        assert_eq!(
            decode_chart_options(&bytes(&options)),
            Err(DecodeError::Malformed)
        );
    }
    options.pie.slice_radii = vec![
        SliceRadii {
            slice: 1,
            inner: 0.,
            outer: 1.
        };
        2
    ];
    assert!(!options.is_valid());
    assert_eq!(
        decode_chart_options(&bytes(&options)),
        Err(DecodeError::Malformed)
    );
    options.pie.slice_radii = (1..=256)
        .map(|slice| SliceRadii {
            slice,
            inner: 0.,
            outer: 1.,
        })
        .collect();
    assert!(options.is_valid());
    assert_eq!(decode_chart_options(&bytes(&options)), Ok(options.clone()));
    assert_eq!(
        options.heap_bytes(),
        options.pie.slice_radii.capacity() * std::mem::size_of::<SliceRadii>()
    );
    options.pie.slice_radii.push(SliceRadii {
        slice: 257,
        inner: 0.,
        outer: 1.,
    });
    assert!(!options.is_valid());
    assert_eq!(
        decode_chart_options(&bytes(&options)),
        Err(DecodeError::LimitExceeded)
    );
    // Reject the advertised count before allocating/reading nonexistent entries.
    for count in [
        vec![254, 1, 1],
        vec![252, 255, 255, 255, 255, 255, 255, 255, 127],
    ] {
        let mut frame = bytes(&Options::default());
        frame.truncate(38);
        frame.extend(count);
        assert_eq!(
            decode_chart_options(&frame),
            Err(DecodeError::LimitExceeded)
        );
    }
    let mut frame = bytes(&Options::default());
    frame[37] = 2;
    assert_eq!(decode_chart_options(&frame), Err(DecodeError::Malformed));
}

#[test]
fn outside_pie_placement_and_gap_are_paired_and_bounded() {
    let mut o = Options::default();
    o.pie.label_placement = LabelPlacement::Outside;
    o.pie.label_gap = 32.;
    assert_eq!(&bytes(&o)[39..48], &[1, 0, 0, 0, 0, 0, 0, 64, 64]);
    assert_eq!(decode_chart_options(&bytes(&o)), Ok(o.clone()));
    for label_gap in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1., 64.01] {
        o.pie.label_gap = label_gap;
        assert!(!o.is_valid());
        assert_eq!(
            decode_chart_options(&bytes(&o)),
            Err(DecodeError::Malformed)
        );
    }
}
