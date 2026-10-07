use binprot::BinProtWrite;
use gpuio_protocol::{chart_data::*, decode_chart_data};
fn fixture() -> Vec<u8> {
    let text = include_str!("../../../test/fixtures/chart-v3-categorical.hex").trim();
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}
#[test]
fn independent_categorical_fixture_and_bounded_decoding() {
    let bytes = fixture();
    let value = decode_chart_data(&bytes).unwrap();
    let Contents::Categorical(categories, layers) = &value.contents else {
        panic!()
    };
    assert_eq!(
        categories.iter().map(|c| c.id).collect::<Vec<_>>(),
        vec![42, 7, 99]
    );
    assert_eq!(categories[0].label, categories[2].label);
    assert_eq!(layers[0].series().points[1].value, None);
    assert!(matches!(layers[0], CategoricalLayer::Bar(_)));
    assert_eq!(value.validate().unwrap().values, 3);
    let mut encoded = vec![];
    value.binprot_write(&mut encoded).unwrap();
    assert_eq!(encoded, bytes);
    for n in 0..bytes.len() {
        assert!(decode_chart_data(&bytes[..n]).is_err());
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(decode_chart_data(&trailing).is_err());
    let invalid = |data: Data| {
        assert!(data.validate().is_err());
        let mut b = vec![];
        data.binprot_write(&mut b).unwrap();
        assert!(decode_chart_data(&b).is_err());
    };
    for mode in 0..7 {
        let mut bad = value.clone();
        let Contents::Categorical(categories, layers) = &mut bad.contents else {
            panic!()
        };
        let CategoricalLayer::Bar(series) = &mut layers[0] else {
            panic!()
        };
        match mode {
            0 => categories.reverse(),
            1 => categories[1].id = categories[0].id,
            2 => series.points[0].category = 1,
            3 => series.points[1].id = series.points[0].id,
            4 => series.points[0].value = Some(f64::NAN),
            5 => {
                series.points.pop();
            }
            6 => categories[0].label = "\n".into(),
            _ => unreachable!(),
        }
        invalid(bad);
    }
    let excessive = Data {
        version: 3,
        bar_baselines: vec![],
        bar_backgrounds: vec![],
        contents: Contents::Categorical(
            (0..=MAX_POINTS)
                .map(|i| Category {
                    id: i as i64 + 1,
                    label: "x".into(),
                })
                .collect(),
            vec![],
        ),
    };
    invalid(excessive);
}
