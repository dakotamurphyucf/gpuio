use gpuio_native::{session::Session, tree::Tree};
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};

fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn handler() -> HandlerId {
    HandlerId::from_parts(0, 1).unwrap()
}
fn transaction(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: window(),
        base,
        revision: base + 1,
        operations,
    }
}
fn initial(control: Control) -> Transaction {
    transaction(
        0,
        vec![
            Op::Create(node(), control.kind(), "Stream".into(), Some(handler())),
            Op::SetControl(node(), control),
            Op::SetRoot(Some(node())),
        ],
    )
}

#[test]
fn controls_validate_configuration_atomically_and_do_not_retain_old_values() {
    let mut tree = Tree::new(window());
    tree.apply(&initial(Control::Checkbox(CheckState::Unchecked, false)))
        .unwrap();
    let bytes = tree.retained_bytes();
    let invalid = transaction(
        1,
        vec![
            Op::SetText(node(), "must roll back".into()),
            Op::SetControl(node(), Control::Switch(true, false)),
        ],
    );
    assert_eq!(tree.apply(&invalid), Err(ErrorCode::InvalidTree));
    assert_eq!(tree.get(node()).unwrap().text.as_ref(), "Stream");
    assert_eq!(tree.revision(), 1);
    for revision in 1..100 {
        let state = if revision % 2 == 0 {
            CheckState::Checked
        } else {
            CheckState::Indeterminate
        };
        tree.apply(&transaction(
            revision,
            vec![Op::SetControl(node(), Control::Checkbox(state, false))],
        ))
        .unwrap();
        assert_eq!(tree.retained_bytes(), bytes);
    }
    tree.apply(&transaction(
        100,
        vec![Op::Remove(node()), Op::SetRoot(None)],
    ))
    .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
}

#[test]
fn native_activation_checks_current_disabled_state_even_before_the_next_paint() {
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window(), "controls", 100., 100.).unwrap();
    session
        .apply(&initial(Control::Switch(false, false)))
        .unwrap();
    for _ in 0..2 {
        assert_eq!(
            session.press(window(), node(), handler(), 1),
            Some(Event::Press(window(), node(), handler(), 1))
        );
    }
    // A malicious/low-level client can retain its handler while disabling.
    // Native validation still rejects callbacks from the previous painted frame.
    session
        .apply(&transaction(
            1,
            vec![Op::SetControl(node(), Control::Switch(false, true))],
        ))
        .unwrap();
    assert_eq!(session.press(window(), node(), handler(), 1), None);
    assert_eq!(session.press(window(), node(), handler(), 2), None);
}

#[test]
fn missing_config_and_enabled_controls_without_handlers_are_rejected() {
    let mut tree = Tree::new(window());
    let create = transaction(
        0,
        vec![
            Op::Create(node(), Kind::Checkbox, "check".into(), Some(handler())),
            Op::SetRoot(Some(node())),
        ],
    );
    assert_eq!(tree.apply(&create), Err(ErrorCode::InvalidTree));
    assert!(tree.is_empty());
    tree.apply(&initial(Control::Checkbox(CheckState::Checked, false)))
        .unwrap();
    assert_eq!(
        tree.apply(&transaction(1, vec![Op::Bind(node(), None)])),
        Err(ErrorCode::InvalidTree)
    );
    tree.apply(&transaction(
        1,
        vec![
            Op::Bind(node(), None),
            Op::SetControl(node(), Control::Checkbox(CheckState::Checked, true)),
        ],
    ))
    .unwrap();
}
