use gpuio_native::{reveal_motion::RESERVED_BYTES, session::Session};
use gpuio_protocol::{NodeId, WindowId, animation::Spring, reveal::Config, v1::*};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn config() -> Config {
    Config {
        expanded: true,
        retain: true,
        spring: Spring {
            stiffness: 400.,
            damping: 40.,
            mass: 1.,
            epsilon: 0.1,
            max_duration_ms: 2000,
        },
    }
}
fn apply(
    session: &mut Session,
    operations: Vec<Op>,
) -> Result<gpuio_native::tree::Applied, ErrorCode> {
    let base = session.tree(window()).unwrap().revision();
    session.apply(&Transaction {
        window: window(),
        base,
        revision: base + 1,
        operations,
    })
}
#[test]
fn admission_is_atomic_and_accounts_for_the_native_owner() {
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window(), "Reveal", 400., 300.).unwrap();
    apply(
        &mut session,
        vec![
            Op::Create(id(0), Kind::Panel, "Details".into(), None),
            Op::Create(id(1), Kind::Text, "body".into(), None),
            Op::Splice(id(0), 0, 0, vec![id(1)]),
            Op::SetRoot(Some(id(0))),
        ],
    )
    .unwrap();
    let baseline = session.retained_bytes();
    apply(&mut session, vec![Op::SetReveal(id(0), Some(config()))]).unwrap();
    let reserved = baseline + RESERVED_BYTES + std::mem::size_of::<Config>();
    assert_eq!(session.retained_bytes(), reserved);
    let before = session.tree(window()).unwrap().revision();
    for operations in [
        vec![Op::SetReveal(id(1), Some(config()))],
        vec![Op::SetReveal(
            id(0),
            Some(Config {
                spring: Spring {
                    mass: 0.,
                    ..config().spring
                },
                ..config()
            }),
        )],
        // Closing must hide the semantic tree, regardless of operation order.
        vec![Op::SetReveal(
            id(0),
            Some(Config {
                expanded: false,
                ..config()
            }),
        )],
        // Unmount may not retain descendants behind an outgoing animation.
        vec![
            Op::SetStyle(id(0), vec![Style::Fields(vec![Field::Display(3)])]),
            Op::SetReveal(
                id(0),
                Some(Config {
                    expanded: false,
                    retain: false,
                    ..config()
                }),
            ),
        ],
    ] {
        assert!(apply(&mut session, operations).is_err());
        assert_eq!(session.tree(window()).unwrap().revision(), before);
        assert_eq!(
            session.tree(window()).unwrap().get(id(0)).unwrap().reveal,
            Some(config())
        );
        assert_eq!(session.retained_bytes(), reserved);
    }
    apply(
        &mut session,
        vec![
            Op::SetReveal(
                id(0),
                Some(Config {
                    expanded: false,
                    ..config()
                }),
            ),
            Op::SetStyle(id(0), vec![Style::Fields(vec![Field::Display(3)])]),
        ],
    )
    .unwrap();
    let closed_revision = session.tree(window()).unwrap().revision();
    assert!(apply(&mut session, vec![Op::SetStyle(id(0), vec![])]).is_err());
    assert_eq!(session.tree(window()).unwrap().revision(), closed_revision);
    apply(
        &mut session,
        vec![Op::SetReveal(id(0), None), Op::SetStyle(id(0), vec![])],
    )
    .unwrap();
    assert_eq!(session.retained_bytes(), baseline);
}
