use super::*;
use gpuio_protocol::notification::{Action, ClosedReason, Content, Event, Sound};
fn content() -> Content {
    Content {
        title: "Ready".into(),
        body: String::new(),
        actions: vec![Action {
            id: "open".into(),
            label: "Open".into(),
        }],
        sound: Sound::Silent,
    }
}
fn post(routing: &mut Routing, id: u32) -> Token {
    let token = routing.state.post(id.to_string(), content()).unwrap();
    routing.bind(id, &token).unwrap();
    routing.state.complete(&token, Ok(())).result.unwrap();
    token
}
#[test]
fn early_action_is_delivered_only_after_exact_native_binding_and_commit() {
    let mut routing = Routing::default();
    let token = routing.state.post("job".into(), content()).unwrap();
    routing.signal(Signal::Activated(99), Some(&token)).unwrap();
    routing
        .signal(
            Signal::Action {
                native_id: 42,
                receipt: token.receipt.id,
                revision: token.serial,
                id: "open".into(),
            },
            Some(&token),
        )
        .unwrap();
    assert!(!routing.state.pending());
    routing.bind(42, &token).unwrap();
    assert!(!routing.state.pending());
    assert!(routing.state.complete(&token, Ok(())).notify);
    assert_eq!(
        routing.state.take().unwrap(),
        vec![Event::Action(token.receipt.clone(), "open".into())]
    );
    assert_eq!(routing.cleanup_ids(), vec![42]);
    routing.released(42);
    assert!(routing.all_ids().is_empty());
}
#[test]
fn foreign_receipts_and_old_revisions_cannot_consume_a_live_native_id() {
    let mut routing = Routing::default();
    let token = post(&mut routing, 42);
    let next = routing.state.replace(&token.receipt, content()).unwrap();
    routing.state.complete(&next, Ok(())).result.unwrap();
    for (receipt, revision) in [
        (token.receipt.id + 1, next.serial),
        (token.receipt.id, token.serial),
    ] {
        assert!(
            !routing
                .signal(
                    Signal::Action {
                        native_id: 42,
                        receipt,
                        revision,
                        id: "open".into()
                    },
                    None
                )
                .unwrap()
        );
    }
    assert!(!routing.signal(Signal::Activated(99), None).unwrap());
    assert_eq!(
        routing.state.receipt(token.receipt.id),
        Some(token.receipt.clone())
    );
    assert!(routing.signal(Signal::Activated(42), None).unwrap());
    assert_eq!(
        routing.state.take().unwrap(),
        vec![Event::Activated(token.receipt)]
    );
    routing
        .signal(Signal::Closed(42, ClosedReason::User), None)
        .unwrap();
    assert!(routing.all_ids().is_empty());
    assert!(routing.state.take().unwrap().is_empty());
}
#[test]
fn terminal_delivery_does_not_release_the_physical_quota() {
    let mut routing = Routing::default();
    for id in 1..=MAX_LIVE as u32 {
        post(&mut routing, id);
        routing.signal(Signal::Activated(id), None).unwrap();
        routing.state.take().unwrap();
    }
    assert!(!routing.can_post());
    assert_eq!(routing.cleanup_ids().len(), MAX_LIVE);
    routing.released(1);
    assert!(routing.can_post());
}
#[test]
fn early_closed_releases_the_artifact_and_keeps_the_original_receipt_event() {
    let mut routing = Routing::default();
    let token = routing.state.post("job".into(), content()).unwrap();
    routing
        .signal(Signal::Closed(42, ClosedReason::Expired), Some(&token))
        .unwrap();
    routing.bind(42, &token).unwrap();
    routing.state.complete(&token, Ok(())).result.unwrap();
    assert!(routing.all_ids().is_empty());
    assert_eq!(
        routing.state.take().unwrap(),
        vec![Event::Closed(token.receipt, ClosedReason::Expired)]
    );
}
#[test]
fn early_overflow_is_explicit_and_native_id_collision_never_rebinds() {
    let mut routing = Routing::default();
    let first = post(&mut routing, 42);
    let next = routing.state.post("next".into(), content()).unwrap();
    assert_eq!(routing.bind(42, &next), Err(Error::NativeFailure));
    assert_eq!(routing.native_id(&first.receipt), Ok(42));
    for id in 100..100 + MAX_EARLY as u32 {
        routing.signal(Signal::Activated(id), Some(&next)).unwrap();
    }
    assert_eq!(
        routing.signal(Signal::Activated(999), Some(&next)),
        Err(Error::Busy)
    );
}

#[test]
fn closed_during_replacement_is_not_resurrected_by_its_reply() {
    let mut routing = Routing::default();
    let old = post(&mut routing, 42);
    let token = routing.state.replace(&old.receipt, content()).unwrap();
    routing.begin_show(42);
    routing
        .signal(Signal::Closed(42, ClosedReason::User), None)
        .unwrap();
    routing.bind(42, &token).unwrap();
    assert_eq!(
        routing.state.complete(&token, Ok(())).result,
        Err(Error::Stale)
    );
    assert!(routing.all_ids().is_empty());
    assert_eq!(
        routing.state.take().unwrap(),
        vec![Event::Closed(old.receipt, ClosedReason::User)]
    );
}
