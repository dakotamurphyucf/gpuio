//! Rendered host view and TestPlatform input/clipboard, not physical macOS input.
use super::super::View;
use super::*;
use crate::session::Session;
use gpui::{TestAppContext, VisualTestContext};
use gpuio_protocol::{
    HandlerId,
    input_format::{Config, Number},
};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn apply(view: &mut View, window: &mut Window, cx: &mut Context<View>, operations: Vec<Op>) {
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
}
fn command(
    view: &mut View,
    slot: i64,
    command: EditorCommand,
    window: &mut Window,
    cx: &mut App,
) -> EditorSnapshot {
    match view
        .editors
        .get_mut(&id(slot))
        .unwrap()
        .command(&command, window, cx)
    {
        EditorResult::Applied(snapshot) => snapshot,
        other => panic!("unexpected editor command: {other:?}"),
    }
}
fn mount(format: Config) -> Vec<Op> {
    let mut ops = vec![Op::Create(id(0), Kind::Container, "".into(), None)];
    for slot in 1..=2 {
        ops.extend([
            Op::Create(
                id(slot),
                Kind::Input,
                "".into(),
                Some(HandlerId::from_parts(slot, 1).unwrap()),
            ),
            Op::SetEditor(
                id(slot),
                EditorConfig {
                    label: if slot == 1 {
                        "Formatted draft".into()
                    } else {
                        "Reference draft".into()
                    },
                    placeholder: "Original placeholder".into(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: false,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::SetStyle(
                id(slot),
                vec![
                    Style::Width(Length::Px(360.)),
                    Style::Height(Length::Px(42.)),
                ],
            ),
        ]);
    }
    ops.extend([
        Op::SetEditorFormat(id(1), Some(format)),
        Op::Splice(id(0), 0, 0, vec![id(1), id(2)]),
        Op::SetRoot(Some(id(0))),
    ]);
    ops
}
fn snapshot(owner: &Entity<View>, cx: &mut VisualTestContext) -> EditorSnapshot {
    cx.update(|window, cx| owner.read(cx).editors[&id(1)].snapshot(window, cx))
}
fn draw_and_check_geometry(owner: &Entity<View>, cx: &mut VisualTestContext, expected: &str) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            command(
                view,
                2,
                EditorCommand::Replace(
                    expected.into(),
                    EditorSelectionPolicy::Start,
                    EditorUndoPolicy::Reset,
                    None,
                ),
                window,
                cx,
            );
        });
        window.draw(cx).clear(cx);
        assert_eq!(
            owner.read(cx).editors[&id(1)].snapshot(window, cx).text,
            expected
        );
        let states: Vec<_> = [1, 2]
            .into_iter()
            .map(|slot| {
                let State::Input(state) = &owner.read(cx).editors[&id(slot)].state else {
                    unreachable!()
                };
                state.clone()
            })
            .collect();
        let positions: Vec<Vec<f32>> = states
            .into_iter()
            .map(|entity| {
                entity.update(cx, |state, cx| {
                    let bounds = state.input_bounds();
                    assert!(bounds.size.width > gpui::px(100.));
                    let mut offsets = vec![0];
                    let mut offset = 0;
                    for c in expected.chars() {
                        offset += c.len_utf16();
                        offsets.push(offset);
                    }
                    offsets
                        .into_iter()
                        .map(|offset| {
                            let caret = state
                                .bounds_for_range(offset..offset, bounds, window, cx)
                                .expect("rendered caret");
                            assert!(caret.origin.x >= bounds.origin.x);
                            assert!(caret.origin.x <= bounds.right());
                            f32::from(caret.origin.x - bounds.origin.x)
                        })
                        .collect()
                })
            })
            .collect();
        assert_eq!(positions[0].len(), positions[1].len());
        for (actual, reference) in positions[0].iter().zip(&positions[1]) {
            assert!(
                (actual - reference).abs() < 0.1,
                "caret differs from freshly rendered text: {positions:?}"
            );
        }
        assert!(
            positions[0].windows(2).all(|pair| pair[0] < pair[1]),
            "collapsed/stale shaped text: {positions:?}"
        );
    });
    cx.run_until_parked();
    let tree = cx.a11y_tree().unwrap();
    let field = tree
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Formatted draft"))
        .unwrap();
    assert_eq!(field.1.value(), Some(expected));
    assert_eq!(field.1.placeholder(), Some("Original placeholder"));
}

#[test]
fn input_format_rendered_paste_copy_cut_and_middle_edits_keep_shaped_text_current() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Format rendering", 500., 300.)
        .unwrap();
    let (owner, cx) = app.add_window_view(|_, _| View::new(wid, session.clone(), transport));
    cx.simulate_a11y_active(true);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                mount(Config::Number(Number {
                    separator: Some(",".into()),
                    fraction_digits: Some(2),
                })),
            );
            command(view, 1, EditorCommand::Focus, window, cx);
        });
        window.draw(cx).clear(cx);
        cx.write_to_clipboard(gpui::ClipboardItem::new_string("1234.50".into()));
    });
    cx.dispatch_action(gpui_base::input::Paste);
    draw_and_check_geometry(&owner, cx, "1,234.50");
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            command(
                view,
                1,
                EditorCommand::Select(EditorSelection { anchor: 3, head: 3 }),
                window,
                cx,
            );
        });
        window.draw(cx).clear(cx);
    });
    cx.simulate_input("9");
    draw_and_check_geometry(&owner, cx, "12,934.50");
    assert_eq!(
        snapshot(&owner, cx).selection,
        EditorSelection { anchor: 4, head: 4 }
    );
    cx.dispatch_action(gpui_base::input::Backspace);
    draw_and_check_geometry(&owner, cx, "1,234.50");
    cx.simulate_input("9");
    draw_and_check_geometry(&owner, cx, "12,934.50");
    let before = snapshot(&owner, cx);
    cx.update(|_, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string("bad".into())));
    cx.dispatch_action(gpui_base::input::Paste);
    assert_eq!(snapshot(&owner, cx), before);
    cx.dispatch_action(gpui_base::input::SelectAll);
    cx.dispatch_action(gpui_base::input::Copy);
    cx.update(|_, cx| {
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("12,934.50")
        )
    });
    cx.dispatch_action(gpui_base::input::Cut);
    draw_and_check_geometry(&owner, cx, "");
    cx.dispatch_action(gpui_base::input::Undo);
    draw_and_check_geometry(&owner, cx, "12,934.50");
    let before = snapshot(&owner, cx);
    let focus = cx.update(|_, cx| owner.read(cx).editors[&id(1)].focus_handle(cx));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetEditorFormat(
                    id(1),
                    Some(Config::Pattern("*–99".into())),
                )],
            );
            assert_eq!(view.editors[&id(1)].snapshot(window, cx), before);
            assert_eq!(view.editors[&id(1)].focus_handle(cx), focus);
        });
        window.draw(cx).clear(cx);
        cx.write_to_clipboard(gpui::ClipboardItem::new_string("界12".into()));
    });
    cx.dispatch_action(gpui_base::input::SelectAll);
    cx.dispatch_action(gpui_base::input::Paste);
    draw_and_check_geometry(&owner, cx, "界–12");
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(view, window, cx, vec![Op::SetEditorFormat(id(1), None)]);
        });
        window.draw(cx).clear(cx);
        cx.write_to_clipboard(gpui::ClipboardItem::new_string("Free text".into()));
    });
    cx.dispatch_action(gpui_base::input::SelectAll);
    cx.dispatch_action(gpui_base::input::Paste);
    draw_and_check_geometry(&owner, cx, "Free text");
    let live = cx.update(|_, cx| owner.read(cx).editors[&id(1)].liveness_probe());
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::Splice(id(0), 0, 1, vec![]), Op::Remove(id(1))],
            )
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    assert!(!live());
}

#[test]
fn input_format_accessibility_value_actions_use_current_policy_and_retired_targets_are_inert() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Format accessibility", 500., 300.)
        .unwrap();
    let (owner, cx) = app.add_window_view(|_, _| View::new(wid, session.clone(), transport));
    cx.simulate_a11y_active(true);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(view, window, cx, mount(Config::Pattern("99-99".into())));
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    let target = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Formatted draft"))
        .unwrap()
        .0;
    let action = |text: &str| gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::SetValue,
        target_node: target,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: Some(gpui::accesskit::ActionData::Value(text.into())),
    };
    let before = snapshot(&owner, cx);
    cx.simulate_a11y_action(action("1234"));
    cx.run_until_parked();
    assert_eq!(
        snapshot(&owner, cx),
        before,
        "AX must not silently format an exact value"
    );
    cx.simulate_a11y_action(action("12-34"));
    cx.run_until_parked();
    draw_and_check_geometry(&owner, cx, "12-34");
    let before = snapshot(&owner, cx);
    // Queue the action, then change policy before its asynchronous delivery.
    cx.simulate_a11y_action(action("56-78"));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetEditorFormat(
                    id(1),
                    Some(Config::Pattern("AAAA".into())),
                )],
            );
        });
    });
    cx.run_until_parked();
    assert_eq!(snapshot(&owner, cx), before);
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    cx.simulate_a11y_action(action("abcd"));
    cx.run_until_parked();
    draw_and_check_geometry(&owner, cx, "abcd");
    let before = snapshot(&owner, cx);
    cx.simulate_a11y_action(action("efgh"));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetEditorValidation(
                    id(1),
                    Some(gpuio_protocol::input_validation::Rule {
                        regex: gpuio_protocol::input_validation::Source {
                            pattern: "[w-z]*".into(),
                            matching: gpuio_protocol::input_validation::Matching::WholeValue,
                            case_sensitive: true,
                        },
                        allow_empty: true,
                    }),
                )],
            );
        });
    });
    cx.run_until_parked();
    assert_eq!(
        snapshot(&owner, cx),
        before,
        "queued AX replacement must use the new edit filter"
    );
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
    });
    cx.simulate_a11y_action(action("wxyz"));
    cx.run_until_parked();
    draw_and_check_geometry(&owner, cx, "wxyz");
    let retired_policy = cx.update(|_, cx| {
        Arc::downgrade(owner.read(cx).editors[&id(1)].validation.as_ref().unwrap())
    });
    let sibling = cx.update(|window, cx| owner.read(cx).editors[&id(2)].snapshot(window, cx));
    cx.simulate_a11y_action(action("efgh"));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::Splice(id(0), 0, 1, vec![]), Op::Remove(id(1))],
            )
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        assert_eq!(owner.read(cx).editors[&id(2)].snapshot(window, cx), sibling);
        assert!(!owner.read(cx).editors.contains_key(&id(1)));
    });
    assert!(
        retired_policy.upgrade().is_none(),
        "removed editor releases its compiled filter and cache"
    );
}
