//! Native cross-node selection, source retirement, exclusions and modal ownership.
use super::*;
#[path = "selection_isolation_test.rs"]
mod isolation;
#[path = "selection_typography_test.rs"]
mod typography;
use crate::host::editor_test::{frame, key as press};

fn apply(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, operations: Vec<Op>) {
    window
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let applied = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: view.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
        .unwrap();
}
fn point_at(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    node: NodeId,
    byte: usize,
) -> gpui::Point<gpui::Pixels> {
    window
        .update(cx, |view, _, _| view.selections[&node].borrow().point(byte))
        .unwrap()
}
async fn drag(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    from: (NodeId, usize),
    to: (NodeId, usize),
) {
    let start = point_at(cx, window, from.0, from.1);
    let end = point_at(cx, window, to.0, to.1);
    move_mouse(cx, window, start, false);
    mouse(cx, window, start, true);
    move_mouse(cx, window, end, true);
    mouse(cx, window, end, false);
    frame(cx, window).await;
}
fn copy(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) -> String {
    cx.update(|cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string("unchanged".into())));
    press(cx, window, "secondary-c");
    cx.update(|cx| {
        cx.read_from_clipboard()
            .and_then(|item| item.text())
            .unwrap()
    })
}
pub(super) async fn exercise(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    let saved = cx.update(|cx| cx.read_from_clipboard());
    apply(
        cx,
        window,
        vec![
            Op::Splice(id(0), 0, 2, vec![]),
            Op::Remove(NodeId::from_parts(1, 2).unwrap()),
            Op::SetText(id(2), "alpha β".into()),
            Op::SetStyle(id(2), vec![]),
            Op::Create(id(3), Kind::Text, "middle 😀".into(), None),
            Op::Create(id(4), Kind::Text, "omega ζ".into(), None),
            Op::Create(id(5), Kind::Text, "excluded".into(), None),
            Op::SetStyle(id(5), vec![Style::Fields(vec![Field::UserSelect(false)])]),
            Op::SetStyle(
                id(0),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(460.)),
                    Field::Height(Length::Px(280.)),
                    Field::Display(1),
                    Field::Direction(1),
                    Field::FontSize(18.),
                    Field::UserSelect(true),
                    Field::SelectionColor(Color::Rgba(0x00ffffff)),
                ])],
            ),
            Op::Splice(id(0), 0, 0, vec![id(2), id(5), id(3), id(4)]),
        ],
    );
    frame(cx, window).await;
    drag(cx, window, (id(2), 2), (id(4), 5)).await;
    assert_eq!(
        copy(cx, window),
        "pha β\nmiddle 😀\nomega",
        "cross-node Copy follows rendered order and skips excluded text"
    );
    drag(cx, window, (id(4), 5), (id(2), 2)).await;
    assert_eq!(
        copy(cx, window),
        "pha β\nmiddle 😀\nomega",
        "reverse drag preserves source order"
    );
    // Text changes invalidate geometric selection before the replacement can be copied.
    apply(cx, window, vec![Op::SetText(id(2), "replacement β".into())]);
    frame(cx, window).await;
    assert_eq!(
        copy(cx, window),
        "unchanged",
        "source replacement retires old range"
    );
    drag(cx, window, (id(2), 0), (id(4), 5)).await;
    apply(
        cx,
        window,
        vec![Op::SetStyle(
            id(3),
            vec![Style::Fields(vec![Field::UserSelect(false)])],
        )],
    );
    frame(cx, window).await;
    assert_eq!(
        copy(cx, window),
        "replacement β\nomega",
        "deselected middle participant cannot be copied"
    );
    apply(cx, window, vec![Op::SetStyle(id(3), vec![])]);
    frame(cx, window).await;
    // Keyboard selection remains local and moves by grapheme, retiring shared geometry.
    window
        .update(cx, |view, window, cx| {
            window.focus(&view.selections[&id(3)].borrow().focus, cx)
        })
        .unwrap();
    press(cx, window, "end");
    press(cx, window, "shift-left");
    assert_eq!(
        copy(cx, window),
        "😀",
        "local keyboard selection excludes prior shared participants"
    );
    // Shift-click extends the keyboard anchor even across participant boundaries.
    let end = point_at(cx, window, id(4), 5);
    window
        .update(cx, |_, w, cx| {
            w.dispatch_event(
                gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
                    position: end,
                    button: gpui::MouseButton::Left,
                    modifiers: gpui::Modifiers {
                        shift: true,
                        ..Default::default()
                    },
                    click_count: 1,
                    first_mouse: false,
                }),
                cx,
            );
        })
        .unwrap();
    mouse(cx, window, end, false);
    frame(cx, window).await;
    assert_eq!(
        copy(cx, window),
        "omega",
        "Shift-click extends keyboard anchor in source order"
    );
    // Clipboard payloads preserve selected whitespace, including middle runs.
    apply(cx, window, vec![Op::SetText(id(3), " \t\n".into())]);
    frame(cx, window).await;
    window
        .update(cx, |view, window, cx| {
            window.focus(&view.selections[&id(3)].borrow().focus, cx)
        })
        .unwrap();
    press(cx, window, "secondary-a");
    assert_eq!(
        copy(cx, window),
        " \t\n",
        "whitespace-only local Copy is exact"
    );
    apply(cx, window, vec![Op::SetText(id(3), "   ".into())]);
    frame(cx, window).await;
    drag(cx, window, (id(2), 0), (id(4), 5)).await;
    assert_eq!(
        copy(cx, window),
        "replacement β\n   \nomega",
        "shared Copy retains a whitespace-only middle participant"
    );
    // Real native shaping must preserve source offsets for both ellipsis sides.
    let source = "α prefix repeated repeated repeated suffix γ";
    for overflow in [1, 2] {
        apply(
            cx,
            window,
            vec![
                Op::SetText(id(2), source.into()),
                Op::SetStyle(
                    id(2),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(150.)),
                        Field::WhiteSpace(1),
                        Field::TextOverflow(overflow),
                        Field::OverflowX(2),
                    ])],
                ),
            ],
        );
        frame(cx, window).await;
        let displayed = window
            .update(cx, |view, _, _| {
                view.selections[&id(2)].borrow().displayed_text().to_owned()
            })
            .unwrap();
        assert!(
            displayed.len() < source.len(),
            "fixture must actually truncate: {displayed:?}"
        );
        let (start, end, expected) = if overflow == 1 {
            let retained = displayed.strip_suffix('…').expect("end ellipsis");
            assert!(source.starts_with(retained));
            (0, retained.len(), retained)
        } else {
            let retained = displayed.strip_prefix('…').expect("start ellipsis");
            assert!(source.ends_with(retained));
            (source.len() - retained.len(), source.len(), retained)
        };
        assert!(!expected.is_empty());
        drag(cx, window, (id(2), start), (id(2), end)).await;
        assert_eq!(
            copy(cx, window),
            expected,
            "ellipsis drag copies real retained bytes"
        );
        drag(cx, window, (id(2), end), (id(2), start)).await;
        assert_eq!(
            copy(cx, window),
            expected,
            "reverse ellipsis drag preserves source bytes"
        );
        press(cx, window, "secondary-a");
        assert_eq!(
            copy(cx, window),
            source,
            "keyboard Select All includes hidden source"
        );
    }
    apply(
        cx,
        window,
        vec![
            Op::SetText(id(2), "alpha β".into()),
            Op::SetStyle(id(2), vec![]),
            Op::SetText(id(3), "middle 😀".into()),
        ],
    );
    frame(cx, window).await;
    drag(cx, window, (id(2), 2), (id(4), 5)).await;
    apply(
        cx,
        window,
        vec![Op::Splice(id(0), 0, 4, vec![id(4), id(3), id(5), id(2)])],
    );
    frame(cx, window).await;
    assert_eq!(
        copy(cx, window),
        " ζ\nmiddle 😀\nal",
        "reordered participants resolve retained endpoints in their new painted order"
    );
    apply(
        cx,
        window,
        vec![Op::SetStyle(
            id(4),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    frame(cx, window).await;
    window
        .update(cx, |_, w, cx| {
            assert!(
                gpui_base::TextSelection::selected_text(w, cx).is_empty(),
                "hiding an endpoint retires shared selection"
            );
        })
        .unwrap();
    apply(cx, window, vec![Op::SetStyle(id(4), vec![])]);
    frame(cx, window).await;
    window
        .update(cx, |_, w, cx| {
            assert!(
                gpui_base::TextSelection::selected_text(w, cx).is_empty(),
                "showing a retired endpoint must not resurrect selection"
            );
        })
        .unwrap();
    apply(
        cx,
        window,
        vec![Op::Splice(id(0), 0, 4, vec![id(2), id(5), id(3), id(4)])],
    );
    frame(cx, window).await;
    drag(cx, window, (id(2), 2), (id(4), 5)).await;
    // Local selection participates in the same modal scope gate as Markdown.
    apply(
        cx,
        window,
        vec![
            Op::Create(id(6), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                id(6),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Create(id(7), Kind::Button, "Modal".into(), None),
            Op::Splice(id(6), 0, 0, vec![id(7)]),
            Op::Splice(id(0), 4, 0, vec![id(6)]),
        ],
    );
    frame(cx, window).await;
    window
        .update(cx, |_, w, cx| {
            assert!(gpui_base::TextSelection::selected_text(w, cx).is_empty())
        })
        .unwrap();
    apply(
        cx,
        window,
        vec![
            Op::Splice(id(0), 4, 1, vec![]),
            Op::Remove(id(7)),
            Op::Remove(id(6)),
        ],
    );
    frame(cx, window).await;
    window
        .update(cx, |_, w, cx| {
            assert!(gpui_base::TextSelection::selected_text(w, cx).is_empty())
        })
        .unwrap();
    isolation::exercise(cx, window).await;
    typography::exercise(cx, window).await;
    cx.update(|cx| {
        cx.write_to_clipboard(
            saved.unwrap_or_else(|| gpui::ClipboardItem::new_string(String::new())),
        )
    });
    eprintln!(
        "GPUIO_SELECTION_WINDOW_OK: forward/reverse cross-node Unicode Copy, whitespace preservation, exclusions, source retirement, local grapheme/Shift-click, native ellipsis mapping, reorder, hidden endpoint retirement, and modal clearing"
    );
}
