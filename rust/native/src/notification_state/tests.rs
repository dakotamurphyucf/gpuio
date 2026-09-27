use super::*;
use gpuio_protocol::notification::{Action, Sound};

fn content(action: &str) -> Content {
    Content {
        title: "Build complete".into(),
        body: "日本語 🦀".into(),
        actions: vec![Action {
            id: action.into(),
            label: "Open".into(),
        }],
        sound: Sound::Silent,
    }
}
fn posted(state: &mut State, tag: &str) -> Token {
    let token = state.post(tag.into(), content("open")).unwrap();
    assert_eq!(
        state.complete(&token, Ok(())),
        Completion {
            result: Ok(()),
            cleanup: false,
            notify: false
        }
    );
    token
}
fn action(token: &Token, id: &str) -> Signal {
    Signal::Action {
        revision: token.serial,
        id: id.into(),
    }
}
fn bounds(state: &State) {
    assert!(state.entries.len() + state.events.len() <= MAX_LIVE);
    assert!(state.operations.len() <= MAX_PENDING);
    assert_eq!(state.entries.len(), state.tags.len());
    for entry in state.entries.values() {
        assert_eq!(state.tags.get(&entry.receipt.tag), Some(&entry.receipt.id));
    }
}

#[test]
fn replacement_retains_lifetime_and_fences_custom_actions() {
    let mut state = State::default();
    let first = posted(&mut state, "build");
    assert_eq!(
        state.post("build".into(), content("open")),
        Err(Error::Busy)
    );
    let next = state.replace(&first.receipt, content("inspect")).unwrap();
    assert_eq!(next.receipt, first.receipt);
    assert!(!state.complete(&next, Ok(())).notify);
    assert_eq!(state.revision(&first.receipt), Some(next.serial));
    assert!(!state.signal(&first.receipt, action(&first, "open")));
    assert!(!state.signal(&first.receipt, action(&next, "open")));
    assert!(state.signal(&first.receipt, action(&next, "inspect")));
    assert!(!state.signal(&first.receipt, Signal::Activated));
    let fresh = posted(&mut state, "build");
    assert_ne!(fresh.receipt, first.receipt);
    assert_eq!(
        state.take().unwrap(),
        vec![Event::Action(first.receipt.clone(), "inspect".into())]
    );
    assert!(!state.signal(&first.receipt, action(&next, "inspect")));
    assert!(state.signal(&fresh.receipt, Signal::Activated));
    bounds(&state);
}

#[test]
fn default_activation_belongs_to_logical_lifetime() {
    let mut state = State::default();
    let first = posted(&mut state, "build");
    let next = state.replace(&first.receipt, content("inspect")).unwrap();
    state.complete(&next, Ok(()));
    assert!(state.signal(&first.receipt, Signal::Activated));
    assert_eq!(state.take().unwrap(), vec![Event::Activated(first.receipt)]);
}

#[test]
fn early_signal_wakes_only_after_successful_acknowledgement() {
    for success in [true, false] {
        let mut state = State::default();
        let first = state.post("build".into(), content("open")).unwrap();
        assert!(!state.signal(&first.receipt, action(&first, "unknown")));
        assert!(!state.signal(&first.receipt, action(&first, "open")));
        assert!(!state.signal(&first.receipt, Signal::Closed(ClosedReason::User)));
        assert!(!state.pending());
        let result = if success { Ok(()) } else { Err(Error::Denied) };
        assert_eq!(state.complete(&first, result).notify, success);
        assert_eq!(
            state.take().unwrap(),
            if success {
                vec![Event::Action(first.receipt, "open".into())]
            } else {
                vec![]
            }
        );
        assert!(state.entries.is_empty());
        bounds(&state);
    }
}

#[test]
fn failed_replace_preserves_previous_content_and_discards_candidate_action() {
    let mut state = State::default();
    let first = posted(&mut state, "build");
    let next = state.replace(&first.receipt, content("inspect")).unwrap();
    assert!(!state.signal(&first.receipt, action(&next, "inspect")));
    assert_eq!(
        state.complete(&next, Err(Error::NativeFailure)).result,
        Err(Error::NativeFailure)
    );
    assert!(!state.pending());
    assert_eq!(state.revision(&first.receipt), Some(first.serial));
    assert!(state.signal(&first.receipt, action(&first, "open")));
    bounds(&state);
}

#[test]
fn terminal_event_during_replace_requires_exact_artifact_cleanup() {
    let mut state = State::default();
    let first = posted(&mut state, "build");
    let next = state.replace(&first.receipt, content("inspect")).unwrap();
    assert!(state.signal(&first.receipt, action(&first, "open")));
    let fresh = posted(&mut state, "build");
    assert_eq!(
        state.complete(&next, Ok(())),
        Completion {
            result: Err(Error::Stale),
            cleanup: true,
            notify: false
        }
    );
    assert_eq!(state.revision(&fresh.receipt), Some(fresh.serial));
    assert!(!state.complete(&next, Ok(())).cleanup);
    bounds(&state);
}

#[test]
fn failed_dismissal_remains_retired_and_retryable() {
    let mut state = State::default();
    let first = posted(&mut state, "build");
    let dismiss = state.dismiss(&first.receipt).unwrap();
    assert!(!state.signal(&first.receipt, Signal::Activated));
    assert_eq!(
        state.complete(&dismiss, Err(Error::NativeFailure)).result,
        Err(Error::NativeFailure)
    );
    assert_eq!(
        state.replace(&first.receipt, content("inspect")),
        Err(Error::Stale)
    );
    assert_eq!(
        state.post("build".into(), content("open")),
        Err(Error::Busy)
    );
    assert!(!state.signal(&first.receipt, action(&first, "open")));
    let retry = state.dismiss(&first.receipt).unwrap();
    assert!(!state.signal(&first.receipt, Signal::Closed(ClosedReason::User)));
    assert_eq!(
        state.complete(&retry, Err(Error::NativeFailure)).result,
        Ok(())
    );
    assert!(state.take().unwrap().is_empty());
    assert!(state.entries.is_empty());
    bounds(&state);
}

#[test]
fn platform_close_after_failed_dismissal_releases_retired_slot() {
    let mut state = State::default();
    let first = posted(&mut state, "build");
    let dismiss = state.dismiss(&first.receipt).unwrap();
    state.complete(&dismiss, Err(Error::NativeFailure));
    assert!(!state.signal(&first.receipt, Signal::Closed(ClosedReason::Platform)));
    assert!(state.entries.is_empty());
    assert!(state.take().unwrap().is_empty());
}

#[test]
fn failure_and_shutdown_keep_bounded_inflight_cleanup_tokens() {
    for close in [false, true] {
        let mut state = State::default();
        let first = posted(&mut state, "admitted");
        assert!(state.signal(&first.receipt, Signal::Closed(ClosedReason::Expired)));
        let pending = state.post("inflight".into(), content("open")).unwrap();
        if close {
            state.close();
        } else {
            assert!(!state.fail(Error::Unavailable));
        }
        let completion = state.complete(&pending, Ok(()));
        assert_eq!(
            completion.result,
            Err(if close { Error::Closed } else { Error::Stale })
        );
        assert!(completion.cleanup);
        assert!(!state.complete(&pending, Ok(())).cleanup);
        assert!(state.operations.is_empty());
        if close {
            assert_eq!(state.take(), Err(Error::Closed));
            assert_eq!(
                state.post("new".into(), content("open")),
                Err(Error::Closed)
            );
        } else {
            assert_eq!(
                state.take().unwrap(),
                vec![
                    Event::Closed(first.receipt, ClosedReason::Expired),
                    Event::Failed(Error::Unavailable)
                ]
            );
        }
        bounds(&state);
    }
}

#[test]
fn foreign_or_duplicate_completion_cannot_retire_live_artifact() {
    let mut a = State::default();
    let mut b = State::default();
    let ta = a.post("same".into(), content("open")).unwrap();
    let tb = b.post("same".into(), content("open")).unwrap();
    assert_eq!(ta.serial, tb.serial);
    assert_ne!(ta.receipt, tb.receipt);
    assert_eq!(
        b.complete(&ta, Ok(())),
        Completion {
            result: Err(Error::Stale),
            cleanup: false,
            notify: false
        }
    );
    assert_eq!(b.complete(&tb, Ok(())).result, Ok(()));
    assert_eq!(b.complete(&tb, Ok(())).result, Err(Error::Stale));
    assert!(b.signal(&tb.receipt, Signal::Activated));
}

#[test]
fn quotas_reserve_terminal_events_and_recover_without_historical_retention() {
    let mut state = State::default();
    let tokens: Vec<_> = (0..MAX_PENDING)
        .map(|i| state.post(format!("pending-{i}"), content("open")).unwrap())
        .collect();
    assert_eq!(
        state.post("excess".into(), content("open")),
        Err(Error::Busy)
    );
    for token in tokens {
        state.complete(&token, Err(Error::Unavailable));
    }
    for cycle in 0..32 {
        for i in 0..MAX_LIVE {
            let token = posted(&mut state, &format!("{cycle}-{i}"));
            assert_eq!(state.signal(&token.receipt, Signal::Activated), i == 0);
            bounds(&state);
        }
        assert_eq!(
            state.post("excess".into(), content("open")),
            Err(Error::Busy)
        );
        assert!(!state.fail(Error::Unavailable));
        assert!(!state.fail(Error::NativeFailure));
        assert_eq!(state.take().unwrap().len(), MAX_LIVE + 1);
        assert!(state.entries.is_empty() && state.tags.is_empty() && state.operations.is_empty());
    }
}

#[test]
fn invalid_requests_have_no_side_effects() {
    let mut state = State::default();
    assert_eq!(
        state.post("\n".into(), content("open")),
        Err(Error::InvalidRequest)
    );
    let mut invalid = content("open");
    invalid.actions.push(invalid.actions[0].clone());
    assert_eq!(
        state.post("valid".into(), invalid),
        Err(Error::InvalidRequest)
    );
    assert_eq!(
        state.dismiss(&Receipt {
            id: 0,
            tag: "valid".into()
        }),
        Err(Error::InvalidRequest)
    );
    assert!(state.entries.is_empty() && state.operations.is_empty());
}

#[test]
fn category_snapshots_retain_only_live_and_inflight_content() {
    let mut state = State::default();
    let tokens: Vec<_> = (0..MAX_LIVE)
        .map(|i| posted(&mut state, &format!("tag-{i}")))
        .collect();
    assert_eq!(state.receipts().len(), MAX_LIVE);
    let pending: Vec<_> = tokens
        .iter()
        .take(MAX_PENDING)
        .map(|token| state.replace(&token.receipt, content("inspect")).unwrap())
        .collect();
    assert_eq!(state.contents().len(), MAX_LIVE + MAX_PENDING);
    for token in pending {
        state.complete(&token, Ok(()));
    }
    assert_eq!(state.contents().len(), MAX_LIVE);
    for token in tokens {
        assert_eq!(state.receipt(token.receipt.id), Some(token.receipt.clone()));
        state.signal(&token.receipt, Signal::Activated);
    }
    assert!(state.contents().is_empty() && state.receipts().is_empty());
}
