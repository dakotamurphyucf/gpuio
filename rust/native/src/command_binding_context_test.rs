//! Resolve the same retained registries through inspection and invocation.
use super::*;

fn other(enabled: bool) -> CommandConfig {
    CommandConfig {
        id: "other".into(),
        ..command(enabled)
    }
}
fn invoke(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
    priority: ShortcutPriority,
    expected: Option<&str>,
) {
    handle
        .update(cx, |view, window, cx| {
            let key = gpui::Keystroke {
                key: "k".into(),
                modifiers: gpui::Modifiers {
                    #[cfg(target_os = "macos")]
                    platform: true,
                    #[cfg(not(target_os = "macos"))]
                    control: true,
                    ..Default::default()
                },
                key_char: None,
            };
            view.command_shortcut(&key, priority, window, cx);
        })
        .unwrap();
    let invoked: Vec<_> = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| match event {
            Event::CommandInvoked(_, _, _, _, id, _, _) => Some(id),
            _ => None,
        })
        .collect();
    assert_eq!(
        invoked,
        expected.into_iter().map(str::to_owned).collect::<Vec<_>>()
    );
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    transport: &Transport,
) {
    // The editor retains identity while a nearer scope is inserted around it.
    let identity = window
        .update(cx, |view, window, cx| {
            let focus = view.editors[&node(2)].focus_handle(cx);
            window.focus(&focus, cx);
            focus
        })
        .unwrap();
    apply(
        cx,
        window,
        vec![
            Op::Create(node(7), Kind::CommandScope, "".into(), Some(handler(7, 1))),
            Op::SetCommands(node(7), vec![command(false)]),
            Op::Splice(node(1), 0, 1, vec![node(7)]),
            Op::Splice(node(7), 0, 0, vec![node(2)]),
        ],
    );
    frame(cx, window).await;
    assert_eq!(
        registry(&observations(transport)[&node(3)]),
        binding::Disposition::Unavailable(binding::Suppression::Disabled)
    );
    invoke(cx, window, transport, ShortcutPriority::Override, None);

    let mut unbound = command(true);
    unbound.shortcuts.clear();
    apply(cx, window, vec![Op::SetCommands(node(7), vec![unbound])]);
    frame(cx, window).await;
    let samples = observations(transport);
    assert!(
        matches!(&samples[&node(3)].state, binding::State::Ready(entries)
        if matches!(&entries[0], binding::Entry::Registry { enabled: true, candidates } if candidates.is_empty())),
        "nearest unbound ID must shadow the outer bound ID"
    );
    invoke(cx, window, transport, ShortcutPriority::Override, None);

    apply(
        cx,
        window,
        vec![Op::SetCommands(node(7), vec![other(false)])],
    );
    frame(cx, window).await;
    assert_eq!(
        registry(&observations(transport)[&node(3)]),
        binding::Disposition::Unavailable(binding::Suppression::Conflict("other".into()))
    );
    invoke(cx, window, transport, ShortcutPriority::Override, None);
    apply(
        cx,
        window,
        vec![Op::SetCommands(node(7), vec![other(true)])],
    );
    frame(cx, window).await;
    assert!(
        observations(transport).is_empty(),
        "same conflict result stays silent"
    );
    invoke(
        cx,
        window,
        transport,
        ShortcutPriority::Override,
        Some("other"),
    );

    let mut native_first = command(true);
    native_first.shortcuts[0].priority = ShortcutPriority::NativeFirst;
    apply(
        cx,
        window,
        vec![
            Op::SetCommands(node(0), vec![native_first.clone()]),
            Op::SetCommands(node(7), vec![other(false)]),
        ],
    );
    frame(cx, window).await;
    assert_eq!(
        registry(&observations(transport)[&node(3)]),
        binding::Disposition::NativeFirst
    );
    invoke(cx, window, transport, ShortcutPriority::Override, None);
    invoke(
        cx,
        window,
        transport,
        ShortcutPriority::NativeFirst,
        Some("run"),
    );
    apply(
        cx,
        window,
        vec![Op::SetCommands(node(7), vec![other(true)])],
    );
    frame(cx, window).await;
    assert_eq!(
        registry(&observations(transport)[&node(3)]),
        binding::Disposition::Unavailable(binding::Suppression::Conflict("other".into()))
    );
    invoke(
        cx,
        window,
        transport,
        ShortcutPriority::Override,
        Some("other"),
    );

    apply(
        cx,
        window,
        vec![Op::SetCommands(node(0), vec![command(true)])],
    );
    frame(cx, window).await;
    // Candidate priority changes even though the conflict ID remains the same.
    assert_eq!(
        registry(&observations(transport)[&node(3)]),
        binding::Disposition::Unavailable(binding::Suppression::Conflict("other".into()))
    );
    apply(
        cx,
        window,
        vec![
            Op::Create(node(8), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                node(8),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Create(
                node(9),
                Kind::Button,
                "Modal action".into(),
                Some(handler(9, 1)),
            ),
            Op::Splice(node(8), 0, 0, vec![node(9)]),
            Op::Splice(node(0), 3, 0, vec![node(8)]),
        ],
    );
    frame(cx, window).await;
    assert!(
        window
            .update(cx, |view, window, _| view.buttons[&node(9)]
                .focus
                .is_focused(window))
            .unwrap()
    );
    assert_eq!(
        registry(&observations(transport)[&node(3)]),
        binding::Disposition::Override,
        "the modal's focus path sees the common ancestor, not the editor's sibling registry"
    );
    invoke(
        cx,
        window,
        transport,
        ShortcutPriority::Override,
        Some("run"),
    );
    apply(
        cx,
        window,
        vec![
            Op::Splice(node(0), 3, 1, vec![]),
            Op::Remove(node(9)),
            Op::Remove(node(8)),
        ],
    );
    frame(cx, window).await;
    assert!(
        window
            .update(cx, |_, window, _| identity.is_focused(window))
            .unwrap(),
        "modal close restores editor focus"
    );
    assert_eq!(
        registry(&observations(transport)[&node(3)]),
        binding::Disposition::Unavailable(binding::Suppression::Conflict("other".into()))
    );
    invoke(
        cx,
        window,
        transport,
        ShortcutPriority::Override,
        Some("other"),
    );
    apply(
        cx,
        window,
        vec![
            Op::Splice(node(7), 0, 1, vec![]),
            Op::Splice(node(1), 0, 1, vec![node(2)]),
            Op::Remove(node(7)),
        ],
    );
    frame(cx, window).await;
    assert_eq!(
        registry(&observations(transport)[&node(3)]),
        binding::Disposition::Override
    );
    assert_eq!(
        window
            .update(cx, |view, _, cx| view.editors[&node(2)].focus_handle(cx))
            .unwrap(),
        identity
    );
    eprintln!(
        "GPUIO_BINDING_CONTEXTS_OK: nested shadowing, disabled/unbound/conflicting commands, phase precedence, modal focus/restoration and invocation"
    );
}
