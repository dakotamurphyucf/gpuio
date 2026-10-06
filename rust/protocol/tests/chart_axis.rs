use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, chart_axis::*, chart_grid::Grid, chart_style::Style, chart_view::Config,
    decode_chart_style, decode_chart_view_config,
};

fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn unhex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks_exact(2)
        .map(|s| u8::from_str_radix(std::str::from_utf8(s).unwrap(), 16).unwrap())
        .collect()
}
fn axis() -> Axis {
    Axis {
        line: false,
        position: Some(0.25),
        ticks: Some(vec![
            Tick {
                position: TickPosition::Value(0.5),
                text: "λ".into(),
                color: Some(7),
                font_size: Some(16.),
                align: LabelAlign::Right,
            },
            Tick {
                position: TickPosition::Category(9),
                text: String::new(),
                color: None,
                font_size: None,
                align: LabelAlign::Auto,
            },
        ]),
        tick_count: Some(4),
        label_side: LabelSide::Before,
        label_align: LabelAlign::Center,
        label_gap: Some(2.),
        label_width: Some(90.),
        font_size: 12.,
        line_width: 2.,
        line_color: Some(3),
        label_color: Some(4),
        ..Default::default()
    }
}
#[test]
fn axis_and_grid_match_independent_ocaml_bytes_and_current_envelope() {
    let axis = axis();
    let grid = Grid {
        x: Some(vec![TickPosition::Fraction(0.75)]),
        y: Some(vec![]),
        dashes: vec![3., 2., 1.],
        width: 2.,
        color: Some(5),
    };
    assert_eq!(
        encode(&axis),
        unhex(
            "000101000000000000d03f010200000000000000e03f02cebb010701000000000000304003010900000000010401020100000000000000400100000000008056400000000000002840000000000000004001030104"
        )
    );
    assert_eq!(
        encode(&grid),
        unhex(
            "010102000000000000e83f01000300000000000008400000000000000040000000000000f03f00000000000000400105"
        )
    );
    let style = Style {
        x_axis: Box::new(axis),
        grid: Box::new(grid),
        ..Default::default()
    };
    assert_eq!(decode_chart_style(&encode(&style)), Ok(style.clone()));
    let bytes = encode(&style);
    for end in 0..bytes.len() {
        assert!(decode_chart_style(&bytes[..end]).is_err());
    }
    let mut bad_utf8 = bytes.clone();
    let index = bad_utf8.windows(2).position(|v| v == [0xce, 0xbb]).unwrap();
    bad_utf8[index] = 255;
    assert!(decode_chart_style(&bad_utf8).is_err());
    for version in [-3, -2, -1, 0, 1] {
        let mut old = style.clone();
        old.version = version;
        assert_eq!(
            decode_chart_style(&encode(&old)),
            Err(DecodeError::Malformed)
        );
    }
    let old = unhex(include_str!("../../../test/fixtures/chart-v9-pie-view.hex").trim());
    assert_eq!(decode_chart_view_config(&old), Err(DecodeError::Malformed));
    let mut bad = encode(&Style::default());
    let start = bad.len() - 66;
    bad[start + 5] = 3; // Invalid side, with all following fields still present.
    assert_eq!(decode_chart_style(&bad), Err(DecodeError::Malformed));
    bad[start + 5] = 0;
    bad[start + 6] = 4;
    assert_eq!(decode_chart_style(&bad), Err(DecodeError::Malformed));
    let mut oversized = bad[..start + 3].to_vec();
    oversized.extend([1, 65]); // Some 65 ticks: fail before reading/allocating them.
    assert_eq!(
        decode_chart_style(&oversized),
        Err(DecodeError::LimitExceeded)
    );
}
#[test]
fn axis_and_grid_reject_all_invalid_dimensions_targets_colors_and_text() {
    let reject = |axis: Axis, grid: Grid| {
        let style = Style {
            x_axis: Box::new(axis),
            grid: Box::new(grid),
            ..Default::default()
        };
        assert!(!style.is_valid());
        assert!(decode_chart_style(&encode(&style)).is_err());
    };
    for n in [f64::NAN, f64::INFINITY, -0.01, 1.01] {
        reject(
            Axis {
                position: Some(n),
                ..Default::default()
            },
            Grid::default(),
        );
    }
    for n in [f64::NAN, f64::INFINITY, 7.99, 32.01] {
        reject(
            Axis {
                font_size: n,
                ..Default::default()
            },
            Grid::default(),
        );
        let mut a = axis();
        a.ticks.as_mut().unwrap()[0].font_size = Some(n);
        reject(a, Grid::default());
    }
    for n in [-1., 64.01, f64::NAN] {
        reject(
            Axis {
                label_gap: Some(n),
                ..Default::default()
            },
            Grid::default(),
        );
    }
    for n in [7.99, 256.01, f64::NAN] {
        reject(
            Axis {
                label_width: Some(n),
                ..Default::default()
            },
            Grid::default(),
        );
    }
    for n in [0.49, 8.01, f64::NAN] {
        reject(
            Axis {
                line_width: n,
                ..Default::default()
            },
            Grid::default(),
        );
        reject(
            Axis::default(),
            Grid {
                width: n,
                ..Default::default()
            },
        );
    }
    for count in [1, 65] {
        reject(
            Axis {
                tick_count: Some(count),
                ..Default::default()
            },
            Grid::default(),
        );
    }
    for n in [0.49, 128.01, f64::NAN, f64::INFINITY] {
        reject(
            Axis::default(),
            Grid {
                dashes: vec![n],
                ..Default::default()
            },
        );
    }
    for c in [-1, 0x1_0000_0000] {
        reject(
            Axis {
                line_color: Some(c),
                ..Default::default()
            },
            Grid::default(),
        );
        reject(
            Axis {
                label_color: Some(c),
                ..Default::default()
            },
            Grid::default(),
        );
        reject(
            Axis::default(),
            Grid {
                color: Some(c),
                ..Default::default()
            },
        );
        let mut a = axis();
        a.ticks.as_mut().unwrap()[0].color = Some(c);
        reject(a, Grid::default());
    }
    for position in [
        TickPosition::Value(f64::NAN),
        TickPosition::Value(1.01e100),
        TickPosition::Category(0),
        TickPosition::Category(-1),
        TickPosition::Fraction(1.01),
    ] {
        let mut a = axis();
        a.ticks.as_mut().unwrap()[0].position = position;
        reject(a, Grid::default());
        reject(
            Axis::default(),
            Grid {
                x: Some(vec![position]),
                ..Default::default()
            },
        );
    }
    for text in ["a\n".into(), "\u{7f}".into(), "x".repeat(257)] {
        let mut a = axis();
        a.ticks.as_mut().unwrap()[0].text = text;
        reject(a, Grid::default());
    }
    reject(
        Axis::default(),
        Grid {
            dashes: vec![1.; 17],
            ..Default::default()
        },
    );
    reject(
        Axis::default(),
        Grid {
            y: Some(vec![TickPosition::Fraction(0.); 65]),
            ..Default::default()
        },
    );
    let mut a = axis();
    a.ticks = Some(vec![a.ticks.as_ref().unwrap()[0].clone(); 65]);
    reject(a, Grid::default());
}
#[test]
fn full_tick_and_grid_storage_is_bounded_roundtrips_and_is_charged() {
    let mut style = Style::default();
    let base = style.heap_bytes();
    let tick = Tick {
        position: TickPosition::Value(1.),
        text: "x".repeat(256),
        color: Some(0xffff_ffff),
        font_size: Some(32.),
        align: LabelAlign::Left,
    };
    style.x_axis.ticks = Some(vec![tick.clone(); 64]);
    style.y_axis.ticks = Some(vec![tick; 64]);
    style.grid.x = Some(vec![TickPosition::Category(i64::MAX); 64]);
    style.grid.y = Some(vec![TickPosition::Fraction(1.); 64]);
    style.grid.dashes = vec![128.; 16];
    assert_eq!(
        style.heap_bytes() - base,
        style.x_axis.heap_bytes() + style.y_axis.heap_bytes() + style.grid.heap_bytes()
    );
    assert!(style.heap_bytes() - base >= 128 * 256);
    let config = Config {
        version: -1,
        source: None,
        label: "Max ticks".into(),
        options: Default::default(),
        sampling: Default::default(),
        style,
        radar_labels: vec![],
        legend: true,
        disabled: false,
    };
    assert_eq!(decode_chart_view_config(&encode(&config)), Ok(config));
}
