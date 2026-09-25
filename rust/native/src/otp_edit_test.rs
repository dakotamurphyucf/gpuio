use super::*;
use gpuio_protocol::otp::Alphabet;

fn state(value: &str) -> State {
    State::new(Policy::new(6, Alphabet::Digits).unwrap(), value).unwrap()
}
fn selection(anchor: usize, head: usize) -> Selection {
    Selection { anchor, head }
}

#[test]
fn accepted_edits_are_atomic_and_completion_is_a_value_transition() {
    let mut s = state("123456");
    s.select(selection(4, 2)).unwrap();
    let change = s.paste("９-８").unwrap();
    assert_eq!(s.value(), "129856");
    assert_eq!(s.selection(), Selection::caret(4));
    assert!(change.value_changed && change.completed);
    assert_eq!(s.history_edits(), 1);
    s.select(selection(4, 2)).unwrap();
    assert_eq!(s.paste(" - \t").unwrap(), Change::default());
    assert_eq!(s.selection(), selection(4, 2));
    let failed = s.paste("９x").unwrap();
    assert_eq!(
        failed.rejection,
        Some(InputError::UnexpectedCharacter { byte_offset: 3 })
    );
    assert!(!failed.state_changed);
    assert_eq!(s.value(), "129856");
    assert_eq!(s.selection(), selection(4, 2));
    assert_eq!(s.history_edits(), 1);
    let same = s.commit_utf16(None, "９８").unwrap();
    assert!(same.state_changed); // collapse the selection
    assert!(!same.value_changed && !same.completed);
    assert_eq!(s.history_edits(), 1);
    assert_eq!(
        s.commit_utf16(None, "1").unwrap().rejection,
        Some(InputError::TooLong)
    );
    let removed = s.delete(Delete::Backward).unwrap();
    assert_eq!(s.value(), "12956");
    assert!(removed.value_changed && !removed.completed);
    assert!(s.undo().unwrap().completed);
    assert_eq!(s.value(), "129856");
    assert!(!s.redo().unwrap().completed);
}

#[test]
fn ime_checkpoint_survives_updates_and_commits_as_one_history_entry() {
    let mut s = state("123456");
    s.select(selection(4, 2)).unwrap();
    let start = s.mark_utf16(None, "に🙂", Some(1..3)).unwrap();
    assert!(start.state_changed && !start.value_changed && !start.completed);
    assert_eq!(s.value(), "123456");
    assert_eq!(s.draft(), "12に🙂56");
    assert_eq!(s.marked(), Some(2..9));
    assert_eq!(s.selection(), selection(5, 9));
    assert_eq!(s.history_edits(), 0);
    s.mark_utf16(None, "９８", Some(1..2)).unwrap();
    assert_eq!(s.value(), "123456");
    assert_eq!(s.draft(), "12９８56");
    assert_eq!(s.selection(), selection(5, 8));
    assert_eq!(s.marked(), Some(2..8));
    let done = s.commit_utf16(None, "７６").unwrap();
    assert!(done.completed && done.value_changed && done.rejection.is_none());
    assert_eq!(s.value(), "127656");
    assert_eq!(s.selection(), Selection::caret(4));
    assert!(!s.is_composing());
    assert_eq!(s.history_edits(), 1);
    s.undo().unwrap();
    assert_eq!(s.value(), "123456");
    assert_eq!(s.selection(), selection(4, 2));
    s.redo().unwrap();
    assert_eq!(s.value(), "127656");
    assert_eq!(s.selection(), Selection::caret(4));
}

#[test]
fn rejected_commit_restores_directional_selection_and_preserves_redo() {
    let mut s = state("12");
    s.commit_utf16(None, "34").unwrap();
    s.undo().unwrap();
    s.select(selection(2, 0)).unwrap();
    s.mark_utf16(None, "かな", None).unwrap();
    let failed = s.commit_utf16(None, "漢字").unwrap();
    assert_eq!(
        failed.rejection,
        Some(InputError::UnexpectedCharacter { byte_offset: 0 })
    );
    assert!(failed.state_changed && !failed.value_changed);
    assert_eq!(s.value(), "12");
    assert_eq!(s.draft(), "12");
    assert_eq!(s.selection(), selection(2, 0));
    assert!(!s.is_composing());
    assert_eq!(s.history_edits(), 1);
    assert!(s.can_redo());
    s.redo().unwrap();
    assert_eq!(s.value(), "1234");
}

#[test]
fn unmark_normalizes_and_maps_selection_but_invalid_unmark_rolls_back() {
    let mut s = state("12");
    s.mark_utf16(None, "３４", Some(0..1)).unwrap();
    s.select(selection(8, 2)).unwrap(); // reverse selection across preedit
    let done = s.unmark();
    assert!(done.value_changed && !done.completed);
    assert_eq!(s.value(), "1234");
    assert_eq!(s.selection(), selection(4, 2));
    assert_eq!(s.unmark(), Change::default());
    s.mark_utf16(None, "字", None).unwrap();
    assert!(s.unmark().rejection.is_some());
    assert_eq!(s.value(), "1234");
    assert_eq!(s.selection(), selection(4, 2));
    assert_eq!(s.history_edits(), 1);
    s.mark_utf16(None, "に", None).unwrap();
    assert!(s.cancel_composition().state_changed);
    assert_eq!(s.selection(), selection(4, 2));
    s.mark_utf16(None, "に", None).unwrap();
    assert!(s.mark_utf16(None, "", None).unwrap().state_changed);
    assert_eq!(s.value(), "1234");
    assert_eq!(s.selection(), selection(4, 2));
}

#[test]
fn preedit_supports_explicit_partial_ranges_and_exact_utf16_boundaries() {
    let mut s = state("12");
    s.mark_utf16(Some(1..2), "🙂あ", Some(2..3)).unwrap();
    assert_eq!(s.draft(), "1🙂あ");
    assert_eq!(s.selection(), selection(5, 8));
    let prior = s.draft().to_owned();
    assert_eq!(
        s.mark_utf16(Some(2..3), "９", None),
        Err(InputError::InvalidSelection)
    );
    assert_eq!(
        s.commit_utf16(Some(2..3), "９"),
        Err(InputError::InvalidSelection)
    );
    assert_eq!(
        s.select(Selection::caret(2)),
        Err(InputError::InvalidSelection)
    );
    assert_eq!(
        s.mark_utf16(None, "🙂", Some(1..2)),
        Err(InputError::InvalidSelection)
    );
    assert_eq!(s.draft(), prior);
    s.mark_utf16(Some(1..3), "９", None).unwrap();
    assert_eq!(s.draft(), "1９あ");
    // Explicit final replacement covers both raw characters, not just last mark.
    let done = s.commit_utf16(Some(1..3), "８７").unwrap();
    assert!(done.value_changed);
    assert_eq!(s.value(), "187");
    assert_eq!(s.selection(), Selection::caret(3));
    s.undo().unwrap();
    assert_eq!(s.value(), "12");
    assert_eq!(s.selection(), Selection::caret(2));
}

#[test]
fn preedit_limits_do_not_destroy_history_or_previous_composition() {
    let mut s = state("");
    for i in 0..MAX_HISTORY_EDITS + 19 {
        let value = if i % 2 == 0 { "123456" } else { "654321" };
        s.replace(value, Selection::caret(6), History::Record)
            .unwrap();
    }
    assert_eq!(s.history_edits(), MAX_HISTORY_EDITS);
    assert!(s.history_bytes() <= 2 * 6 * MAX_HISTORY_EDITS);
    s.select(selection(6, 0)).unwrap();
    let original = s.value().to_owned();
    let payload = "あ".repeat(MAX_INPUT_BYTES / 3);
    s.mark_utf16(None, &payload, None).unwrap();
    let bytes = s.history_bytes();
    for _ in 0..100 {
        assert_eq!(
            s.mark_utf16(None, &payload, None).unwrap(),
            Change::default()
        );
    }
    assert_eq!(
        s.mark_utf16(None, &"x".repeat(MAX_INPUT_BYTES + 1), None),
        Err(InputError::InputTooLarge)
    );
    assert_eq!(s.draft(), payload);
    assert_eq!(s.history_bytes(), bytes);
    assert_eq!(
        s.unmark().rejection,
        Some(InputError::UnexpectedCharacter { byte_offset: 0 })
    );
    assert_eq!(s.value(), original);
    assert_eq!(s.selection(), selection(6, 0));
    assert_eq!(s.history_bytes(), bytes);
    for _ in 0..MAX_HISTORY_EDITS {
        s.undo().unwrap();
    }
    assert!(!s.can_undo() && s.can_redo());
    assert_eq!(s.history_edits(), MAX_HISTORY_EDITS);
    for _ in 0..MAX_HISTORY_EDITS {
        s.redo().unwrap();
    }
    assert!(!s.can_redo() && s.can_undo());
    s.replace(&original, Selection::caret(6), History::Reset)
        .unwrap();
    assert_eq!(s.history_edits(), 0);
    assert_eq!(s.history_bytes(), 0);
}

#[test]
fn raw_draft_bounds_and_control_characters_reject_before_mutation() {
    let mut s = state("12");
    assert_eq!(
        s.mark_utf16(None, &"x".repeat(MAX_INPUT_BYTES), None),
        Err(InputError::InputTooLarge)
    );
    assert!(!s.is_composing());
    s.mark_utf16(None, "３", None).unwrap();
    for text in ["a\0", "a\r", "a\n"] {
        assert_eq!(
            s.mark_utf16(None, text, None),
            Err(InputError::UnexpectedCharacter { byte_offset: 1 })
        );
        assert_eq!(s.draft(), "12３");
    }
    let change = s
        .commit_utf16(None, &"x".repeat(MAX_INPUT_BYTES + 1))
        .unwrap();
    assert_eq!(change.rejection, Some(InputError::InputTooLarge));
    assert!(change.state_changed && !change.value_changed);
    assert_eq!(s.value(), "12");
    assert_eq!(s.selection(), Selection::caret(2));
    assert!(!s.is_composing());
    assert_eq!(s.history_edits(), 0);
    s.mark_utf16(None, "３４５６７", None).unwrap();
    assert_eq!(s.unmark().rejection, Some(InputError::TooLong));
    assert_eq!(s.value(), "12");
    assert_eq!(s.history_edits(), 0);
}

#[test]
fn commands_and_rejected_replacements_cannot_erase_active_composition() {
    let mut s = state("12");
    s.mark_utf16(None, "３", None).unwrap();
    assert_eq!(
        s.replace("", Selection::caret(0), History::Reset),
        Err(Error::Composing)
    );
    assert_eq!(s.undo(), Err(Error::Composing));
    assert_eq!(s.redo(), Err(Error::Composing));
    assert_eq!(s.paste("45"), Err(Error::Composing));
    assert_eq!(s.move_caret(Movement::Left, false), Err(Error::Composing));
    assert_eq!(s.delete(Delete::Backward), Err(Error::Composing));
    assert_eq!(s.draft(), "12３");
    s.unmark();
    assert_eq!(
        s.replace("AB", Selection::caret(2), History::Reset),
        Err(InputError::InvalidValue.into())
    );
    assert_eq!(
        s.replace("1", Selection::caret(2), History::Reset),
        Err(InputError::InvalidSelection.into())
    );
    assert_eq!(s.value(), "123");
    assert_eq!(s.history_edits(), 1);
    assert!(
        s.replace("123", Selection::caret(3), History::Reset)
            .unwrap()
            .state_changed
    );
    assert_eq!(s.history_edits(), 0);
}

#[test]
fn navigation_and_deletion_operate_on_cells_and_preserve_selection_direction() {
    let mut s = state("123456");
    s.move_caret(Movement::Left, true).unwrap();
    s.move_caret(Movement::Left, true).unwrap();
    assert_eq!(s.selection(), selection(6, 4));
    s.move_caret(Movement::Right, false).unwrap();
    assert_eq!(s.selection(), Selection::caret(6));
    s.move_caret(Movement::Start, true).unwrap();
    assert_eq!(s.selection(), selection(6, 0));
    s.move_caret(Movement::Left, false).unwrap();
    assert_eq!(s.selection(), Selection::caret(0));
    assert_eq!(s.delete(Delete::Backward).unwrap(), Change::default());
    s.delete(Delete::Forward).unwrap();
    assert_eq!(s.value(), "23456");
    s.move_caret(Movement::End, true).unwrap();
    assert_eq!(s.selection(), selection(0, 5));
    s.delete(Delete::Backward).unwrap();
    assert_eq!(s.value(), "");
    assert_eq!(s.selection(), Selection::caret(0));
    s.undo().unwrap();
    assert_eq!(s.value(), "23456");
    assert_eq!(s.selection(), selection(0, 5));
}

#[test]
fn conversion_is_reversible_only_at_scalar_boundaries() {
    for text in ["", "123", "🙂", "1🙂２a", "👨‍👩‍👧‍👦", "e\u{301}"] {
        for byte in 0..=text.len() {
            let units = byte_to_utf16(text, byte);
            if text.is_char_boundary(byte) {
                assert_eq!(utf16_to_byte(text, units.unwrap()), Ok(byte));
            } else {
                assert_eq!(units, Err(InputError::InvalidSelection));
            }
        }
        let length = text.encode_utf16().count();
        assert_eq!(
            utf16_to_byte(text, length + 1),
            Err(InputError::InvalidSelection)
        );
        assert_eq!(
            byte_to_utf16(text, text.len() + 1),
            Err(InputError::InvalidSelection)
        );
        assert_eq!(
            utf16_range(text, Range { start: 1, end: 0 }),
            Err(InputError::InvalidSelection)
        );
    }
    assert_eq!(utf16_to_byte("🙂", 1), Err(InputError::InvalidSelection));
}

#[test]
fn editing_all_policy_lengths_agrees_with_the_atomic_protocol_contract() {
    for length in 1..=32 {
        for alphabet in [Alphabet::Digits, Alphabet::AsciiAlphanumeric] {
            let policy = Policy::new(length, alphabet).unwrap();
            let initial = "1".repeat(length as usize);
            for anchor in 0..=initial.len() {
                for head in 0..=initial.len() {
                    for input in ["", "９", "Ａ", "12", " ", "🙂"] {
                        let mut s = State::new(policy, &initial).unwrap();
                        s.select(selection(anchor, head)).unwrap();
                        let expected = policy.replace(&initial, anchor, head, input, false);
                        let change = s.commit_utf16(None, input).unwrap();
                        match expected {
                            Ok((value, anchor, head)) => {
                                assert_eq!(s.value(), value);
                                assert_eq!(s.selection(), selection(anchor, head));
                                assert_eq!(change.value_changed, value != initial);
                                assert_eq!(
                                    change.completed,
                                    value != initial && value.len() == policy.length()
                                );
                            }
                            Err(error) => {
                                assert_eq!(change.rejection, Some(error));
                                assert_eq!(s.value(), initial);
                                assert_eq!(s.selection(), selection(anchor, head));
                                assert_eq!(s.history_edits(), 0);
                            }
                        }
                    }
                }
            }
        }
    }
}
