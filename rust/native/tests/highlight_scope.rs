use gpuio_native::{
    mailbox::{MAX_INPUT_EVENTS, Mailbox},
    session::Session,
};
use gpuio_protocol::{HandlerId, NodeId, WindowId, highlight::*, v1::*};
fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn handler(generation: i64) -> HandlerId {
    HandlerId::from_parts(0, generation).unwrap()
}
fn config() -> Config {
    Config(vec![Spec {
        query: Some(Query {
            text: "a".into(),
            case_sensitive: false,
            whole_word: false,
        }),
        ranges: vec![],
        appearance: Appearance {
            color: 1,
            active_color: 2,
            radius: 2.,
        },
        active_index: None,
        match_index_offset: 0,
    }])
}
fn observation() -> Observation {
    Observation {
        epoch: 1,
        state: State::Ready(vec![Count {
            total: 1,
            stored: 1,
        }]),
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
fn setup(observer: bool) -> Session {
    let mut s = Session::default();
    s.hello(VERSION, CAPABILITIES).unwrap();
    s.open(1, window(), "highlight", 300., 200.).unwrap();
    apply(
        &mut s,
        vec![
            Op::Create(
                node(),
                Kind::HighlightScope,
                "".into(),
                observer.then(|| handler(1)),
            ),
            Op::SetHighlightScope(node(), config()),
            Op::SetRoot(Some(node())),
        ],
    )
    .unwrap();
    s
}
#[test]
fn atomic_scope_validation_optional_observer_and_retirement() {
    let mut s = setup(true);
    assert!(s.press(window(), node(), handler(1), 1).is_none());
    assert!(
        s.highlight_observed(window(), node(), handler(1), 1, observation())
            .is_some()
    );
    assert!(
        s.highlight_observed(window(), node(), handler(1), 2, observation())
            .is_none()
    );
    assert!(
        s.highlight_observed(window(), node(), handler(2), 1, observation())
            .is_none()
    );
    assert!(
        s.highlight_observed(
            window(),
            node(),
            handler(1),
            1,
            Observation {
                epoch: 1,
                state: State::Ready(vec![])
            }
        )
        .is_none()
    );
    let retained = s.retained_bytes();
    let mut changed = config();
    changed.0[0].appearance.radius = 4.;
    assert_eq!(
        apply(&mut s, vec![Op::SetHighlightScope(node(), changed.clone())]),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(s.tree(window()).unwrap().revision(), 1);
    assert_eq!(s.retained_bytes(), retained);
    let mut invalid = changed.clone();
    invalid.0[0].active_index = Some(-1);
    assert_eq!(
        apply(
            &mut s,
            vec![
                Op::Bind(node(), Some(handler(2))),
                Op::SetHighlightScope(node(), invalid)
            ]
        ),
        Err(ErrorCode::InvalidTree)
    );
    assert!(
        s.highlight_observed(window(), node(), handler(1), 1, observation())
            .is_some()
    );
    apply(
        &mut s,
        vec![
            Op::Bind(node(), Some(handler(2))),
            Op::SetHighlightScope(node(), changed),
        ],
    )
    .unwrap();
    assert!(
        s.highlight_observed(window(), node(), handler(1), 1, observation())
            .is_none()
    );
    assert!(
        s.highlight_observed(window(), node(), handler(2), 2, observation())
            .is_some()
    );
    apply(
        &mut s,
        vec![
            Op::Bind(node(), None),
            Op::SetHighlightScope(node(), Config(vec![])),
        ],
    )
    .unwrap();
    assert!(
        s.highlight_observed(window(), node(), handler(2), 2, observation())
            .is_none()
    );
    apply(&mut s, vec![Op::SetHighlightScope(node(), config())]).unwrap(); // config changes without observers
    s.close(window()).unwrap();
    assert_eq!(s.retained_bytes(), 0);
    assert!(
        s.highlight_observed(window(), node(), handler(2), 2, observation())
            .is_none()
    );
    let unobserved = setup(false);
    assert!(
        unobserved
            .tree(window())
            .unwrap()
            .get(node())
            .unwrap()
            .handler
            .is_none()
    );
}
#[test]
fn empty_nested_declaration_is_distinct_from_missing_configuration() {
    let mut s = setup(false);
    let child = NodeId::from_parts(1, 1).unwrap();
    let before = s.retained_bytes();
    assert_eq!(
        apply(
            &mut s,
            vec![
                Op::Create(child, Kind::HighlightScope, "".into(), None),
                Op::Splice(node(), 0, 0, vec![child])
            ]
        ),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(s.retained_bytes(), before);
    apply(
        &mut s,
        vec![
            Op::Create(child, Kind::HighlightScope, "".into(), None),
            Op::SetHighlightScope(child, Config(vec![])),
            Op::Splice(node(), 0, 0, vec![child]),
        ],
    )
    .unwrap();
    assert!(
        s.tree(window())
            .unwrap()
            .get(child)
            .unwrap()
            .highlight_scope
            .as_ref()
            .unwrap()
            .0
            .is_empty()
    );
    assert_eq!(
        apply(&mut s, vec![Op::SetText(child, "not a label".into())]),
        Err(ErrorCode::InvalidTree)
    );
}
#[test]
fn bounded_observations_remain_ordered_and_participate_in_window_lifetime() {
    let mut q = Mailbox::default();
    for i in 1..=MAX_INPUT_EVENTS {
        q.input(Event::HighlightObserved(
            window(),
            node(),
            handler(1),
            1,
            Observation {
                epoch: i as i64,
                state: State::Pending,
            },
        ))
        .unwrap();
    }
    assert!(q.has_window_output(window().slot()));
    assert!(
        q.input(Event::HighlightObserved(
            window(),
            node(),
            handler(1),
            1,
            observation()
        ))
        .is_err()
    );
    let events = q.drain(MAX_INPUT_EVENTS);
    assert_eq!(events.len(), MAX_INPUT_EVENTS);
    for (i, event) in events.into_iter().enumerate() {
        match event {
            Event::HighlightObserved(_, _, _, _, o) => assert_eq!(o.epoch, i as i64 + 1),
            _ => panic!("wrong event"),
        }
    }
    assert!(!q.has_window_output(window().slot()));
}
