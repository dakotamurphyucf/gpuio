use super::*;
use gpuio_protocol::choice_picker::{Group, Item, Search};

fn config(open_state: OpenState, multiple: bool) -> Config {
    Config {
        label: "Pick".into(),
        options: Collection::Grouped(vec![Group {
            id: "g".into(),
            label: "Group".into(),
            items: vec![
                Item {
                    id: "a".into(),
                    label: "Alpha".into(),
                    disabled: false,
                },
                Item {
                    id: "b".into(),
                    label: "Beta".into(),
                    disabled: true,
                },
            ],
        }]),
        selected: if multiple {
            Selection::Multiple(vec!["b".into()])
        } else {
            Selection::Single(None)
        },
        disabled: false,
        search: Search::None,
        clearable: true,
        open_state,
        placeholder: String::new(),
        search_placeholder: String::new(),
    }
}
fn changed(open: bool, reason: VisibilityReason) -> Event {
    Event::Visibility(Visibility::Changed(open, reason))
}
fn interaction(open: bool, reason: OpenReason) -> Event {
    changed(open, VisibilityReason::Interaction(reason))
}

#[test]
fn managed_single_orders_selection_close_intent_then_visibility() {
    let mut state = State::new(
        Arc::new(config(OpenState::Managed(false), false)),
        true,
        None,
    )
    .unwrap();
    assert_eq!(
        state.snapshot(),
        Event::Visibility(Visibility::Snapshot(false))
    );
    assert!(state.activate("a", false, None).is_empty());
    assert_eq!(
        state.request_open(true, OpenReason::Keyboard),
        [
            Event::OpenRequested(true, OpenReason::Keyboard),
            interaction(true, OpenReason::Keyboard),
        ]
    );
    assert!(state.request_open(true, OpenReason::Trigger).is_empty());
    for (id, composing) in [("missing", false), ("b", false), ("a", true)] {
        assert!(state.activate(id, composing, None).is_empty());
        assert!(state.is_open());
    }
    assert_eq!(
        state.activate("a", false, None),
        [
            Event::SelectionRequested(Request::Select("a".into()), None),
            Event::OpenRequested(false, OpenReason::Selection),
            interaction(false, OpenReason::Selection),
        ]
    );
    assert!(!state.is_open());
    assert_eq!(state.config().selected, Selection::Single(None));
}

#[test]
fn controlled_intents_are_retryable_until_application_changes_actual_visibility() {
    let mut accepted = config(OpenState::Controlled(false), false);
    let mut state = State::new(Arc::new(accepted.clone()), true, None).unwrap();
    for _ in 0..2 {
        assert_eq!(
            state.request_open(true, OpenReason::Trigger),
            [Event::OpenRequested(true, OpenReason::Trigger)]
        );
        assert!(!state.is_open());
    }
    assert!(
        state
            .configure(Arc::new(accepted.clone()), true, None)
            .unwrap()
            .is_empty()
    );
    accepted.open_state = OpenState::Controlled(true);
    assert_eq!(
        state
            .configure(Arc::new(accepted.clone()), true, None)
            .unwrap(),
        [changed(true, VisibilityReason::Application)]
    );
    assert_eq!(
        state.request_open(false, OpenReason::Escape),
        [Event::OpenRequested(false, OpenReason::Escape)]
    );
    assert!(state.is_open());
    assert_eq!(
        state.activate("a", false, None),
        [
            Event::SelectionRequested(Request::Select("a".into()), None),
            Event::OpenRequested(false, OpenReason::Selection),
        ]
    );
    accepted.open_state = OpenState::Managed(false);
    assert!(
        state
            .configure(Arc::new(accepted.clone()), true, None)
            .unwrap()
            .is_empty()
    );
    assert!(state.is_open()); // unaccepted close intent must not leak into managed state
    assert_eq!(
        state.request_open(false, OpenReason::OutsidePointer),
        [
            Event::OpenRequested(false, OpenReason::OutsidePointer),
            interaction(false, OpenReason::OutsidePointer),
        ]
    );
    accepted.open_state = OpenState::Managed(true);
    assert!(
        state
            .configure(Arc::new(accepted), true, None)
            .unwrap()
            .is_empty()
    );
    assert!(!state.is_open()); // initially_open is mount-only
}

#[test]
fn unavailable_closes_without_an_intent_and_only_controlled_state_recovers() {
    for open_state in [OpenState::Managed(true), OpenState::Controlled(true)] {
        for disable in [false, true] {
            let original = Arc::new(config(open_state, false));
            let mut state = State::new(original.clone(), true, None).unwrap();
            let mut unavailable = (*original).clone();
            unavailable.disabled = disable;
            let unavailable = Arc::new(unavailable);
            assert_eq!(
                state.configure(unavailable.clone(), disable, None).unwrap(),
                [changed(false, VisibilityReason::Unavailable)]
            );
            assert!(
                state
                    .configure(unavailable, disable, None)
                    .unwrap()
                    .is_empty()
            );
            assert!(state.request_open(true, OpenReason::Keyboard).is_empty());
            assert!(state.activate("a", false, None).is_empty());
            assert!(state.clear(false, None).is_empty());
            let recovery = state.configure(original.clone(), true, None).unwrap();
            match open_state {
                OpenState::Controlled(_) => {
                    assert_eq!(recovery, [changed(true, VisibilityReason::Application)])
                }
                OpenState::Managed(_) => {
                    assert!(recovery.is_empty());
                    assert!(!state.is_open());
                }
            }
            let initially_ineligible = State::new(original, false, None).unwrap();
            assert!(!initially_ineligible.is_open());
        }
    }
}

#[test]
fn multiple_requests_preserve_order_and_use_the_latest_catalog_without_optimism() {
    let initial = Arc::new(config(OpenState::Managed(true), true));
    let mut state = State::new(initial.clone(), true, None).unwrap();
    for _ in 0..2 {
        assert_eq!(
            state.activate("a", false, None),
            [Event::SelectionRequested(Request::Toggle("a".into()), None)]
        );
        assert!(state.is_open());
        assert_eq!(
            state.config().selected,
            Selection::Multiple(vec!["b".into()])
        );
    }
    assert!(state.clear(true, None).is_empty());
    assert_eq!(
        state.clear(false, None),
        [Event::SelectionRequested(Request::Clear, None)]
    );
    assert!(state.is_open());
    let mut updated = (*initial).clone();
    updated.options = Collection::Flat(vec![]);
    updated.selected = Selection::Multiple(vec![]);
    updated.clearable = false;
    assert!(
        state
            .configure(Arc::new(updated), true, None)
            .unwrap()
            .is_empty()
    );
    assert!(state.activate("a", false, None).is_empty());
    assert!(state.clear(false, None).is_empty());
}

#[test]
fn invalid_updates_and_invalid_direction_reasons_leave_state_unchanged() {
    let original = Arc::new(config(OpenState::Managed(true), false));
    let mut state = State::new(original.clone(), true, None).unwrap();
    let mut invalid = (*original).clone();
    invalid.label.clear();
    assert!(matches!(
        State::new(Arc::new(invalid.clone()), true, None),
        Err(ConfigError::Invalid)
    ));
    assert_eq!(
        state.configure(Arc::new(invalid), false, None),
        Err(ConfigError::Invalid)
    );
    assert!(Arc::ptr_eq(state.config(), &original));
    assert!(state.is_open());
    assert!(state.request_open(false, OpenReason::Keyboard).is_empty());
    state.request_open(false, OpenReason::FocusLeft);
    for reason in [
        OpenReason::Escape,
        OpenReason::OutsidePointer,
        OpenReason::FocusLeft,
        OpenReason::Selection,
    ] {
        assert!(state.request_open(true, reason).is_empty());
        assert!(!state.is_open());
    }
    assert_eq!(
        state.clear(false, None),
        [Event::SelectionRequested(Request::Clear, None)]
    );
}

#[test]
fn selection_captures_query_and_fences_editor_remounts_and_composition() {
    use gpuio_protocol::{
        NodeId,
        v1::{EditorSelection, EditorSnapshot},
    };
    let first = NodeId::from_parts(7, 2).unwrap();
    let replacement = NodeId::from_parts(7, 3).unwrap();
    let mut config = config(OpenState::Managed(true), true);
    config.search = Search::Substring;
    let config = Arc::new(config);
    assert!(matches!(
        State::new(config.clone(), true, None),
        Err(ConfigError::Invalid)
    ));
    let mut state = State::new(config.clone(), true, Some(first)).unwrap();
    let mut query = Query {
        node: first,
        snapshot: EditorSnapshot {
            revision: 9,
            text: "λx".into(),
            selection: EditorSelection { anchor: 3, head: 3 },
            composition: None,
            focused: true,
        },
    };
    assert!(state.activate("a", false, None).is_empty());
    assert!(state.clear(false, None).is_empty());
    let captured = state.activate("a", false, Some(&query));
    assert_eq!(
        captured,
        [Event::SelectionRequested(
            Request::Toggle("a".into()),
            Some(query.clone())
        )]
    );
    query.snapshot.text.push('y');
    query.snapshot.revision += 1;
    if let Event::SelectionRequested(_, Some(old)) = &captured[0] {
        assert_eq!(old.snapshot.text, "λx");
        assert_eq!(old.snapshot.revision, 9);
    } else {
        panic!("missing captured query");
    }
    query.snapshot.composition = Some(EditorSelection { anchor: 0, head: 2 });
    assert!(state.activate("a", false, Some(&query)).is_empty());
    assert!(state.clear(false, Some(&query)).is_empty());
    query.snapshot.composition = None;
    assert_eq!(
        state.configure(config.clone(), false, None),
        Err(ConfigError::Invalid)
    );
    assert!(state.is_open()); // Invalid child update did not change eligibility.
    assert!(
        state
            .configure(config.clone(), true, Some(replacement))
            .unwrap()
            .is_empty()
    );
    assert!(state.activate("a", false, Some(&query)).is_empty());
    query.node = replacement;
    query.snapshot.revision = 9; // Revisions can repeat across distinct leases.
    assert_eq!(
        state.clear(false, Some(&query)),
        [Event::SelectionRequested(
            Request::Clear,
            Some(query.clone())
        )]
    );
    let mut no_search = (*config).clone();
    no_search.search = Search::None;
    assert!(
        state
            .configure(Arc::new(no_search), true, None)
            .unwrap()
            .is_empty()
    );
    assert!(state.activate("a", false, Some(&query)).is_empty());
    assert_eq!(
        state.activate("a", false, None),
        [Event::SelectionRequested(Request::Toggle("a".into()), None)]
    );
}
