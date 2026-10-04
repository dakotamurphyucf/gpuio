use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
#[path = "../../protocol/tests/common/toast_fixture.rs"]
mod fixture;
fn w() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn n(i: i64) -> NodeId {
    NodeId::from_parts(i, 1).unwrap()
}
fn h() -> HandlerId {
    HandlerId::from_parts(0, 1).unwrap()
}
fn mounted() -> Session {
    let mut s = Session::default();
    s.hello(VERSION, CAPABILITIES).unwrap();
    s.open(1, w(), "Toasts", 400., 300.).unwrap();
    let Message::Apply(tx) = fixture::request() else {
        panic!()
    };
    s.apply(&tx).unwrap();
    s
}
fn update(s: &mut Session, operations: Vec<Op>) {
    let base = s.tree(w()).unwrap().revision();
    s.apply(&Transaction {
        window: w(),
        base,
        revision: base + 1,
        operations,
    })
    .unwrap();
}
#[test]
fn accepted_timeout_survives_persistent_update_while_new_timeout_is_rejected() {
    let mut s = mounted();
    let token = s
        .accept_toast_dismissal(w(), n(1), h(), 1, ToastDismissal::Timeout)
        .unwrap();
    update(
        &mut s,
        vec![Op::SetToast(
            n(1),
            ToastConfig {
                timeout_ns: None,
                ..fixture::config()
            },
        )],
    );
    assert!(
        s.accept_toast_dismissal(w(), n(1), h(), 2, ToastDismissal::Timeout)
            .is_none()
    );
    assert_eq!(
        s.complete_toast_dismissal(token),
        Some(fixture::events()[0].clone())
    );
}
#[test]
fn delayed_delivery_requires_the_original_live_session_owner_and_handler() {
    let mut s = mounted();
    let token = s
        .accept_toast_dismissal(w(), n(1), h(), 1, ToastDismissal::Timeout)
        .unwrap();
    assert!(mounted().complete_toast_dismissal(token).is_none());
    let token = s
        .accept_toast_dismissal(w(), n(1), h(), 1, ToastDismissal::Timeout)
        .unwrap();
    update(
        &mut s,
        vec![Op::Bind(n(1), Some(HandlerId::from_parts(1, 1).unwrap()))],
    );
    assert!(s.complete_toast_dismissal(token).is_none());
    let mut s = mounted();
    let token = s
        .accept_toast_dismissal(w(), n(1), h(), 1, ToastDismissal::Timeout)
        .unwrap();
    update(
        &mut s,
        vec![
            Op::Splice(n(0), 0, 1, vec![]),
            Op::Remove(n(2)),
            Op::Remove(n(1)),
        ],
    );
    assert!(s.complete_toast_dismissal(token).is_none());
    let mut s = mounted();
    let token = s
        .accept_toast_dismissal(w(), n(1), h(), 1, ToastDismissal::Timeout)
        .unwrap();
    s.overload(w());
    assert!(s.complete_toast_dismissal(token).is_none());
    let mut s = mounted();
    let token = s
        .accept_toast_dismissal(w(), n(1), h(), 1, ToastDismissal::Timeout)
        .unwrap();
    s.close(w()).unwrap();
    assert!(s.complete_toast_dismissal(token).is_none());
}
