//! Native search prerequisites, on TestPlatform without OS windows.
use gpui::{AppContext, Context, EntityInputHandler, TestAppContext, Window};
use gpui_base::input::TextareaState;

fn with_editor(test: impl FnOnce(&mut TextareaState, &mut Window, &mut Context<TextareaState>)) {
    with_editor_limit(32, test);
}

fn with_editor_limit(
    max_bytes: usize,
    test: impl FnOnce(&mut TextareaState, &mut Window, &mut Context<TextareaState>),
) {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let cx = app.add_empty_window();
    cx.update(|window, cx| {
        let editor = cx.new(|cx| {
            TextareaState::new(window, cx)
                .bridge_max_bytes(max_bytes)
                .searchable(true)
        });
        editor.update(cx, |state, cx| test(state, window, cx));
    });
}

#[test]
fn rejected_search_replacements_preserve_editor_and_match_position() {
    with_editor(|state, window, cx| {
        state.set_value("é é é", window, cx);
        state.set_search_query("é", false, cx);
        state.open_search(true, cx);
        assert_eq!(state.next_search_match(cx), Some(3..5));
        assert!(state.bridge_select(5, 3, cx));
        let before = (
            state.bridge_revision(),
            state.bridge_selection(),
            state.bridge_history_bytes(),
        );
        let too_large = "x".repeat(33);
        assert!(!state.replace_current_search_match(&too_large, window, cx));
        assert_eq!(state.replace_all_search_matches(&too_large, window, cx), 0);
        // One occurrence would fit, but expanding all three would not.
        assert_eq!(
            state.replace_all_search_matches(&"x".repeat(15), window, cx),
            0
        );
        assert_eq!(state.replace_all_search_matches("\0", window, cx), 0);
        assert_eq!(state.value(), "é é é");
        assert_eq!(state.search_session().matcher.current_match_index(), 1);
        assert_eq!(
            (
                state.bridge_revision(),
                state.bridge_selection(),
                state.bridge_history_bytes()
            ),
            before
        );
        assert!(state.replace_current_search_match("界", window, cx));
        assert_eq!(state.value(), "é 界 é");
        state.bridge_undo(window, cx);
        assert_eq!(state.value(), "é é é");
    });
}

#[test]
fn search_replacement_does_not_commit_composition() {
    with_editor(|state, window, cx| {
        state.set_value("é é", window, cx);
        state.set_search_query("é", false, cx);
        state.open_search(true, cx);
        state.replace_and_mark_text_in_range(Some(0..1), "é", Some(0..1), window, cx);
        let before = (
            state.bridge_revision(),
            state.bridge_selection(),
            state.bridge_composition(),
        );
        assert!(before.2.is_some());
        assert!(!state.replace_current_search_match("x", window, cx));
        assert_eq!(state.replace_all_search_matches("x", window, cx), 0);
        assert_eq!(
            (
                state.bridge_revision(),
                state.bridge_selection(),
                state.bridge_composition()
            ),
            before
        );
        assert_eq!(state.value(), "é é");
    });
}

#[test]
fn unicode_search_replace_all_is_one_undo_step_and_respects_editability() {
    with_editor(|state, window, cx| {
        state.set_value("é🙂 é🙂", window, cx);
        state.set_search_query("é🙂", false, cx);
        state.open_search(true, cx);
        assert_eq!(state.replace_all_search_matches("界", window, cx), 2);
        assert_eq!(state.value(), "界 界");
        state.bridge_undo(window, cx);
        assert_eq!(state.value(), "é🙂 é🙂");
        state.bridge_redo(window, cx);
        assert_eq!(state.value(), "界 界");
        state.set_search_query("界", false, cx);
        state.set_readonly(true, cx);
        assert_eq!(state.replace_all_search_matches("x", window, cx), 0);
        state.set_readonly(false, cx);
        state.set_disabled(true, cx);
        assert!(!state.replace_current_search_match("x", window, cx));
        state.set_disabled(false, cx);
        assert_eq!(state.replace_all_search_matches("界", window, cx), 2);
        assert_eq!(state.value(), "界 界");
    });
}

#[test]
fn search_highlight_slice_is_bounded_by_layout_and_preserves_global_indices() {
    with_editor(|state, window, cx| {
        state.set_value("é🙂 é🙂 é🙂", window, cx);
        state.set_search_query("é🙂", false, cx);
        let matcher = &state.search_session().matcher;
        assert_eq!(matcher.contained_match_indices(0..20), 0..3);
        assert_eq!(matcher.contained_match_indices(7..13), 1..2);
        assert_eq!(matcher.contained_match_indices(8..20), 2..3);
        assert_eq!(matcher.contained_match_indices(0..5), 0..0);
        assert_eq!(matcher.contained_match_indices(8..9), 2..2);
        assert_eq!(matcher.contained_match_indices(20..20), 3..3);
        state.set_search_query("", false, cx);
        assert_eq!(
            state
                .search_session()
                .matcher
                .contained_match_indices(0..20),
            0..0
        );
    });
}

#[test]
fn dense_document_search_layout_selects_only_local_matches() {
    with_editor_limit(262_144, |state, window, cx| {
        state.set_value("a".repeat(262_144), window, cx);
        state.set_search_query("a", false, cx);
        let matcher = &state.search_session().matcher;
        assert_eq!(matcher.len(), 262_144);
        assert_eq!(
            matcher.contained_match_indices(200_000..200_080),
            200_000..200_080
        );
        assert_eq!(
            matcher.contained_match_indices(262_144..262_144),
            262_144..262_144
        );
        assert_eq!(state.replace_all_search_matches("", window, cx), 262_144);
        assert_eq!(state.value(), "");
        state.bridge_undo(window, cx);
        assert_eq!(state.value().len(), 262_144);
    });
}

#[test]
fn search_stamps_detect_query_and_navigation_round_trips() {
    with_editor(|state, window, cx| {
        state.set_value("one two one", window, cx);
        state.set_search_query("one", false, cx);
        state.open_search(false, cx);
        let before = state.search_session().revision();
        let editor_revision = state.bridge_revision();
        state.set_search_query("two", false, cx);
        state.set_search_query("one", false, cx);
        assert!(state.search_session().revision() > before);
        assert_eq!(state.bridge_revision(), editor_revision);
        let before = state.search_session().revision();
        state.next_search_match(cx);
        state.previous_search_match(cx);
        assert_eq!(state.search_session().matcher.current_match_index(), 0);
        assert!(state.search_session().revision() > before);
        let before = state.search_session().revision();
        state.set_search_query("one", false, cx);
        assert_eq!(state.search_session().revision(), before);
        let activation = state.search_activation_revision();
        state.open_search(false, cx);
        assert!(state.search_session().revision() > before);
        assert_eq!(state.search_activation_revision(), activation + 1);
        let before = state.search_session().revision();
        assert!(!state.replace_current_search_match(&"x".repeat(33), window, cx));
        assert_eq!(state.search_session().revision(), before);
        state.set_search_replace_mode(true, cx);
        let before = state.search_session().revision();
        state.set_readonly(true, cx);
        assert!(!state.search_session().replace_mode);
        assert!(state.search_session().revision() > before);
        state.set_readonly(false, cx);
        let before = state.search_session().revision();
        state.set_searchable(false, cx);
        assert!(!state.search_session().open);
        assert!(state.search_session().revision() > before);
    });
}

#[test]
fn native_selection_seed_and_query_share_the_bridge_query_bound() {
    with_editor_limit(4096, |state, window, cx| {
        let text = "é".repeat(1025);
        state.set_value(text.clone(), window, cx);
        state.set_search_query("é", false, cx);
        state.bridge_select(0, text.len(), cx);
        state.open_search(false, cx);
        assert_eq!(state.search_session().query, "é");
        let before = state.search_session().revision();
        state.set_search_query(text, true, cx);
        state.set_search_query("a\0b", true, cx);
        assert_eq!(state.search_session().revision(), before);
        assert_eq!(state.search_session().query, "é");
        state.set_search_query("é".repeat(1024), false, cx);
        assert_eq!(state.search_session().query.len(), 2048);
        assert!(state.search_session().revision() > before);
    });
}
