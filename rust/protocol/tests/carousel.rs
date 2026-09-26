use binprot::BinProtWrite;
use gpuio_protocol::{
    carousel::{Axis, Config, Direction, Request},
    decode_carousel_config, decode_carousel_request,
};
fn config() -> Config {
    Config {
        revision: 7,
        ids: vec!["a".into(), "β".into(), "c".into()],
        selected: Some(1),
        looping: true,
        disabled: false,
        axis: Axis::Vertical,
        auto_advance_ms: Some(5000),
        direction: Direction::Next,
    }
}
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
#[test]
fn independent_fixtures_and_malformed_inputs() {
    let config = config();
    let mut encoded = bytes(&config);
    assert_eq!(
        hex(&encoded),
        include_str!("../../../test/fixtures/carousel-config.hex").trim()
    );
    assert_eq!(decode_carousel_config(&encoded), Ok(config.clone()));
    for end in 0..encoded.len() {
        assert!(decode_carousel_config(&encoded[..end]).is_err());
    }
    for index in [11, 12, 13, 14] {
        let mut invalid = encoded.clone();
        invalid[index] = 2;
        assert!(decode_carousel_config(&invalid).is_err(), "field {index}");
    }
    let mut invalid = encoded.clone();
    *invalid.last_mut().unwrap() = 3;
    assert!(decode_carousel_config(&invalid).is_err());
    encoded.push(0);
    assert!(decode_carousel_config(&encoded).is_err());
    let request = Request::AutoNext {
        revision: 7,
        from: "β".into(),
        target: "c".into(),
    };
    let mut encoded = bytes(&request);
    assert_eq!(
        hex(&encoded),
        include_str!("../../../test/fixtures/carousel-request.hex").trim()
    );
    assert_eq!(decode_carousel_request(&encoded), Ok(request));
    for end in 0..encoded.len() {
        assert!(decode_carousel_request(&encoded[..end]).is_err());
    }
    encoded[0] = 6;
    assert!(decode_carousel_request(&encoded).is_err());
    for request in [
        Request::Select("".into()),
        Request::AutoNext {
            revision: -1,
            from: "a".into(),
            target: "b".into(),
        },
        Request::AutoNext {
            revision: 0,
            from: "a".into(),
            target: "a".into(),
        },
    ] {
        assert!(decode_carousel_request(&bytes(&request)).is_err());
    }
    for invalid in [
        Config {
            revision: -1,
            ..config.clone()
        },
        Config {
            ids: vec!["a".into(), "a".into()],
            ..config.clone()
        },
        Config {
            selected: None,
            ..config.clone()
        },
        Config {
            selected: Some(3),
            ..config.clone()
        },
        Config {
            auto_advance_ms: Some(999),
            ..config.clone()
        },
        Config {
            auto_advance_ms: Some(3_600_001),
            ..config
        },
    ] {
        assert!(decode_carousel_config(&bytes(&invalid)).is_err());
    }
}
#[test]
fn maximum_ids_are_bounded_before_allocating_and_fit_codec_limit() {
    let mut config = config();
    config.ids = (0..128)
        .map(|i| format!("{i:03}{}", "x".repeat(253)))
        .collect();
    config.selected = Some(127);
    assert_eq!(decode_carousel_config(&bytes(&config)), Ok(config.clone()));
    config.ids.push("extra".into());
    assert!(decode_carousel_config(&bytes(&config)).is_err());
    config.ids = vec!["x".repeat(257)];
    config.selected = Some(0);
    assert!(decode_carousel_config(&bytes(&config)).is_err());
    let max_request = Request::AutoNext {
        revision: i64::MAX,
        from: "x".repeat(256),
        target: "y".repeat(256),
    };
    assert_eq!(
        decode_carousel_request(&bytes(&max_request)),
        Ok(max_request)
    );
}
#[test]
fn relative_and_automatic_selection_resolve_without_native_mutation() {
    let mut config = config();
    assert_eq!(config.target(&Request::Previous), Some(0));
    assert_eq!(config.target(&Request::Next), Some(2));
    assert_eq!(config.target(&Request::Select("absent".into())), None);
    let request = config.automatic_request().unwrap();
    assert_eq!(config.target(&request), Some(2));
    config.revision += 1;
    assert_eq!(config.target(&request), None);
    config.selected = Some(2);
    assert_eq!(config.target(&Request::Next), Some(0));
    config.looping = false;
    assert_eq!(config.automatic_request(), None);
    config.disabled = true;
    assert_eq!(config.target(&Request::First), None);
    config.disabled = false;
    config.ids.truncate(1);
    config.selected = Some(0);
    config.looping = true;
    assert_eq!(config.automatic_request(), None);
    config.ids.clear();
    config.selected = None;
    assert_eq!(config.target(&Request::Previous), None);
}

#[test]
fn appended_transaction_and_event_tags_match_independent_fixtures() {
    use gpuio_protocol::{HandlerId, NodeId, WindowId, decode, v1::*};
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let message = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![Op::SetCarousel(node, config())],
    });
    let encoded = bytes(&message);
    assert_eq!(
        hex(&encoded),
        include_str!("../../../test/fixtures/carousel-transaction.hex").trim()
    );
    assert_eq!(decode(&encoded), Ok(message));
    for end in 0..encoded.len() {
        assert!(decode(&encoded[..end]).is_err());
    }
    let event = Event::CarouselRequested(
        window,
        node,
        HandlerId::from_parts(0, 1).unwrap(),
        1,
        Request::AutoNext {
            revision: 7,
            from: "β".into(),
            target: "c".into(),
        },
    );
    assert_eq!(
        hex(&bytes(&event)),
        include_str!("../../../test/fixtures/carousel-event.hex").trim()
    );
    assert_eq!(bytes(&Kind::Carousel), vec![47]);
}

#[test]
fn navigation_capability_uses_the_shared_64_bit_handshake() {
    use gpuio_protocol::{decode, v1::*};
    assert_eq!(CAPABILITIES & CAP_NAVIGATION_COMPONENTS, 1_i64 << 38);
    let hello = Message::Hello(VERSION, CAPABILITIES);
    let encoded = bytes(&hello);
    assert_eq!(hex(&encoded), "0001fcffffffffff000000");
    assert_eq!(decode(&encoded), Ok(hello));
}
