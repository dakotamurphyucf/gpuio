use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, chart_data::*, decode_chart_data};
fn encode(data: &Data) -> Vec<u8> {
    let mut bytes = Vec::new();
    data.binprot_write(&mut bytes).unwrap();
    bytes
}
fn point(id: i64, x: f64, y: Option<f64>, label: &str) -> Point {
    Point {
        id,
        x,
        y,
        label: label.into(),
    }
}
fn series(id: i64, name: &str, points: Vec<Point>) -> Series {
    Series {
        id,
        name: name.into(),
        points,
    }
}
fn data(contents: Contents) -> Data {
    Data {
        version: 3,
        bar_baselines: vec![],
        bar_backgrounds: vec![],
        contents,
    }
}
fn fixtures() -> Vec<(&'static str, Data)> {
    vec![
        (
            "cartesian",
            data(Contents::Cartesian(vec![
                Layer::Line(series(
                    1,
                    "Rate",
                    vec![point(1, 0., Some(-2.), "α"), point(2, 1., None, "")],
                )),
                Layer::Area(series(2, "Load", vec![point(1, 0., Some(3.), "")])),
                Layer::Bar(series(3, "Count", vec![point(1, 0., Some(0.), "0")])),
            ])),
        ),
        (
            "pie",
            data(Contents::Pie(vec![
                Slice {
                    id: 7,
                    label: "Cache".into(),
                    value: 2.,
                },
                Slice {
                    id: 8,
                    label: "Other".into(),
                    value: 0.,
                },
            ])),
        ),
        (
            "radar",
            data(Contents::Radar(
                vec![
                    RadarAxis {
                        id: 1,
                        label: "A".into(),
                        maximum: 10.,
                    },
                    RadarAxis {
                        id: 2,
                        label: "B".into(),
                        maximum: 20.,
                    },
                    RadarAxis {
                        id: 3,
                        label: "C".into(),
                        maximum: 30.,
                    },
                ],
                vec![RadarSeries {
                    id: 9,
                    name: "Agent".into(),
                    values: vec![(3, 15.), (1, 5.), (2, 0.)],
                }],
            )),
        ),
        (
            "candlestick",
            data(Contents::Candlestick(vec![Candle {
                id: 1,
                x: 1.,
                label: "Day".into(),
                open_: -2.,
                high: 0.,
                low: -4.,
                close: -1.,
            }])),
        ),
        (
            "sankey",
            data(Contents::Sankey(
                vec![
                    Node {
                        id: 1,
                        label: "Start".into(),
                    },
                    Node {
                        id: 2,
                        label: "End".into(),
                    },
                ],
                vec![Edge {
                    id: 5,
                    source: 1,
                    target: 2,
                    value: 1.5,
                }],
            )),
        ),
    ]
}
#[test]
fn all_families_match_independent_fixture_and_reject_every_truncation() {
    let fixture = include_str!("../../../test/fixtures/chart-v3-data.hex");
    let lines = fixture.lines().collect::<Vec<_>>();
    let values = fixtures();
    assert_eq!(lines.len(), values.len());
    for ((name, data), line) in values.into_iter().zip(lines) {
        let (expected_name, hex) = line.split_once(' ').unwrap();
        assert_eq!(name, expected_name);
        let expected = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect::<Vec<_>>();
        assert!(data.validate().is_ok());
        assert_eq!(encode(&data), expected);
        assert_eq!(decode_chart_data(&expected), Ok(data));
        for end in 0..expected.len() {
            assert!(
                decode_chart_data(&expected[..end]).is_err(),
                "{name}: truncation {end}"
            );
        }
        let mut trailing = expected;
        trailing.push(0);
        assert_eq!(decode_chart_data(&trailing), Err(DecodeError::Malformed));
    }
}
#[test]
fn domain_validation_cannot_be_bypassed_by_valid_binprot_structure() {
    let mut invalid = Vec::new();
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e101, -1e101] {
        invalid.push(data(Contents::Cartesian(vec![Layer::Line(series(
            1,
            "Line",
            vec![point(1, value, None, "")],
        ))])));
        invalid.push(data(Contents::Cartesian(vec![Layer::Area(series(
            1,
            "Area",
            vec![point(1, 0., Some(value), "")],
        ))])));
        invalid.push(data(Contents::Pie(vec![Slice {
            id: 1,
            label: "Slice".into(),
            value,
        }])));
        invalid.push(data(Contents::Candlestick(vec![Candle {
            id: 1,
            x: 0.,
            label: String::new(),
            open_: value,
            high: 0.,
            low: 0.,
            close: 0.,
        }])));
    }
    invalid.extend([
        data(Contents::Cartesian(vec![Layer::Bar(series(
            1,
            "Bar",
            vec![point(1, 0., None, "")],
        ))])),
        data(Contents::Cartesian(vec![Layer::Line(series(
            1,
            "Line",
            vec![point(1, 0., Some(1.), ""), point(2, 0., Some(2.), "")],
        ))])),
        data(Contents::Cartesian(vec![Layer::Line(series(
            1,
            "Line",
            vec![point(1, 0., Some(1.), ""), point(1, 1., Some(2.), "")],
        ))])),
        data(Contents::Pie(vec![Slice {
            id: 0,
            label: "Slice".into(),
            value: 1.,
        }])),
        data(Contents::Pie(vec![Slice {
            id: 1,
            label: "Slice".into(),
            value: -1.,
        }])),
        data(Contents::Pie(vec![Slice {
            id: 1,
            label: "\u{b}\u{c}\t ".into(),
            value: 1.,
        }])),
        data(Contents::Sankey(
            vec![Node {
                id: 1,
                label: "One".into(),
            }],
            vec![Edge {
                id: 1,
                source: 1,
                target: 2,
                value: 1.,
            }],
        )),
    ]);
    for candidate in invalid {
        assert!(candidate.validate().is_err());
        assert!(decode_chart_data(&encode(&candidate)).is_err());
    }
    let mut future = fixtures()[0].1.clone();
    future.version = 4;
    assert!(decode_chart_data(&encode(&future)).is_err());
    for name in ["\u{a0}", "\u{2003}"] {
        assert!(
            data(Contents::Cartesian(vec![Layer::Line(series(
                1,
                name,
                vec![]
            ))]))
            .validate()
            .is_ok()
        );
    }
}
#[test]
fn bounded_decoder_checks_aggregate_counts_text_and_utf8_before_allocation() {
    let mut huge = b"\x03\x00\x01\x00\x01\x01A".to_vec();
    binprot::Nat0((MAX_POINTS + 1) as u64)
        .binprot_write(&mut huge)
        .unwrap();
    assert_eq!(decode_chart_data(&huge), Err(DecodeError::LimitExceeded));
    assert_eq!(
        decode_chart_data(&vec![0; MAX_BYTES + 1]),
        Err(DecodeError::LimitExceeded)
    );
    assert!(decode_chart_data(&[3, 255]).is_err());
    let mut invalid_utf8 = b"\x03\x01\x01\x01\x01\xff".to_vec();
    invalid_utf8.extend(0f64.to_le_bytes());
    assert_eq!(
        decode_chart_data(&invalid_utf8),
        Err(DecodeError::Malformed)
    );
    let points = |n: usize, label: &str| {
        (0..n)
            .map(|i| point(i as i64 + 1, i as f64, Some(0.), label))
            .collect()
    };
    let aggregate = data(Contents::Cartesian(vec![
        Layer::Line(series(1, "A", points(50_000, ""))),
        Layer::Line(series(2, "B", points(50_001, ""))),
    ]));
    assert_eq!(
        decode_chart_data(&encode(&aggregate)),
        Err(DecodeError::LimitExceeded)
    );
    let text = data(Contents::Cartesian(vec![Layer::Line(series(
        1,
        "A",
        points(32_768, &"x".repeat(256)),
    ))]));
    assert_eq!(
        decode_chart_data(&encode(&text)),
        Err(DecodeError::LimitExceeded)
    );
    let maximum = data(Contents::Cartesian(vec![Layer::Line(series(
        1,
        "A",
        points(MAX_POINTS, ""),
    ))]));
    let decoded = decode_chart_data(&encode(&maximum)).unwrap();
    assert_eq!(
        decoded.validate(),
        Ok(Stats {
            values: MAX_POINTS,
            text_bytes: 1
        })
    );
}
#[test]
fn radar_candlestick_and_graph_checks_match_public_domains() {
    let (_, mut radar) = fixtures().remove(2);
    if let Contents::Radar(_, series) = &mut radar.contents {
        series[0].values[0].1 = 31.;
    }
    assert!(decode_chart_data(&encode(&radar)).is_err());
    let (_, mut candle) = fixtures().remove(3);
    if let Contents::Candlestick(candles) = &mut candle.contents {
        candles[0].low = 2.;
    }
    assert!(decode_chart_data(&encode(&candle)).is_err());
    let nodes = vec![
        Node {
            id: 1,
            label: "One".into(),
        },
        Node {
            id: 2,
            label: "Two".into(),
        },
    ];
    let cycle = data(Contents::Sankey(
        nodes.clone(),
        vec![
            Edge {
                id: 1,
                source: 1,
                target: 2,
                value: 0.,
            },
            Edge {
                id: 2,
                source: 2,
                target: 1,
                value: 0.,
            },
        ],
    ));
    assert!(decode_chart_data(&encode(&cycle)).is_err());
    let parallel = data(Contents::Sankey(
        nodes,
        vec![
            Edge {
                id: 1,
                source: 1,
                target: 2,
                value: 0.,
            },
            Edge {
                id: 2,
                source: 1,
                target: 2,
                value: 4.,
            },
        ],
    ));
    assert!(decode_chart_data(&encode(&parallel)).is_ok());
}
