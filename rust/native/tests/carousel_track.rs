use gpuio_native::{mailbox::Mailbox, session::Session};
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    carousel::{self, Axis, Direction},
    carousel_track::{Config, Layout, Loop, Proposal, Request, Stops},
    v1::*,
};
fn n(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn w() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn h() -> HandlerId {
    HandlerId::from_parts(0, 1).unwrap()
}
fn config() -> Config {
    Config {
        carousel: carousel::Config {
            revision: 0,
            ids: vec!["a".into(), "b".into()],
            selected: Some(0),
            looping: false,
            disabled: false,
            axis: Axis::Horizontal,
            auto_advance_ms: Some(1000),
            direction: Direction::Direct,
        },
        lineage: 0,
    }
}
fn layout(epoch: i64) -> Request {
    Request::Layout(Layout {
        lineage: 0,
        epoch,
        stops: Some(Stops {
            canonical: vec![0, 1],
            looping: Loop::Finite,
        }),
    })
}
fn tx(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: w(),
        base,
        revision: base + 1,
        operations,
    }
}
fn mounted() -> Session {
    let mut s = Session::default();
    s.hello(VERSION, CAPABILITIES).unwrap();
    s.open(1, w(), "Track", 400., 300.).unwrap();
    s.apply(&tx(
        0,
        vec![
            Op::Create(n(0), Kind::CarouselTrack, "Cards".into(), Some(h())),
            Op::Create(n(1), Kind::Container, "".into(), None),
            Op::Create(n(2), Kind::Panel, "A".into(), None),
            Op::Create(n(3), Kind::Panel, "B".into(), None),
            Op::SetCarouselTrack(n(0), config()),
            Op::Splice(n(1), 0, 0, vec![n(2), n(3)]),
            Op::Splice(n(0), 0, 0, vec![n(1)]),
            Op::SetRoot(Some(n(0))),
        ],
    ))
    .unwrap();
    s
}
fn event(s: &Session, request: Request) -> Option<Event> {
    s.request_carousel_track(w(), n(0), h(), 1, request)
}

#[test]
fn track_admission_is_atomic_and_checks_unchanged_owners_on_track_only_updates() {
    let mut s = mounted();
    let retained = s.retained_bytes();
    let mut wrong_lineage = config();
    wrong_lineage.carousel.revision = 1;
    wrong_lineage.carousel.ids.reverse();
    for ops in [
        vec![Op::SetCarouselTrack(n(1), config())],
        vec![Op::SetCarouselTrack(n(0), wrong_lineage.clone())],
        vec![Op::Bind(n(0), None)],
        vec![Op::Bind(n(1), Some(h()))],
        vec![Op::Splice(n(1), 0, 1, vec![])],
        vec![Op::Splice(n(0), 0, 1, vec![])],
        vec![Op::SetText(n(0), "".into())],
        vec![Op::SetText(n(1), "unexpected implicit child".into())],
        vec![
            Op::Create(n(4), Kind::Text, "wrong item".into(), None),
            Op::Splice(n(1), 0, 1, vec![n(4)]),
        ],
    ] {
        assert_eq!(s.apply(&tx(1, ops)), Err(ErrorCode::InvalidTree));
        assert_eq!(s.tree(w()).unwrap().revision(), 1);
        assert_eq!(s.retained_bytes(), retained);
        assert_eq!(
            s.tree(w()).unwrap().get(n(1)).unwrap().children.as_ref(),
            &[n(2), n(3)]
        );
    }
    wrong_lineage.lineage = 1;
    s.apply(&tx(1, vec![Op::SetCarouselTrack(n(0), wrong_lineage)]))
        .unwrap();
    assert!(event(&s, layout(1)).is_none());
    s.apply(&tx(
        2,
        vec![
            Op::SetRoot(None),
            Op::Remove(n(2)),
            Op::Remove(n(3)),
            Op::Remove(n(1)),
            Op::Remove(n(0)),
        ],
    ))
    .unwrap();
    assert!(event(&s, Request::Next).is_none());
    assert_eq!(s.retained_bytes(), 0);
}

#[test]
fn layout_delivery_survives_disabled_policy_but_obeys_identity_lineage_and_shutdown() {
    let mut s = mounted();
    assert!(event(&s, layout(0)).is_some());
    assert!(event(&s, Request::Next).is_some());
    assert!(event(&s, Request::Select("absent".into())).is_none());
    for (window, node, handler, revision) in [
        (WindowId::from_parts(0, 2).unwrap(), n(0), h(), 1),
        (w(), NodeId::from_parts(0, 2).unwrap(), h(), 1),
        (w(), n(0), HandlerId::from_parts(0, 2).unwrap(), 1),
        (w(), n(0), h(), -1),
        (w(), n(0), h(), 2),
    ] {
        assert!(
            s.request_carousel_track(window, node, handler, revision, layout(0))
                .is_none()
        );
    }
    let mut bad = match layout(0) {
        Request::Layout(l) => l,
        _ => unreachable!(),
    };
    bad.stops.as_mut().unwrap().canonical.pop();
    assert!(event(&s, Request::Layout(bad)).is_none());
    let proposal = Proposal {
        revision: 0,
        geometry_epoch: 0,
        from: "a".into(),
        target: "b".into(),
    };
    assert!(event(&s, Request::AutoNext(proposal.clone())).is_some());
    let mut disabled = config();
    disabled.carousel.revision = 1;
    disabled.carousel.disabled = true;
    s.apply(&tx(1, vec![Op::SetCarouselTrack(n(0), disabled)]))
        .unwrap();
    assert!(event(&s, layout(1)).is_some());
    assert!(event(&s, Request::Next).is_none());
    assert!(event(&s, Request::AutoNext(proposal)).is_none());
    let next = HandlerId::from_parts(0, 2).unwrap();
    s.apply(&tx(2, vec![Op::Bind(n(0), Some(next))])).unwrap();
    assert!(event(&s, layout(2)).is_none());
    assert!(
        s.request_carousel_track(w(), n(0), next, 3, layout(0))
            .is_some()
    );
    s.overload(w());
    assert!(
        s.request_carousel_track(w(), n(0), next, 3, layout(1))
            .is_none()
    );
    s.shutdown();
    assert!(
        s.request_carousel_track(w(), n(0), next, 3, layout(2))
            .is_none()
    );
}

#[test]
fn layout_and_intent_mailbox_order_is_lossless_bounded_and_window_accounted() {
    let s = mounted();
    let mut m = Mailbox::default();
    let events = [
        event(&s, layout(1)).unwrap(),
        event(&s, Request::Next).unwrap(),
        event(&s, layout(2)).unwrap(),
    ];
    for e in &events {
        m.input(e.clone()).unwrap();
    }
    assert!(m.has_window_output(0));
    assert_eq!(m.drain(1), vec![events[0].clone()]);
    assert_eq!(m.drain(8), events[1..]);
    assert!(!m.has_window_output(0));
    for _ in 0..gpuio_native::mailbox::MAX_INPUT_EVENTS {
        m.input(events[0].clone()).unwrap();
    }
    assert!(m.input(events[1].clone()).is_err());
    m.close();
    assert!(m.input(events[0].clone()).is_err());
}

#[test]
fn motion_presentation_is_atomic_typed_and_does_not_advance_selection_revision() {
    use gpuio_protocol::{animation::Easing, carousel_track::Motion};
    let mut s = mounted();
    let initial = s.retained_bytes();
    let valid = Motion {
        duration_ms: 200,
        easing: Easing::EaseOut,
    };
    for (node, value) in [
        (n(1), Some(valid.clone())),
        (n(1), None),
        (
            n(0),
            Some(Motion {
                duration_ms: 0,
                ..valid.clone()
            }),
        ),
        (
            n(0),
            Some(Motion {
                duration_ms: 10001,
                ..valid.clone()
            }),
        ),
    ] {
        assert_eq!(
            s.apply(&tx(1, vec![Op::SetCarouselTrackMotion(node, value)])),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(s.tree(w()).unwrap().revision(), 1);
        assert_eq!(s.retained_bytes(), initial);
    }
    s.apply(&tx(
        1,
        vec![Op::SetCarouselTrackMotion(n(0), Some(valid.clone()))],
    ))
    .unwrap();
    let node = s.tree(w()).unwrap().get(n(0)).unwrap();
    assert_eq!(node.carousel_track_motion, Some(valid.clone()));
    assert_eq!(node.carousel_track.as_deref(), Some(&config()));
    s.apply(&tx(2, vec![Op::SetCarouselTrackMotion(n(0), None)]))
        .unwrap();
    assert_eq!(
        s.tree(w())
            .unwrap()
            .get(n(0))
            .unwrap()
            .carousel_track_motion,
        None
    );
    assert_eq!(s.retained_bytes(), initial);
}

#[test]
fn control_group_is_explicit_and_rechecks_shape_after_child_only_changes() {
    let mut s = mounted();
    s.apply(&tx(
        1,
        vec![
            Op::Create(n(4), Kind::CarouselTrackGroup, String::new(), None),
            Op::Create(n(5), Kind::Container, String::new(), None),
            Op::Create(n(6), Kind::Button, "Next".into(), Some(h())),
            Op::SetControl(n(6), Control::Button(false)),
            Op::Splice(n(5), 0, 0, vec![n(6)]),
            Op::Splice(n(4), 0, 0, vec![n(0), n(5)]),
            Op::SetRoot(Some(n(4))),
        ],
    ))
    .unwrap();
    let retained = s.retained_bytes();
    for operations in [
        vec![Op::SetText(n(4), "implicit content".into())],
        vec![Op::Bind(n(4), Some(h()))],
        vec![Op::SetText(n(5), "implicit content".into())],
        vec![Op::Bind(n(5), Some(h()))],
        vec![Op::Splice(n(4), 0, 2, vec![n(5), n(0)])],
        vec![Op::Splice(n(4), 0, 1, vec![])],
        vec![
            Op::Create(n(7), Kind::Text, "not a control".into(), None),
            Op::Splice(n(5), 0, 0, vec![n(7)]),
        ],
    ] {
        assert!(s.apply(&tx(2, operations)).is_err());
        assert_eq!(s.tree(w()).unwrap().revision(), 2);
        assert_eq!(s.retained_bytes(), retained);
    }
    // A group can have no visible controls without remounting its viewport.
    s.apply(&tx(
        2,
        vec![
            Op::Splice(n(4), 1, 1, vec![]),
            Op::Remove(n(6)),
            Op::Remove(n(5)),
        ],
    ))
    .unwrap();
}
