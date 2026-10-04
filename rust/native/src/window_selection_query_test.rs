//! Production window requests on TestPlatform, without reading the OS clipboard.
use super::input_tests::{apply, draw, mount};
use super::*;
use gpui::{Entity, MouseButton, TestAppContext, VisualTestContext};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn install() -> Vec<Op> {
    vec![
        Op::Create(id(0), Kind::Container, "".into(), None),
        Op::Create(id(1), Kind::Text, "alpha β".into(), None),
        Op::Create(id(2), Kind::Text, "omega 😀".into(), None),
        Op::SetStyle(
            id(0),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(500.)),
                Field::Height(Length::Px(200.)),
                Field::Display(1),
                Field::Direction(1),
                Field::FontSize(18.),
                Field::UserSelect(true),
            ])],
        ),
        Op::Splice(id(0), 0, 0, vec![id(1), id(2)]),
        Op::SetRoot(Some(id(0))),
    ]
}
fn query(
    owner: &Entity<View>,
    cx: &mut VisualTestContext,
    command: wire::Command,
) -> wire::Response {
    cx.update(|w, cx| owner.update(cx, |view, cx| request(view, &command, w, cx)))
}
fn point(
    owner: &Entity<View>,
    cx: &mut VisualTestContext,
    node: NodeId,
    byte: usize,
) -> gpui::Point<gpui::Pixels> {
    cx.update(|_, cx| owner.read(cx).selections[&node].borrow().point(byte))
}
fn select_all(owner: &Entity<View>, cx: &mut VisualTestContext, node: NodeId) {
    cx.update(|w, cx| {
        let focus = owner.read(cx).selections[&node].borrow().focus.clone();
        w.focus(&focus, cx);
    });
    cx.simulate_keystrokes("secondary-a");
    draw(cx);
}
#[test]
fn window_selection_queries_bound_utf8_preserve_clipboard_and_clear_local_selection() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    apply(&owner, &mut cx, install());
    cx.update(|_, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string("sentinel".into())));
    assert_eq!(
        query(&owner, &mut cx, wire::Command::HasTextSelection),
        wire::Response::SelectionPresent(false)
    );
    assert_eq!(
        query(&owner, &mut cx, wire::Command::SelectedText(0)),
        wire::Response::SelectedText(String::new())
    );
    select_all(&owner, &mut cx, id(2));
    assert_eq!(
        query(&owner, &mut cx, wire::Command::HasTextSelection),
        wire::Response::SelectionPresent(true)
    );
    for limit in [0, 9] {
        assert_eq!(
            query(&owner, &mut cx, wire::Command::SelectedText(limit)),
            wire::Response::Failed(wire::Error::LimitExceeded)
        );
    }
    for limit in [-1, wire::MAX_SELECTION_BYTES as i64 + 1] {
        assert_eq!(
            query(&owner, &mut cx, wire::Command::SelectedText(limit)),
            wire::Response::Failed(wire::Error::InvalidRequest)
        );
    }
    assert_eq!(
        query(&owner, &mut cx, wire::Command::SelectedText(10)),
        wire::Response::SelectedText("omega 😀".into())
    );
    assert_eq!(
        query(&owner, &mut cx, wire::Command::EndTextSelection),
        wire::Response::SelectionUpdated
    );
    assert_eq!(
        query(&owner, &mut cx, wire::Command::SelectedText(10)),
        wire::Response::SelectedText("omega 😀".into())
    );
    assert_eq!(
        query(&owner, &mut cx, wire::Command::ClearTextSelection),
        wire::Response::SelectionUpdated
    );
    assert_eq!(
        query(&owner, &mut cx, wire::Command::HasTextSelection),
        wire::Response::SelectionPresent(false)
    );
    assert_eq!(
        query(&owner, &mut cx, wire::Command::SelectedText(0)),
        wire::Response::SelectedText(String::new())
    );
    assert_eq!(
        cx.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text())),
        Some("sentinel".into())
    );
}
#[test]
fn window_selection_end_stops_cross_node_drag_and_preserves_ordered_range() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    apply(&owner, &mut cx, install());
    let start = point(&owner, &mut cx, id(1), 2);
    let end = point(&owner, &mut cx, id(2), 5);
    cx.simulate_mouse_move(start, None, Default::default());
    cx.simulate_mouse_down(start, MouseButton::Left, Default::default());
    cx.simulate_mouse_move(end, MouseButton::Left, Default::default());
    draw(&mut cx);
    let selected = wire::Response::SelectedText("pha β\nomega".into());
    assert_eq!(
        query(&owner, &mut cx, wire::Command::SelectedText(12)),
        selected
    );
    assert_eq!(
        query(&owner, &mut cx, wire::Command::EndTextSelection),
        wire::Response::SelectionUpdated
    );
    let farther = point(&owner, &mut cx, id(2), 10);
    cx.simulate_mouse_move(farther, MouseButton::Left, Default::default());
    cx.simulate_mouse_up(farther, MouseButton::Left, Default::default());
    draw(&mut cx);
    assert_eq!(
        query(&owner, &mut cx, wire::Command::SelectedText(12)),
        selected
    );
    apply(
        &owner,
        &mut cx,
        vec![Op::SetText(id(1), "new source".into())],
    );
    assert_eq!(
        query(&owner, &mut cx, wire::Command::HasTextSelection),
        wire::Response::SelectionPresent(false)
    );
}
#[test]
fn window_selection_query_excludes_old_modal_scope_and_removed_participants() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    apply(&owner, &mut cx, install());
    select_all(&owner, &mut cx, id(1));
    apply(
        &owner,
        &mut cx,
        vec![
            Op::Create(id(3), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                id(3),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Create(id(4), Kind::Text, "modal".into(), None),
            Op::Splice(id(3), 0, 0, vec![id(4)]),
            Op::Splice(id(0), 2, 0, vec![id(3)]),
        ],
    );
    assert_eq!(
        query(&owner, &mut cx, wire::Command::SelectedText(50)),
        wire::Response::SelectedText(String::new())
    );
    select_all(&owner, &mut cx, id(4));
    assert_eq!(
        query(&owner, &mut cx, wire::Command::SelectedText(50)),
        wire::Response::SelectedText("modal".into())
    );
    apply(
        &owner,
        &mut cx,
        vec![
            Op::Splice(id(0), 2, 1, vec![]),
            Op::Remove(id(4)),
            Op::Remove(id(3)),
        ],
    );
    assert_eq!(
        query(&owner, &mut cx, wire::Command::HasTextSelection),
        wire::Response::SelectionPresent(false)
    );
    assert_eq!(
        query(&owner, &mut cx, wire::Command::SelectedText(0)),
        wire::Response::SelectedText(String::new())
    );
}

#[test]
fn window_selection_does_not_read_password_editor_selection() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount(&mut app);
    apply(
        &owner,
        &mut cx,
        vec![
            Op::Create(
                id(0),
                Kind::Input,
                "private value".into(),
                Some(gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetEditor(
                id(0),
                EditorConfig {
                    label: "Password".into(),
                    placeholder: "".into(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: false,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::SetEditorPrivacy(id(0), EditorPrivacy::PasswordHidden),
            Op::SetRoot(Some(id(0))),
        ],
    );
    cx.update(|w, cx| {
        let focus = owner.read(cx).editors[&id(0)].focus_handle(cx);
        w.focus(&focus, cx);
    });
    cx.simulate_keystrokes("secondary-a");
    draw(&mut cx);
    assert_eq!(
        query(&owner, &mut cx, wire::Command::HasTextSelection),
        wire::Response::SelectionPresent(false)
    );
    assert_eq!(
        query(&owner, &mut cx, wire::Command::SelectedText(50)),
        wire::Response::SelectedText(String::new())
    );
    assert_eq!(
        query(&owner, &mut cx, wire::Command::ClearTextSelection),
        wire::Response::SelectionUpdated
    );
    // Replacing the editor selection still replaces the whole private draft:
    // the window-wide clear did not change its independent selection model.
    cx.simulate_input("x");
    assert_eq!(
        cx.update(|w, cx| owner.read(cx).editors[&id(0)].snapshot(w, cx).text),
        "x"
    );
}
