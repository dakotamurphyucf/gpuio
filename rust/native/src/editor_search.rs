use gpui::{Context, Window};
use gpui_base::input::{InputBaseState, InputModeKind, TextareaState};
use gpuio_protocol::{editor_search::*, v1::*};

pub(super) fn capture<M: InputModeKind>(state: &InputBaseState<M>) -> Snapshot {
    let session = state.search_session();
    let matcher = &session.matcher;
    let index = matcher.current_match_index();
    Snapshot {
        stamp: Stamp {
            editor_revision: state.bridge_revision(),
            search_revision: session.revision(),
        },
        activation_revision: state.search_activation_revision() as i64,
        mode: if !session.open {
            Mode::Closed
        } else if session.replace_mode {
            Mode::Replace
        } else {
            Mode::Find
        },
        query: session.query.clone(),
        case: if session.case_insensitive {
            Case::AsciiInsensitive
        } else {
            Case::Sensitive
        },
        text_bytes: state.text().len() as i64,
        match_count: matcher.len() as i64,
        current: matcher.matched_ranges().get(index).map(|range| Occurrence {
            index: index as i64,
            byte_start: range.start as i64,
            byte_end: range.end as i64,
        }),
        can_replace: session.open && state.is_replaceable() && state.bridge_composition().is_none(),
    }
}

fn replace(
    state: &mut TextareaState,
    stamp: Stamp,
    replacement: &str,
    all: bool,
    window: &mut Window,
    cx: &mut Context<TextareaState>,
) -> Result<EditorResult, EditorError> {
    if stamp.editor_revision != state.bridge_revision() {
        return Err(EditorError::StaleRevision);
    }
    if stamp.search_revision != state.search_session().revision() {
        return Err(EditorError::StaleSearch);
    }
    if state.bridge_composition().is_some() {
        return Err(EditorError::Composing);
    }
    if !state.is_replaceable() {
        return Err(EditorError::NotEditable);
    }
    if replacement.contains('\0') {
        return Err(EditorError::InvalidText);
    }
    if replacement.len() > MAX_TEXT_BYTES || state.bridge_revision() == i64::MAX {
        return Err(EditorError::LimitExceeded);
    }
    let session = state.search_session();
    let ranges = session.matcher.matched_ranges();
    let selected = if all {
        ranges.as_slice()
    } else {
        let index = session.matcher.current_match_index().min(ranges.len());
        &ranges[index..(index + 1).min(ranges.len())]
    };
    let removed: usize = selected.iter().map(|range| range.len()).sum();
    let size = replacement
        .len()
        .checked_mul(selected.len())
        .and_then(|added| state.text().len().checked_sub(removed)?.checked_add(added));
    if size.is_none_or(|size| size > MAX_TEXT_BYTES) {
        return Err(EditorError::LimitExceeded);
    }
    let count = selected.len();
    let accepted = if count == 0 {
        0
    } else if all {
        state.replace_all_search_matches(replacement, window, cx)
    } else {
        usize::from(state.replace_current_search_match(replacement, window, cx))
    };
    if accepted != count {
        return Err(EditorError::InvalidText);
    }
    Ok(EditorResult::SearchReplaced(
        super::snapshot(state, window, cx),
        capture(state),
        accepted as i64,
    ))
}

pub(super) fn command(
    state: &mut TextareaState,
    command: &Command,
    disabled: bool,
    focus_allowed: bool,
    window: &mut Window,
    cx: &mut Context<TextareaState>,
) -> EditorResult {
    if !matches!(
        command,
        Command::Read | Command::Close | Command::CloseAndFocus(_)
    ) && disabled
    {
        return EditorResult::Failed(EditorError::NotEditable);
    }
    if !matches!(command, Command::Read) && state.search_session().revision() == i64::MAX {
        return EditorResult::Failed(EditorError::LimitExceeded);
    }
    if !state.search_session().open
        && !matches!(command, Command::Read | Command::Open(_) | Command::Close)
    {
        return EditorResult::Failed(EditorError::SearchUnavailable);
    }
    match command {
        Command::Read => (),
        Command::Open(replace) => {
            if state.bridge_composition().is_some() {
                return EditorResult::Failed(EditorError::Composing);
            }
            state.open_search(*replace, cx);
        }
        Command::Close => state.close_search(cx),
        Command::CloseAndFocus(activation) => {
            if *activation != state.search_activation_revision() as i64 {
                return EditorResult::Failed(EditorError::StaleSearch);
            }
            if state.bridge_composition().is_some() {
                return EditorResult::Failed(EditorError::Composing);
            }
            state.close_search(cx);
            if focus_allowed {
                state.focus(window, cx);
            }
        }
        Command::SetQuery(query, case) => {
            if !valid_query(query) {
                return EditorResult::Failed(EditorError::InvalidText);
            }
            state.set_search_query(query.clone(), *case == Case::AsciiInsensitive, cx);
        }
        Command::SetQueryText(query) => {
            if !valid_query(query) {
                return EditorResult::Failed(EditorError::InvalidText);
            }
            let case_insensitive = state.search_session().case_insensitive;
            state.set_search_query(query, case_insensitive, cx);
        }
        Command::SetCase(case) => {
            let query = state.search_session().query.clone();
            state.set_search_query(query, *case == Case::AsciiInsensitive, cx);
        }
        Command::ToggleCase => {
            let query = state.search_session().query.clone();
            let case_insensitive = !state.search_session().case_insensitive;
            state.set_search_query(query, case_insensitive, cx);
        }
        Command::Next => {
            state.next_search_match(cx);
        }
        Command::Previous => {
            state.previous_search_match(cx);
        }
        Command::ReplaceCurrent(stamp, text) | Command::ReplaceAll(stamp, text) => {
            return replace(
                state,
                *stamp,
                text,
                matches!(command, Command::ReplaceAll(..)),
                window,
                cx,
            )
            .unwrap_or_else(EditorResult::Failed);
        }
    }
    let value = capture(state);
    if value.is_valid() {
        EditorResult::SearchObserved(value)
    } else {
        EditorResult::Failed(EditorError::NativeFailure)
    }
}
