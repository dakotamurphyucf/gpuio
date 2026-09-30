use gpuio_native::{mailbox::Mailbox, session::Session};
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    input::{self, Kind as K},
    v1::*,
};
fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn handler(generation: i64) -> HandlerId {
    HandlerId::from_parts(0, generation).unwrap()
}
fn config() -> input::Config {
    input::Config {
        label: "Observe".into(),
        disabled: false,
        focus: input::Focus::Tab,
        subscriptions: vec![
            K::MouseDown,
            K::MouseMove,
            K::KeyDown,
            K::Focus,
            K::Blur,
            K::Scroll,
        ]
        .into_iter()
        .map(|kind| input::Subscription {
            kind,
            phase: input::Phase::Bubble,
            policy: input::Policy::Observe,
        })
        .collect(),
    }
}
fn apply(s: &mut Session, operations: Vec<Op>) -> Result<(), ErrorCode> {
    let base = s.tree(window()).unwrap().revision();
    s.apply(&Transaction {
        window: window(),
        base,
        revision: base + 1,
        operations,
    })
    .map(|_| ())
}
fn setup() -> Session {
    let mut s = Session::default();
    s.hello(VERSION, CAPABILITIES).unwrap();
    s.open(1, window(), "input", 300., 200.).unwrap();
    apply(
        &mut s,
        vec![
            Op::Create(node(), Kind::InputRegion, "".into(), Some(handler(1))),
            Op::SetInputRegion(node(), config()),
            Op::SetRoot(Some(node())),
        ],
    )
    .unwrap();
    s
}
#[test]
fn admission_binding_retirement_and_atomic_rejection() {
    assert_eq!(CAPABILITIES & CAP_INPUT_REGIONS, 1_i64 << 45);
    let mut s = setup();
    assert!(s.press(window(), node(), handler(1), 1).is_none());
    assert!(
        s.input_observed(window(), node(), handler(1), 1, input::Event::Focus)
            .is_some()
    );
    assert!(
        s.input_observed(window(), node(), handler(1), 1, input::Event::MouseEnter)
            .is_none()
    );
    assert!(
        s.input_observed(window(), node(), handler(1), 2, input::Event::Focus)
            .is_none()
    );
    assert!(
        s.input_observed(window(), node(), handler(2), 1, input::Event::Focus)
            .is_none()
    );
    let bytes = s.retained_bytes();
    let mut changed = config();
    changed.label = "New label".into();
    assert_eq!(
        apply(&mut s, vec![Op::SetInputRegion(node(), changed.clone())]),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(s.tree(window()).unwrap().revision(), 1);
    assert_eq!(s.retained_bytes(), bytes);
    let mut malformed = changed.clone();
    malformed.subscriptions.reverse();
    assert_eq!(
        apply(
            &mut s,
            vec![
                Op::Bind(node(), Some(handler(2))),
                Op::SetInputRegion(node(), malformed)
            ]
        ),
        Err(ErrorCode::InvalidTree)
    );
    assert!(
        s.input_observed(window(), node(), handler(1), 1, input::Event::Focus)
            .is_some()
    );
    apply(
        &mut s,
        vec![
            Op::Bind(node(), Some(handler(2))),
            Op::SetInputRegion(node(), changed.clone()),
        ],
    )
    .unwrap();
    assert!(
        s.input_observed(window(), node(), handler(1), 1, input::Event::Focus)
            .is_none()
    );
    assert!(
        s.input_observed(window(), node(), handler(2), 2, input::Event::Focus)
            .is_some()
    );
    changed.disabled = true;
    apply(
        &mut s,
        vec![
            Op::Bind(node(), Some(handler(3))),
            Op::SetInputRegion(node(), changed),
        ],
    )
    .unwrap();
    assert!(
        s.input_observed(window(), node(), handler(3), 3, input::Event::Focus)
            .is_none()
    );
    s.close(window()).unwrap();
    assert_eq!(s.retained_bytes(), 0);
    assert!(
        s.input_observed(window(), node(), handler(3), 3, input::Event::Focus)
            .is_none()
    );
}
fn motion(x: f64) -> input::Motion {
    input::Motion {
        location: input::Location {
            window: input::Position { x, y: 0. },
            local: input::Position { x, y: 0. },
            modifiers: Default::default(),
        },
        pressed_button: None,
    }
}
fn observed(event: input::Event) -> Event {
    Event::InputObserved(window(), node(), handler(1), 1, event)
}
#[test]
fn only_contiguous_motion_with_unchanged_route_modifiers_and_button_coalesces() {
    let mut q = Mailbox::default();
    for n in 0..10_000 {
        q.input(observed(input::Event::MouseMove(motion(n as f64))))
            .unwrap();
    }
    let mut modified = motion(10_000.);
    modified.location.modifiers.shift = true;
    q.input(observed(input::Event::MouseMove(modified)))
        .unwrap();
    modified.pressed_button = Some(PointerButton::Left);
    q.input(observed(input::Event::MouseMove(modified)))
        .unwrap();
    q.input(observed(input::Event::Focus)).unwrap();
    q.input(observed(input::Event::MouseMove(modified)))
        .unwrap();
    q.input(Event::InputObserved(
        window(),
        node(),
        handler(2),
        1,
        input::Event::MouseMove(modified),
    ))
    .unwrap();
    q.input(Event::InputObserved(
        window(),
        node(),
        handler(2),
        2,
        input::Event::MouseMove(modified),
    ))
    .unwrap();
    let batch = q.drain(128);
    assert_eq!(batch.len(), 7);
    assert_eq!(batch[0], observed(input::Event::MouseMove(motion(9999.))));
    assert_eq!(batch[3], observed(input::Event::Focus));
    assert!(q.drain(128).is_empty());
}
#[test]
fn wheel_deltas_and_edges_are_ordered_and_bounded() {
    let mut q = Mailbox::default();
    let wheel = input::Event::Scroll(input::Scroll {
        location: motion(0.).location,
        delta: input::Delta::Lines(input::Position { x: 0., y: 1. }),
        phase: input::TouchPhase::Moved,
    });
    for _ in 0..128 {
        q.input(observed(wheel.clone())).unwrap();
    }
    assert!(q.input(observed(wheel.clone())).is_err());
    assert_eq!(q.drain(128), vec![observed(wheel); 128]);
    assert!(q.drain(128).is_empty());
}

#[test]
fn pointer_occlusion_is_explicit_base_only_and_rejects_invalid_updates_atomically() {
    assert_eq!(CAPABILITIES & CAP_POINTER_OCCLUSION, 1_i64 << 46);
    let mut session = setup();
    for mode in 0..=2 {
        apply(
            &mut session,
            vec![Op::SetStyle(
                node(),
                vec![Style::Fields(vec![Field::PointerOcclusion(mode)])],
            )],
        )
        .unwrap();
    }
    let revision = session.tree(window()).unwrap().revision();
    let retained = session.retained_bytes();
    let mut invalid = vec![
        Style::Fields(vec![Field::PointerOcclusion(-1)]),
        Style::Fields(vec![Field::PointerOcclusion(3)]),
    ];
    invalid.extend((1..=7).map(|state| Style::State(state, vec![Field::PointerOcclusion(1)])));
    for style in invalid {
        assert!(apply(&mut session, vec![Op::SetStyle(node(), vec![style])]).is_err());
        assert_eq!(session.tree(window()).unwrap().revision(), revision);
        assert_eq!(session.retained_bytes(), retained);
    }
    apply(&mut session, vec![Op::SetStyle(node(), vec![])]).unwrap();
    assert!(
        session
            .tree(window())
            .unwrap()
            .get(node())
            .unwrap()
            .style
            .is_empty()
    );
}
