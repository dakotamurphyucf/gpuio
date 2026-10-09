use binprot::BinProtWrite;
use gpuio_protocol::{
    decode_split_group_config, decode_split_group_snapshot, split::Axis, split_group::*,
};
fn bytes(v: &impl BinProtWrite) -> Vec<u8> {
    let mut data = vec![];
    v.binprot_write(&mut data).unwrap();
    data
}
fn fixture(hex: &str) -> Vec<u8> {
    let hex = hex.trim();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}
fn config() -> Config {
    Config {
        label: "Workspace".into(),
        axis: Axis::Vertical,
        keyboard_step: 8.,
        reset_generation: 2,
        resize: Some(ResizeRequest {
            id: "b".into(),
            size: 175.,
            serial: 3,
        }),
        panels: vec![
            Panel {
                id: "a".into(),
                label: "Files".into(),
                initial_size: Some(120.),
                minimum_size: 80.,
                maximum_size: 400.,
                visible: true,
            },
            Panel {
                id: "b".into(),
                label: "Editor".into(),
                initial_size: None,
                minimum_size: 40.,
                maximum_size: 600.,
                visible: true,
            },
            Panel {
                id: "c".into(),
                label: "Inspector".into(),
                initial_size: Some(90.),
                minimum_size: 20.,
                maximum_size: 300.,
                visible: false,
            },
        ],
    }
}
#[test]
fn exact_independent_public_values_truncation_and_trailing_bytes() {
    let cfg = config();
    let data = fixture(include_str!(
        "../../../test/fixtures/split-group-config.hex"
    ));
    assert_eq!(bytes(&cfg), data);
    assert_eq!(decode_split_group_config(&data), Ok(cfg.clone()));
    for end in 0..data.len() {
        assert!(decode_split_group_config(&data[..end]).is_err());
    }
    let mut extra = data;
    extra.push(0);
    assert!(decode_split_group_config(&extra).is_err());
    let snapshot = Snapshot {
        source: Source::Request(3),
        sizes: vec![("a".into(), 120.), ("b".into(), 175.), ("c".into(), 90.)],
    };
    let data = fixture(include_str!(
        "../../../test/fixtures/split-group-snapshot.hex"
    ));
    assert_eq!(bytes(&snapshot), data);
    assert_eq!(decode_split_group_snapshot(&data), Ok(snapshot.clone()));
    assert!(snapshot.valid_for(&cfg));
    for end in 0..data.len() {
        assert!(decode_split_group_snapshot(&data[..end]).is_err());
    }
    let mut extra = data;
    extra.push(0);
    assert!(decode_split_group_snapshot(&extra).is_err());
    let mut reordered = cfg;
    reordered.panels.swap(0, 1);
    assert!(!snapshot.valid_for(&reordered));
    for source in [
        Source::Pointer,
        Source::Keyboard,
        Source::Accessibility,
        Source::Request(i64::MAX),
    ] {
        let snapshot = Snapshot {
            source,
            ..snapshot.clone()
        };
        assert_eq!(decode_split_group_snapshot(&bytes(&snapshot)), Ok(snapshot));
    }
}
#[test]
fn malformed_size_labels_collections_and_requests_are_bounded() {
    let mut invalid = vec![];
    for value in [f64::NAN, f64::INFINITY, -1., 16385.] {
        let mut c = config();
        c.keyboard_step = value;
        invalid.push(c);
        let mut c = config();
        c.panels[0].minimum_size = value;
        invalid.push(c);
        let mut c = config();
        c.resize.as_mut().unwrap().size = value;
        invalid.push(c);
    }
    for serial in [-1, 0] {
        let mut c = config();
        c.resize.as_mut().unwrap().serial = serial;
        invalid.push(c);
    }
    for id in ["".into(), "a\0b".into(), "a".repeat(257)] {
        let mut c = config();
        c.panels[0].id = id;
        invalid.push(c);
    }
    let mut c = config();
    c.panels[0].initial_size = Some(20.);
    invalid.push(c);
    let mut c = config();
    c.panels[0].maximum_size = 20.;
    invalid.push(c);
    let mut c = config();
    c.label = " \n".into();
    invalid.push(c);
    let mut c = config();
    c.panels[0].label = "x".repeat(1025);
    invalid.push(c);
    let mut c = config();
    c.reset_generation = -1;
    invalid.push(c);
    let mut c = config();
    c.keyboard_step = 0.;
    invalid.push(c);
    let mut c = config();
    c.panels[1].id = "a".into();
    invalid.push(c);
    let mut c = config();
    c.panels = vec![c.panels[0].clone(); 65];
    invalid.push(c);
    for c in invalid {
        assert!(decode_split_group_config(&bytes(&c)).is_err(), "{c:?}");
    }
    let mut maximum = config();
    maximum.panels = (0..64)
        .map(|i| Panel {
            id: format!("p{i}"),
            ..maximum.panels[0].clone()
        })
        .collect();
    assert_eq!(
        decode_split_group_config(&bytes(&maximum)),
        Ok(maximum.clone())
    );
    maximum.panels.clear();
    assert!(decode_split_group_config(&bytes(&maximum)).is_ok());
    for snapshot in [
        Snapshot {
            source: Source::Request(0),
            sizes: vec![],
        },
        Snapshot {
            source: Source::Pointer,
            sizes: vec![("a".into(), 1.), ("a".into(), 2.)],
        },
        Snapshot {
            source: Source::Pointer,
            sizes: vec![("a".into(), f64::NAN)],
        },
    ] {
        assert!(decode_split_group_snapshot(&bytes(&snapshot)).is_err());
    }
}
