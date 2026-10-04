use binprot::BinProtWrite;
use gpuio_protocol::{decode_split_group_appearance, split_group_appearance::Config, v1::*};
fn bytes(v: &Config) -> Vec<u8> {
    let mut data = vec![];
    v.binprot_write(&mut data).unwrap();
    data
}
#[test]
fn independent_appearance_fixture_and_bounded_decode() {
    let cfg = Config {
        thickness: 3.,
        hit_extent: 16.,
        handle_style: vec![Style::Fields(vec![Field::Opacity(0.75)])],
        item_styles: vec![("a".into(), vec![Style::State(2, vec![Field::Opacity(0.5)])])],
    };
    let hex = include_str!("../../../test/fixtures/split-group-appearance.hex").trim();
    let data: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    assert_eq!(bytes(&cfg), data);
    assert_eq!(decode_split_group_appearance(&data), Ok(cfg.clone()));
    for end in 0..data.len() {
        assert!(decode_split_group_appearance(&data[..end]).is_err());
    }
    let mut trailing = data;
    trailing.push(0);
    assert!(decode_split_group_appearance(&trailing).is_err());
    let mut bad = vec![];
    for v in [f64::NAN, f64::INFINITY, -1., 0., 17.] {
        bad.push(Config {
            thickness: v,
            ..cfg.clone()
        });
    }
    for v in [f64::NAN, 0., 7., 33.] {
        bad.push(Config {
            hit_extent: v,
            ..cfg.clone()
        });
    }
    bad.push(Config {
        thickness: 12.,
        hit_extent: 8.,
        ..cfg.clone()
    });
    for id in ["", "a\0b", &"x".repeat(257)] {
        bad.push(Config {
            item_styles: vec![(id.into(), vec![])],
            ..cfg.clone()
        });
    }
    bad.push(Config {
        item_styles: vec![("a".into(), vec![]); 2],
        ..cfg.clone()
    });
    bad.push(Config {
        item_styles: (0..65).map(|i| (i.to_string(), vec![])).collect(),
        ..cfg.clone()
    });
    for state in [0, 4, 5, 7, 8] {
        bad.push(Config {
            handle_style: vec![Style::State(state, vec![])],
            ..cfg.clone()
        });
    }
    bad.push(Config {
        handle_style: vec![Style::Fields(vec![]); 257],
        ..cfg.clone()
    });
    bad.push(Config {
        item_styles: (0..64)
            .map(|i| {
                (
                    i.to_string(),
                    vec![Style::Fields(vec![Field::Opacity(1.); 5])],
                )
            })
            .collect(),
        ..cfg.clone()
    });
    for c in bad {
        assert!(decode_split_group_appearance(&bytes(&c)).is_err(), "{c:?}");
    }
    let maximum = Config {
        item_styles: (0..64).map(|i| (i.to_string(), vec![])).collect(),
        ..cfg
    };
    assert!(decode_split_group_appearance(&bytes(&maximum)).is_ok());
}

#[test]
fn appended_mount_operation_and_resize_event_match_independent_envelopes() {
    use gpuio_protocol::{HandlerId, NodeId, WindowId};
    let fixture = |hex: &str| {
        let hex = hex.trim();
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect::<Vec<_>>()
    };
    let config = gpuio_protocol::decode_split_group_config(&fixture(include_str!(
        "../../../test/fixtures/split-group-config.hex"
    )))
    .unwrap();
    let paint = decode_split_group_appearance(&fixture(include_str!(
        "../../../test/fixtures/split-group-appearance.hex"
    )))
    .unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let window = WindowId::from_parts(0, 1).unwrap();
    let op = Op::SetSplitGroup(node, config, paint);
    let mut data = vec![];
    op.binprot_write(&mut data).unwrap();
    assert_eq!(
        data,
        fixture(include_str!(
            "../../../test/fixtures/split-group-operation.hex"
        ))
    );
    let message = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![op],
    });
    let mut data = vec![];
    message.binprot_write(&mut data).unwrap();
    assert_eq!(gpuio_protocol::decode(&data), Ok(message));
    let mut kind = vec![];
    Kind::SplitGroup.binprot_write(&mut kind).unwrap();
    assert_eq!(kind, vec![56]);
    let snapshot = gpuio_protocol::decode_split_group_snapshot(&fixture(include_str!(
        "../../../test/fixtures/split-group-snapshot.hex"
    )))
    .unwrap();
    let mut event = vec![];
    vec![Event::SplitGroupResized(
        window,
        node,
        HandlerId::from_parts(0, 1).unwrap(),
        4,
        2,
        snapshot,
    )]
    .binprot_write(&mut event)
    .unwrap();
    assert_eq!(
        event,
        fixture(include_str!("../../../test/fixtures/split-group-event.hex"))
    );
}
