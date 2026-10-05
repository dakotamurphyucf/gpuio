//! Rendered TestPlatform layout; no physical macOS input or visual acceptance.
use super::super::View;
use super::*;
use crate::session::Session;
use gpui::{TestAppContext, VisualTestContext, point, px};
use gpuio_protocol::{
    HandlerId,
    text_area_layout::{Config, WrappingIndent},
};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn editor_config() -> EditorConfig {
    EditorConfig {
        label: "Notes".into(),
        placeholder: "".into(),
        read_only: false,
        disabled: false,
        submit_on_enter: false,
        auto_focus: false,
        min_rows: 8,
        max_rows: 8,
    }
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
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
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn state(owner: &Entity<View>, cx: &mut VisualTestContext) -> Entity<TextareaState> {
    cx.update(|_, cx| {
        let State::Textarea(state) = &owner.read(cx).editors[&id(1)].state else {
            unreachable!()
        };
        state.clone()
    })
}
fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn setup(text: &str, test: impl FnOnce(Entity<View>, &mut VisualTestContext)) {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Textarea layout", 500., 400.)
        .unwrap();
    let (owner, cx) = app.add_window_view(|_, _| View::new(wid, session, transport));
    apply(
        &owner,
        cx,
        vec![
            Op::Create(id(0), Kind::Container, "".into(), None),
            Op::Create(
                id(1),
                Kind::Textarea,
                text.into(),
                Some(HandlerId::from_parts(1, 1).unwrap()),
            ),
            Op::SetEditor(id(1), editor_config()),
            Op::SetStyle(
                id(1),
                vec![
                    Style::Width(Length::Px(240.)),
                    Style::Height(Length::Px(200.)),
                ],
            ),
            Op::Splice(id(0), 0, 0, vec![id(1)]),
            Op::SetRoot(Some(id(0))),
        ],
    );
    let state = state(&owner, cx);
    cx.update(|window, cx| {
        state.update(cx, |state, cx| {
            state.focus(window, cx);
            assert!(state.bridge_select(0, 0, cx));
        })
    });
    draw(cx);
    test(owner, cx);
}
fn set_layout(owner: &Entity<View>, cx: &mut VisualTestContext, layout: Option<Config>) {
    apply(owner, cx, vec![Op::SetTextAreaLayout(id(1), layout)]);
}

#[test]
fn textarea_accessible_selection_preserves_unicode_direction_and_rejects_stale_runs() {
    use gpui::accesskit::{self, Action, ActionData, ActionRequest, TextPosition, TextSelection};
    setup("λ🙂\n日本語\n", |owner, cx| {
        cx.simulate_a11y_active(true);
        draw(cx);
        let tree = cx.a11y_tree().unwrap();
        let (editor_id, node) = tree
            .nodes
            .iter()
            .find(|(_, n)| n.label() == Some("Notes"))
            .unwrap();
        let editor_id = *editor_id;
        assert!(node.supports_action(Action::SetTextSelection));
        let start = node.text_selection().unwrap().anchor;
        let (run_id, _) = tree
            .nodes
            .iter()
            .find(|(_, n)| n.role() == accesskit::Role::TextRun && n.value() == Some("日本語\n"))
            .unwrap();
        let selection = TextSelection {
            anchor: TextPosition {
                node: *run_id,
                character_index: 2,
            },
            focus: start,
        };
        let request = |selection: TextSelection| ActionRequest {
            action: Action::SetTextSelection,
            target_node: editor_id,
            target_tree: accesskit::TreeId::ROOT,
            data: Some(ActionData::SetTextSelection(selection)),
        };
        cx.simulate_a11y_action(request(selection));
        cx.run_until_parked();
        let editor = state(&owner, cx);
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.bridge_selection(), (13, 0))
        });
        draw(cx);
        let tree = cx.a11y_tree().unwrap();
        assert_eq!(
            tree.nodes
                .iter()
                .find(|(id, _)| *id == editor_id)
                .unwrap()
                .1
                .text_selection(),
            Some(&selection)
        );

        // Read-only permits selection, but disabled rejects the same request.
        let mut config = editor_config();
        config.read_only = true;
        apply(&owner, cx, vec![Op::SetEditor(id(1), config.clone())]);
        let caret = TextSelection {
            anchor: start,
            focus: start,
        };
        cx.simulate_a11y_action(request(caret));
        cx.run_until_parked();
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.bridge_selection(), (0, 0))
        });
        config.disabled = true;
        apply(&owner, cx, vec![Op::SetEditor(id(1), config)]);
        cx.simulate_a11y_action(request(selection));
        cx.run_until_parked();
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.bridge_selection(), (0, 0))
        });

        apply(&owner, cx, vec![Op::SetEditor(id(1), editor_config())]);
        cx.update(|window, cx| {
            editor.update(cx, |state, cx| {
                state.bridge_replace_all("replacement".into(), (3, 3), true, window, cx);
            })
        });
        draw(cx);
        cx.simulate_a11y_action(request(selection));
        cx.run_until_parked();
        editor.read_with(cx, |editor, _| {
            assert_eq!(editor.bridge_selection(), (3, 3))
        });

        // Even a current-tree request must not move the IME's marked selection.
        cx.update(|window, cx| {
            editor.update(cx, |state, cx| {
                state.replace_and_mark_text_in_range(None, "に", Some(0..1), window, cx);
            })
        });
        draw(cx);
        let before = editor.read_with(cx, |editor, _| {
            (editor.bridge_selection(), editor.bridge_composition())
        });
        let tree = cx.a11y_tree().unwrap();
        let mut composing = *tree
            .nodes
            .iter()
            .find(|(id, _)| *id == editor_id)
            .unwrap()
            .1
            .text_selection()
            .unwrap();
        composing.anchor.character_index = 0;
        composing.focus = composing.anchor;
        cx.simulate_a11y_action(request(composing));
        cx.run_until_parked();
        editor.read_with(cx, |editor, _| {
            assert_eq!(
                (editor.bridge_selection(), editor.bridge_composition()),
                before
            );
        });
    });
}
fn points(
    state: &Entity<TextareaState>,
    cx: &mut VisualTestContext,
    length: usize,
) -> Vec<gpui::Point<gpui::Pixels>> {
    cx.update(|_, cx| {
        (0..=length)
            .map(|offset| {
                state
                    .read(cx)
                    .range_to_bounds(&(offset..offset))
                    .expect("rendered ASCII caret")
                    .origin
            })
            .collect()
    })
}

#[test]
fn textarea_wrap_indent_and_horizontal_scroll_survive_unrelated_configuration() {
    let text = "    alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi";
    setup(text, |owner, cx| {
        let field = state(&owner, cx);
        let original = points(&field, cx, text.len());
        assert!(original.last().unwrap().y > original[0].y, "must soft wrap");
        let continuation_x = |positions: &[gpui::Point<gpui::Pixels>]| {
            positions
                .iter()
                .filter(|p| p.y > positions[0].y)
                .map(|p| p.x)
                .min()
                .unwrap()
        };
        let indented = continuation_x(&original);
        set_layout(
            &owner,
            cx,
            Some(Config {
                wrapping_indent: WrappingIndent::FlushLeft,
                ..Config::default()
            }),
        );
        let flush = points(&field, cx, text.len());
        assert!(
            indented > continuation_x(&flush) + px(10.),
            "continuation indentation must affect geometry"
        );
        set_layout(
            &owner,
            cx,
            Some(Config {
                soft_wrap: false,
                ..Config::default()
            }),
        );
        let unwrapped = points(&field, cx, text.len());
        assert!(unwrapped.iter().all(|p| p.y == unwrapped[0].y));
        cx.update(|_, cx| {
            field.update(cx, |state, cx| {
                state.set_scroll_offset(point(px(-70.), px(0.)), cx)
            })
        });
        draw(cx);
        let before = cx.update(|_, cx| field.read(cx).scroll_offset());
        assert!(
            before.x < px(-50.),
            "long unwrapped line must scroll: {before:?}"
        );
        let mut config = editor_config();
        config.label = "Renamed notes".into();
        config.read_only = true;
        apply(&owner, cx, vec![Op::SetEditor(id(1), config)]);
        assert_eq!(state(&owner, cx), field);
        assert_eq!(cx.update(|_, cx| field.read(cx).scroll_offset()), before);
        set_layout(&owner, cx, None);
        assert_eq!(cx.update(|_, cx| field.read(cx).scroll_offset().x), px(0.));
        let restored = points(&field, cx, text.len());
        assert_eq!(original, restored);
    });
}

#[test]
fn textarea_explicit_cursor_margins_scroll_and_clamp_without_redraw_jitter() {
    let text = (0..60)
        .map(|n| format!("line {n:02}"))
        .collect::<Vec<_>>()
        .join("\n");
    let target = text.find("line 29").unwrap();
    setup(&text, |owner, cx| {
        let field = state(&owner, cx);
        let mut offsets = Vec::new();
        for margin in [0, 3, 256] {
            cx.update(|_, cx| {
                field.update(cx, |state, cx| {
                    assert!(state.bridge_select(0, 0, cx));
                })
            });
            draw(cx);
            set_layout(
                &owner,
                cx,
                Some(Config {
                    cursor_margin_lines: Some(margin),
                    ..Config::default()
                }),
            );
            cx.update(|_, cx| {
                field.update(cx, |state, cx| {
                    assert!(state.bridge_select(target, target, cx));
                })
            });
            draw(cx);
            cx.dispatch_action(gpui_base::input::MoveDown);
            draw(cx);
            let offset = cx.update(|_, cx| {
                let state = field.read(cx);
                assert!(state.visible_row_range().unwrap().contains(&30));
                state.scroll_offset()
            });
            for _ in 0..4 {
                draw(cx);
                assert_eq!(
                    cx.update(|_, cx| field.read(cx).scroll_offset()),
                    offset,
                    "stationary cursor must not oscillate"
                );
            }
            offsets.push(offset.y);
        }
        assert!(
            offsets[1] < offsets[0] - px(10.),
            "explicit margin must create more clearance: {offsets:?}"
        );
        assert!(
            offsets[2] <= offsets[1],
            "large margin centers within viewport: {offsets:?}"
        );
        cx.update(|_, cx| {
            let state = field.read(cx);
            let line_height = state.line_height().expect("rendered line height");
            let y = line_height * 30. + state.scroll_offset().y;
            let height = state.input_bounds().size.height;
            assert!(
                y >= px(0.) && y + line_height <= height,
                "oversized margin must keep caret visible: {y:?}/{height:?}"
            );
        });
    });
}

#[test]
fn textarea_layout_preserves_native_composition_selection_revision_focus_and_undo() {
    setup("draft", |owner, cx| {
        let field = state(&owner, cx);
        cx.update(|window, cx| {
            field.update(cx, |state, cx| {
                assert!(state.bridge_select(5, 5, cx));
                state.replace_and_mark_text_in_range(None, "jie", Some(0..3), window, cx);
            })
        });
        draw(cx);
        let before = cx.update(|window, cx| owner.read(cx).editors[&id(1)].snapshot(window, cx));
        assert!(before.composition.is_some());
        for layout in [
            Some(Config {
                soft_wrap: false,
                show_whitespace: true,
                wrapping_indent: WrappingIndent::FlushLeft,
                cursor_margin_lines: Some(3),
            }),
            None,
        ] {
            set_layout(&owner, cx, layout);
            assert_eq!(state(&owner, cx), field);
            assert_eq!(
                cx.update(|window, cx| owner.read(cx).editors[&id(1)].snapshot(window, cx)),
                before
            );
            cx.update(|window, cx| assert!(field.read(cx).focus_handle(cx).is_focused(window)));
        }
        cx.update(|window, cx| {
            field.update(cx, |state, cx| {
                state.replace_text_in_range(None, "界", window, cx);
                assert!(state.bridge_composition().is_none());
                assert_eq!(state.value().as_str(), "draft界");
                state.bridge_undo(window, cx);
                assert_eq!(state.value().as_str(), "draft");
                state.bridge_redo(window, cx);
                assert_eq!(state.value().as_str(), "draft界");
            })
        });
        draw(cx);
    });
}

#[test]
fn textarea_cancelled_recomposition_restores_committed_text_selection_and_history() {
    // Input methods may explicitly reconvert preceding text even when the
    // editor's current selection is collapsed. Cancellation must restore the
    // draft from before provisional replacement, not merely delete the mark.
    for explicit_replacement in [false, true] {
        let prefix = "prefix 🙂\n";
        setup(prefix, |owner, cx| {
            let field = state(&owner, cx);
            cx.update(|window, cx| {
                field.update(cx, |state, cx| {
                    assert!(state.bridge_select(prefix.len(), prefix.len(), cx));
                    state.replace_and_mark_text_in_range(None, "nihongo", Some(0..7), window, cx);
                    state.replace_text_in_range(None, "日本語", window, cx);
                    let committed = format!("{prefix}日本語");
                    assert_eq!(state.value().as_str(), committed);
                    state.bridge_undo(window, cx);
                    assert_eq!(state.value().as_str(), prefix);
                    state.bridge_redo(window, cx);
                    assert_eq!(state.value().as_str(), committed);
                    let (anchor, head, replacement) = if explicit_replacement {
                        let start = prefix.encode_utf16().count();
                        (committed.len(), committed.len(), Some(start..start + 3))
                    } else {
                        (committed.len(), prefix.len(), None)
                    };
                    assert!(state.bridge_select(anchor, head, cx));
                    state.replace_and_mark_text_in_range(replacement, "に", Some(0..1), window, cx);
                    assert_eq!(state.value().as_str(), format!("{prefix}に"));
                    state.replace_and_mark_text_in_range(None, "", None, window, cx);
                    assert_eq!(
                        state.value().as_str(),
                        committed,
                        "cancelled composition erased committed text"
                    );
                    assert_eq!(state.bridge_selection(), (anchor, head));
                    assert!(state.bridge_composition().is_none());
                    // The cancelled provisional edit must add no undo entry.
                    state.bridge_undo(window, cx);
                    assert_eq!(state.value().as_str(), prefix);
                    state.bridge_redo(window, cx);
                    assert_eq!(state.value().as_str(), committed);
                });
            });
        });
    }
}
