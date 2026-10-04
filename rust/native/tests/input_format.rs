//! GPUI Base formatting behavior required before exposing ordinary input policies.
//! These are pure mask tests, not desktop/IME acceptance.
use gpui_base::input::MaskPattern;

#[test]
fn wildcard_masks_accept_utf8_scalars_without_confusing_bytes_with_slots() {
    for (pattern, text) in [
        ("*", "界"),
        ("*", "é"),
        ("**", "e\u{301}"),
        ("*99", "界12"),
        ("界-99", "界-12"),
    ] {
        let mask = MaskPattern::new(pattern);
        assert!(mask.is_valid(text), "pattern={pattern:?}, text={text:?}");
        assert_eq!(mask.mask(text), text);
    }
    // Slots follow Unicode scalar values, not bytes or grapheme clusters.
    assert!(!MaskPattern::new("*").is_valid("e\u{301}"));
    assert!(!MaskPattern::new("99").is_valid("１２"));
    assert!(!MaskPattern::new("A").is_valid("é"));
    assert!(!MaskPattern::new("*").is_valid("界x"));
}

#[test]
fn formatted_and_raw_values_keep_unicode_literals_separate_from_slots() {
    let mask = MaskPattern::new("界-99");
    assert!(mask.is_valid("12"));
    assert_eq!(mask.mask("12"), "界-12");
    assert_eq!(mask.unmask("界-12"), "12");
    let mask = MaskPattern::new("*–99");
    assert_eq!(mask.mask("界12"), "界–12");
    assert_eq!(mask.unmask("界–12"), "界12");
}

#[cfg(feature = "native-image-tests")]
#[test]
fn native_formatted_typing_and_undo_preserve_utf8_text_and_offsets() {
    use gpui::{AppContext, EntityInputHandler, TestAppContext};
    use gpui_base::input::InputState;
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let cx = app.add_empty_window();
    cx.update(|window, cx| {
        let input = cx.new(|cx| InputState::new(window, cx).mask_pattern("*–99"));
        input.update(cx, |state, cx| {
            state.replace_text_in_range(None, "界12", window, cx);
            assert_eq!(state.value().as_str(), "界–12");
            assert_eq!(state.bridge_selection(), (8, 8));
            state.bridge_undo(window, cx);
            assert_eq!(state.value().as_str(), "");
            state.bridge_redo(window, cx);
            assert_eq!(state.value().as_str(), "界–12");
            assert_eq!(state.bridge_selection(), (8, 8));
        });
    });
}

#[test]
fn invalid_slot_input_is_rejected_instead_of_being_silently_dropped_by_formatting() {
    let mask = MaskPattern::new("9A");
    assert!(!mask.is_valid("x"));
    assert!(!mask.is_valid("x1"));
    assert!(mask.is_valid("1x"));
    assert_eq!(mask.mask("1x"), "1x");
    let mask = MaskPattern::new("(99)-AA");
    assert!(mask.is_valid("12AB"));
    assert!(mask.is_valid("(12)-AB"));
    assert_eq!(mask.mask("12AB"), "(12)-AB");
}

#[cfg(feature = "native-image-tests")]
#[test]
fn native_mask_rejection_preserves_text_selection_revision_and_undo() {
    use gpui::{AppContext, EntityInputHandler, TestAppContext};
    use gpui_base::input::InputState;
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let cx = app.add_empty_window();
    cx.update(|window, cx| {
        let input = cx.new(|cx| InputState::new(window, cx).mask_pattern("9A"));
        input.update(cx, |state, cx| {
            state.replace_text_in_range(None, "1a", window, cx);
            assert_eq!(state.value().as_str(), "1a");
            assert!(state.bridge_select(0, 2, cx));
            let revision = state.bridge_revision();
            state.replace_text_in_range(None, "x", window, cx);
            assert_eq!(state.value().as_str(), "1a");
            assert_eq!(state.bridge_selection(), (0, 2));
            assert_eq!(state.bridge_revision(), revision);
            state.bridge_undo(window, cx);
            assert_eq!(state.value().as_str(), "");
            state.bridge_redo(window, cx);
            assert_eq!(state.value().as_str(), "1a");
        });
    });
}

#[cfg(feature = "native-image-tests")]
#[test]
fn native_unicode_composition_formats_on_commit_as_one_undoable_change() {
    use gpui::{AppContext, EntityInputHandler, TestAppContext};
    use gpui_base::input::InputState;
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let cx = app.add_empty_window();
    cx.update(|window, cx| {
        let input = cx.new(|cx| InputState::new(window, cx).mask_pattern("*–99"));
        input.update(cx, |state, cx| {
            state.replace_and_mark_text_in_range(None, "界12", Some(0..3), window, cx);
            assert_eq!(state.value().as_str(), "界12");
            assert_eq!(state.bridge_composition(), Some(0..5));
            state.replace_text_in_range(None, "界12", window, cx);
            assert_eq!(state.value().as_str(), "界–12");
            assert_eq!(state.bridge_composition(), None);
            assert_eq!(state.bridge_selection(), (8, 8));
            state.bridge_undo(window, cx);
            assert_eq!(state.value().as_str(), "");
            state.bridge_redo(window, cx);
            assert_eq!(state.value().as_str(), "界–12");
        });
    });
}

#[cfg(feature = "native-image-tests")]
#[test]
fn history_restores_recorded_text_after_policy_changes_but_new_edits_still_format() {
    use gpui::{AppContext, EntityInputHandler, TestAppContext};
    use gpui_base::input::InputState;
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let cx = app.add_empty_window();
    cx.update(|window, cx| {
        let input = cx.new(|cx| InputState::new(window, cx).mask_pattern("9A"));
        input.update(cx, |state, cx| {
            state.replace_text_in_range(None, "1a", window, cx);
            state.set_mask_pattern("99", window, cx);
            assert_eq!(state.value().as_str(), "1a");
            state.bridge_undo(window, cx);
            assert_eq!(state.value().as_str(), "");
            state.bridge_redo(window, cx);
            assert_eq!(state.value().as_str(), "1a");
            state.set_mask_pattern("99-99", window, cx);
            state.set_value("1234", window, cx);
            assert_eq!(state.value().as_str(), "12-34");
            state.replace_text_in_range(Some(0..5), "invalid", window, cx);
            assert_eq!(state.value().as_str(), "12-34");
        });
    });
}

#[cfg(feature = "native-image-tests")]
#[test]
fn changing_policy_allows_invalid_draft_recovery_without_discarding_text() {
    use gpui::{AppContext, EntityInputHandler, TestAppContext};
    use gpui_base::input::InputState;
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let cx = app.add_empty_window();
    cx.update(|window, cx| {
        let input = cx.new(|cx| InputState::new(window, cx).default_value("abcd"));
        input.update(cx, |state, cx| {
            state.set_mask_pattern("99", window, cx);
            assert_eq!(state.value().as_str(), "abcd");
            state.replace_text_in_range(Some(0..4), "abc", window, cx);
            assert_eq!(state.value().as_str(), "abc");
            state.replace_text_in_range(Some(0..3), "12", window, cx);
            assert_eq!(state.value().as_str(), "12");
            state.replace_text_in_range(Some(0..2), "x", window, cx);
            assert_eq!(state.value().as_str(), "12");
            state.bridge_undo(window, cx);
            assert_eq!(state.value().as_str(), "abc");
            state.bridge_undo(window, cx);
            assert_eq!(state.value().as_str(), "abcd");
        });
    });
}
