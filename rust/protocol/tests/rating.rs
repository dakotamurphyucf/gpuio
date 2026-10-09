use binprot::BinProtWrite;
use gpuio_protocol::{
    HandlerId, NodeId, WindowId, decode,
    rating::{Config, Request},
    v1::*,
};
fn config(value: i64, maximum: i64, star_size: f64, disabled: bool, read_only: bool) -> Config {
    Config {
        label: "Quality".into(),
        value,
        maximum,
        star_size,
        disabled,
        read_only,
    }
}
fn message(configs: Vec<Config>) -> Message {
    Message::Apply(Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: 0,
        revision: 1,
        operations: configs
            .into_iter()
            .enumerate()
            .flat_map(|(i, config)| {
                let node = NodeId::from_parts(i as i64, 1).unwrap();
                [
                    Op::Create(node, Kind::Rating, "".into(), None),
                    Op::SetRating(node, config),
                ]
            })
            .collect(),
    })
}
fn encode<T: BinProtWrite>(value: &T) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn independent_rating_config_and_requests_strict_bounds() {
    let message = message(vec![
        config(2, 5, 24., false, false),
        config(0, 1, 8., true, false),
        config(32, 32, 128., false, true),
    ]);
    let bytes = encode(&message);
    assert_eq!(
        hex(&bytes),
        include_str!("../../../test/fixtures/rating-request.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(decode(&extra).is_err());
    let mut invalid = bytes;
    let at = invalid.windows(7).position(|w| w == b"Quality").unwrap();
    invalid[at] = 255;
    assert!(decode(&invalid).is_err());
    let events: Vec<_> = [
        Request::Set(4),
        Request::Toggle(3),
        Request::Increase,
        Request::Decrease,
    ]
    .into_iter()
    .map(|request| {
        Event::RatingRequested(
            WindowId::from_parts(0, 1).unwrap(),
            NodeId::from_parts(0, 1).unwrap(),
            HandlerId::from_parts(0, 1).unwrap(),
            1,
            request,
        )
    })
    .collect();
    assert_eq!(
        hex(&encode(&events)),
        include_str!("../../../test/fixtures/rating-events.hex").trim()
    );
}
#[test]
fn malformed_configuration_and_policy_reject() {
    let good = config(2, 5, 24., false, false);
    let mut invalid = vec![];
    for value in [-1, 6, i64::MAX] {
        invalid.push(Config {
            value,
            ..good.clone()
        });
    }
    for maximum in [0, 33, i64::MAX] {
        invalid.push(Config {
            maximum,
            ..good.clone()
        });
    }
    for star_size in [f64::NAN, f64::INFINITY, 7.9, 128.1] {
        invalid.push(Config {
            star_size,
            ..good.clone()
        });
    }
    for label in ["".into(), " \t".into(), "a\0b".into(), "a".repeat(4097)] {
        invalid.push(Config {
            label,
            ..good.clone()
        });
    }
    for config in invalid {
        assert!(!config.is_valid());
        assert!(decode(&encode(&message(vec![config]))).is_err());
    }
    for request in [
        Request::Set(-1),
        Request::Set(33),
        Request::Toggle(0),
        Request::Toggle(33),
    ] {
        assert!(!request.is_valid());
        assert!(!good.can_apply(request));
    }
    for request in [
        Request::Set(0),
        Request::Toggle(5),
        Request::Increase,
        Request::Decrease,
    ] {
        assert!(good.can_apply(request));
        assert!(
            !Config {
                disabled: true,
                ..good.clone()
            }
            .can_apply(request)
        );
        assert!(
            !Config {
                read_only: true,
                ..good.clone()
            }
            .can_apply(request)
        );
    }
    assert!(!good.can_apply(Request::Set(6)));
}

#[test]
fn appearance_has_independent_bytes_and_strict_rgba_bounds() {
    use gpuio_protocol::rating::Appearance;
    let node = NodeId::from_parts(0, 1).unwrap();
    let transaction = |appearances: Vec<Option<Appearance>>| {
        Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: appearances
                .into_iter()
                .map(|a| Op::SetRatingAppearance(node, a))
                .collect(),
        })
    };
    let message = transaction(vec![
        Some(Appearance {
            active: Some(0x11223344),
            inactive: Some(0xffffffff),
        }),
        Some(Appearance {
            active: None,
            inactive: Some(0),
        }),
        Some(Appearance {
            active: None,
            inactive: None,
        }),
        None,
    ]);
    let bytes = encode(&message);
    assert_eq!(
        hex(&bytes),
        include_str!("../../../test/fixtures/rating-appearance.hex").trim()
    );
    assert_eq!(decode(&bytes), Ok(message));
    for end in 0..bytes.len() {
        assert!(decode(&bytes[..end]).is_err());
    }
    let mut extra = bytes;
    extra.push(0);
    assert!(decode(&extra).is_err());
    for invalid in [-1, 0x100000000, i64::MAX] {
        for appearance in [
            Appearance {
                active: Some(invalid),
                inactive: None,
            },
            Appearance {
                active: None,
                inactive: Some(invalid),
            },
        ] {
            assert!(!appearance.is_valid());
            assert!(decode(&encode(&transaction(vec![Some(appearance)]))).is_err());
        }
    }
    assert_eq!(
        hex(&encode(&Message::Hello(VERSION, CAP_RATING_APPEARANCE))),
        "0003fc0000000000000002"
    );
    assert_eq!(
        hex(&encode(&Message::Hello(VERSION, CAPABILITIES))),
        "0003fcffffffffffffff7f"
    );
}
