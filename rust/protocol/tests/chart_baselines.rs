use binprot::BinProtWrite;
use gpuio_protocol::{DecodeError, chart_data::*, decode_chart_data};
fn bytes(data: &Data) -> Vec<u8> {
    let mut bytes = vec![];
    data.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex(text: &str) -> Vec<u8> {
    let text = text.trim();
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}
fn source() -> Data {
    decode_chart_data(&hex(include_str!(
        "../../../test/fixtures/chart-v3-backgrounds.hex"
    )))
    .unwrap()
}
#[test]
fn independent_baseline_fixture_truncation_and_old_schema() {
    let mut data = source();
    data.bar_baselines = vec![
        BarBaseline {
            series: 7,
            datum: 3,
            baseline: -1.,
        },
        BarBaseline {
            series: 7,
            datum: 9,
            baseline: 1.,
        },
    ];
    let fixture = hex(include_str!(
        "../../../test/fixtures/chart-v3-baselines.hex"
    ));
    assert_eq!(bytes(&data), fixture);
    assert_eq!(decode_chart_data(&fixture), Ok(data.clone()));
    assert_eq!(data.bar_baseline(7, 9), Some(1.));
    assert_eq!(data.bar_baseline(7, 8), None);
    for n in 0..fixture.len() {
        assert!(decode_chart_data(&fixture[..n]).is_err());
    }
    let mut trailing = fixture;
    trailing.push(0);
    assert!(decode_chart_data(&trailing).is_err());
    for line in include_str!("../../../test/fixtures/chart-v2-data.hex").lines() {
        let (_, text) = line.split_once(' ').unwrap();
        assert!(decode_chart_data(&hex(text)).is_err());
    }
}
#[test]
fn malformed_baselines_fail_before_publication() {
    let reject = |data: Data| {
        assert!(data.validate().is_err());
        assert!(decode_chart_data(&bytes(&data)).is_err());
    };
    for baseline in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1.1e100] {
        let mut data = source();
        data.bar_baselines = vec![BarBaseline {
            series: 7,
            datum: 9,
            baseline,
        }];
        reject(data);
    }
    for entries in [
        vec![(7, 9), (7, 9)],
        vec![(7, 9), (7, 3)],
        vec![(8, 9)],
        vec![(7, 10)],
    ] {
        let mut data = source();
        data.bar_baselines = entries
            .into_iter()
            .map(|(series, datum)| BarBaseline {
                series,
                datum,
                baseline: 1.,
            })
            .collect();
        reject(data);
    }
    let mut data = source();
    data.bar_backgrounds.clear();
    data.bar_baselines = vec![BarBaseline {
        series: 7,
        datum: 9,
        baseline: 1.,
    }];
    let Contents::Cartesian(layers) = &mut data.contents else {
        panic!()
    };
    layers[0] = Layer::Line(layers[0].series().clone());
    reject(data);
    let mut encoded = bytes(&source());
    assert_eq!(encoded.pop(), Some(0));
    binprot::Nat0((MAX_POINTS + 1) as u64)
        .binprot_write(&mut encoded)
        .unwrap();
    assert_eq!(decode_chart_data(&encoded), Err(DecodeError::LimitExceeded));
}

#[test]
fn maximum_baseline_source_roundtrips_and_counts_both_sidecars() {
    let mut data = Data {
        version: 3,
        bar_backgrounds: vec![],
        bar_baselines: vec![],
        contents: Contents::Cartesian(vec![Layer::Bar(Series {
            id: 7,
            name: "Dense".into(),
            points: (1..=MAX_POINTS)
                .map(|id| Point {
                    id: id as i64,
                    x: id as f64,
                    y: Some(3.),
                    label: String::new(),
                })
                .collect(),
        })]),
    };
    data.bar_baselines = (1..=MAX_POINTS)
        .map(|id| BarBaseline {
            series: 7,
            datum: id as i64,
            baseline: 2.,
        })
        .collect();
    data.bar_backgrounds = (1..=MAX_POINTS)
        .map(|id| BarBackground {
            series: 7,
            datum: id as i64,
            brush: gpuio_protocol::chart_appearance::Brush::Solid(0x00ff00ff),
        })
        .collect();
    assert!(bytes(&data).len() < MAX_BYTES);
    assert_eq!(decode_chart_data(&bytes(&data)), Ok(data.clone()));
    data.bar_baselines.push(data.bar_baselines[0].clone());
    assert_eq!(data.validate(), Err(ValidationError::LimitExceeded));
}
