//! Native hit testing and editor preservation on TestPlatform, not OS IME evidence.
use super::*;
use crate::session::Session;
use gpui::{TestAppContext, VisualTestContext};
use gpuio_protocol::v1::{
    CAPABILITIES, Event, Field, FocusScopeConfig, Kind, Op, Transaction, VERSION,
};
use gpuio_protocol::v1::{Length, Style};
use std::{cell::RefCell, os::fd::AsRawFd, os::unix::net::UnixStream, rc::Rc};

#[path = "color_input_palette_test.rs"]
mod palette;
#[path = "color_input_panels_test.rs"]
mod panels;

fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn parent() -> NodeId {
    NodeId::from_parts(1, 1).unwrap()
}
fn handler() -> HandlerId {
    HandlerId::from_parts(0, 1).unwrap()
}
fn config() -> c::Config {
    let mut config = super::test::config();
    config.palette.push(c::PaletteEntry {
        color: config.palette[0].color,
        label: "Red again".into(),
    });
    config
}
fn set(config: c::Config) -> Op {
    Op::SetColorInput(
        node(),
        Box::new(config),
        Value::Color(Rgba::new(0, 255, 0, 255)),
    )
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let result = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: view.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            view.update_editors(&result.dirty, window, cx);
            cx.notify();
        });
    });
    draw(cx);
    draw(cx);
}
fn input(owner: &Entity<View>, cx: &VisualTestContext) -> Entity<ColorInput> {
    owner.read_with(cx, |v, _| v.color_inputs[&node()].state.clone())
}
fn command(owner: &Entity<View>, cx: &mut VisualTestContext, command: c::Command) {
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| {
            assert!(matches!(
                v.color_inputs[&node()].command(&command, window, cx),
                c::Response::Applied(_)
            ));
        })
    });
    draw(cx);
}
fn move_pointer(cx: &mut VisualTestContext, position: Point<Pixels>) {
    cx.simulate_event(MouseMoveEvent {
        position,
        pressed_button: None,
        modifiers: Default::default(),
    });
    draw(cx);
}
fn location(cx: &mut VisualTestContext, label: &str) -> Point<Pixels> {
    let tree = cx.a11y_tree().unwrap();
    let bounds = tree
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some(label))
        .unwrap()
        .1
        .bounds()
        .unwrap();
    let scale = cx.update(|window, _| f64::from(window.scale_factor()));
    point(
        px(((bounds.x0 + bounds.x1) / (2. * scale)) as f32),
        px(((bounds.y0 + bounds.y1) / (2. * scale)) as f32),
    )
}
fn hover(cx: &mut VisualTestContext, label: &str) {
    move_pointer(cx, point(px(800.), px(600.)));
    let point = location(cx, label);
    move_pointer(cx, point);
}
fn with_picker(f: impl FnOnce(&Entity<View>, &mut VisualTestContext, &Transport)) {
    with_picker_presentation(None, f);
}
fn with_picker_presentation(
    presentation: Option<gpuio_protocol::color_presentation::Presentation>,
    f: impl FnOnce(&Entity<View>, &mut VisualTestContext, &Transport),
) {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    let id = WindowId::from_parts(0, 1).unwrap();
    session
        .borrow_mut()
        .open(1, id, "Color preview", 400., 600.)
        .unwrap();
    let (owner, cx) = app.add_window_view(|_, _| View::new(id, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    apply(
        &owner,
        cx,
        vec![
            Op::Create(node(), Kind::ColorInput, "".into(), Some(handler())),
            Op::Create(parent(), Kind::Container, "".into(), None),
            set(config()),
            Op::SetColorPresentation(node(), presentation),
            Op::SetStyle(node(), vec![Style::Width(Length::Px(320.))]),
            Op::Splice(parent(), 0, 0, vec![node()]),
            Op::SetRoot(Some(parent())),
        ],
    );
    transport.mailbox.lock().unwrap().drain(256);
    f(&owner, cx, &transport);
}

#[::core::prelude::v1::test]
fn hover_preserves_invalid_and_composing_drafts_focus_selection_history_and_layout() {
    with_picker(|owner, cx, transport| {
        let state = input(owner, cx);
        let editor = state.read_with(cx, |s, _| s.editors.fields[0].state.clone());
        command(owner, cx, c::Command::Focus(c::Field::Hex));
        for composing in [false, true] {
            cx.update(|window, cx| {
                editor.update(cx, |e, cx| {
                    let length = e.value().encode_utf16().count();
                    if composing {
                        e.replace_and_mark_text_in_range(
                            Some(0..length),
                            "#abcdef",
                            Some(1..4),
                            window,
                            cx,
                        );
                    } else {
                        e.replace_text_in_range(Some(0..length), "#bAd draft", window, cx);
                        assert!(e.bridge_select(1, 4, cx));
                    }
                })
            });
            draw(cx);
            let before = state.read_with(cx, |s, _| s.model.snapshot());
            assert_eq!(before.draft.as_ref().unwrap().composing, composing);
            let stamp = |cx: &VisualTestContext| {
                editor.read_with(cx, |e, _| {
                    (
                        e.value().to_string(),
                        e.bridge_selection(),
                        e.bridge_composition(),
                        e.bridge_revision(),
                        e.bridge_history_bytes(),
                    )
                })
            };
            let original_editor = stamp(cx);
            let original_bounds = location(cx, "Translucent red");
            transport.mailbox.lock().unwrap().drain(256);
            hover(cx, "Translucent red");
            state.read_with(cx, |s, _| {
                assert_eq!(s.palette_preview, Some((0, config().palette[0].color)));
                assert_eq!(s.displayed_color(), Value::Color(config().palette[0].color));
                assert_eq!(s.model.snapshot(), before);
            });
            assert_eq!(stamp(cx), original_editor);
            assert_eq!(location(cx, "Translucent red"), original_bounds);
            cx.update(|window, cx| assert!(editor.read(cx).focus_handle(cx).is_focused(window)));
            let tree = cx.a11y_tree().unwrap();
            assert_eq!(
                tree.nodes
                    .iter()
                    .find(|(_, n)| n.label() == Some("Accent"))
                    .unwrap()
                    .1
                    .value(),
                Some("#00FF00")
            );
            assert_eq!(
                tree.nodes
                    .iter()
                    .find(|(_, n)| n.label() == Some("Green"))
                    .unwrap()
                    .1
                    .is_selected(),
                Some(true)
            );
            assert!(
                !tree
                    .nodes
                    .iter()
                    .any(|(_, n)| n.value() == Some("#FF000080")),
                "hover caption must not masquerade as an edited value"
            );
            hover(cx, "Red again");
            cx.update(|_, cx| {
                state.update(cx, |s, cx| {
                    s.hover_palette(0, &config().palette[0], handler(), false, cx);
                    assert_eq!(
                        s.palette_preview,
                        Some((2, config().palette[2].color)),
                        "duplicate values are separate slots"
                    );
                })
            });
            move_pointer(cx, point(px(800.), px(600.)));
            state.read_with(cx, |s, _| {
                assert_eq!(s.palette_preview, None);
                assert_eq!(s.model.snapshot(), before);
                assert_eq!(s.displayed_color(), before.value);
            });
            assert_eq!(stamp(cx), original_editor);
            assert!(
                transport.mailbox.lock().unwrap().drain(256).is_empty(),
                "hover sends no events"
            );
            hover(cx, "Translucent red");
            apply(
                owner,
                cx,
                vec![Op::SetStyle(
                    parent(),
                    vec![Style::Fields(vec![Field::PointerEvents(false)])],
                )],
            );
            state.read_with(cx, |s, _| {
                assert_eq!(s.palette_preview, None);
                assert_eq!(
                    s.model.snapshot(),
                    before,
                    "pointer-only policy must preserve keyboard composition"
                );
            });
            assert_eq!(stamp(cx), original_editor);
            apply(owner, cx, vec![Op::SetStyle(parent(), vec![])]);
            assert_eq!(state.read_with(cx, |s, _| s.model.snapshot()), before);
            assert_eq!(stamp(cx), original_editor);
            let emitted = transport.mailbox.lock().unwrap().drain(256);
            assert!(
                emitted
                    .iter()
                    .all(|event| matches!(event, Event::Rendered(..))),
                "style transactions may acknowledge rendering but never emit color edits: {emitted:?}"
            );
        }
    });
}

#[::core::prelude::v1::test]
fn preview_obeys_policy_commands_ancestor_gates_and_owner_lifetime() {
    with_picker(|owner, cx, transport| {
        let state = input(owner, cx);
        let cleared = |cx: &VisualTestContext| {
            assert_eq!(state.read_with(cx, |s, _| s.palette_preview), None)
        };
        for mode in 0..3 {
            hover(cx, "Translucent red");
            assert!(state.read_with(cx, |s, _| s.palette_preview.is_some()));
            let mut restricted = config();
            match mode {
                0 => restricted.read_only = true,
                1 => restricted.disabled = true,
                _ => restricted.alpha_policy = AlphaPolicy::OpaqueOnly,
            }
            apply(owner, cx, vec![set(restricted)]);
            cleared(cx);
            hover(cx, "Translucent red");
            cleared(cx);
            apply(owner, cx, vec![set(config())]);
        }
        for command_value in [
            c::Command::Cancel,
            c::Command::Reset { if_revision: None },
            c::Command::Focus(c::Field::Hex),
            c::Command::Set {
                value: Value::Empty,
                if_revision: None,
            },
        ] {
            hover(cx, "Translucent red");
            command(owner, cx, command_value);
            cleared(cx);
        }
        for field in [
            Field::Disabled(true),
            Field::Inert(true),
            Field::PointerEvents(false),
            Field::Visibility(1),
            Field::Display(3),
        ] {
            hover(cx, "Translucent red");
            apply(
                owner,
                cx,
                vec![Op::SetStyle(parent(), vec![Style::Fields(vec![field])])],
            );
            cleared(cx);
            // A saved frame's listener cannot revive a gated preview.
            cx.update(|_, cx| {
                state.update(cx, |s, cx| {
                    s.hover_palette(0, &config().palette[0], handler(), true, cx)
                })
            });
            cleared(cx);
            apply(owner, cx, vec![Op::SetStyle(parent(), vec![])]);
        }
        hover(cx, "Translucent red");
        let replacement_handler = HandlerId::from_parts(1, 1).unwrap();
        apply(owner, cx, vec![Op::Bind(node(), Some(replacement_handler))]);
        cleared(cx);
        cx.update(|_, cx| {
            state.update(cx, |s, cx| {
                s.hover_palette(0, &config().palette[0], handler(), true, cx)
            })
        });
        cleared(cx);
        hover(cx, "Translucent red");
        cx.update(|_, cx| {
            state.update(cx, |s, cx| {
                s.hover_palette(0, &config().palette[0], handler(), false, cx);
                assert!(
                    s.palette_preview.is_some(),
                    "old handler leave cannot erase a new preview"
                );
            });
        });
        let mut replacement = config();
        replacement.palette[0].color = Rgba::new(0, 0, 255, 255);
        apply(owner, cx, vec![set(replacement)]);
        cleared(cx);
        cx.update(|_, cx| {
            state.update(cx, |s, cx| {
                s.hover_palette(0, &config().palette[0], replacement_handler, true, cx)
            })
        });
        cleared(cx);
        hover(cx, "Translucent red");
        let weak = state.downgrade();
        apply(
            owner,
            cx,
            vec![Op::Splice(parent(), 0, 1, vec![]), Op::Remove(node())],
        );
        cleared(cx);
        assert!(state.read_with(cx, |s, _| s.closed));
        drop(state);
        draw(cx);
        assert!(weak.upgrade().is_none());
        assert!(
            !transport
                .mailbox
                .lock()
                .unwrap()
                .drain(256)
                .iter()
                .any(|event| matches!(event, Event::Overloaded(_)))
        );
    });
}

#[::core::prelude::v1::test]
fn preview_cannot_override_channel_capture_modal_policy_or_explicit_commands() {
    with_picker(|owner, cx, transport| {
        let state = input(owner, cx);
        hover(cx, "Translucent red");
        let preview = state.read_with(cx, |s, _| s.palette_preview);
        command(owner, cx, c::Command::ReadSnapshot);
        assert_eq!(state.read_with(cx, |s, _| s.palette_preview), preview);
        cx.update(|window, cx| {
            owner.update(cx, |v, cx| {
                assert!(matches!(
                    v.color_inputs[&node()].command(
                        &c::Command::Set {
                            value: Value::Empty,
                            if_revision: Some(100),
                        },
                        window,
                        cx
                    ),
                    c::Response::Failed(_)
                ));
            })
        });
        assert_eq!(
            state.read_with(cx, |s, _| s.palette_preview),
            preview,
            "rejected commands preserve presentation"
        );
        let point = state.read_with(cx, |s, _| s.tracks[0].center());
        move_pointer(cx, point);
        cx.simulate_mouse_down(point, MouseButton::Left, Modifiers::default());
        draw(cx);
        state.read_with(cx, |s, _| {
            assert!(s.capture.is_some());
            assert!(s.model.snapshot().interaction.is_some());
        });
        let target = location(cx, "Translucent red");
        cx.simulate_event(MouseMoveEvent {
            position: target,
            pressed_button: Some(MouseButton::Left),
            modifiers: Default::default(),
        });
        draw(cx);
        cx.update(|_, cx| {
            state.update(cx, |s, cx| {
                s.hover_palette(0, &config().palette[0], handler(), true, cx);
                assert_eq!(s.palette_preview, None);
                assert_eq!(s.displayed_color(), s.model.snapshot().value);
            })
        });
        cx.simulate_mouse_up(target, MouseButton::Left, Modifiers::default());
        draw(cx);
        assert!(state.read_with(cx, |s, _| s.model.snapshot().interaction.is_none()));
        hover(cx, "Translucent red");
        let before = state.read_with(cx, |s, _| s.model.snapshot());
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
        cx.update(|_, cx| {
            state.update(cx, |s, cx| {
                assert_eq!(
                    s.palette_preview, None,
                    "modal entry retires unfocused hover"
                );
                s.hover_palette(0, &config().palette[0], handler(), true, cx);
                assert_eq!(s.palette_preview, None);
                assert_eq!(s.model.snapshot(), before);
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
        hover(cx, "Translucent red");
        assert!(state.read_with(cx, |s, _| s.palette_preview.is_some()));
        transport.mailbox.lock().unwrap().drain(256);
        cx.update(|window, cx| {
            owner.update(cx, |v, cx| {
                v.cancel_color_inputs(c::CancelReason::WindowInactive, window, cx);
            })
        });
        assert_eq!(state.read_with(cx, |s, _| s.palette_preview), None);
        assert!(transport.mailbox.lock().unwrap().drain(256).is_empty());
    });
}
