//! Embedded native ownership, focus and repeated command dispatch.
use super::*;

fn embedded() -> palette_options::Config {
    palette_options::Config {
        presentation: palette_options::Presentation::Embedded,
        escape: palette_options::Escape::ClearQueryFirst,
        ..Default::default()
    }
}
fn key(cx: &mut VisualTestContext, key: &str) {
    cx.simulate_event(gpui::KeyDownEvent {
        keystroke: gpui::Keystroke::parse(key).unwrap(),
        is_held: false,
        prefer_character_input: false,
    });
    draw(cx);
}
fn events(owner: &Entity<View>, cx: &VisualTestContext) -> Vec<Event> {
    owner.read_with(cx, |view, _| {
        view.transport.mailbox.lock().unwrap().drain(128)
    })
}

#[test]
fn embedded_palette_coexists_with_sibling_focus_and_repeats_selection() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_extra(
        &mut app,
        vec![
            Op::SetPaletteOptions(id(1), Some(embedded())),
            Op::Create(
                id(2),
                Kind::Button,
                "Outside".into(),
                Some(HandlerId::from_parts(2, 1).unwrap()),
            ),
            Op::Splice(id(0), 1, 0, vec![id(2)]),
        ],
    );
    cx.simulate_a11y_active(true);
    draw(&mut cx);
    let input = owner.read_with(&cx, |view, _| view.palettes[&id(1)].query.clone());
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            assert!(
                !input.read(cx).focus_handle(cx).is_focused(w),
                "mount must not steal focus"
            );
            assert!(view.focus.borrow().allows(id(2)));
            assert!(!view.focus.borrow().top_overlay(id(1)));
            w.focus(&input.read(cx).focus_handle(cx), cx);
        })
    });
    key(&mut cx, "tab");
    cx.update(|w, cx| assert!(!input.read(cx).focus_handle(cx).is_focused(w)));
    key(&mut cx, "shift-tab");
    cx.update(|w, cx| assert!(input.read(cx).focus_handle(cx).is_focused(w)));
    query(&owner, &mut cx, "run");
    events(&owner, &cx);
    key(&mut cx, "enter");
    key(&mut cx, "enter");
    let delivered = events(&owner, &cx);
    assert_eq!(
        delivered
            .iter()
            .filter(|e| matches!(e,Event::CommandInvoked(_,_,_,_,command,_,_) if command=="run"))
            .count(),
        2
    );
    assert!(
        !delivered
            .iter()
            .any(|e| matches!(e, Event::PaletteDismissed(..)))
    );
    owner.read_with(&cx, |view, cx| {
        assert!(!view.palettes[&id(1)].closed);
        assert_eq!(input.read(cx).value().as_str(), "run");
    });
    key(&mut cx, "escape");
    assert!(
        !events(&owner, &cx)
            .iter()
            .any(|e| matches!(e, Event::PaletteDismissed(..)))
    );
    key(&mut cx, "escape");
    assert_eq!(
        events(&owner, &cx)
            .iter()
            .filter(|e| matches!(
                e,
                Event::PaletteDismissed(_, _, _, _, PaletteDismissal::Escape)
            ))
            .count(),
        1
    );
    owner.read_with(&cx, |view, _| assert!(!view.palettes[&id(1)].closed));
    let tree = cx.a11y_tree().unwrap();
    assert!(
        tree.nodes
            .iter()
            .any(|(_, node)| node.label() == Some("Actions") && node.role() == gpui::Role::Group)
    );
}

#[test]
fn embedded_palette_retains_hidden_query_and_survives_presentation_changes() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_extra(
        &mut app,
        vec![Op::SetPaletteOptions(id(1), Some(embedded()))],
    );
    let input = owner.read_with(&cx, |view, _| view.palettes[&id(1)].query.clone());
    query(&owner, &mut cx, "run");
    apply(
        &owner,
        &mut cx,
        vec![Op::SetStyle(
            id(1),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    owner.read_with(&cx, |view, cx| {
        assert!(!view.palettes[&id(1)].closed);
        assert!(!view.palette_interactive(id(1)));
        assert_eq!(input.read(cx).value().as_str(), "run");
    });
    apply(&owner, &mut cx, vec![Op::SetStyle(id(1), vec![])]);
    assert_eq!(rows(&owner, &cx), ["run"]);
    for field in [Field::Disabled(true), Field::Inert(true)] {
        apply(
            &owner,
            &mut cx,
            vec![Op::SetStyle(id(1), vec![Style::Fields(vec![field])])],
        );
        owner.read_with(&cx, |view, _| {
            assert!(!view.palettes[&id(1)].closed);
            assert!(!view.palette_interactive(id(1)));
        });
        apply(&owner, &mut cx, vec![Op::SetStyle(id(1), vec![])]);
    }
    let mut modal = embedded();
    modal.presentation = palette_options::Presentation::Modal;
    apply(
        &owner,
        &mut cx,
        vec![Op::SetPaletteOptions(id(1), Some(modal))],
    );
    owner.read_with(&cx, |view, _| {
        assert!(view.focus.borrow().top_overlay(id(1)))
    });
    apply(
        &owner,
        &mut cx,
        vec![Op::SetPaletteOptions(id(1), Some(embedded()))],
    );
    owner.read_with(&cx, |view, cx| {
        assert!(!view.focus.borrow().top_overlay(id(1)));
        assert_eq!(view.palettes[&id(1)].query.entity_id(), input.entity_id());
        assert_eq!(input.read(cx).value().as_str(), "run");
    });
}

#[test]
fn embedded_palette_rejects_activation_beneath_another_modal() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_extra(
        &mut app,
        vec![Op::SetPaletteOptions(id(1), Some(embedded()))],
    );
    let route = owner.read_with(&cx, |view, _| view.palettes[&id(1)].rows[0].route.clone());
    apply(
        &owner,
        &mut cx,
        vec![
            Op::Create(
                id(2),
                Kind::FocusScope,
                "".into(),
                Some(HandlerId::from_parts(2, 1).unwrap()),
            ),
            Op::SetFocusScope(
                id(2),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::SetOverlay(
                id(2),
                Some(OverlayConfig {
                    kind: OverlayKind::Dialog,
                    label: "Cover".into(),
                    width: 300.,
                    dismiss_on_escape: true,
                    dismiss_on_outside_pointer: false,
                }),
            ),
            Op::Splice(id(0), 1, 0, vec![id(2)]),
        ],
    );
    events(&owner, &cx);
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            assert!(!view.palette_interactive(id(1)));
            view.select_palette(id(1), &route, w, cx);
            assert!(!view.palettes[&id(1)].closed);
        })
    });
    assert!(
        !events(&owner, &cx)
            .iter()
            .any(|e| matches!(e, Event::CommandInvoked(..)))
    );
    apply(
        &owner,
        &mut cx,
        vec![Op::Splice(id(0), 1, 1, vec![]), Op::Remove(id(2))],
    );
    cx.update(|w, cx| owner.update(cx, |view, cx| view.select_palette(id(1), &route, w, cx)));
    assert_eq!(
        events(&owner, &cx)
            .iter()
            .filter(|e| matches!(e, Event::CommandInvoked(..)))
            .count(),
        1
    );
}

#[test]
fn embedded_palette_native_edit_targets_document_and_keeps_query() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_document(&mut app);
    let input = owner.read_with(&cx, |view, _| view.palettes[&id(1)].query.clone());
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            w.focus(&view.editors[&id(2)].focus_handle(cx), cx);
        })
    });
    draw(&mut cx);
    cx.update(|w, cx| w.focus(&input.read(cx).focus_handle(cx), cx));
    query(&owner, &mut cx, "run");
    key(&mut cx, "enter");
    cx.update(|w, cx| {
        owner.read_with(cx, |view, cx| {
            assert!(!view.palettes[&id(1)].closed);
            assert_eq!(input.read(cx).value().as_str(), "run");
            assert!(view.editors[&id(2)].focus_handle(cx).is_focused(w));
            let snapshot = view.editors[&id(2)].snapshot(w, cx);
            assert_eq!(snapshot.selection.anchor, 0);
            assert_eq!(snapshot.selection.head, 8);
        })
    });
}

#[test]
fn embedded_palette_wheel_consumes_movement_and_bubbles_at_boundary() {
    let mut app = TestAppContext::single();
    let registry = (0..40)
        .map(|i| CommandConfig {
            id: format!("cmd-{i}"),
            label: format!("Command {i}"),
            enabled: true,
            generation: 1,
            checked: None,
            shortcuts: vec![],
            target: CommandTarget::Callback,
        })
        .collect::<Vec<_>>();
    let (owner, mut cx, _reader) = mount_extra(
        &mut app,
        vec![
            Op::SetPaletteOptions(id(1), Some(embedded())),
            Op::SetPalette(
                id(1),
                PaletteConfig {
                    label: "Actions".into(),
                    placeholder: "Find".into(),
                    commands: registry.iter().map(|c| c.id.clone()).collect(),
                    dismiss_on_outside_pointer: true,
                },
            ),
            Op::SetCommands(id(0), registry),
            Op::SetStyle(
                id(0),
                vec![
                    Style::Direction(1),
                    Style::Fields(vec![
                        Field::Display(0),
                        Field::Width(Length::Px(400.)),
                        Field::Height(Length::Px(400.)),
                        Field::OverflowY(3),
                        Field::OverflowX(3),
                    ]),
                ],
            ),
            Op::SetStyle(id(1), vec![Style::Shrink(0.)]),
            Op::Create(id(2), Kind::Text, "Extent".into(), None),
            Op::SetStyle(
                id(2),
                vec![
                    Style::Width(Length::Px(900.)),
                    Style::Height(Length::Px(900.)),
                    Style::Shrink(0.),
                ],
            ),
            Op::Splice(id(0), 1, 0, vec![id(2)]),
        ],
    );
    let wheel = |cx: &mut VisualTestContext, x: f32, y: f32| {
        let position = owner.read_with(cx, |view, _| {
            view.palettes[&id(1)]
                .scroll
                .handle
                .viewport_bounds()
                .center()
        });
        cx.simulate_event(gpui::ScrollWheelEvent {
            position,
            delta: gpui::ScrollDelta::Pixels(gpui::point(px(x), px(y))),
            touch_phase: gpui::TouchPhase::Moved,
            modifiers: Default::default(),
        });
        draw(cx);
    };
    wheel(&mut cx, 0., -20.);
    owner.read_with(&cx, |view, _| {
        assert!(
            view.palettes[&id(1)]
                .scroll
                .handle
                .scroll_px_offset_for_scrollbar()
                .y
                < px(0.)
        );
        assert_eq!(view.scrolls[&id(0)].handle.offset(), gpui::Point::default());
    });
    cx.update(|w, cx| {
        owner.update(cx, |view, _| {
            view.palettes[&id(1)]
                .scroll
                .handle
                .scroll_to(gpui::ListOffset {
                    item_ix: 39,
                    offset_in_item: px(32.),
                });
            w.refresh();
        })
    });
    draw(&mut cx);
    wheel(&mut cx, 0., -20.);
    owner.read_with(&cx, |view, _| {
        assert_eq!(view.scrolls[&id(0)].handle.offset().y, px(-20.))
    });
    wheel(&mut cx, -10., 0.);
    owner.read_with(&cx, |view, _| {
        assert_eq!(view.scrolls[&id(0)].handle.offset().x, px(-10.))
    });
}

fn mount_document(app: &mut TestAppContext) -> (Entity<View>, VisualTestContext, UnixStream) {
    let mut registry = commands();
    registry[0].target = CommandTarget::Native(NativeCommand::SelectAll);
    registry[0].generation = 2;
    mount_extra(
        app,
        vec![
            Op::SetPaletteOptions(id(1), Some(embedded())),
            Op::SetCommands(id(0), registry),
            Op::Create(
                id(2),
                Kind::Input,
                "Document".into(),
                Some(HandlerId::from_parts(2, 1).unwrap()),
            ),
            Op::SetEditor(
                id(2),
                EditorConfig {
                    label: "Document".into(),
                    placeholder: "".into(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: true,
                    auto_focus: true,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::Splice(id(0), 1, 0, vec![id(2)]),
        ],
    )
}

#[test]
fn becoming_modal_captures_current_document_and_restores_focus() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_document(&mut app);
    let input = owner.read_with(&cx, |view, _| view.palettes[&id(1)].query.clone());
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            w.focus(&view.editors[&id(2)].focus_handle(cx), cx);
        })
    });
    draw(&mut cx);
    query(&owner, &mut cx, "run");
    let mut modal = embedded();
    modal.presentation = palette_options::Presentation::Modal;
    apply(
        &owner,
        &mut cx,
        vec![Op::SetPaletteOptions(id(1), Some(modal))],
    );
    cx.update(|w, cx| w.simulate_next_frame(cx));
    draw(&mut cx); // Deliver the post-paint focus callback, not only a repaint.
    cx.update(|w, cx| {
        owner.read_with(cx, |view, cx| {
            assert_eq!(view.palettes[&id(1)].editor, Some(id(2)));
            assert_eq!(view.palettes[&id(1)].query.entity_id(), input.entity_id());
            assert!(input.read(cx).focus_handle(cx).is_focused(w));
            assert!(!view.focus.borrow().allows(id(2)));
        })
    });
    key(&mut cx, "enter");
    cx.update(|w, cx| {
        owner.read_with(cx, |view, cx| {
            assert!(view.palettes[&id(1)].closed);
            assert!(view.editors[&id(2)].focus_handle(cx).is_focused(w));
            let snapshot = view.editors[&id(2)].snapshot(w, cx);
            assert_eq!(snapshot.selection.anchor, 0);
            assert_eq!(snapshot.selection.head, 8);
        })
    });
    assert!(events(&owner, &cx).iter().any(|e| matches!(e,
        Event::PaletteDismissed(_, _, _, _, PaletteDismissal::Selected(command)) if command == "run")));
}

#[test]
fn becoming_modal_escape_restores_the_outside_focus_at_transition() {
    for toggle_with_query_focused in [false, true] {
        let mut app = TestAppContext::single();
        let (owner, mut cx, _reader) = mount_document(&mut app);
        cx.update(|w, cx| {
            owner.update(cx, |view, cx| {
                w.focus(&view.editors[&id(2)].focus_handle(cx), cx);
            })
        });
        draw(&mut cx);
        let mut modal = embedded();
        modal.presentation = palette_options::Presentation::Modal;
        apply(
            &owner,
            &mut cx,
            vec![Op::SetPaletteOptions(id(1), Some(modal.clone()))],
        );
        cx.update(|w, cx| w.simulate_next_frame(cx));
        draw(&mut cx);
        if toggle_with_query_focused {
            apply(
                &owner,
                &mut cx,
                vec![Op::SetPaletteOptions(id(1), Some(embedded()))],
            );
            apply(
                &owner,
                &mut cx,
                vec![Op::SetPaletteOptions(id(1), Some(modal))],
            );
            cx.update(|w, cx| w.simulate_next_frame(cx));
            draw(&mut cx);
        }
        key(&mut cx, "escape");
        cx.update(|w, cx| {
            owner.read_with(cx, |view, cx| {
                assert!(view.palettes[&id(1)].closed);
                assert!(view.editors[&id(2)].focus_handle(cx).is_focused(w));
            })
        });
    }
}

#[test]
fn earlier_frame_callback_does_not_consume_new_modal_entry_before_paint() {
    let mut app = TestAppContext::single();
    let (owner, mut cx, _reader) = mount_document(&mut app);
    cx.update(|w, cx| {
        owner.update(cx, |view, cx| {
            let config = view.palettes[&id(1)].config.as_ref().clone();
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let applied = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: view.id,
                    base,
                    revision: base + 1,
                    operations: vec![
                        Op::Create(
                            id(3),
                            Kind::CommandPalette,
                            "".into(),
                            Some(HandlerId::from_parts(3, 1).unwrap()),
                        ),
                        Op::SetPalette(id(3), config),
                        Op::Splice(id(0), 2, 0, vec![id(3)]),
                    ],
                })
                .unwrap();
            view.update_editors(&applied.dirty, w, cx);
            // Deliver the previous frame's reconciliation at this exact boundary.
            // Keep delivery inside this update to pin it before any new paint.
            view.focus
                .borrow_mut()
                .finish_frame(view.root_focus.as_ref().unwrap(), w, cx);
            cx.notify();
        })
    });
    let input = owner.read_with(&cx, |view, _| view.palettes[&id(3)].query.clone());
    draw(&mut cx);
    cx.update(|w, cx| w.simulate_next_frame(cx));
    draw(&mut cx);
    cx.update(|w, cx| assert!(input.read(cx).focus_handle(cx).is_focused(w)));
}
