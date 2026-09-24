//! Actual window tests for the shared command registry and native priority.
use super::*;
fn handler(slot: i64) -> gpuio_protocol::HandlerId {
    gpuio_protocol::HandlerId::from_parts(slot, 1).unwrap()
}
fn shortcut(key: &str, modifiers: Vec<ShortcutModifier>) -> Shortcut {
    Shortcut {
        key: key.into(),
        modifiers,
        priority: ShortcutPriority::NativeFirst,
        text_input: ShortcutTextInput::ModifiedOnly,
        during_composition: false,
    }
}
fn config(id: &str, generation: i64, keys: Vec<Shortcut>) -> CommandConfig {
    CommandConfig {
        id: id.into(),
        generation,
        label: format!("{id} command"),
        enabled: true,
        checked: None,
        shortcuts: keys,
        target: CommandTarget::Callback,
    }
}
fn events(transport: &Transport) -> Vec<(NodeId, String, i64)> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::CommandInvoked(_, scope, _, _, id, generation, _) => {
                Some((scope, id, generation))
            }
            _ => None,
        })
        .collect()
}
fn focus(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, slot: i64) {
    handle
        .update(cx, |view, window, cx| {
            let focus = view
                .editors
                .get(&node(slot))
                .map(|editor| editor.focus_handle(cx))
                .unwrap_or_else(|| view.buttons[&node(slot)].focus.clone());
            window.focus(&focus, cx);
        })
        .unwrap();
}
async fn independent_window(
    cx: &mut gpui::AsyncApp,
    primary: WindowHandle<View>,
    transport: &Transport,
    chord: &str,
) {
    let (session, native_transport) = primary
        .update(cx, |view, _, _| {
            (view.session.clone(), view.transport.clone())
        })
        .unwrap();
    let id = WindowId::from_parts(1, 1).unwrap();
    let secondary = cx.update(|cx| {
        cx.open_window(
            WindowOptions {
                focus: false,
                inactive_frame_interval: Some(std::time::Duration::from_millis(16)),
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(id, session.clone(), native_transport)),
        )
        .unwrap()
    });
    session
        .borrow_mut()
        .open(2, id, "Command isolation", 400., 280.)
        .unwrap();
    let commands = vec![config(
        "run",
        40,
        vec![Shortcut {
            priority: ShortcutPriority::Override,
            ..shortcut("k", vec![ShortcutModifier::Primary])
        }],
    )];
    apply(
        cx,
        secondary,
        vec![
            Op::Create(node(0), Kind::CommandScope, "".into(), Some(handler(0))),
            Op::SetCommands(node(0), commands),
            Op::Create(node(1), Kind::CommandButton, "".into(), None),
            Op::SetCommandRef(node(1), "run".into()),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    frame(cx, secondary).await;
    focus(cx, secondary, 1);
    events(transport);
    let routed = |transport: &Transport| {
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .into_iter()
            .filter_map(|event| match event {
                Event::CommandInvoked(window, _, _, _, id, generation, _) => {
                    Some((window, id, generation))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    key(cx, secondary, chord);
    assert_eq!(routed(transport), [(id, "run".into(), 40)]);
    key(cx, primary, chord);
    assert_eq!(
        routed(transport),
        [(WindowId::from_parts(0, 1).unwrap(), "run".into(), 8)]
    );
    secondary
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    session.borrow_mut().close(id).unwrap();
    frame(cx, primary).await;
    key(cx, primary, chord);
    assert_eq!(
        routed(transport),
        [(WindowId::from_parts(0, 1).unwrap(), "run".into(), 8)]
    );
    println!(
        "GPUIO_COMMAND_WINDOWS_OK: per-window shortcut routing and surviving-window operation after close"
    );
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    let primary = if cfg!(target_os = "macos") {
        "cmd"
    } else {
        "ctrl"
    };
    let mut commands = vec![
        config(
            "run",
            1,
            vec![shortcut("k", vec![ShortcutModifier::Primary])],
        ),
        config(
            "copy-override",
            2,
            vec![shortcut("c", vec![ShortcutModifier::Primary])],
        ),
        config(
            "enter",
            3,
            vec![Shortcut {
                text_input: ShortcutTextInput::Always,
                ..shortcut("enter", vec![])
            }],
        ),
        config(
            "tab",
            4,
            vec![Shortcut {
                text_input: ShortcutTextInput::Always,
                ..shortcut("tab", vec![])
            }],
        ),
        CommandConfig {
            target: CommandTarget::Native(NativeCommand::Copy),
            ..config("native-copy", 5, vec![])
        },
        config(
            "typing",
            6,
            vec![Shortcut {
                priority: ShortcutPriority::Override,
                ..shortcut("x", vec![])
            }],
        ),
    ];
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(33), Kind::CommandScope, "".into(), Some(handler(33))),
            Op::SetCommands(node(33), commands.clone()),
            Op::SetStyle(
                node(33),
                vec![
                    Style::Width(Length::Percent(100.)),
                    Style::Height(Length::Percent(100.)),
                ],
            ),
            Op::Create(node(34), Kind::CommandButton, "".into(), None),
            Op::SetCommandRef(node(34), "run".into()),
            Op::SetStyle(
                node(34),
                vec![
                    Style::Width(Length::Px(160.)),
                    Style::Height(Length::Px(28.)),
                ],
            ),
            Op::Splice(node(0), 4, 0, vec![node(34)]),
            Op::Splice(node(33), 0, 0, vec![node(0)]),
            Op::SetRoot(Some(node(33))),
        ],
    );
    frame(cx, handle).await;
    focus(cx, handle, 4);
    frame(cx, handle).await;
    events(transport);
    key(cx, handle, &format!("{primary}-k"));
    assert_eq!(events(transport), [(node(33), "run".into(), 1)]);
    key(cx, handle, "x");
    assert!(
        events(transport).is_empty(),
        "modified-only preserves ordinary editing"
    );
    key(cx, handle, "enter");
    assert!(
        events(transport).is_empty(),
        "native submission wins over Native_first"
    );
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(
        events(transport).is_empty(),
        "native traversal wins over Native_first"
    );
    assert!(focused(cx, handle, node(34)));
    key(cx, handle, "enter");
    assert_eq!(
        events(transport),
        [(node(33), "run".into(), 1)],
        "command button activates once, without the window Enter shortcut"
    );
    #[cfg(target_os = "macos")]
    {
        assert!(
            accessible(cx, handle, "run command", false)
                .unwrap()
                .enabled
        );
        accessible(cx, handle, "run command", true).unwrap();
        frame(cx, handle).await;
        assert_eq!(events(transport), [(node(33), "run".into(), 1)]);
    }
    focus(cx, handle, 4);
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, cx| {
            let result = view.editors.get_mut(&node(4)).unwrap().command(
                &EditorCommand::Replace(
                    "native command copy".into(),
                    EditorSelectionPolicy::Select(EditorSelection {
                        anchor: 0,
                        head: 19,
                    }),
                    EditorUndoPolicy::Reset,
                    None,
                ),
                window,
                cx,
            );
            assert!(matches!(result, EditorResult::Applied(_)));
        })
        .unwrap();
    frame(cx, handle).await;
    events(transport);
    key(cx, handle, &format!("{primary}-c"));
    assert!(events(transport).is_empty(), "native Copy takes precedence");
    commands[1].shortcuts[0].priority = ShortcutPriority::Override;
    apply(
        cx,
        handle,
        vec![Op::SetCommands(node(33), commands.clone())],
    );
    frame(cx, handle).await;
    key(cx, handle, &format!("{primary}-c"));
    assert_eq!(events(transport), [(node(33), "copy-override".into(), 2)]);
    #[cfg(target_os = "macos")]
    {
        super::super::editor_test::native_text(cx, handle, "に", true);
        frame(cx, handle).await;
        events(transport);
        key(cx, handle, &format!("{primary}-c"));
        assert!(
            events(transport).is_empty(),
            "composition suppresses override by default"
        );
        commands[1].shortcuts[0].during_composition = true;
        apply(
            cx,
            handle,
            vec![Op::SetCommands(node(33), commands.clone())],
        );
        frame(cx, handle).await;
        key(cx, handle, &format!("{primary}-c"));
        assert_eq!(events(transport), [(node(33), "copy-override".into(), 2)]);
        key(cx, handle, "escape");
        frame(cx, handle).await;
    }
    // A native command button shares availability and restores the editing
    // target after keyboard focus moves to the toolbar.
    handle
        .update(cx, |view, window, cx| {
            view.editors.get_mut(&node(4)).unwrap().command(
                &EditorCommand::Replace(
                    "native command copy".into(),
                    EditorSelectionPolicy::Select(EditorSelection {
                        anchor: 0,
                        head: 19,
                    }),
                    EditorUndoPolicy::Reset,
                    None,
                ),
                window,
                cx,
            );
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetCommandRef(node(34), "native-copy".into())],
    );
    frame(cx, handle).await;
    events(transport);
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(34)));
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert!(
        events(transport).is_empty(),
        "native edit commands do not roundtrip to OCaml"
    );
    assert!(focused(cx, handle, node(4)));
    handle
        .update(cx, |_, _, cx| {
            assert_eq!(
                cx.read_from_clipboard()
                    .and_then(|item| item.text())
                    .as_deref(),
                Some("native command copy")
            )
        })
        .unwrap();
    commands[0].enabled = false;
    commands[0].generation = 7;
    apply(
        cx,
        handle,
        vec![
            Op::SetCommandRef(node(34), "run".into()),
            Op::SetCommands(node(33), commands.clone()),
        ],
    );
    frame(cx, handle).await;
    key(cx, handle, &format!("{primary}-k"));
    assert!(events(transport).is_empty());
    #[cfg(target_os = "macos")]
    assert!(
        !accessible(cx, handle, "run command", false)
            .unwrap()
            .enabled
    );
    commands[0].enabled = true;
    commands[0].generation = 8;
    apply(
        cx,
        handle,
        vec![Op::SetCommands(node(33), commands.clone())],
    );
    frame(cx, handle).await;
    key(cx, handle, &format!("{primary}-k"));
    assert_eq!(events(transport), [(node(33), "run".into(), 8)]);
    let mut inner = config(
        "run",
        9,
        vec![shortcut("k", vec![ShortcutModifier::Primary])],
    );
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(35), Kind::CommandScope, "".into(), Some(handler(35))),
            Op::SetCommands(node(35), vec![inner.clone()]),
            Op::SetStyle(
                node(35),
                vec![Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(220.)),
                    Field::Top(Length::Px(150.)),
                    Field::Width(Length::Px(150.)),
                ])],
            ),
            Op::Create(node(36), Kind::CommandButton, "".into(), None),
            Op::SetCommandRef(node(36), "run".into()),
            Op::Create(
                node(37),
                Kind::Button,
                "Inner focus".into(),
                Some(handler(37)),
            ),
            Op::SetControl(node(37), Control::Button(false)),
            Op::Splice(node(35), 0, 0, vec![node(36), node(37)]),
            Op::Splice(node(0), 5, 0, vec![node(35)]),
        ],
    );
    frame(cx, handle).await;
    focus(cx, handle, 37);
    frame(cx, handle).await;
    key(cx, handle, &format!("{primary}-k"));
    assert_eq!(events(transport), [(node(35), "run".into(), 9)]);
    inner.enabled = false;
    inner.generation = 10;
    apply(cx, handle, vec![Op::SetCommands(node(35), vec![inner])]);
    frame(cx, handle).await;
    key(cx, handle, &format!("{primary}-k"));
    assert!(
        events(transport).is_empty(),
        "disabled inner definition shadows the outer command"
    );
    apply(cx, handle, vec![Op::SetCommands(node(35), vec![])]);
    frame(cx, handle).await;
    key(cx, handle, &format!("{primary}-k"));
    assert_eq!(events(transport), [(node(33), "run".into(), 8)]);
    independent_window(cx, handle, transport, &format!("{primary}-k")).await;
    apply(
        cx,
        handle,
        vec![
            Op::Remove(node(37)),
            Op::Remove(node(36)),
            Op::Remove(node(35)),
            Op::Remove(node(34)),
            Op::Splice(node(0), 4, 2, vec![]),
            Op::Splice(node(33), 0, 1, vec![]),
            Op::SetRoot(Some(node(0))),
            Op::Remove(node(33)),
        ],
    );
    frame(cx, handle).await;
    key(cx, handle, &format!("{primary}-k"));
    assert!(events(transport).is_empty());
    println!(
        "GPUIO_COMMANDS_NATIVE_OK: shared buttons/shortcuts, native-first/override, edit targets, composition policy, nested scopes, disabled shadowing and disposal"
    );
}
