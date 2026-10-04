//! Native tabs on TestPlatform. Marked-text bridge tests are not OS IME evidence.
use super::*;
use gpuio_protocol::color_presentation::{Panels, Presentation};
fn tabs(initial: Panel) -> Presentation {
    Presentation {
        panels: Panels::Tabs {
            palette_label: "Swatches".into(),
            channels_label: "HSLA".into(),
            initial,
        },
        ..Presentation::default()
    }
}
fn configure(owner: &Entity<View>, cx: &mut VisualTestContext, initial: Panel) {
    apply(
        owner,
        cx,
        vec![Op::SetColorPresentation(node(), Some(tabs(initial)))],
    );
}
fn activate(cx: &mut VisualTestContext, label: &str, action: accesskit::Action) {
    let tree = cx.a11y_tree().unwrap();
    let target = tree
        .nodes
        .iter()
        .find(|(_, n)| n.role() == accesskit::Role::Tab && n.label() == Some(label))
        .unwrap()
        .0;
    cx.simulate_a11y_action(accesskit::ActionRequest {
        action,
        target_node: target,
        target_tree: accesskit::TreeId::ROOT,
        data: None,
    });
    draw(cx);
}
fn click_tab(cx: &mut VisualTestContext, label: &str) {
    let tree = cx.a11y_tree().unwrap();
    let bounds = tree
        .nodes
        .iter()
        .find(|(_, n)| n.role() == accesskit::Role::Tab && n.label() == Some(label))
        .unwrap()
        .1
        .bounds()
        .unwrap();
    let scale = cx.update(|w, _| f64::from(w.scale_factor()));
    let point = point(
        px(((bounds.x0 + bounds.x1) / (2. * scale)) as f32),
        px(((bounds.y0 + bounds.y1) / (2. * scale)) as f32),
    );
    cx.simulate_click(point, Modifiers::default());
    draw(cx);
}
fn edit(
    state: &Entity<ColorInput>,
    cx: &mut VisualTestContext,
    index: usize,
    text: &str,
    composing: bool,
) {
    let editor = state.read_with(cx, |s, _| s.editors.fields[index].state.clone());
    cx.update(|w, cx| {
        editor.update(cx, |e, cx| {
            let length = e.value().encode_utf16().count();
            if composing {
                e.replace_and_mark_text_in_range(Some(0..length), text, Some(0..text.len()), w, cx);
            } else {
                e.replace_text_in_range(Some(0..length), text, w, cx);
            }
        })
    });
}
fn color_events(transport: &Transport) -> Vec<c::Event> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| match event {
            Event::ColorInputEvent(_, _, _, _, event) => Some(event),
            Event::Overloaded(_) => panic!("unexpected overload"),
            _ => None,
        })
        .collect()
}

#[::core::prelude::v1::test]
fn tabs_route_keys_and_accessibility_with_one_owner_and_one_roving_stop() {
    with_picker(|owner, cx, transport| {
        let state = input(owner, cx);
        let fields = state.read_with(cx, |s, _| {
            s.editors
                .fields
                .iter()
                .map(|e| e.state.entity_id())
                .collect::<Vec<_>>()
        });
        let before = state.read_with(cx, |s, _| s.model.snapshot());
        configure(owner, cx, Panel::Palette);
        let tree = cx.a11y_tree().unwrap();
        assert_eq!(
            tree.nodes
                .iter()
                .filter(|(_, n)| n.role() == accesskit::Role::TabList)
                .count(),
            1
        );
        assert_eq!(
            tree.nodes
                .iter()
                .filter(|(_, n)| n.role() == accesskit::Role::Tab)
                .count(),
            2
        );
        assert_eq!(
            tree.nodes
                .iter()
                .filter(|(_, n)| n.role() == accesskit::Role::Slider)
                .count(),
            0
        );
        assert!(
            tree.nodes.iter().any(
                |(_, n)| n.role() == accesskit::Role::TabPanel && n.label() == Some("Swatches")
            )
        );
        command(owner, cx, c::Command::Focus(c::Field::Hex));
        cx.simulate_keystrokes("tab");
        draw(cx);
        cx.update(|w, cx| assert!(state.read(cx).tab_focus[0].is_focused(w)));
        cx.simulate_keystrokes("right");
        draw(cx);
        cx.update(|w, cx| assert!(state.read(cx).tab_focus[1].is_focused(w)));
        assert_eq!(state.read_with(cx, |s, _| s.panel), Panel::Channels);
        assert_eq!(
            cx.a11y_tree()
                .unwrap()
                .nodes
                .iter()
                .filter(|(_, n)| n.role() == accesskit::Role::Slider)
                .count(),
            4
        );
        cx.simulate_keystrokes("shift-tab");
        draw(cx);
        cx.update(|w, cx| {
            assert!(
                state.read(cx).editors.fields[0].focus.is_focused(w),
                "inactive tab is not a second stop"
            )
        });
        cx.simulate_keystrokes("tab");
        cx.simulate_keystrokes("home");
        draw(cx);
        assert_eq!(state.read_with(cx, |s, _| s.panel), Panel::Palette);
        cx.simulate_keystrokes("end");
        draw(cx);
        assert_eq!(state.read_with(cx, |s, _| s.panel), Panel::Channels);
        cx.simulate_keystrokes("home");
        cx.simulate_keystrokes("left");
        draw(cx);
        assert_eq!(state.read_with(cx, |s, _| s.panel), Panel::Channels);
        activate(cx, "Swatches", accesskit::Action::Click);
        activate(cx, "HSLA", accesskit::Action::Focus);
        configure(owner, cx, Panel::Channels);
        configure(owner, cx, Panel::Palette);
        assert_eq!(
            state.read_with(cx, |s, _| s.panel),
            Panel::Channels,
            "initial is not a controlled active panel"
        );
        state.read_with(cx, |s, _| {
            assert_eq!(s.model.snapshot(), before);
            assert_eq!(
                s.editors
                    .fields
                    .iter()
                    .map(|e| e.state.entity_id())
                    .collect::<Vec<_>>(),
                fields
            );
        });
        assert!(color_events(transport).is_empty());
        apply(owner, cx, vec![Op::SetColorPresentation(node(), None)]);
        cx.update(|w, cx| assert!(state.read(cx).editors.fields[0].focus.is_focused(w)));
        assert!(
            !cx.a11y_tree()
                .unwrap()
                .nodes
                .iter()
                .any(|(_, n)| n.role() == accesskit::Role::Tab)
        );
        assert_eq!(input(owner, cx).entity_id(), state.entity_id());
    });
}

#[::core::prelude::v1::test]
fn presentation_preserves_visible_hex_but_tab_focus_and_hidden_channels_settle_edits() {
    with_picker(|owner, cx, transport| {
        let state = input(owner, cx);
        command(owner, cx, c::Command::Focus(c::Field::Hex));
        edit(&state, cx, 0, "#abcdef", true);
        draw(cx);
        let composing = state.read_with(cx, |s, _| s.model.snapshot());
        assert!(composing.draft.as_ref().unwrap().composing);
        configure(owner, cx, Panel::Palette);
        assert_eq!(state.read_with(cx, |s, _| s.model.snapshot()), composing);
        click_tab(cx, "HSLA");
        state.read_with(cx, |s, _| {
            assert_eq!(s.panel, Panel::Channels);
            assert!(
                s.model.snapshot().interaction.is_none(),
                "{:?}",
                s.model.snapshot()
            );
            assert_eq!(s.model.snapshot().value, composing.committed);
        });
        for (text, composing) in [("180.25", false), ("invalid", false), ("200", true)] {
            apply(owner, cx, vec![Op::SetColorPresentation(node(), None)]);
            command(
                owner,
                cx,
                c::Command::Focus(c::Field::Channel(c::Channel::Hue)),
            );
            let old = state.read_with(cx, |s, _| s.model.snapshot());
            color_events(transport);
            edit(&state, cx, 1, text, composing);
            // No explicit draw/observer drain before accepted presentation hides it.
            configure(owner, cx, Panel::Palette);
            let snapshot = state.read_with(cx, |s, _| s.model.snapshot());
            assert!(snapshot.interaction.is_none());
            cx.update(|w, cx| assert!(state.read(cx).tab_focus[0].is_focused(w)));
            let events = color_events(transport);
            if text == "180.25" {
                assert_eq!(snapshot.channels.hue_degrees(), 180.25);
                assert_eq!(
                    events
                        .iter()
                        .filter(|e| matches!(e, c::Event::Committed(c::Source::Text, _)))
                        .count(),
                    1
                );
            } else {
                assert_eq!(snapshot.value, old.committed);
                assert!(
                    events
                        .iter()
                        .any(|e| matches!(e, c::Event::Cancelled(c::CancelReason::Interrupted, _)))
                );
            }
            command(
                owner,
                cx,
                c::Command::Focus(c::Field::Channel(c::Channel::Hue)),
            );
            cx.update(|w, cx| {
                let s = state.read(cx);
                assert_eq!(s.panel, Panel::Channels);
                assert!(s.editors.fields[1].focus.is_focused(w));
            });
        }
        // User-driven hiding settles the same way, without a configuration update.
        edit(&state, cx, 1, "250.125", false);
        draw(cx);
        click_tab(cx, "Swatches");
        let snapshot = state.read_with(cx, |s, _| s.model.snapshot());
        assert!(snapshot.interaction.is_none());
        assert_eq!(snapshot.channels.hue_degrees(), 250.125);
    });
}

#[::core::prelude::v1::test]
fn tab_switch_cancels_capture_and_hidden_controls_reject_stale_actions() {
    with_picker(|owner, cx, transport| {
        let state = input(owner, cx);
        configure(owner, cx, Panel::Channels);
        let before = state.read_with(cx, |s, _| s.model.snapshot());
        let tree = cx.a11y_tree().unwrap();
        let hue = tree
            .nodes
            .iter()
            .find(|(_, n)| n.role() == accesskit::Role::Slider && n.label() == Some("Hue"))
            .unwrap()
            .0;
        let point = state.read_with(cx, |s, _| s.tracks[0].center());
        move_pointer(cx, point);
        cx.simulate_mouse_down(point, MouseButton::Left, Modifiers::default());
        draw(cx);
        assert!(state.read_with(cx, |s, _| s.capture.is_some()));
        activate(cx, "Swatches", accesskit::Action::Click);
        cx.update(|w, _| assert!(w.captured_hitbox().is_none()));
        let restored = state.read_with(cx, |s, _| s.model.snapshot());
        assert!(restored.interaction.is_none());
        assert_eq!(restored.value, before.committed);
        cx.simulate_mouse_up(point, MouseButton::Left, Modifiers::default());
        cx.simulate_a11y_action(accesskit::ActionRequest {
            action: accesskit::Action::Increment,
            target_node: hue,
            target_tree: accesskit::TreeId::ROOT,
            data: None,
        });
        draw(cx);
        assert_eq!(state.read_with(cx, |s, _| s.model.snapshot()), restored);
        command(
            owner,
            cx,
            c::Command::Focus(c::Field::Channel(c::Channel::Hue)),
        );
        let before = state.read_with(cx, |s, _| s.model.snapshot());
        cx.update(|w, cx| {
            state.update(cx, |s, cx| {
                s.choose(
                    Value::Color(config().palette[0].color),
                    c::Source::Palette,
                    false,
                    w,
                    cx,
                );
                s.hover_palette(0, &config().palette[0], handler(), true, cx);
                assert_eq!(s.palette_preview, None);
                assert_eq!(s.model.snapshot(), before);
            })
        });
        activate(cx, "Swatches", accesskit::Action::Click);
        let mut opaque = config();
        opaque.alpha_policy = AlphaPolicy::OpaqueOnly;
        apply(owner, cx, vec![set(opaque)]);
        cx.update(|w, cx| {
            owner.update(cx, |v, cx| {
                assert_eq!(
                    v.color_inputs[&node()].command(
                        &c::Command::Focus(c::Field::Channel(c::Channel::Alpha)),
                        w,
                        cx
                    ),
                    c::Response::Failed(c::Error::FocusBlocked)
                )
            })
        });
        assert_eq!(state.read_with(cx, |s, _| s.panel), Panel::Palette);
        assert!(
            !color_events(transport)
                .iter()
                .any(|e| matches!(e, c::Event::Committed(c::Source::Palette, _)))
        );
    });
}

#[::core::prelude::v1::test]
fn tab_policy_gates_mode_changes_and_owner_cleanup_fence_old_callbacks() {
    with_picker(|owner, cx, transport| {
        let state = input(owner, cx);
        configure(owner, cx, Panel::Palette);
        let mut readonly = config();
        readonly.read_only = true;
        apply(owner, cx, vec![set(readonly)]);
        color_events(transport);
        click_tab(cx, "HSLA");
        assert_eq!(state.read_with(cx, |s, _| s.panel), Panel::Channels);
        assert!(
            color_events(transport).is_empty(),
            "read-only permits presentation navigation"
        );
        let mut disabled = config();
        disabled.disabled = true;
        apply(owner, cx, vec![set(disabled)]);
        cx.update(|w, cx| {
            state.update(cx, |s, cx| {
                s.activate_tab(Panel::Palette, handler(), false, w, cx);
                assert_eq!(s.panel, Panel::Channels);
                assert!(!s.focused(w));
            })
        });
        apply(owner, cx, vec![set(config())]);
        for field in [Field::Disabled(true), Field::Inert(true), Field::Display(3)] {
            apply(
                owner,
                cx,
                vec![Op::SetStyle(parent(), vec![Style::Fields(vec![field])])],
            );
            cx.update(|w, cx| {
                state.update(cx, |s, cx| {
                    s.activate_tab(Panel::Palette, handler(), false, w, cx);
                    assert_eq!(s.panel, Panel::Channels);
                })
            });
            apply(owner, cx, vec![Op::SetStyle(parent(), vec![])]);
        }
        apply(
            owner,
            cx,
            vec![Op::SetStyle(
                parent(),
                vec![Style::Fields(vec![Field::PointerEvents(false)])],
            )],
        );
        cx.update(|w, cx| {
            state.update(cx, |s, cx| {
                s.activate_tab(Panel::Palette, handler(), true, w, cx);
                assert_eq!(s.panel, Panel::Channels);
                s.activate_tab(Panel::Palette, handler(), false, w, cx);
                assert_eq!(s.panel, Panel::Palette);
            })
        });
        apply(owner, cx, vec![Op::SetStyle(parent(), vec![])]);
        let scope = NodeId::from_parts(2, 1).unwrap();
        let button = NodeId::from_parts(3, 1).unwrap();
        apply(
            owner,
            cx,
            vec![
                Op::Create(scope, Kind::FocusScope, "".into(), None),
                Op::SetFocusScope(
                    scope,
                    FocusScopeConfig {
                        trap: true,
                        auto_focus: true,
                        restore_focus: true,
                    },
                ),
                Op::Create(
                    button,
                    Kind::Button,
                    "Modal action".into(),
                    Some(HandlerId::from_parts(3, 1).unwrap()),
                ),
                Op::Splice(scope, 0, 0, vec![button]),
                Op::Splice(parent(), 1, 0, vec![scope]),
            ],
        );
        cx.update(|w, cx| {
            state.update(cx, |s, cx| {
                s.activate_tab(Panel::Channels, handler(), false, w, cx);
                assert_eq!(s.panel, Panel::Palette);
                assert!(!s.focused(w));
            })
        });
        apply(
            owner,
            cx,
            vec![
                Op::Splice(parent(), 1, 1, vec![]),
                Op::Remove(button),
                Op::Remove(scope),
            ],
        );
        let replacement = HandlerId::from_parts(1, 1).unwrap();
        apply(owner, cx, vec![Op::Bind(node(), Some(replacement))]);
        cx.update(|w, cx| {
            state.update(cx, |s, cx| {
                s.activate_tab(Panel::Channels, handler(), false, w, cx);
                assert_eq!(s.panel, Panel::Palette);
            })
        });
        apply(owner, cx, vec![Op::SetColorPresentation(node(), None)]);
        cx.update(|w, cx| {
            state.update(cx, |s, cx| {
                s.activate_tab(Panel::Channels, replacement, false, w, cx);
                assert_eq!(s.panel, Panel::Palette);
            })
        });
        configure(owner, cx, Panel::Channels);
        let fields = state.read_with(cx, |s, _| {
            s.editors
                .fields
                .iter()
                .map(|e| e.state.downgrade())
                .collect::<Vec<_>>()
        });
        let weak = state.downgrade();
        activate(cx, "HSLA", accesskit::Action::Focus);
        apply(
            owner,
            cx,
            vec![Op::Splice(parent(), 0, 1, vec![]), Op::Remove(node())],
        );
        cx.update(|w, cx| {
            state.update(cx, |s, cx| {
                s.activate_tab(Panel::Palette, replacement, false, w, cx);
                assert_eq!(s.panel, Panel::Channels);
                assert!(s.closed);
            })
        });
        drop(state);
        draw(cx);
        assert!(weak.upgrade().is_none());
        assert!(fields.iter().all(|f| f.upgrade().is_none()));
    });
}

#[::core::prelude::v1::test]
fn initial_channels_mount_and_explicit_focus_interrupt_capture() {
    with_picker_presentation(Some(tabs(Panel::Channels)), |owner, cx, transport| {
        let state = input(owner, cx);
        assert_eq!(state.read_with(cx, |s, _| s.panel), Panel::Channels);
        let before = state.read_with(cx, |s, _| s.model.snapshot());
        let point = state.read_with(cx, |s, _| s.tracks[0].center());
        move_pointer(cx, point);
        cx.simulate_mouse_down(point, MouseButton::Left, Modifiers::default());
        draw(cx);
        assert!(state.read_with(cx, |s, _| s.capture.is_some()));
        command(owner, cx, c::Command::Focus(c::Field::Hex));
        cx.update(|w, cx| {
            let s = state.read(cx);
            assert!(s.capture.is_none());
            assert!(w.captured_hitbox().is_none());
            assert!(s.editors.fields[0].focus.is_focused(w));
            assert_eq!(s.model.snapshot().value, before.committed);
            assert!(s.model.snapshot().interaction.is_none());
        });
        cx.simulate_mouse_up(point, MouseButton::Left, Modifiers::default());
        draw(cx);
        assert!(
            !color_events(transport)
                .iter()
                .any(|e| matches!(e, c::Event::Committed(..)))
        );
    });
}
