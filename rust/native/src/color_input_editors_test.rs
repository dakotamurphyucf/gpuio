use super::*;
use gpui_base::input::InputState;
fn editor(cx: &mut AsyncApp, handle: WindowHandle<View>, index: usize) -> Entity<InputState> {
    handle
        .update(cx, |v, _, cx| {
            v.color_inputs[&node()].state.read(cx).editors.fields[index]
                .state
                .clone()
        })
        .unwrap()
}
fn focus(cx: &mut AsyncApp, handle: WindowHandle<View>, index: usize) {
    let editor = editor(cx, handle, index);
    handle
        .update(cx, |_, w, cx| {
            w.focus(&editor.read(cx).focus_handle(cx), cx)
        })
        .unwrap();
}
fn replace(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    index: usize,
    text: &str,
    composing: bool,
) {
    let editor = editor(cx, handle, index);
    handle
        .update(cx, |_, w, cx| {
            editor.update(cx, |s, cx| {
                let length = s.value().encode_utf16().count();
                if composing {
                    s.replace_and_mark_text_in_range(
                        Some(0..length),
                        text,
                        Some(0..text.encode_utf16().count()),
                        w,
                        cx,
                    );
                } else {
                    s.replace_text_in_range(Some(0..length), text, w, cx);
                }
            })
        })
        .unwrap();
}
fn text(cx: &mut AsyncApp, handle: WindowHandle<View>, index: usize) -> String {
    let editor = editor(cx, handle, index);
    cx.update(|cx| editor.read(cx).value().to_string())
}
pub(super) async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    focus(cx, handle, 0);
    frame(cx, handle).await;
    let original = snapshot(cx, handle).value;
    replace(cx, handle, 0, "#12", false);
    frame(cx, handle).await;
    let draft = snapshot(cx, handle);
    assert_eq!(draft.value, original);
    assert_eq!(
        draft.draft.as_ref().unwrap().status,
        c::DraftStatus::Incomplete
    );
    assert_eq!(text(cx, handle, 0), "#12");
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle), draft);
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).value, original);
    assert!(snapshot(cx, handle).interaction.is_none());
    assert_eq!(text(cx, handle, 0), "#00FF00");
    events(transport);
    replace(cx, handle, 0, "#ff000080", false);
    frame(cx, handle).await;
    assert_eq!(
        snapshot(cx, handle).value,
        Value::Color(Rgba::new(255, 0, 0, 128))
    );
    assert_eq!(text(cx, handle, 1), "0");
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert!(snapshot(cx, handle).interaction.is_none());
    assert_eq!(text(cx, handle, 0), "#ff000080");
    assert!(matches!(
        events(transport).last(),
        Some(c::Event::Committed(c::Source::Text, _))
    ));
    assert_eq!(text(cx, handle, 4), "50.2");
    let before_focus = snapshot(cx, handle);
    focus(cx, handle, 4);
    frame(cx, handle).await;
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).value, before_focus.value);
    assert_eq!(snapshot(cx, handle).channels, before_focus.channels);
    assert!(snapshot(cx, handle).interaction.is_none());
    focus(cx, handle, 1);
    frame(cx, handle).await;
    replace(cx, handle, 1, "123.456", false);
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).channels.hue_degrees(), 123.456);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(text(cx, handle, 1), "123.456");
    let committed = snapshot(cx, handle).value;
    replace(cx, handle, 1, "999", false);
    frame(cx, handle).await;
    assert_eq!(
        snapshot(cx, handle).draft.as_ref().unwrap().status,
        c::DraftStatus::OutOfRange
    );
    assert_eq!(snapshot(cx, handle).value, committed);
    focus(cx, handle, 0);
    frame(cx, handle).await;
    assert!(snapshot(cx, handle).interaction.is_none());
    assert_eq!(text(cx, handle, 1), "123.46");
    assert_eq!(snapshot(cx, handle).channels.hue_degrees(), 123.456);
    events(transport);
    replace(cx, handle, 0, "#123456", true);
    frame(cx, handle).await;
    assert!(snapshot(cx, handle).draft.as_ref().unwrap().composing);
    assert_eq!(snapshot(cx, handle).value, committed);
    let marked = snapshot(cx, handle);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle), marked);
    let field = editor(cx, handle, 0);
    let before = cx.update(|cx| {
        (
            field.read(cx).bridge_selection(),
            field.read(cx).bridge_history_bytes(),
        )
    });
    let mut renamed = config();
    renamed.labels.hex = "Hexadecimal".into();
    apply(cx, handle, vec![set(renamed)]);
    frame(cx, handle).await;
    assert_eq!(text(cx, handle, 0), "#123456");
    assert!(snapshot(cx, handle).draft.as_ref().unwrap().composing);
    assert_eq!(
        cx.update(|cx| (
            field.read(cx).bridge_selection(),
            field.read(cx).bridge_history_bytes()
        )),
        before
    );
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).value, committed);
    assert!(cx.update(|cx| field.read(cx).bridge_composition().is_none()));
    assert_eq!(cx.update(|cx| field.read(cx).bridge_history_bytes()), 0);
    replace(cx, handle, 0, "#654321", true);
    frame(cx, handle).await;
    handle
        .update(cx, |_, w, cx| {
            field.update(cx, |s, cx| {
                s.unmark_text(w, cx);
                cx.notify();
            })
        })
        .unwrap();
    frame(cx, handle).await;
    assert!(!snapshot(cx, handle).draft.as_ref().unwrap().composing);
    assert_eq!(
        snapshot(cx, handle).value,
        Value::Color(Rgba::new(0x65, 0x43, 0x21, 255))
    );
    key(cx, handle, "enter");
    frame(cx, handle).await;
    // Undo/redo changes the draft, never silently commits it.
    handle
        .update(cx, |_, w, cx| {
            field.update(cx, |s, cx| s.bridge_undo(w, cx))
        })
        .unwrap();
    frame(cx, handle).await;
    assert!(snapshot(cx, handle).interaction.is_some());
    let undone = text(cx, handle, 0);
    handle
        .update(cx, |_, w, cx| {
            field.update(cx, |s, cx| s.bridge_redo(w, cx))
        })
        .unwrap();
    frame(cx, handle).await;
    assert_ne!(undone, text(cx, handle, 0));
    assert_eq!(text(cx, handle, 0), "#654321");
    key(cx, handle, "enter");
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    {
        focus(cx, handle, 0);
        frame(cx, handle).await;
        key(cx, handle, "cmd-a");
        let before = snapshot(cx, handle).committed;
        super::super::super::editor_test::native_text(cx, handle, "#aabbcc", true);
        frame(cx, handle).await;
        assert!(snapshot(cx, handle).draft.as_ref().unwrap().composing);
        assert_eq!(snapshot(cx, handle).value, before);
        super::super::super::editor_test::native_text(cx, handle, "#112233", false);
        frame(cx, handle).await;
        assert!(!snapshot(cx, handle).draft.as_ref().unwrap().composing);
        assert_eq!(
            snapshot(cx, handle).value,
            Value::Color(Rgba::new(0x11, 0x22, 0x33, 255))
        );
        key(cx, handle, "enter");
        frame(cx, handle).await;
        assert!(snapshot(cx, handle).interaction.is_none());
        eprintln!(
            "GPUIO_COLOR_NSTEXT_OK: actual AppKit marked-text and insertion delegates preserve preview/commit ordering"
        );
    }
    let before_text = text(cx, handle, 0);
    replace(cx, handle, 0, &"x".repeat(c::MAX_DRAFT_BYTES + 1), false);
    frame(cx, handle).await;
    assert_eq!(text(cx, handle, 0), before_text);
    for i in 0..80 {
        let text = format!("{i:04}{}", "x".repeat(c::MAX_DRAFT_BYTES - 4));
        replace(cx, handle, 0, &text, false);
    }
    frame(cx, handle).await;
    assert_eq!(text(cx, handle, 0).len(), c::MAX_DRAFT_BYTES);
    assert!(cx.update(|cx| field.read(cx).bridge_history_bytes()) <= 64 * 1024);
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert_eq!(text(cx, handle, 0), before_text);
    assert_eq!(cx.update(|cx| field.read(cx).bridge_history_bytes()), 0);
    events(transport);
    // The retained command router recognizes the field and remembers its
    // exact focus handle after a toolbar steals focus.
    handle
        .update(cx, |v, w, cx| {
            assert_eq!(v.command_editor(w, cx), Some(node()));
            let input = &v.color_inputs[&node()];
            assert!(input.command_available(gpuio_protocol::v1::NativeCommand::Undo, w, cx));
            assert_eq!(
                input.command_focus(w, cx),
                Some(field.read(cx).focus_handle(cx))
            );
            w.blur(cx);
            assert_eq!(v.command_editor(w, cx), Some(node()));
            assert_eq!(
                input.command_focus(w, cx),
                Some(field.read(cx).focus_handle(cx))
            );
        })
        .unwrap();
    frame(cx, handle).await;
    focus(cx, handle, 0);
    frame(cx, handle).await;
    replace(cx, handle, 0, "#12", false);
    frame(cx, handle).await;
    let baseline = snapshot(cx, handle).committed;
    let mut readonly = config();
    readonly.read_only = true;
    apply(cx, handle, vec![set(readonly)]);
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).value, baseline);
    assert!(snapshot(cx, handle).interaction.is_none());
    let baseline_text = text(cx, handle, 0);
    replace(cx, handle, 0, "#ffffff", false);
    frame(cx, handle).await;
    assert_eq!(text(cx, handle, 0), baseline_text);
    handle
        .update(cx, |v, w, cx| {
            assert!(!v.color_inputs[&node()].command_available(
                gpuio_protocol::v1::NativeCommand::Paste,
                w,
                cx
            ))
        })
        .unwrap();
    apply(cx, handle, vec![set(config())]);
    frame(cx, handle).await;
    replace(cx, handle, 0, "#abcdef", true);
    frame(cx, handle).await;
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    frame(cx, handle).await;
    frame(cx, handle).await;
    assert!(snapshot(cx, handle).interaction.is_none());
    assert_eq!(snapshot(cx, handle).value, baseline);
    assert!(cx.update(|cx| field.read(cx).bridge_composition().is_none()));
    handle
        .update(cx, |v, w, cx| {
            assert!(!v.color_inputs[&node()].focused(w, cx))
        })
        .unwrap();
    replace(cx, handle, 0, "#ffffff", false);
    frame(cx, handle).await;
    assert_eq!(text(cx, handle, 0), baseline_text);
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(),
            vec![Style::Fields(vec![
                Field::Background(Fill::Solid(Color::Rgba(0x182332ff))),
                Field::Foreground(Color::Rgba(0xe3e9f3ff)),
            ])],
        )],
    );
    frame(cx, handle).await;
    frame(cx, handle).await;
    commands(cx, handle, transport).await;
    // Restore the seed through the actual palette keyboard route for the channel suite.
    handle
        .update(cx, |v, w, cx| {
            let focus = v.color_inputs[&node()].state.read(cx).palette_focus[1].clone();
            w.focus(&focus, cx);
        })
        .unwrap();
    frame(cx, handle).await;
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).value, original);
    events(transport);
    eprintln!(
        "GPUIO_COLOR_EDITORS_OK: invalid/valid drafts, unsnapped channels, Enter/Escape/blur, native composition and label retention, undo/redo and synchronized fields"
    );
}

fn command(cx: &mut AsyncApp, handle: WindowHandle<View>, command: c::Command) -> c::Response {
    handle
        .update(cx, |v, w, cx| {
            v.color_inputs[&node()].command(&command, w, cx)
        })
        .unwrap()
}
fn applied(response: c::Response) -> c::Snapshot {
    match response {
        c::Response::Applied(snapshot) => snapshot,
        other => panic!("{other:?}"),
    }
}
async fn commands(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    events(transport);
    let green = Value::Color(Rgba::new(0, 255, 0, 255));
    let red = Value::Color(Rgba::new(255, 0, 0, 128));
    applied(command(cx, handle, c::Command::Reset { if_revision: None }));
    applied(command(cx, handle, c::Command::Focus(c::Field::Hex)));
    frame(cx, handle).await;
    let before = applied(command(cx, handle, c::Command::ReadSnapshot));
    let field = editor(cx, handle, 0);
    // Invoke the command before GPUI can deliver the InputState observer.
    handle
        .update(cx, |v, w, cx| {
            field.update(cx, |s, cx| {
                let length = s.value().encode_utf16().count();
                s.replace_and_mark_text_in_range(Some(0..length), "#ff000080", Some(0..9), w, cx);
            });
            assert_eq!(
                v.color_inputs[&node()].command(
                    &c::Command::Set {
                        value: red,
                        if_revision: Some(before.revision)
                    },
                    w,
                    cx
                ),
                c::Response::Failed(c::Error::StaleRevision)
            );
            assert!(field.read(cx).bridge_composition().is_some());
            assert_eq!(field.read(cx).value().as_ref(), "#ff000080");
        })
        .unwrap();
    let composing = applied(command(cx, handle, c::Command::ReadSnapshot));
    assert!(composing.draft.as_ref().unwrap().composing);
    assert!(composing.revision > before.revision);
    assert_eq!(composing.value, green);
    events(transport);
    let replaced = applied(command(
        cx,
        handle,
        c::Command::Set {
            value: red,
            if_revision: Some(composing.revision),
        },
    ));
    assert_eq!(replaced.value, red);
    assert!(replaced.interaction.is_none());
    assert!(matches!(
        events(transport).as_slice(),
        [
            c::Event::Cancelled(c::CancelReason::Programmatic, _),
            c::Event::Observed(_)
        ]
    ));
    assert_eq!(text(cx, handle, 0), "#FF000080");
    assert!(cx.update(|cx| field.read(cx).bridge_composition().is_none()));
    assert_eq!(cx.update(|cx| field.read(cx).bridge_history_bytes()), 0);
    frame(cx, handle).await;
    assert!(
        events(transport).is_empty(),
        "delayed child observers must not restart edits"
    );
    assert_eq!(snapshot(cx, handle), replaced);

    // Valid text commit retains raw spelling, but explicit equal Set retires it.
    replace(cx, handle, 0, "#ff000080", false);
    frame(cx, handle).await;
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(text(cx, handle, 0), "#ff000080");
    applied(command(
        cx,
        handle,
        c::Command::Set {
            value: red,
            if_revision: None,
        },
    ));
    assert_eq!(text(cx, handle, 0), "#FF000080");
    assert_eq!(cx.update(|cx| field.read(cx).bridge_history_bytes()), 0);
    events(transport);

    // Focus commits a valid old field before returning the new-field snapshot.
    replace(cx, handle, 0, "#123456", false);
    frame(cx, handle).await;
    let focused = applied(command(
        cx,
        handle,
        c::Command::Focus(c::Field::Channel(c::Channel::Hue)),
    ));
    assert!(focused.interaction.is_none());
    assert_eq!(
        focused.value,
        Value::Color(Rgba::new(0x12, 0x34, 0x56, 255))
    );
    assert!(matches!(
        events(transport).last(),
        Some(c::Event::Committed(c::Source::Text, _))
    ));
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle), focused);
    assert!(events(transport).is_empty());

    let mut restricted = config();
    restricted.alpha_policy = AlphaPolicy::OpaqueOnly;
    restricted.allow_empty = false;
    apply(cx, handle, vec![set(restricted.clone())]);
    frame(cx, handle).await;
    applied(command(cx, handle, c::Command::Focus(c::Field::Hex)));
    replace(cx, handle, 0, "#12", false);
    frame(cx, handle).await;
    let invalid = snapshot(cx, handle);
    assert_eq!(
        command(
            cx,
            handle,
            c::Command::Set {
                value: red,
                if_revision: None
            }
        ),
        c::Response::Failed(c::Error::InvalidValue)
    );
    assert_eq!(
        command(
            cx,
            handle,
            c::Command::Focus(c::Field::Channel(c::Channel::Alpha))
        ),
        c::Response::Failed(c::Error::FocusBlocked)
    );
    assert_eq!(snapshot(cx, handle), invalid);
    assert_eq!(text(cx, handle, 0), "#12");
    applied(command(cx, handle, c::Command::Cancel));
    assert!(snapshot(cx, handle).interaction.is_none());
    assert_eq!(snapshot(cx, handle).value, focused.value);
    events(transport);
    restricted.read_only = true;
    apply(cx, handle, vec![set(restricted.clone())]);
    frame(cx, handle).await;
    applied(command(cx, handle, c::Command::Focus(c::Field::Hex)));
    restricted.disabled = true;
    apply(cx, handle, vec![set(restricted)]);
    frame(cx, handle).await;
    assert_eq!(
        command(cx, handle, c::Command::Focus(c::Field::Hex)),
        c::Response::Failed(c::Error::FocusBlocked)
    );
    assert_eq!(
        applied(command(cx, handle, c::Command::Reset { if_revision: None })).value,
        green
    );
    applied(command(cx, handle, c::Command::ReadSnapshot));
    apply(cx, handle, vec![set(config())]);
    frame(cx, handle).await;
    events(transport);
    eprintln!(
        "GPUIO_COLOR_COMMANDS_OK: pending platform edits, guarded Set, IME/reset/history, focus completion, policy rejection, read-only/disabled commands"
    );
}
