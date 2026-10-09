//! Rendered geometry regressions; TestPlatform is not physical display evidence.
use super::viewport_tests::{apply, request, setup, setup_kind, state};
use super::*;
use gpui::px;

fn draw(cx: &mut gpui::VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}

#[test]
fn editor_range_bounds_enclose_intermediate_lines_and_never_have_negative_width() {
    setup("abcdefghij\nx", |owner, cx| {
        let field = state(&owner, cx);
        cx.update(|_, cx| {
            let bounds = field.read(cx).range_to_bounds(&(5..12)).unwrap();
            assert!(bounds.size.width >= px(0.), "multiline bounds: {bounds:?}");
        });
    });
    let text = "a\nabcdefghijklmno\nz";
    setup(text, |owner, cx| {
        let field = state(&owner, cx);
        cx.update(|_, cx| {
            let state = field.read(cx);
            let middle = state.range_to_bounds(&(2..17)).unwrap();
            let all = state.range_to_bounds(&(0..text.len())).unwrap();
            assert!(
                all.left() <= middle.left() && all.right() >= middle.right(),
                "range {all:?} omits intermediate line {middle:?}"
            );
        });
    });
}

#[test]
fn editor_range_bounds_validate_unicode_and_invalidate_until_the_edited_source_is_painted() {
    setup("A界λ👩Z", |owner, cx| {
        let field = state(&owner, cx);
        cx.update(|window, cx| {
            field.update(cx, |state, cx| {
                let before = snapshot(state, window, cx);
                let history = state.bridge_history_bytes();
                for range in [0..1, 1..4, 4..6, 6..10, 0..11, 11..11] {
                    let bounds = state.range_to_bounds(&range).unwrap();
                    assert!(bounds.size.width >= px(0.) && bounds.size.height > px(0.));
                }
                for range in [
                    2..4,
                    1..3,
                    0..12,
                    12..12,
                    std::ops::Range { start: 5, end: 4 },
                ] {
                    assert!(state.range_to_bounds(&range).is_none(), "invalid {range:?}");
                }
                assert_eq!(snapshot(state, window, cx), before);
                assert_eq!(state.bridge_history_bytes(), history);
                state.bridge_replace_all("new".into(), (0, 0), true, window, cx);
                assert!(
                    state.range_to_bounds(&(0..1)).is_none(),
                    "old layout belongs to old source"
                );
            })
        });
        draw(cx);
        cx.update(|_, cx| assert!(field.read(cx).range_to_bounds(&(0..3)).is_some()));
    });
}

#[test]
fn editor_range_bounds_use_source_offsets_in_masked_input_and_reject_old_mask_layout() {
    setup_kind(Kind::Input, "ab界λ👩Z", |owner, cx| {
        let field = cx.update(|_, cx| {
            let State::Input(field) =
                &owner.read(cx).editors[&NodeId::from_parts(1, 1).unwrap()].state
            else {
                unreachable!()
            };
            field.clone()
        });
        cx.update(|window, cx| {
            field.update(cx, |state, cx| {
                state.set_masked(true, window, cx);
                assert!(state.range_to_bounds(&(0..1)).is_none());
            })
        });
        draw(cx);
        cx.update(|window, cx| {
            field.update(cx, |state, cx| {
                let before = snapshot(state, window, cx);
                let first = state.range_to_bounds(&(0..1)).unwrap();
                let second = state.range_to_bounds(&(1..2)).unwrap();
                let third = state.range_to_bounds(&(2..5)).unwrap();
                let fourth = state.range_to_bounds(&(5..7)).unwrap();
                assert!(first.size.width > px(0.));
                for (left, right) in [(first, second), (second, third), (third, fourth)] {
                    assert!((left.size.width - right.size.width).abs() < px(0.01));
                    assert!((left.right() - right.left()).abs() < px(0.01));
                }
                assert_eq!(snapshot(state, window, cx), before);
                state.set_masked(false, window, cx);
                assert!(state.range_to_bounds(&(0..1)).is_none());
            })
        });
        draw(cx);
        cx.update(|_, cx| assert!(field.read(cx).range_to_bounds(&(0..1)).is_some()));
    });
}

#[test]
fn editor_range_bounds_cover_soft_wraps_and_handle_wrap_boundary_affinity() {
    let text = "word ".repeat(24);
    setup(&text, |owner, cx| {
        let field = state(&owner, cx);
        cx.update(|_, cx| {
            let state = field.read(cx);
            let first = state.range_to_bounds(&(0..0)).unwrap();
            let boundary = (1..text.len())
                .find(|&offset| {
                    state.range_to_bounds(&(offset..offset)).unwrap().top() > first.top()
                })
                .unwrap();
            let one_row = state.range_to_bounds(&(0..boundary)).unwrap();
            let leading = state.range_to_bounds(&(boundary..boundary)).unwrap();
            assert_eq!(
                one_row.size.height, first.size.height,
                "exclusive end closes preceding row"
            );
            assert_eq!(leading.top(), first.bottom());
            let all = state.range_to_bounds(&(0..text.len())).unwrap();
            assert!(all.size.height > first.size.height);
            assert!(all.size.width >= one_row.size.width);
        });
    });
}

#[test]
fn editor_range_bounds_scroll_with_the_painted_layout_and_expire_off_layout() {
    let text = (0..60)
        .map(|n| format!("line {n:02}\n"))
        .collect::<String>();
    setup(&text, |owner, cx| {
        let field = state(&owner, cx);
        let range = 40..47; // line 05, initially visible and still laid out after scrolling.
        let before = cx.update(|_, cx| field.read(cx).range_to_bounds(&range).unwrap());
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                assert_eq!(
                    view.editors
                        .get_mut(&NodeId::from_parts(1, 1).unwrap())
                        .unwrap()
                        .command(
                            &EditorCommand::ScrollViewport(
                                gpuio_protocol::editor_viewport::Offset { x: 0., y: 100. }
                            ),
                            window,
                            cx
                        ),
                    EditorResult::ViewportScrollAccepted
                );
            });
            // Stay inside this update: TestPlatform may paint on return to the
            // event loop, so two separate updates do not establish pre-paint order.
            assert_eq!(field.read(cx).range_to_bounds(&range), Some(before));
        });
        draw(cx);
        let after = cx.update(|_, cx| field.read(cx).range_to_bounds(&range).unwrap());
        assert_eq!(before.top() - after.top(), px(100.));
        assert_eq!(before.size, after.size);
        request(
            &owner,
            cx,
            EditorCommand::ScrollViewport(gpuio_protocol::editor_viewport::Offset {
                x: 0.,
                y: 1e9,
            }),
        );
        draw(cx);
        cx.update(|_, cx| assert!(field.read(cx).range_to_bounds(&range).is_none()));
    });
}

#[test]
fn editor_range_bounds_keep_empty_carets_and_align_wrapped_rows_with_their_own_width() {
    setup("", |owner, cx| {
        let field = state(&owner, cx);
        cx.update(|_, cx| {
            let caret = field.read(cx).range_to_bounds(&(0..0)).unwrap();
            assert_eq!(caret.size.width, px(0.));
            assert!(caret.size.height > px(0.));
        });
    });
    setup(&"word ".repeat(13), |owner, cx| {
        let field = state(&owner, cx);
        for align in [1, 2] {
            // center, right
            apply(
                &owner,
                cx,
                vec![Op::SetStyle(
                    NodeId::from_parts(1, 1).unwrap(),
                    vec![
                        Style::Width(Length::Px(240.)),
                        Style::Height(Length::Px(200.)),
                        Style::Fields(vec![Field::TextAlign(align)]),
                    ],
                )],
            );
            cx.update(|_, cx| {
                let state = field.read(cx);
                // Final visual row is short. Its start and last glyph must align
                // together, rather than positioning its caret using the longest row.
                let end = state.range_to_bounds(&(65..65)).unwrap();
                let last_glyph = state.range_to_bounds(&(64..65)).unwrap();
                assert!((end.right() - last_glyph.right()).abs() < px(0.01));
                let whole = state.range_to_bounds(&(0..65)).unwrap();
                assert!(whole.left() <= last_glyph.left() && whole.right() >= last_glyph.right());
            });
        }
    });
}

#[test]
fn editor_range_command_rechecks_revision_boundaries_and_direction_without_editing() {
    setup("A界λ\nsecond", |owner, cx| {
        let field = state(&owner, cx);
        let before = cx.update(|w, cx| snapshot(field.read(cx), w, cx));
        let command = |revision, anchor, head| {
            EditorCommand::ReadRangeBounds(revision, EditorSelection { anchor, head })
        };
        let forward = request(&owner, cx, command(before.revision, 1, 4));
        assert_eq!(request(&owner, cx, command(before.revision, 4, 1)), forward);
        let EditorResult::RangeBounds(Some(geometry)) = forward else {
            panic!("missing range geometry")
        };
        assert_eq!(geometry.revision, before.revision);
        assert!(geometry.width > 0.);
        assert_eq!(
            request(&owner, cx, command(before.revision + 1, 1, 4)),
            EditorResult::Failed(EditorError::StaleRevision)
        );
        for (anchor, head) in [(2, 4), (0, 99), (-1, 0)] {
            assert_eq!(
                request(&owner, cx, command(before.revision, anchor, head)),
                EditorResult::Failed(EditorError::InvalidSelection)
            );
        }
        assert_eq!(cx.update(|w, cx| snapshot(field.read(cx), w, cx)), before);
        cx.update(|w, cx| {
            owner.update(cx, |view, cx| {
                let node = NodeId::from_parts(1, 1).unwrap();
                let editor = view.editors.get_mut(&node).unwrap();
                editor.mark_test_text("mark", w, cx);
                let revision = field.read(cx).bridge_revision();
                assert_eq!(
                    editor.command(&command(before.revision, 0, 0), w, cx),
                    EditorResult::Failed(EditorError::StaleRevision)
                );
                assert_eq!(
                    editor.command(&command(revision, 0, 0), w, cx),
                    EditorResult::RangeBounds(None)
                );
            })
        });
        draw(cx);
        let composing = cx.update(|w, cx| snapshot(field.read(cx), w, cx));
        assert!(composing.composition.is_some());
        assert!(matches!(
            request(&owner, cx, command(composing.revision, 0, 0)),
            EditorResult::RangeBounds(Some(_))
        ));
        assert_eq!(
            cx.update(|w, cx| snapshot(field.read(cx), w, cx)),
            composing
        );
    });
}

#[test]
fn editor_range_command_allows_disabled_reads_but_rejects_retired_native_identity() {
    setup("read only", |owner, cx| {
        let field = state(&owner, cx);
        let node = NodeId::from_parts(1, 1).unwrap();
        let config = cx.update(|_, cx| {
            let mut config = owner.read(cx).editors[&node].config.clone();
            config.disabled = true;
            config.read_only = true;
            config
        });
        apply(&owner, cx, vec![Op::SetEditor(node, config)]);
        let before = cx.update(|w, cx| snapshot(field.read(cx), w, cx));
        let command =
            EditorCommand::ReadRangeBounds(before.revision, EditorSelection { anchor: 0, head: 4 });
        assert!(matches!(
            request(&owner, cx, command.clone()),
            EditorResult::RangeBounds(Some(_))
        ));
        assert_eq!(cx.update(|w, cx| snapshot(field.read(cx), w, cx)), before);
        cx.update(|w, cx| {
            owner.update(cx, |view, cx| {
                let base = view.session.borrow().tree(view.id).unwrap().revision();
                view.session
                    .borrow_mut()
                    .apply(&Transaction {
                        window: view.id,
                        base,
                        revision: base + 1,
                        operations: vec![
                            Op::Splice(NodeId::from_parts(0, 1).unwrap(), 0, 1, vec![]),
                            Op::Remove(node),
                        ],
                    })
                    .unwrap();
                assert_eq!(
                    view.editors
                        .get_mut(&node)
                        .unwrap()
                        .command(&command, w, cx),
                    EditorResult::Failed(EditorError::StaleEditor)
                );
            })
        });
    });
}
