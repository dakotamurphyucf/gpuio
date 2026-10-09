use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, button, v1::*};
fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn handler(slot: i64) -> HandlerId {
    HandlerId::from_parts(slot, 1).unwrap()
}
fn apply(session: &mut Session, operations: Vec<Op>) -> Result<(), ErrorCode> {
    let base = session.tree(window()).unwrap().revision();
    session
        .apply(&Transaction {
            window: window(),
            base,
            revision: base + 1,
            operations,
        })
        .map(|_| ())
}
fn presentation(loading: bool) -> Op {
    Op::SetButtonPresentation(
        node(0),
        Some(button::Config {
            policy: button::Policy {
                loading,
                focus: button::Focus::default(),
            },
            content: button::Content::Rich,
        }),
    )
}
#[test]
fn observer_admission_rolls_back_and_keeps_action_identity_independent() {
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window(), "Hover", 400., 200.).unwrap();
    apply(
        &mut session,
        vec![
            Op::Create(node(0), Kind::Button, "Action".into(), Some(handler(0))),
            Op::Create(node(1), Kind::Text, "Unobserved".into(), None),
            presentation(false),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    )
    .unwrap();
    assert!(
        session
            .hover_changed(window(), node(0), handler(0), 1, true)
            .is_none()
    );
    for observer in [None, Some(handler(1))] {
        assert_eq!(
            apply(
                &mut session,
                vec![
                    Op::SetHoverObserver(node(0), Some(handler(1))),
                    Op::SetHoverObserver(node(1), observer),
                ]
            ),
            Err(ErrorCode::InvalidTree)
        );
        assert!(
            session
                .tree(window())
                .unwrap()
                .get(node(0))
                .unwrap()
                .hover_handler
                .is_none()
        );
        assert_eq!(session.tree(window()).unwrap().revision(), 1);
    }
    apply(
        &mut session,
        vec![Op::SetHoverObserver(node(0), Some(handler(1)))],
    )
    .unwrap();
    assert!(session.press(window(), node(0), handler(0), 2).is_some());
    assert!(session.press(window(), node(0), handler(1), 2).is_none());
    assert!(
        session
            .hover_changed(window(), node(0), handler(0), 2, true)
            .is_none()
    );
    assert!(
        session
            .hover_changed(window(), node(0), handler(1), 2, true)
            .is_some()
    );
    for revision in [-1, 3] {
        assert!(
            session
                .hover_changed(window(), node(0), handler(1), revision, true)
                .is_none()
        );
    }
    apply(&mut session, vec![presentation(true)]).unwrap();
    assert!(
        session
            .hover_changed(window(), node(0), handler(1), 3, true)
            .is_none()
    );
    assert!(
        session
            .hover_changed(window(), node(0), handler(1), 3, false)
            .is_some()
    );
    apply(
        &mut session,
        vec![
            presentation(false),
            Op::SetControl(node(0), Control::Button(true)),
        ],
    )
    .unwrap();
    assert!(
        session
            .hover_changed(window(), node(0), handler(1), 4, true)
            .is_none()
    );
    assert!(
        session
            .hover_changed(window(), node(0), handler(1), 4, false)
            .is_some()
    );
    apply(&mut session, vec![Op::SetHoverObserver(node(0), None)]).unwrap();
    assert!(
        session
            .hover_changed(window(), node(0), handler(1), 5, false)
            .is_none()
    );
    session.close(window()).unwrap();
    assert!(
        session
            .hover_changed(window(), node(0), handler(1), 5, false)
            .is_none()
    );
}
