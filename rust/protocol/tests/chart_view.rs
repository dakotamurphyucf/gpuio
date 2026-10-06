use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, ResourceId, chart_view::*, decode_chart_view_config};
fn config() -> Config {
    Config {
        version: -1,
        radar_labels: vec![],
        source: Some(ResourceId::from_parts(7, 2).unwrap()),
        label: "Chart 🦀".into(),
        legend: true,
        disabled: false,
        options: Default::default(),
        sampling: Default::default(),
        style: gpuio_protocol::chart_style::Style {
            version: -7,
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
            pie_labels: vec![],
            pie_label_line_color: None,
            x_axis: Default::default(),
            y_axis: Default::default(),
            grid: Default::default(),
            appearance: Default::default(),
        },
    }
}
fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut out = vec![];
    value.binprot_write(&mut out).unwrap();
    out
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn bounded_chart_view_and_transaction_match_independent_fixture() {
    use gpuio_protocol::{HandlerId, NodeId, WindowId, decode, v1::*};
    let config = config();
    let legacy = include_str!("../../../test/fixtures/chart-v5-link-colors-view.hex")
        .trim()
        .as_bytes()
        .chunks_exact(2)
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        decode_chart_view_config(&legacy),
        Err(DecodeError::Malformed)
    );
    let bytes = encode(&config);
    let fixture = include_str!("../../../test/fixtures/chart-view-v1-labels.hex").trim();
    assert_eq!(hex(&bytes), fixture);
    assert_eq!(decode_chart_view_config(&bytes), Ok(config.clone()));
    for end in 0..bytes.len() {
        assert!(decode_chart_view_config(&bytes[..end]).is_err());
    }
    let mut hidden_legend = config.clone();
    hidden_legend.legend = false;
    assert_eq!(
        decode_chart_view_config(&encode(&hidden_legend)),
        Ok(hidden_legend)
    );
    let mut disabled = config.clone();
    disabled.disabled = true;
    assert_eq!(decode_chart_view_config(&encode(&disabled)), Ok(disabled));
    // The final two fields are independently validated booleans.
    for offset in [1, 2] {
        let mut malformed = bytes.clone();
        let index = malformed.len() - offset;
        malformed[index] = 2;
        assert_eq!(
            decode_chart_view_config(&malformed),
            Err(DecodeError::Malformed)
        );
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert_eq!(
        decode_chart_view_config(&trailing),
        Err(DecodeError::Malformed)
    );
    assert_eq!(
        decode_chart_view_config(
            &[0; gpuio_protocol::chart_style::MAX_STYLE_BYTES
                + gpuio_protocol::chart_options::MAX_OPTIONS_BYTES
                + 2 * 1024
                + 1]
        ),
        Err(DecodeError::LimitExceeded)
    );
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let message = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(node, Kind::ChartView, "".into(), Some(handler)),
            Op::SetChart(node, Box::new(config)),
        ],
    });
    let bytes = encode(&message);
    assert_eq!(
        hex(&bytes),
        format!("0300010001020000013000010001370001{fixture}")
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let event = Event::ChartEvent(
        window,
        node,
        handler,
        1,
        Some(ResourceId::from_parts(7, 2).unwrap()),
        2,
        3,
        Observation::Ready(Metrics {
            source_values: 10,
            retained_values: 5,
            mesh_vertices: 12,
            quads: 2,
            bytes: 100,
        }),
    );
    assert_eq!(
        hex(&encode(&vec![event])),
        "013e000100010001010107020203000a050c0264"
    );
}
#[test]
fn invalid_nested_configuration_and_label_cannot_reach_native_tree() {
    for label in [
        "".into(),
        " \t\u{b}\u{c}".into(),
        "\0".into(),
        "chart\n".into(),
        "x".repeat(1025),
    ] {
        let value = Config { label, ..config() };
        assert!(!value.is_valid());
        assert!(decode_chart_view_config(&encode(&value)).is_err());
    }
    let mut value = config();
    value.options.version = 1;
    assert!(decode_chart_view_config(&encode(&value)).is_err());
    let mut value = config();
    value.sampling.line = gpuio_protocol::chart_sampling::Line::Envelope(0);
    assert!(decode_chart_view_config(&encode(&value)).is_err());
    let mut value = config();
    value.style.palette.clear();
    assert!(decode_chart_view_config(&encode(&value)).is_err());
}

#[test]
fn radar_label_targets_match_independent_bytes_and_bound_native_allocation() {
    let mut config = config();
    config.radar_labels = vec![7, 9];
    let bytes = encode(&config);
    assert_eq!(
        hex(&bytes),
        include_str!("../../../test/fixtures/chart-view-v1-rich-labels.hex").trim()
    );
    assert_eq!(decode_chart_view_config(&bytes), Ok(config.clone()));
    for end in 0..bytes.len() {
        assert!(decode_chart_view_config(&bytes[..end]).is_err());
    }
    for axes in [vec![], vec![i64::MAX], (1..=64).collect()] {
        config.radar_labels = axes;
        assert!(config.is_valid());
        assert_eq!(
            decode_chart_view_config(&encode(&config)),
            Ok(config.clone())
        );
    }
    for axes in [vec![0], vec![-1], vec![7, 7], (1..=65).collect()] {
        config.radar_labels = axes;
        assert!(!config.is_valid());
        assert!(decode_chart_view_config(&encode(&config)).is_err());
    }
    config.radar_labels.clear();
    config.version = 0;
    assert!(!config.is_valid());
    assert_eq!(
        decode_chart_view_config(&encode(&config)),
        Err(DecodeError::Malformed)
    );
    let previous = include_str!("../../../test/fixtures/chart-v7-radar-view.hex")
        .trim()
        .as_bytes()
        .chunks_exact(2)
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        decode_chart_view_config(&previous),
        Err(DecodeError::Malformed)
    );
}

#[test]
fn pie_radius_storage_is_charged_and_full_override_lists_decode() {
    use gpuio_protocol::chart_options::SliceRadii;
    let mut value = config();
    let before = value.retained_bytes();
    value.options.pie.slice_radii = (1..=256)
        .map(|slice| SliceRadii {
            slice,
            inner: 0.,
            outer: 60.,
        })
        .collect();
    assert_eq!(
        value.retained_bytes() - before,
        value.options.pie.slice_radii.capacity() * std::mem::size_of::<SliceRadii>()
    );
    assert_eq!(decode_chart_view_config(&encode(&value)), Ok(value));
}

#[test]
fn independently_full_chart_captions_fit_the_config_envelope_and_are_charged() {
    use gpuio_protocol::{chart_node_labels as node, chart_pie_labels as pie};
    let mut value = config();
    let before = value.retained_bytes();
    value.style.pie_labels = (1..=256)
        .map(|slice| pie::Entry {
            slice,
            text: Some("p".repeat(128)),
            line_color: Some(0xffff_ffff),
        })
        .collect();
    value.style.node_labels = (1..=128)
        .map(|id| node::Node {
            node: id,
            lines: (0..4)
                .map(|_| node::Line {
                    text: "s".repeat(64),
                    color: Some(0xffff_ffff),
                    font_size: Some(32.),
                })
                .collect(),
        })
        .collect();
    use gpuio_protocol::chart_axis::{LabelAlign, Tick, TickPosition};
    let tick = Tick {
        position: TickPosition::Value(1.),
        text: "t".repeat(256),
        color: Some(0xffff_ffff),
        font_size: Some(32.),
        align: LabelAlign::Right,
    };
    value.style.x_axis.ticks = Some(vec![tick.clone(); 64]);
    value.style.y_axis.ticks = Some(vec![tick; 64]);
    value.style.grid.x = Some(vec![TickPosition::Category(i64::MAX); 64]);
    value.style.grid.y = Some(vec![TickPosition::Value(1e100); 64]);
    value.style.grid.dashes = vec![128.; 16];
    use gpuio_protocol::chart_appearance as a;
    let color = 0xffff_ffff;
    let brush = a::Brush::Linear {
        oklab: true,
        angle: 360.,
        from: color,
        start: 0.,
        to: color,
        stop: 1.,
    };
    let marker = Some(a::Marker {
        visible: Some(true),
        radius: Some(24.),
        fill: Some(color),
        stroke: Some(color),
        stroke_width: Some(8.),
    });
    let bar = Some(a::Bar {
        fill: Some(a::BarFill::Background(brush)),
        corners: Some(a::Corners {
            top_left: 32.,
            top_right: 32.,
            bottom_right: 32.,
            bottom_left: 32.,
        }),
    });
    let path = Some(a::Path {
        stroke: Some(a::Stroke {
            visible: true,
            width: Some(8.),
            brush,
        }),
        fill: Some(brush),
        curve: Some(gpuio_protocol::chart_options::Curve::Natural),
    });
    value.style.appearance.series = (0..a::MAX_SERIES)
        .map(|i| a::Series {
            series: i64::MAX - i as i64,
            path,
            marker,
            bar,
            legend: Some(color),
        })
        .collect();
    value.style.appearance.data = (0..a::MAX_DATA)
        .map(|i| a::Datum {
            series: i64::MAX,
            datum: i64::MAX - i as i64,
            marker,
            bar,
        })
        .collect();
    // Exercise all other independently bounded style sections alongside the
    // maximum appearance. Use long integer encodings, not tiny fixture IDs.
    value.style.palette = vec![color; 32];
    value.style.axis_color = color;
    value.style.grid_color = color;
    value.style.label_color = color;
    value.style.selection_color = color;
    value.style.gradient_end = Some(color);
    value.style.pie_label_line_color = Some(color);
    value.style.ordinal = Some(gpuio_protocol::chart_style::Ordinal {
        domain: (0..1024)
            .map(|i| gpuio_protocol::chart_style::Key::Series(i64::MAX - i))
            .collect(),
        range: vec![color; 32],
        unknown: Some(color),
    });
    for (i, node) in value.style.node_labels.iter_mut().enumerate() {
        node.node = i64::MAX - i as i64;
        for (j, line) in node.lines.iter_mut().enumerate() {
            line.text = if j == 0 {
                "s".repeat(256)
            } else {
                String::new()
            };
        }
    }
    for (i, label) in value.style.pie_labels.iter_mut().enumerate() {
        label.slice = i64::MAX - i as i64;
    }
    for axis in [&mut value.style.x_axis, &mut value.style.y_axis] {
        axis.position = Some(1.);
        axis.tick_count = Some(64);
        axis.label_gap = Some(64.);
        axis.label_width = Some(256.);
        axis.line_color = Some(color);
        axis.label_color = Some(color);
        for tick in axis.ticks.as_mut().unwrap() {
            tick.position = TickPosition::Category(i64::MAX);
        }
    }
    value.style.grid.y = Some(vec![TickPosition::Category(i64::MAX); 64]);
    value.style.grid.color = Some(color);
    value.style.inspection.card.text_color = Some(color);
    value.style.inspection.card.background = Some(color);
    value.style.inspection.card.border_color = Some(color);
    value.style.inspection.crosshair.color = Some(color);
    value.style.inspection.crosshair.vertical_span =
        gpuio_protocol::chart_inspection::Span::Pixels(-32768., 65536.);
    value.style.inspection.crosshair.horizontal_span =
        gpuio_protocol::chart_inspection::Span::Fraction(-1., 2.);
    value.style.inspection.marker.fill = Some(color);
    value.style.inspection.marker.stroke = Some(color);
    assert!(value.is_valid());
    assert!(
        value.retained_bytes() - before
            >= pie::heap_bytes(&value.style.pie_labels)
                + node::heap_bytes(&value.style.node_labels)
                + value.style.x_axis.heap_bytes()
                + value.style.y_axis.heap_bytes()
                + value.style.grid.heap_bytes()
                + value.style.appearance.heap_bytes()
                + value.style.ordinal.as_ref().unwrap().heap_bytes()
    );
    let style_bytes = encode(&value.style);
    assert!(style_bytes.len() <= gpuio_protocol::chart_style::MAX_STYLE_BYTES);
    assert_eq!(
        gpuio_protocol::decode_chart_style(&style_bytes),
        Ok(value.style.clone())
    );
    let bytes = encode(&value);
    assert!(bytes.len() > 192 * 1024);
    assert!(bytes.len() < gpuio_protocol::v1::MAX_MESSAGE_BYTES);
    assert_eq!(decode_chart_view_config(&bytes), Ok(value));
}

#[test]
fn configuration_failure_appends_an_observation_error_without_retagging_existing_errors() {
    for (i, error) in [
        Error::WrongApplication,
        Error::UnavailableData,
        Error::RenderLimit,
        Error::NativeFailure,
        Error::InvalidConfig,
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(encode(&Observation::Failed(error)), vec![1, i as u8]);
    }
}
