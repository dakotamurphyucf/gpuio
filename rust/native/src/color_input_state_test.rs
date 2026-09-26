use super::*;

fn config() -> Config {
    Config {
        labels: Labels {
            control: "Color".into(),
            hue: "Hue".into(),
            saturation: "Saturation".into(),
            lightness: "Lightness".into(),
            alpha: "Opacity".into(),
            hex: "Hex color".into(),
            clear: "Clear color".into(),
        },
        palette: vec![PaletteEntry {
            color: Rgba::new(255, 0, 0, 255),
            label: "Red".into(),
        }],
        alpha_policy: AlphaPolicy::AllowAlpha,
        allow_empty: true,
        disabled: false,
        read_only: false,
    }
}
fn color(text: &str) -> Value {
    Value::Color(Rgba::of_hex(text).unwrap())
}
fn state() -> State {
    State::new(config().into(), color("#00FF00")).unwrap()
}
fn id(event: Event) -> i64 {
    assert!(event.is_valid());
    event.snapshot().interaction.unwrap().id
}
fn check(events: &[Event]) {
    assert!(events.len() <= 2);
    assert!(events.iter().all(Event::is_valid));
    for pair in events.windows(2) {
        assert!(pair[0].snapshot().revision < pair[1].snapshot().revision);
    }
}

#[test]
fn achromatic_hue_survives_preview_commit_and_full_cancel() {
    let mut s = state();
    let first = id(s.begin_drag(Channel::Saturation, Access::Allowed).unwrap());
    assert!(
        s.preview_drag(first, 0., Access::Allowed)
            .unwrap()
            .unwrap()
            .is_valid()
    );
    assert_eq!(s.snapshot().channels.hue_degrees(), 120.);
    assert_eq!(s.snapshot().value, color("#808080"));
    assert_eq!(s.snapshot().committed, color("#00FF00"));
    assert!(s.finish(first, Access::Allowed).unwrap().is_valid());
    let gray = s.snapshot();
    let second = id(s.begin_drag(Channel::Hue, Access::Allowed).unwrap());
    s.preview_drag(second, 240., Access::Allowed).unwrap();
    assert_eq!(s.snapshot().value, gray.value); // HSLA preview can change without RGBA changing.
    assert_eq!(s.snapshot().channels.hue_degrees(), 240.);
    assert!(s.cancel(CancelReason::Escape).unwrap().unwrap().is_valid());
    assert_eq!(s.snapshot().channels, gray.channels);
    let third = id(s.begin_drag(Channel::Saturation, Access::Allowed).unwrap());
    s.preview_drag(third, 100., Access::Allowed).unwrap();
    assert_eq!(s.snapshot().value, color("#00FF00"));
    s.finish(third, Access::Allowed).unwrap();
    s.set(color("#888888"), None).unwrap();
    assert_eq!(s.snapshot().channels.hue_degrees(), 0.); // Explicit replacement is canonical.
}

#[test]
fn set_reset_and_new_interaction_fence_stale_callbacks() {
    let mut s = state();
    let old = id(s.begin_drag(Channel::Hue, Access::Allowed).unwrap());
    s.preview_drag(old, 240., Access::Allowed).unwrap();
    let before = s.snapshot();
    assert_eq!(s.set(color("#FF0000"), Some(0)), Err(Error::StaleRevision));
    assert_eq!(s.snapshot(), before);
    let events = s.set(color("#FF0000"), Some(before.revision)).unwrap();
    check(&events);
    assert!(matches!(
        events[0],
        Event::Cancelled(CancelReason::Programmatic, _)
    ));
    assert_eq!(events[0].snapshot().value, color("#00FF00"));
    assert!(matches!(events[1], Event::Observed(_)));
    let next = id(s.begin_drag(Channel::Lightness, Access::Allowed).unwrap());
    assert!(next > old);
    let before = s.snapshot();
    assert_eq!(
        s.preview_drag(old, 0., Access::Allowed),
        Err(Error::StaleInteraction)
    );
    assert_eq!(s.finish(old, Access::Allowed), Err(Error::StaleInteraction));
    assert_eq!(s.snapshot(), before);
    check(&s.reset(None).unwrap());
    assert_eq!(s.snapshot().value, color("#00FF00"));
    assert_eq!(
        s.finish(next, Access::Allowed),
        Err(Error::StaleInteraction)
    );
}

#[test]
fn text_preview_composition_invalid_commit_and_cancel_are_distinct() {
    let mut s = state();
    let edit = id(s
        .begin_text(Field::Hex, "#00FF00".into(), Access::Allowed)
        .unwrap());
    s.preview_text(edit, "#FF0000".into(), true, Access::Allowed)
        .unwrap();
    assert_eq!(s.snapshot().value, color("#00FF00"));
    assert_eq!(s.finish(edit, Access::Allowed), Err(Error::Composing));
    s.preview_text(edit, "#FF0000".into(), false, Access::Allowed)
        .unwrap();
    assert_eq!(s.snapshot().value, color("#FF0000"));
    assert_eq!(s.snapshot().committed, color("#00FF00"));
    for text in ["#", "#xyz", ""] {
        s.preview_text(edit, text.into(), false, Access::Allowed)
            .unwrap();
        assert_eq!(s.snapshot().value, color("#FF0000"));
        assert_eq!(s.finish(edit, Access::Allowed), Err(Error::InvalidDraft));
    }
    let before = s.snapshot();
    for text in ["a".repeat(4097), "#123\0".into(), "\n".into()] {
        assert_eq!(
            s.preview_text(edit, text, false, Access::Allowed),
            Err(Error::InvalidDraft)
        );
        assert_eq!(s.snapshot(), before);
    }
    s.cancel(CancelReason::Escape).unwrap();
    assert_eq!(s.snapshot().value, color("#00FF00"));
    assert!(s.snapshot().draft.is_none());
    assert_eq!(
        s.preview_text(edit, "#000".into(), false, Access::Allowed),
        Err(Error::StaleInteraction)
    );
    let edit = id(s
        .begin_text(Field::Hex, "#123".into(), Access::Allowed)
        .unwrap());
    let event = s.finish(edit, Access::Allowed).unwrap();
    assert!(matches!(event, Event::Committed(Source::Text, _)));
    assert_eq!(s.snapshot().value, color("#112233")); // No intervening Change event required.
}

#[test]
fn percentages_and_hue_text_are_unsnapped_bounded_and_achromatic_edits_keep_hue() {
    let mut s = state();
    let edit = id(s
        .begin_text(
            Field::Channel(Channel::Alpha),
            "100".into(),
            Access::Allowed,
        )
        .unwrap());
    for (text, status) in [
        ("-", DraftStatus::Incomplete),
        ("101", DraftStatus::OutOfRange),
        ("nan", DraftStatus::Invalid),
        ("50.25", DraftStatus::Valid),
    ] {
        s.preview_text(edit, text.into(), false, Access::Allowed)
            .unwrap();
        assert_eq!(s.snapshot().draft.unwrap().status, status);
    }
    assert_eq!(s.snapshot().channels.alpha(), 0.5025);
    assert_eq!(s.snapshot().value, color("#00FF0080"));
    s.finish(edit, Access::Allowed).unwrap();
    let edit = id(s
        .begin_text(Field::Hex, "#00FF0080".into(), Access::Allowed)
        .unwrap());
    s.preview_text(edit, "#00000080".into(), false, Access::Allowed)
        .unwrap();
    assert_eq!(s.snapshot().channels.hue_degrees(), 120.);
    s.finish(edit, Access::Allowed).unwrap();
    assert_eq!(s.snapshot().channels.hue_degrees(), 120.);
    let edit = id(s
        .begin_text(Field::Channel(Channel::Hue), "120".into(), Access::Allowed)
        .unwrap());
    s.preview_text(edit, "270".into(), false, Access::Allowed)
        .unwrap();
    assert_eq!(s.snapshot().channels.hue_degrees(), 270.);
    s.finish(edit, Access::Allowed).unwrap();
    assert_eq!(s.snapshot().channels.hue_degrees(), 270.);
}

#[test]
fn alpha_policy_and_empty_changes_preserve_historical_values_without_silent_coercion() {
    let seed = color("#10203080");
    let mut s = State::new(config().into(), seed).unwrap();
    let edit = id(s.begin_drag(Channel::Alpha, Access::Allowed).unwrap());
    s.preview_drag(edit, 100., Access::Allowed).unwrap();
    let mut c = config();
    c.alpha_policy = AlphaPolicy::OpaqueOnly;
    c.allow_empty = false;
    let events = s.configure(c.into()).unwrap();
    check(&events);
    assert!(matches!(
        events[0],
        Event::Cancelled(CancelReason::ConfigurationChanged, _)
    ));
    assert_eq!(s.snapshot().value, seed);
    assert!(!s.snapshot().value_allowed && !s.snapshot().committed_allowed);
    assert_eq!(s.set(seed, None), Err(Error::InvalidValue));
    assert_eq!(s.reset(None), Err(Error::InvalidValue));
    assert_eq!(
        s.choose(Value::Empty, Source::Clear, Access::Allowed),
        Err(Error::InvalidValue)
    );
    assert_eq!(
        s.finish(edit, Access::Allowed),
        Err(Error::StaleInteraction)
    );
    let edit = id(s
        .begin_text(Field::Channel(Channel::Alpha), "50".into(), Access::Allowed)
        .unwrap());
    s.preview_text(edit, "99.99".into(), false, Access::Allowed)
        .unwrap();
    assert_eq!(s.snapshot().draft.unwrap().status, DraftStatus::Forbidden);
    assert_eq!(s.finish(edit, Access::Allowed), Err(Error::InvalidDraft));
    s.preview_text(edit, "100".into(), false, Access::Allowed)
        .unwrap();
    s.finish(edit, Access::Allowed).unwrap();
    assert_eq!(s.snapshot().value, color("#102030"));
    assert!(s.snapshot().value_allowed);
    assert_eq!(s.reset(None), Err(Error::InvalidValue));
    assert!(State::new(s.config.clone(), seed).is_err());
    let mut empty = State::new(config().into(), Value::Empty).unwrap();
    let mut c = config();
    c.allow_empty = false;
    empty.configure(c.into()).unwrap();
    assert_eq!(empty.snapshot().value, Value::Empty);
    assert!(!empty.snapshot().value_allowed);
}

#[test]
fn labels_and_palette_preserve_edits_while_policy_gates_cancel_them() {
    let mut s = state();
    let edit = id(s
        .begin_text(Field::Hex, "#00FF00".into(), Access::Allowed)
        .unwrap());
    s.preview_text(edit, "#12".into(), true, Access::Allowed)
        .unwrap();
    let draft = s.snapshot().draft;
    let mut c = config();
    c.labels.control = "Couleur".into();
    c.palette.clear();
    check(&s.configure(c.clone().into()).unwrap());
    assert_eq!(s.snapshot().draft, draft);
    assert_eq!(s.snapshot().interaction.unwrap().id, edit);
    c.read_only = true;
    let events = s.configure(c.clone().into()).unwrap();
    check(&events);
    assert!(matches!(
        events[0],
        Event::Cancelled(CancelReason::ReadOnly, _)
    ));
    assert_eq!(
        s.begin_drag(Channel::Hue, Access::Allowed),
        Err(Error::ReadOnly)
    );
    check(&s.set(color("#123"), None).unwrap()); // Programmatic writes remain allowed.
    c.read_only = false;
    c.disabled = true;
    s.configure(c.into()).unwrap();
    assert_eq!(
        s.begin_drag(Channel::Hue, Access::Allowed),
        Err(Error::Disabled)
    );
    check(&s.reset(None).unwrap());
    assert_eq!(s.snapshot().value, color("#00FF00"));
}

#[test]
fn input_access_and_lifecycle_cancellation_never_require_ocaml_roundtrip() {
    for reason in [
        CancelReason::Escape,
        CancelReason::Hidden,
        CancelReason::Modal,
        CancelReason::WindowInactive,
        CancelReason::Unmounted,
        CancelReason::Interrupted,
    ] {
        let mut s = state();
        assert_eq!(
            s.begin_drag(Channel::Hue, Access::Blocked),
            Err(Error::FocusBlocked)
        );
        let edit = id(s.begin_drag(Channel::Hue, Access::Allowed).unwrap());
        s.preview_drag(edit, 300., Access::Allowed).unwrap();
        let before = s.snapshot();
        assert_eq!(
            s.preview_drag(edit, 20., Access::Blocked),
            Err(Error::FocusBlocked)
        );
        assert_eq!(s.finish(edit, Access::Blocked), Err(Error::FocusBlocked));
        assert_eq!(s.snapshot(), before);
        assert!(s.cancel(reason).unwrap().unwrap().is_valid());
        assert_eq!(s.snapshot().value, color("#00FF00"));
        assert!(s.cancel(reason).unwrap().is_none());
        assert_eq!(
            s.finish(edit, Access::Allowed),
            Err(Error::StaleInteraction)
        );
    }
}

#[test]
fn discrete_adjustment_cancels_old_preview_then_commits_from_baseline() {
    let mut s = state();
    let edit = id(s.begin_drag(Channel::Hue, Access::Allowed).unwrap());
    s.preview_drag(edit, 240., Access::Allowed).unwrap();
    let before = s.snapshot();
    assert_eq!(
        s.set_channel(
            Channel::Lightness,
            f64::NAN,
            Source::Keyboard,
            Access::Allowed
        ),
        Err(Error::InvalidValue)
    );
    assert_eq!(s.snapshot(), before);
    let events = s
        .set_channel(Channel::Lightness, 25., Source::Keyboard, Access::Allowed)
        .unwrap();
    check(&events);
    assert!(matches!(
        events[0],
        Event::Cancelled(CancelReason::Interrupted, _)
    ));
    assert!(matches!(events[1], Event::Committed(Source::Keyboard, _)));
    assert_eq!(s.snapshot().value, color("#008000"));
    assert_eq!(
        s.finish(edit, Access::Allowed),
        Err(Error::StaleInteraction)
    );
    check(
        &s.choose(color("#0000FF"), Source::Palette, Access::Allowed)
            .unwrap(),
    );
    check(
        &s.choose(Value::Empty, Source::Clear, Access::Allowed)
            .unwrap(),
    );
    assert_eq!(s.snapshot().value, Value::Empty);
}

#[test]
fn revision_limits_reject_two_event_mutations_atomically() {
    let mut s = state();
    let edit = id(s.begin_drag(Channel::Hue, Access::Allowed).unwrap());
    s.preview_drag(edit, 240., Access::Allowed).unwrap();
    s.revision = i64::MAX - 1;
    let before = s.snapshot();
    let mut c = config();
    c.disabled = true;
    assert_eq!(s.configure(c.into()), Err(Error::LimitExceeded));
    assert_eq!(s.set(Value::Empty, None), Err(Error::LimitExceeded));
    assert_eq!(s.reset(None), Err(Error::LimitExceeded));
    assert_eq!(
        s.choose(color("#123"), Source::Palette, Access::Allowed),
        Err(Error::LimitExceeded)
    );
    assert_eq!(s.snapshot(), before);
    assert!(!s.config().disabled);
    s.cancel(CancelReason::Hidden).unwrap();
    assert_eq!(s.snapshot().revision, i64::MAX);
    assert_eq!(
        s.begin_drag(Channel::Hue, Access::Allowed),
        Err(Error::LimitExceeded)
    );
}

#[test]
fn bounded_configuration_equal_replacement_disposal_and_output_failure() {
    let mut c = config();
    c.palette = vec![c.palette[0].clone(); MAX_PALETTE_ENTRIES];
    c.palette
        .iter_mut()
        .for_each(|e| e.label = "x".repeat(MAX_PALETTE_LABEL_BYTES));
    assert!(c.is_valid());
    let shared = Arc::new(c.clone());
    let weak = Arc::downgrade(&shared);
    let mut s = State::new(shared, color("#123")).unwrap();
    assert!(s.config().retained_bytes() >= MAX_PALETTE_ENTRIES * MAX_PALETTE_LABEL_BYTES);
    assert!(s.configure(Arc::new(c.clone())).unwrap().is_empty());
    assert!(weak.upgrade().is_none());
    c.palette.push(c.palette[0].clone());
    let before = s.snapshot();
    assert_eq!(s.configure(c.into()), Err(Error::InvalidConfig));
    assert_eq!(s.snapshot(), before);
    for label in ["".into(), " \t".into(), "a\0b".into(), "x".repeat(4097)] {
        let mut c = config();
        c.labels.hex = label;
        assert!(!c.is_valid());
    }
    let edit = id(s.begin_drag(Channel::Hue, Access::Allowed).unwrap());
    s.fault();
    assert_eq!(s.finish(edit, Access::Allowed), Err(Error::NativeFailure));
    assert_eq!(s.set(Value::Empty, None), Err(Error::NativeFailure));
    assert_eq!(s.cancel(CancelReason::Escape), Err(Error::NativeFailure));
    s.close();
    assert_eq!(s.set(Value::Empty, None), Err(Error::Closed));
    assert_eq!(s.snapshot().value, s.snapshot().committed);
    assert!(s.snapshot().interaction.is_none());
    let weak = Arc::downgrade(&s.config);
    drop(s);
    assert!(weak.upgrade().is_none());
}
