use binprot::BinProtWrite;
use gpuio_protocol::{
    carousel::{self, Axis, Direction},
    carousel_track::{Config, Layout, Loop, Proposal, Request, Stops},
    decode_carousel_track_config, decode_carousel_track_request,
};
fn config() -> Config {
    Config {
        carousel: carousel::Config {
            revision: 7,
            ids: vec!["a".into(), "β".into(), "c".into()],
            selected: Some(1),
            looping: true,
            disabled: false,
            axis: Axis::Vertical,
            auto_advance_ms: Some(5000),
            direction: Direction::Next,
        },
        lineage: 3,
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
fn layout() -> Layout {
    Layout {
        lineage: 3,
        epoch: 9,
        stops: Some(Stops {
            canonical: vec![0, 1, 1],
            looping: Loop::Jump,
        }),
    }
}
#[test]
fn independent_payload_vectors_roundtrip_with_strict_truncation_and_trailing_checks() {
    let encoded = bytes(&config());
    assert_eq!(
        hex(&encoded),
        include_str!("../../../test/fixtures/carousel-track-config.hex").trim()
    );
    assert_eq!(decode_carousel_track_config(&encoded), Ok(config()));
    for end in 0..encoded.len() {
        assert!(decode_carousel_track_config(&encoded[..end]).is_err());
    }
    let mut trailing = encoded;
    trailing.push(0);
    assert!(decode_carousel_track_config(&trailing).is_err());
    for (request, fixture) in [
        (
            Request::Layout(layout()),
            include_str!("../../../test/fixtures/carousel-track-layout.hex"),
        ),
        (
            Request::Layout(Layout {
                lineage: 3,
                epoch: 10,
                stops: None,
            }),
            include_str!("../../../test/fixtures/carousel-track-unavailable.hex"),
        ),
        (
            Request::AutoNext(Proposal {
                revision: 7,
                geometry_epoch: 9,
                from: "β".into(),
                target: "c".into(),
            }),
            include_str!("../../../test/fixtures/carousel-track-auto.hex"),
        ),
    ] {
        let encoded = bytes(&request);
        assert_eq!(hex(&encoded), fixture.trim());
        assert_eq!(decode_carousel_track_request(&encoded), Ok(request));
        for end in 0..encoded.len() {
            assert!(decode_carousel_track_request(&encoded[..end]).is_err());
        }
        let mut trailing = encoded;
        trailing.push(0);
        assert!(decode_carousel_track_request(&trailing).is_err());
    }
}
#[test]
fn canonical_stops_are_bounded_ordered_groups_with_valid_loop_modes() {
    for canonical in [
        vec![1],
        vec![-1],
        vec![0, 2],
        vec![0, 0, 1],
        vec![0, 1, 0],
        vec![0; 129],
    ] {
        let mut layout = layout();
        layout.stops.as_mut().unwrap().canonical = canonical;
        assert!(!layout.is_valid());
        assert!(decode_carousel_track_request(&bytes(&Request::Layout(layout))).is_err());
    }
    for canonical in [
        vec![],
        vec![0],
        vec![0, 0],
        vec![0, 1],
        vec![0, 0, 2, 2],
        (0..128).collect(),
    ] {
        let stops = Stops {
            canonical,
            looping: Loop::Finite,
        };
        assert!(stops.is_valid());
    }
    assert!(
        !Stops {
            canonical: vec![],
            looping: Loop::Jump
        }
        .is_valid()
    );
    assert!(
        !Stops {
            canonical: vec![0, 0],
            looping: Loop::Continuous
        }
        .is_valid()
    );
    let mut encoded = bytes(&Request::Layout(layout()));
    *encoded.last_mut().unwrap() = 3;
    assert!(decode_carousel_track_request(&encoded).is_err());
    assert!(decode_carousel_track_request(&[7]).is_err());
    assert!(decode_carousel_track_request(&[4, 1, 0xff]).is_err());
    for request in [
        Request::Previous,
        Request::Next,
        Request::First,
        Request::Last,
        Request::Select("名".into()),
    ] {
        assert_eq!(decode_carousel_track_request(&bytes(&request)), Ok(request));
    }
}
#[test]
fn layout_and_config_transitions_require_the_current_collection_lineage() {
    let original = config();
    assert!(original.accepts_layout(&layout()));
    let mut invalid = layout();
    invalid.lineage += 1;
    assert!(!original.accepts_layout(&invalid));
    invalid = layout();
    invalid.stops.as_mut().unwrap().canonical.pop();
    assert!(!original.accepts_layout(&invalid));
    let mut disabled = original.clone();
    disabled.carousel.disabled = true;
    assert!(
        disabled.accepts_layout(&layout()),
        "geometry remains observable while disabled"
    );
    let mut finite = original.clone();
    finite.carousel.looping = false;
    assert!(!finite.accepts_layout(&layout()));
    invalid = layout();
    invalid.stops.as_mut().unwrap().looping = Loop::Finite;
    assert!(finite.accepts_layout(&invalid));
    for mutate in 0..3 {
        let mut next = original.clone();
        next.carousel.revision += 1;
        match mutate {
            0 => next.carousel.ids.swap(0, 1),
            1 => next.carousel.axis = Axis::Horizontal,
            _ => next.carousel.looping = false,
        }
        assert!(!next.can_replace(&original));
        next.lineage += 1;
        assert!(next.can_replace(&original));
    }
    let mut impossible = original.clone();
    impossible.lineage = impossible.carousel.revision + 1;
    assert!(decode_carousel_track_config(&bytes(&impossible)).is_err());
    let mut backwards = original.clone();
    backwards.lineage -= 1;
    backwards.carousel.revision += 1;
    assert!(!backwards.can_replace(&original));
}
#[test]
fn invalid_proposals_negative_epochs_and_oversized_payloads_are_rejected() {
    for proposal in [
        Proposal {
            revision: -1,
            geometry_epoch: 0,
            from: "a".into(),
            target: "b".into(),
        },
        Proposal {
            revision: 0,
            geometry_epoch: -1,
            from: "a".into(),
            target: "b".into(),
        },
        Proposal {
            revision: 0,
            geometry_epoch: 0,
            from: "a".into(),
            target: "a".into(),
        },
        Proposal {
            revision: 0,
            geometry_epoch: 0,
            from: "".into(),
            target: "b".into(),
        },
        Proposal {
            revision: 0,
            geometry_epoch: 0,
            from: "a".repeat(257),
            target: "b".into(),
        },
    ] {
        assert!(decode_carousel_track_request(&bytes(&Request::AutoNext(proposal))).is_err());
    }
    for (lineage, epoch) in [(-1, 0), (0, -1)] {
        assert!(
            decode_carousel_track_request(&bytes(&Request::Layout(Layout {
                lineage,
                epoch,
                stops: None
            })))
            .is_err()
        );
    }
    let maximum = Request::Layout(Layout {
        lineage: i64::MAX,
        epoch: i64::MAX,
        stops: None,
    });
    assert_eq!(decode_carousel_track_request(&bytes(&maximum)), Ok(maximum));
    assert!(decode_carousel_track_request(&vec![0; 4096]).is_err());
}

#[test]
fn checked_track_operation_and_event_envelopes_match_independent_vectors() {
    use gpuio_protocol::{HandlerId, NodeId, WindowId, decode, v1::*};
    let w = WindowId::from_parts(0, 1).unwrap();
    let n = NodeId::from_parts(1, 2).unwrap();
    let h = HandlerId::from_parts(3, 4).unwrap();
    let message = Message::Apply(Transaction {
        window: w,
        base: 0,
        revision: 1,
        operations: vec![Op::SetCarouselTrack(n, config())],
    });
    let encoded = bytes(&message);
    assert_eq!(
        hex(&encoded),
        include_str!("../../../test/fixtures/carousel-track-operation.hex").trim()
    );
    assert_eq!(decode(&encoded), Ok(message));
    for end in 0..encoded.len() {
        assert!(decode(&encoded[..end]).is_err());
    }
    let mut trailing = encoded;
    trailing.push(0);
    assert!(decode(&trailing).is_err());
    let events = vec![
        Event::CarouselTrackRequested(w, n, h, 5, Request::Layout(layout())),
        Event::CarouselTrackRequested(
            w,
            n,
            h,
            5,
            Request::AutoNext(Proposal {
                revision: 7,
                geometry_epoch: 9,
                from: "β".into(),
                target: "c".into(),
            }),
        ),
        Event::CarouselTrackRequested(w, n, h, 5, Request::Next),
    ];
    assert_eq!(
        hex(&bytes(&events)),
        include_str!("../../../test/fixtures/carousel-track-events.hex").trim()
    );
    assert_eq!(bytes(&Kind::CarouselTrack), vec![54]);
    assert_eq!(bytes(&Kind::CarouselTrackGroup), vec![55]);
    let group = Message::Apply(Transaction {
        window: w,
        base: 0,
        revision: 1,
        operations: vec![Op::Create(n, Kind::CarouselTrackGroup, String::new(), None)],
    });
    assert_eq!(decode(&bytes(&group)).unwrap(), group);
    let invalid = Config {
        lineage: 8,
        ..config()
    };
    let malformed = Message::Apply(Transaction {
        window: w,
        base: 0,
        revision: 1,
        operations: vec![Op::SetCarouselTrack(n, invalid)],
    });
    assert!(decode(&bytes(&malformed)).is_err());
}

#[test]
fn motion_operation_is_checked_and_matches_independent_bytes() {
    use gpuio_protocol::{
        NodeId, WindowId, animation::Easing, carousel_track::Motion, decode, v1::*,
    };
    let node = NodeId::from_parts(0, 1).unwrap();
    let message = |motion| {
        Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![
                Op::SetCarouselTrackMotion(node, Some(motion)),
                Op::SetCarouselTrackMotion(node, None),
            ],
        })
    };
    let valid = message(Motion {
        duration_ms: 200,
        easing: Easing::EaseOut,
    });
    let encoded = bytes(&valid);
    assert_eq!(
        hex(&encoded),
        include_str!("../../../test/fixtures/carousel-track-motion.hex").trim()
    );
    assert_eq!(decode(&encoded), Ok(valid));
    for end in 0..encoded.len() {
        assert!(decode(&encoded[..end]).is_err());
    }
    let mut trailing = encoded;
    trailing.push(0);
    assert!(decode(&trailing).is_err());
    for duration_ms in [-1, 0, 10001] {
        assert!(
            decode(&bytes(&message(Motion {
                duration_ms,
                easing: Easing::Linear
            })))
            .is_err()
        );
    }
    for easing in [
        Easing::CubicBezier(-0.1, 0., 1., 1.),
        Easing::CubicBezier(0., f64::NAN, 1., 1.),
        Easing::CubicBezier(0., 0., f64::INFINITY, 1.),
    ] {
        assert!(
            decode(&bytes(&message(Motion {
                duration_ms: 1,
                easing
            })))
            .is_err()
        );
    }
    for duration_ms in [1, 10000] {
        let m = message(Motion {
            duration_ms,
            easing: Easing::Linear,
        });
        assert_eq!(decode(&bytes(&m)), Ok(m));
    }
}
