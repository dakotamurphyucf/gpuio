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
            for placement in [Placement::Corner, Placement::Anchor] {
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
