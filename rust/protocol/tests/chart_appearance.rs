use binprot::BinProtWrite;
use gpuio_protocol::{
    DecodeError, chart_appearance::*, chart_options::Curve, decode_chart_appearance,
};

fn encode(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn gradient() -> Brush {
    Brush::Linear {
        oklab: true,
        angle: 90.,
        from: 3,
        start: 0.25,
        to: 4,
        stop: 0.75,
    }
}
fn fixture() -> Appearance {
    Appearance {
        series: vec![Series {
            series: 1,
            path: Some(Path {
                stroke: Some(Stroke {
                    visible: false,
                    width: Some(2.),
                    brush: Brush::Solid(2),
                }),
                fill: Some(gradient()),
                curve: Some(Curve::StepAfter),
            }),
            marker: Some(Marker {
                visible: Some(true),
                radius: Some(24.),
                fill: Some(5),
                stroke: Some(6),
                stroke_width: Some(8.),
            }),
            bar: None,
            legend: Some(13),
            area_baseline: None,
        }],
        data: [
            BarFill::Background(gradient()),
            BarFill::BaseToTip(7, 8),
            BarFill::Domain(9, 10),
            BarFill::Values(-2., 11, 3., 12),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, fill)| Datum {
            series: 1,
            datum: i as i64 + 1,
            marker: None,
            bar: Some(Bar {
                fill: Some(fill),
                corners: Some(Corners {
                    top_left: 1.,
                    top_right: 2.,
                    bottom_right: 3.,
                    bottom_left: 4.,
                }),
            }),
        })
        .collect(),
        aggregates: Aggregates::Uniform,
    }
}

#[test]
fn independent_ocaml_bytes_and_all_truncations() {
    let value = fixture();
    let bytes = encode(&value);
    // OCaml independently constructs these fields in chart_appearance_test.ml.
    let expected = include_str!("../../../test/fixtures/chart-appearance-v2.hex").trim();
    let actual: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(actual, expected);
    assert_eq!(decode_chart_appearance(&bytes).unwrap(), value);
    for end in 0..bytes.len() {
        assert!(
            decode_chart_appearance(&bytes[..end]).is_err(),
            "prefix {end}"
        );
    }
    let mut extra = bytes;
    extra.push(0);
    assert_eq!(decode_chart_appearance(&extra), Err(DecodeError::Malformed));
    assert_eq!(encode(&Appearance::default()), vec![0, 0, 0]);
    assert_eq!(
        decode_chart_appearance(&[0, 0, 0]).unwrap(),
        Appearance::default()
    );
    assert_eq!(
        decode_chart_appearance(&[0, 0, 2]),
        Err(DecodeError::Malformed)
    );
    assert_eq!(
        decode_chart_appearance(&vec![0; MAX_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
}

fn rejected(value: Appearance) {
    assert!(!value.is_valid());
    assert!(decode_chart_appearance(&encode(&value)).is_err());
}

#[test]
fn identities_counts_and_nested_invalid_values_are_rejected() {
    let mut value = fixture();
    value.series.push(value.series[0].clone());
    rejected(value);
    let mut value = fixture();
    value.data.push(value.data[0].clone());
    rejected(value);
    let mut value = fixture();
    value.data[1].series = 2;
    value.data[1].datum = 1;
    assert!(decode_chart_appearance(&encode(&value)).is_ok());
    for bad in [0, -1] {
        let mut value = fixture();
        value.series[0].series = bad;
        rejected(value);
        let mut value = fixture();
        value.data[0].datum = bad;
        rejected(value);
        let mut value = fixture();
        value.data[0].series = bad;
        rejected(value);
    }
    for bad in [-1, 0x1_0000_0000] {
        let mut value = fixture();
        value.series[0].legend = Some(bad);
        rejected(value);
        let mut value = fixture();
        value.series[0].marker.as_mut().unwrap().fill = Some(bad);
        rejected(value);
        let mut value = fixture();
        value.series[0].path.as_mut().unwrap().fill = Some(Brush::Solid(bad));
        rejected(value);
    }
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 24.01, 0.99] {
        let mut value = fixture();
        value.series[0].marker.as_mut().unwrap().radius = Some(bad);
        rejected(value);
    }
    for bad in [f64::NAN, f64::INFINITY, -0.01, 32.01] {
        let mut value = fixture();
        value.data[0]
            .bar
            .as_mut()
            .unwrap()
            .corners
            .as_mut()
            .unwrap()
            .bottom_left = bad;
        rejected(value);
    }
    for fill in [
        BarFill::Values(1., 0, 1., 0),
        BarFill::Values(2., 0, 1., 0),
        BarFill::Values(f64::NAN, 0, 1., 0),
        BarFill::Values(-1.01e100, 0, 1., 0),
        BarFill::Domain(0, -1),
    ] {
        let mut value = fixture();
        value.data[0].bar.as_mut().unwrap().fill = Some(fill);
        rejected(value);
    }
    for brush in [
        Brush::Linear {
            oklab: false,
            angle: f64::NAN,
            from: 0,
            start: 0.,
            to: 0,
            stop: 1.,
        },
        Brush::Linear {
            oklab: false,
            angle: 90.,
            from: 0,
            start: 0.8,
            to: 0,
            stop: 0.2,
        },
        Brush::Linear {
            oklab: false,
            angle: 90.,
            from: 0,
            start: 0.,
            to: 0,
            stop: 1.01,
        },
    ] {
        let mut value = fixture();
        value.series[0].path.as_mut().unwrap().fill = Some(brush);
        rejected(value);
    }
    // Count admission occurs before reading/allocating any list element.
    assert_eq!(
        decode_chart_appearance(&[0xfe, 129, 0]),
        Err(DecodeError::LimitExceeded)
    );
    assert_eq!(
        decode_chart_appearance(&[0, 0xfe, 1, 4]),
        Err(DecodeError::LimitExceeded)
    );
}

#[test]
fn largest_valid_encoding_fits_and_is_not_a_dense_source_allocation() {
    let c = 0xffff_ffff;
    let brush = Brush::Linear {
        oklab: true,
        angle: 360.,
        from: c,
        start: 0.,
        to: c,
        stop: 1.,
    };
    let marker = Some(Marker {
        visible: Some(true),
        radius: Some(24.),
        fill: Some(c),
        stroke: Some(c),
        stroke_width: Some(8.),
    });
    let bar = Some(Bar {
        fill: Some(BarFill::Background(brush)),
        corners: Some(Corners {
            top_left: 32.,
            top_right: 32.,
            bottom_right: 32.,
            bottom_left: 32.,
        }),
    });
    let path = Some(Path {
        stroke: Some(Stroke {
            visible: true,
            width: Some(8.),
            brush,
        }),
        fill: Some(brush),
        curve: Some(Curve::Natural),
    });
    let value = Appearance {
        series: (0..MAX_SERIES)
            .map(|i| Series {
                series: i64::MAX - i as i64,
                path,
                marker,
                bar,
                legend: Some(c),
                area_baseline: Some(1e100),
            })
            .collect(),
        data: (0..MAX_DATA)
            .map(|i| Datum {
                series: i64::MAX,
                datum: i64::MAX - i as i64,
                marker,
                bar,
            })
            .collect(),
        aggregates: Aggregates::Uniform,
    };
    let bytes = encode(&value);
    assert!(bytes.len() <= MAX_BYTES, "{}", bytes.len());
    assert_eq!(bytes.len(), 174599); // Independently measured by the OCaml maximum test.
    assert_eq!(decode_chart_appearance(&bytes).unwrap(), value);
    assert!(value.heap_bytes() < 512 * 1024);
    let mut over = value.clone();
    let mut series = over.series[0].clone();
    series.series = 1;
    over.series.push(series);
    rejected(over);
    let mut over = value;
    let mut datum = over.data[0].clone();
    datum.datum = 1;
    over.data.push(datum);
    rejected(over);
}

#[test]
fn patterns_have_paired_bytes_and_validate_before_preparation() {
    for (brush, hex) in [
        (
            Brush::PatternSlash(0, 2., 4.),
            "020000000000000000400000000000001040",
        ),
        (Brush::Checkerboard(0, 8.), "03000000000000002040"),
    ] {
        assert_eq!(
            encode(&brush)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            hex
        );
        let mut value = fixture();
        value.series[0].path.as_mut().unwrap().fill = Some(brush);
        value.data[0].bar.as_mut().unwrap().fill = Some(BarFill::Background(brush));
        let bytes = encode(&value);
        assert_eq!(decode_chart_appearance(&bytes), Ok(value));
        for end in 0..bytes.len() {
            assert!(decode_chart_appearance(&bytes[..end]).is_err());
        }
    }
    for n in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0., 0.49, 64.01] {
        for brush in [
            Brush::PatternSlash(0, n, 4.),
            Brush::PatternSlash(0, 2., n),
            Brush::Checkerboard(0, n),
        ] {
            let mut value = fixture();
            value.series[0].path.as_mut().unwrap().fill = Some(brush);
            assert!(!value.is_valid());
            assert!(decode_chart_appearance(&encode(&value)).is_err());
        }
    }
}

#[test]
fn area_baseline_bytes_and_semantic_bounds() {
    let mut value = Appearance {
        series: vec![Series {
            series: 1,
            path: None,
            marker: None,
            bar: None,
            legend: None,
            area_baseline: Some(42.),
        }],
        ..Default::default()
    };
    assert_eq!(
        encode(&value)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "0101000000000100000000000045400000"
    );
    assert_eq!(decode_chart_appearance(&encode(&value)), Ok(value.clone()));
    for n in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.01e100,
        1.01e100,
    ] {
        value.series[0].area_baseline = Some(n);
        rejected(value.clone());
    }
    for n in [-1e100, 0., 1e100] {
        value.series[0].area_baseline = Some(n);
        assert_eq!(decode_chart_appearance(&encode(&value)), Ok(value.clone()));
    }
}
