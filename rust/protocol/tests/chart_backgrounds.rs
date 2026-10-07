use binprot::BinProtWrite;
use gpuio_protocol::{chart_appearance::Brush, chart_data::*, decode_chart_data};

fn encode(data: &Data) -> Vec<u8> {
    let mut bytes = Vec::new();
    data.binprot_write(&mut bytes).unwrap();
    bytes
}
fn source() -> Data {
    Data {
        version: 2,
        contents: Contents::Cartesian(vec![Layer::Bar(Series {
            id: 7,
            name: "Bars".into(),
            points: [(9, 0., 2.), (3, 1., -3.)]
                .into_iter()
                .map(|(id, x, y)| Point {
                    id,
                    x,
                    y: Some(y),
                    label: String::new(),
                })
                .collect(),
        })]),
        bar_backgrounds: vec![
            BarBackground {
                series: 7,
                datum: 3,
                brush: Brush::Checkerboard(10, 8.),
            },
            BarBackground {
                series: 7,
                datum: 9,
                brush: Brush::Solid(9),
            },
        ],
    }
}
#[test]
fn paired_background_fixture_and_every_truncation() {
    let text = include_str!("../../../test/fixtures/chart-v2-backgrounds.hex").trim();
    let bytes = (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    let data = source();
    assert_eq!(encode(&data), bytes);
    assert_eq!(decode_chart_data(&bytes), Ok(data));
    for n in 0..bytes.len() {
        assert!(decode_chart_data(&bytes[..n]).is_err(), "truncation {n}");
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(decode_chart_data(&trailing).is_err());
}

#[test]
fn sidecar_count_is_bounded_before_allocating_entries() {
    let mut data = source();
    data.bar_backgrounds.clear();
    let mut bytes = encode(&data);
    assert_eq!(bytes.pop(), Some(0));
    binprot::Nat0((MAX_POINTS + 1) as u64)
        .binprot_write(&mut bytes)
        .unwrap();
    assert_eq!(
        decode_chart_data(&bytes),
        Err(gpuio_protocol::DecodeError::LimitExceeded)
    );
}
#[test]
fn invalid_sidecars_cannot_enter_a_snapshot() {
    let reject = |data: Data| {
        assert!(data.validate().is_err());
        assert!(decode_chart_data(&encode(&data)).is_err());
    };
    let mut data = source();
    data.bar_backgrounds.reverse();
    reject(data);
    let mut data = source();
    data.bar_backgrounds.push(data.bar_backgrounds[1].clone());
    reject(data);
    for (series, datum, brush) in [
        (8, 3, Brush::Solid(9)),
        (7, 10, Brush::Solid(9)),
        (7, 3, Brush::Checkerboard(9, f64::NAN)),
        (7, 3, Brush::Solid(-1)),
    ] {
        let mut data = source();
        data.bar_backgrounds = vec![BarBackground {
            series,
            datum,
            brush,
        }];
        reject(data);
    }
    let mut data = source();
    let Contents::Cartesian(layers) = &mut data.contents else {
        unreachable!()
    };
    layers[0] = Layer::Line(layers[0].series().clone());
    reject(data);
    let mut data = source();
    data.version = 1;
    reject(data);
    let mut data = source();
    data.bar_backgrounds = vec![data.bar_backgrounds[0].clone(); MAX_POINTS + 1];
    reject(data);
}
#[test]
fn old_data_fixtures_are_rejected_instead_of_defaulting_the_new_field() {
    for line in include_str!("../../../test/fixtures/chart-v1-data.hex").lines() {
        let (_, text) = line.split_once(' ').unwrap();
        let bytes = (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect::<Vec<_>>();
        assert!(decode_chart_data(&bytes).is_err());
    }
}
#[test]
fn dense_100k_source_roundtrips_without_growing_point_records() {
    let count = MAX_POINTS;
    let mut data = source();
    data.contents = Contents::Cartesian(vec![Layer::Bar(Series {
        id: 7,
        name: "Dense".into(),
        points: (0..count)
            .map(|i| Point {
                id: (count - i) as i64,
                x: i as f64,
                y: Some((i % 7) as f64),
                label: String::new(),
            })
            .collect(),
    })]);
    data.bar_backgrounds = (1..=count)
        .map(|i| BarBackground {
            series: 7,
            datum: i as i64,
            brush: Brush::Solid((i % 256) as i64),
        })
        .collect();
    assert_eq!(data.validate().unwrap().values, count);
    let bytes = encode(&data);
    assert!(bytes.len() < MAX_BYTES);
    assert_eq!(decode_chart_data(&bytes), Ok(data));
}

#[test]
fn combined_encoding_limit_applies_even_when_counts_and_text_fit() {
    let data = Data {
        version: 2,
        contents: Contents::Cartesian(vec![Layer::Bar(Series {
            id: i64::MAX,
            name: "Dense".into(),
            points: (0..MAX_POINTS)
                .map(|i| Point {
                    id: i64::MAX - i as i64,
                    x: i as f64,
                    y: Some(1.),
                    label: "x".repeat(83),
                })
                .collect(),
        })]),
        bar_backgrounds: (0..MAX_POINTS)
            .rev()
            .map(|i| BarBackground {
                series: i64::MAX,
                datum: i64::MAX - i as i64,
                brush: Brush::Linear {
                    oklab: false,
                    angle: 180.,
                    from: 0xffffffff,
                    start: 0.,
                    to: 0xffffffff,
                    stop: 1.,
                },
            })
            .collect(),
    };
    let bytes = encode(&data);
    assert!(bytes.len() > MAX_BYTES);
    assert_eq!(data.validate(), Err(ValidationError::LimitExceeded));
    assert_eq!(
        decode_chart_data(&bytes),
        Err(gpuio_protocol::DecodeError::LimitExceeded)
    );
}
