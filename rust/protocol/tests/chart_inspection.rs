use binprot::BinProtWrite;
use gpuio_protocol::{chart_inspection::*, chart_style::Style, decode_chart_style};
fn encode(style: &Style) -> Vec<u8> {
    let mut out = vec![];
    style.binprot_write(&mut out).unwrap();
    out
}
#[test]
fn inspector_controls_roundtrip_all_axes_patterns_and_placements() {
    for axis in [Axis::Off, Axis::Vertical, Axis::Horizontal, Axis::Both] {
        for pattern in [Pattern::Dashed, Pattern::Solid] {
            for placement in [Placement::Corner, Placement::Anchor, Placement::Cursor] {
                let inspection = Inspection {
                    card: Card {
                        placement,
                        visible: false,
                        title: false,
                        values: false,
                        width: 96.,
                        gap: 64.,
                        padding: 24.,
                        radius: 24.,
                        font_size: 32.,
                        line_height: 48.,
                        border_width: 8.,
                        text_color: Some(0xffff_ffff),
                        background: Some(0),
                        border_color: Some(1),
                    },
                    crosshair: Crosshair {
                        axis,
                        pattern,
                        thickness: 64.,
                        color: Some(2),
                        vertical_span: Span::Pixels(-12., 30.),
                        horizontal_span: Span::Fraction(0.25, 0.5),
                    },
                    marker: Marker {
                        visible: false,
                        status: false,
                        size: 48.,
                        stroke_width: 8.,
                        fill: Some(3),
                        stroke: Some(4),
                    },
                };
                let style = Style {
                    inspection: Box::new(inspection),
                    ..Default::default()
                };
                assert!(style.is_valid());
                assert_eq!(decode_chart_style(&encode(&style)), Ok(style));
            }
        }
    }
}
#[test]
fn invalid_inspector_values_are_rejected_at_admission() {
    let base = Inspection::default();
    for n in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1., 65537.] {
        for invalid in [
            Inspection {
                card: Card {
                    width: n,
                    ..base.card
                },
                ..base
            },
            Inspection {
                card: Card {
                    gap: n,
                    ..base.card
                },
                ..base
            },
            Inspection {
                card: Card {
                    padding: n,
                    ..base.card
                },
                ..base
            },
            Inspection {
                card: Card {
                    radius: n,
                    ..base.card
                },
                ..base
            },
            Inspection {
                card: Card {
                    font_size: n,
                    ..base.card
                },
                ..base
            },
            Inspection {
                card: Card {
                    line_height: n,
                    ..base.card
                },
                ..base
            },
            Inspection {
                card: Card {
                    border_width: n,
                    ..base.card
                },
                ..base
            },
            Inspection {
                crosshair: Crosshair {
                    thickness: n,
                    ..base.crosshair
                },
                ..base
            },
            Inspection {
                marker: Marker {
                    size: n,
                    ..base.marker
                },
                ..base
            },
            Inspection {
                marker: Marker {
                    stroke_width: n,
                    ..base.marker
                },
                ..base
            },
        ] {
            assert!(!invalid.is_valid());
            let style = Style {
                inspection: Box::new(invalid),
                ..Default::default()
            };
            assert!(decode_chart_style(&encode(&style)).is_err());
        }
    }
    for invalid in [
        Inspection {
            card: Card {
                font_size: 32.,
                line_height: 31.,
                ..base.card
            },
            ..base
        },
        Inspection {
            marker: Marker {
                size: 2.,
                stroke_width: 2.,
                ..base.marker
            },
            ..base
        },
        Inspection {
            card: Card {
                background: Some(-1),
                ..base.card
            },
            ..base
        },
        Inspection {
            card: Card {
                text_color: Some(0x1_0000_0000),
                ..base.card
            },
            ..base
        },
        Inspection {
            card: Card {
                border_color: Some(-1),
                ..base.card
            },
            ..base
        },
        Inspection {
            crosshair: Crosshair {
                color: Some(-1),
                ..base.crosshair
            },
            ..base
        },
        Inspection {
            marker: Marker {
                fill: Some(-1),
                ..base.marker
            },
            ..base
        },
        Inspection {
            marker: Marker {
                stroke: Some(-1),
                ..base.marker
            },
            ..base
        },
    ] {
        assert!(!invalid.is_valid());
        assert!(
            decode_chart_style(&encode(&Style {
                inspection: Box::new(invalid),
                ..Default::default()
            }))
            .is_err()
        );
    }
    let previous = include_str!("../../../test/fixtures/chart-v3-style-v0-view.hex").trim();
    let bytes = previous
        .as_bytes()
        .chunks_exact(2)
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    assert!(gpuio_protocol::decode_chart_view_config(&bytes).is_err());
}

#[test]
fn cursor_placement_appends_tag_two() {
    for (tag, placement) in [Placement::Corner, Placement::Anchor, Placement::Cursor]
        .into_iter()
        .enumerate()
    {
        let mut bytes = Vec::new();
        placement.binprot_write(&mut bytes).unwrap();
        assert_eq!(bytes, vec![tag as u8]);
    }
}

#[test]
fn guide_span_paired_bytes_and_independent_fields() {
    for (span, hex) in [
        (Span::Full, "00"),
        (
            Span::Pixels(-12., 30.),
            "0100000000000028c00000000000003e40",
        ),
        (
            Span::Fraction(0.25, 0.5),
            "02000000000000d03f000000000000e03f",
        ),
    ] {
        let mut bytes = Vec::new();
        span.binprot_write(&mut bytes).unwrap();
        assert_eq!(
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>(),
            hex
        );
        for horizontal in [
            Span::Full,
            Span::Pixels(32768., 65536.),
            Span::Fraction(-1., 2.),
        ] {
            let mut style = Style::default();
            style.inspection.crosshair.vertical_span = span;
            style.inspection.crosshair.horizontal_span = horizontal;
            assert_eq!(decode_chart_style(&encode(&style)), Ok(style));
        }
    }
}

#[test]
fn invalid_guide_spans_reject_either_axis_and_unknown_tags() {
    for span in [
        Span::Pixels(f64::NAN, 0.),
        Span::Pixels(f64::INFINITY, 0.),
        Span::Pixels(-32769., 1.),
        Span::Pixels(32769., 1.),
        Span::Pixels(0., -0.01),
        Span::Pixels(0., 65537.),
        Span::Pixels(0., f64::NEG_INFINITY),
        Span::Fraction(-1.01, 1.),
        Span::Fraction(1.01, 1.),
        Span::Fraction(0., -0.01),
        Span::Fraction(0., 2.01),
        Span::Fraction(f64::NAN, 0.),
        Span::Fraction(0., f64::INFINITY),
    ] {
        for vertical in [true, false] {
            let mut style = Style::default();
            if vertical {
                style.inspection.crosshair.vertical_span = span;
            } else {
                style.inspection.crosshair.horizontal_span = span;
            }
            assert!(!style.is_valid());
            assert!(decode_chart_style(&encode(&style)).is_err());
        }
    }
    let mut style = Style::default();
    style.inspection.crosshair.vertical_span = Span::Pixels(-12., 30.);
    let bytes = encode(&style);
    let mut needle = vec![];
    style
        .inspection
        .crosshair
        .vertical_span
        .binprot_write(&mut needle)
        .unwrap();
    let offsets = bytes
        .windows(needle.len())
        .enumerate()
        .filter_map(|(i, v)| (v == needle).then_some(i))
        .collect::<Vec<_>>();
    assert_eq!(offsets.len(), 1);
    let offset = offsets[0];
    for tag in [3, 255] {
        let mut bad = bytes.clone();
        bad[offset] = tag;
        assert!(decode_chart_style(&bad).is_err());
    }
    for end in offset + 1..offset + needle.len() {
        assert!(decode_chart_style(&bytes[..end]).is_err());
    }
}
