//! Actual native command-palette query, activation and document focus checks.
use super::*;
fn handler(slot: i64) -> gpuio_protocol::HandlerId {
    gpuio_protocol::HandlerId::from_parts(slot, 1).unwrap()
}
fn config() -> PaletteConfig {
    PaletteConfig {
        label: "Command search".into(),
        placeholder: "Find an action".into(),
        commands: vec!["run".into(), "disabled".into(), "copy".into()],
        dismiss_on_outside_pointer: true,
    }
}
fn events(transport: &Transport) -> Vec<Event> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter(|event| {
            matches!(
                event,
                Event::CommandInvoked(..) | Event::PaletteDismissed(..)
            )
        })
        .collect()
}
async fn mount(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, slot: i64) {
    mount_config(cx, handle, slot, config()).await;
}
async fn mount_config(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    slot: i64,
    configuration: PaletteConfig,
) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(slot),
                Kind::CommandPalette,
                "".into(),
                Some(handler(slot)),
            ),
            Op::SetPalette(node(slot), configuration),
            Op::SetChoiceAppearance(
                node(slot),
                ChoiceAppearance {
                    popup_width: 360.,
                    row_height: 28.,
                    max_visible_rows: 5,
                    ..Default::default()
                },
            ),
            Op::Splice(node(0), 4, 0, vec![node(slot)]),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, cx| {
            assert!(
                view.palettes[&node(slot)]
                    .query
                    .read(cx)
                    .focus_handle(cx)
                    .is_focused(window),
                "query receives native modal focus"
            );
            assert!(!view.editors[&node(4)].focus_handle(cx).is_focused(window));
        })
        .unwrap();
}
async fn remove(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, slot: i64) {
    apply(
        cx,
        handle,
        vec![Op::Splice(node(0), 4, 1, vec![]), Op::Remove(node(slot))],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| {
            assert!(!view.palettes.contains_key(&node(slot)))
        })
        .unwrap();
}
async fn large_palette(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let mut commands = handle
        .update(cx, |view, _, _| {
            view.session
                .borrow()
                .tree(view.id)
                .unwrap()
                .get(node(47))
                .unwrap()
                .commands
                .as_ref()
                .unwrap()
                .to_vec()
        })
        .unwrap();
    let ids = (0..1000)
        .map(|i| format!("item.{i:04}"))
        .collect::<Vec<_>>();
    commands.extend(ids.iter().enumerate().map(|(i, id)| CommandConfig {
        id: id.clone(),
        label: format!("Entry {i:04}"),
        generation: i as i64 + 100,
        enabled: true,
        checked: None,
        shortcuts: vec![],
        target: CommandTarget::Callback,
    }));
    apply(cx, handle, vec![Op::SetCommands(node(47), commands)]);
    mount_config(
        cx,
        handle,
        53,
        PaletteConfig {
            commands: ids.clone(),
            ..config()
        },
    )
    .await;
    let (bounds, _, count) = handle
        .update(cx, |view, _, _| view.palettes[&node(53)].geometry())
        .unwrap();
    assert!(count < 32, "result rows must be virtualized");
    let position = bounds.center();
    super::super::native_test::move_mouse(cx, handle, position, false);
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                position,
                delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(-280.))),
                touch_phase: gpui::TouchPhase::Moved,
                modifiers: Default::default(),
            }),
            cx,
        );
    })
    .unwrap();
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| {
            let (_, offset, count) = view.palettes[&node(53)].geometry();
            assert!(offset.y < px(0.));
            assert!(count < 32);
        })
        .unwrap();
    for _ in 0..124 {
        key(cx, handle, "pagedown");
    }
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| {
            assert_eq!(
                view.palettes[&node(53)].probe().1.as_deref(),
                Some("item.0992")
            );
            let (_, offset, count) = view.palettes[&node(53)].geometry();
            assert!(offset.y < px(-27000.));
            assert!(count < 32);
        })
        .unwrap();
    // Reordering an unchanged highlighted ID must still reveal its new position.
    let mut reversed = ids;
    reversed.reverse();
    apply(
        cx,
        handle,
        vec![Op::SetPalette(
            node(53),
            PaletteConfig {
                commands: reversed,
                ..config()
            },
        )],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| {
            let (_, offset, _) = view.palettes[&node(53)].geometry();
            assert!(offset.y > px(-250.));
        })
        .unwrap();
    cx.update(|cx| {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
            "x".repeat(PALETTE_QUERY_BYTES + 1),
        ))
    });
    key(
        cx,
        handle,
        if cfg!(target_os = "macos") {
            "cmd-v"
        } else {
            "ctrl-v"
        },
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, cx| {
            let query = view.palettes[&node(53)].query.read(cx);
            assert_eq!(
                query.value().len(),
                0,
                "oversize query insertion is rejected atomically"
            );
            assert!(query.bridge_history_bytes() <= PALETTE_HISTORY_BYTES);
        })
        .unwrap();
    events(transport);
    let position = gpui::point(px(1.), px(1.));
    super::super::native_test::move_mouse(cx, handle, position, false);
    super::super::native_test::mouse(cx, handle, position, true);
    super::super::native_test::mouse(cx, handle, position, false);
    frame(cx, handle).await;
    assert!(matches!(
        events(transport).as_slice(),
        [Event::PaletteDismissed(
            _,
            _,
            _,
            _,
            PaletteDismissal::OutsidePointer
        )]
    ));
    assert!(focused(cx, handle, node(4)));
    remove(cx, handle, 53).await;
    println!(
        "GPUIO_PALETTE_VIRTUAL_OK: 1000 commands, wheel/keyboard/reorder reveal, bounded query/history and outside dismissal"
    );
}
async fn surfaces(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    mount_config(
        cx,
        handle,
        54,
        PaletteConfig {
            dismiss_on_outside_pointer: false,
            ..config()
        },
    )
    .await;
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(54),
            vec![Style::Fields(vec![Field::PointerEvents(false)])],
        )],
    );
    frame(cx, handle).await;
    let position = handle
        .update(cx, |view, _, _| {
            view.palettes[&node(54)].row_bounds("run").center()
        })
        .unwrap();
    events(transport);
    super::super::native_test::move_mouse(cx, handle, position, false);
    super::super::native_test::mouse(cx, handle, position, true);
    super::super::native_test::mouse(cx, handle, position, false);
    frame(cx, handle).await;
    assert!(events(transport).is_empty());
    apply(cx, handle, vec![Op::SetStyle(node(54), vec![])]);
    frame(cx, handle).await;
    let position = gpui::point(px(1.), px(1.));
    super::super::native_test::move_mouse(cx, handle, position, false);
    super::super::native_test::mouse(cx, handle, position, true);
    super::super::native_test::mouse(cx, handle, position, false);
    frame(cx, handle).await;
    assert!(
        events(transport).is_empty(),
        "outside dismissal policy is enforced"
    );
    key(cx, handle, "escape");
    frame(cx, handle).await;
    remove(cx, handle, 54).await;
    mount(cx, handle, 55).await;
    events(transport);
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(55),
            vec![Style::Fields(vec![Field::Visibility(1)])],
        )],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| assert!(view.palettes[&node(55)].closed))
        .unwrap();
    assert!(focused(cx, handle, node(4)));
    assert!(matches!(
        events(transport).as_slice(),
        [Event::PaletteDismissed(
            _,
            _,
            _,
            _,
            PaletteDismissal::Escape
        )]
    ));
    remove(cx, handle, 55).await;
    // A palette can open above an existing modal and retain that modal's editor target.
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(56), Kind::FocusScope, "".into(), Some(handler(56))),
            Op::SetFocusScope(
                node(56),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::SetOverlay(
                node(56),
                Some(OverlayConfig {
                    kind: OverlayKind::Dialog,
                    label: "Document modal".into(),
                    width: 320.,
                    dismiss_on_escape: true,
                    dismiss_on_outside_pointer: false,
                }),
            ),
            Op::Splice(node(56), 0, 0, vec![node(4)]),
            Op::Splice(node(0), 3, 1, vec![node(56)]),
        ],
    );
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(4)));
    mount(cx, handle, 57).await;
    key(cx, handle, "down");
    events(transport);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert!(
        matches!(events(transport).as_slice(), [Event::PaletteDismissed(_, _, _, _, PaletteDismissal::Selected(id))] if id == "copy")
    );
    assert!(focused(cx, handle, node(4)));
    cx.update(|cx| {
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("Document selection")
        )
    });
    remove(cx, handle, 57).await;
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(56), 0, 1, vec![]),
            Op::Splice(node(0), 3, 1, vec![node(4)]),
            Op::Remove(node(56)),
        ],
    );
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(4)));
    println!(
        "GPUIO_PALETTE_SURFACES_OK: pointer/outside policies, hidden cleanup and nested-modal document editing/restoration"
    );
}
async fn updates(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    mount(cx, handle, 58).await;
    let query = handle
        .update(cx, |view, _, _| view.palettes[&node(58)].query.downgrade())
        .unwrap();
    let mut commands = handle
        .update(cx, |view, _, _| {
            view.session
                .borrow()
                .tree(view.id)
                .unwrap()
                .get(node(47))
                .unwrap()
                .commands
                .as_ref()
                .unwrap()
                .to_vec()
        })
        .unwrap();
    for command in &mut commands {
        command.enabled = false;
    }
    apply(
        cx,
        handle,
        vec![Op::SetCommands(node(47), commands.clone())],
    );
    events(transport);
    // Do not wait for paint: the old highlighted Run row must no longer execute.
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert!(events(transport).is_empty());
    commands[0].enabled = true;
    commands[0].generation += 2000;
    commands[0].label = "Updated run".into();
    apply(cx, handle, vec![Op::SetCommands(node(47), commands)]);
    events(transport);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert!(matches!(events(transport).as_slice(),
        [Event::CommandInvoked(_, _, _, _, id, 2001, _),
         Event::PaletteDismissed(_, _, _, _, PaletteDismissal::Selected(selected))]
        if id == "run" && selected == "run"));
    remove(cx, handle, 58).await;
    assert!(
        query.upgrade().is_none(),
        "removing a palette releases its query entity"
    );
    mount(cx, handle, 59).await;
    events(transport);
    remove(cx, handle, 59).await;
    assert!(
        focused(cx, handle, node(4)),
        "unmounting an open palette restores focus"
    );
    assert!(
        events(transport).is_empty(),
        "application unmount is not a user dismissal"
    );
    println!(
        "GPUIO_PALETTE_UPDATES_OK: current command disable/generation, query disposal and open unmount restoration"
    );
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    let commands = ["run", "disabled", "copy"]
        .iter()
        .enumerate()
        .map(|(i, id)| CommandConfig {
            id: (*id).into(),
            label: format!("Palette {id}"),
            generation: i as i64 + 1,
            enabled: *id != "disabled",
            checked: (*id == "run").then_some(true),
            shortcuts: vec![],
            target: if *id == "copy" {
                CommandTarget::Native(NativeCommand::Copy)
            } else {
                CommandTarget::Callback
            },
        })
        .collect();
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(47), Kind::CommandScope, "".into(), Some(handler(47))),
            Op::SetCommands(node(47), commands),
            Op::Splice(node(47), 0, 0, vec![node(0)]),
            Op::SetRoot(Some(node(47))),
        ],
    );
    handle
        .update(cx, |view, window, cx| {
            let result = view.editors.get_mut(&node(4)).unwrap().command(
                &EditorCommand::Replace(
                    "Document selection".into(),
                    EditorSelectionPolicy::Select(EditorSelection {
                        anchor: 0,
                        head: 18,
                    }),
                    EditorUndoPolicy::Reset,
                    None,
                ),
                window,
                cx,
            );
            assert!(matches!(result, EditorResult::Applied(_)));
            window.focus(&view.editors[&node(4)].focus_handle(cx), cx);
        })
        .unwrap();
    frame(cx, handle).await;
    mount(cx, handle, 48).await;
    events(transport);
    key(cx, handle, "down");
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| {
            let (closed, selected, rows) = view.palettes[&node(48)].probe();
            assert!(!closed);
            assert_eq!(selected.as_deref(), Some("copy"));
            assert_eq!(
                rows,
                [
                    ("run".into(), true),
                    ("disabled".into(), false),
                    ("copy".into(), true)
                ]
            );
        })
        .unwrap();
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(4)), "Escape restores the document");
    assert!(matches!(
        events(transport).as_slice(),
        [Event::PaletteDismissed(
            _,
            _,
            _,
            _,
            PaletteDismissal::Escape
        )]
    ));
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert!(
        events(transport).is_empty(),
        "closed palette cannot invoke again while awaiting removal"
    );
    remove(cx, handle, 48).await;
    mount(cx, handle, 49).await;
    cx.update(|cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string("copy".into())));
    key(
        cx,
        handle,
        if cfg!(target_os = "macos") {
            "cmd-v"
        } else {
            "ctrl-v"
        },
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, cx| {
            assert_eq!(
                view.palettes[&node(49)].query.read(cx).value().as_ref(),
                "copy"
            );
            assert_eq!(view.palettes[&node(49)].probe().2, [("copy".into(), true)]);
        })
        .unwrap();
    events(transport);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(4)));
    cx.update(|cx| {
        assert_eq!(
            cx.read_from_clipboard()
                .and_then(|item| item.text())
                .as_deref(),
            Some("Document selection")
        )
    });
    assert!(
        matches!(events(transport).as_slice(), [Event::PaletteDismissed(_, _, _, _, PaletteDismissal::Selected(id))] if id == "copy")
    );
    remove(cx, handle, 49).await;
    mount(cx, handle, 50).await;
    events(transport);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert!(
        matches!(events(transport).as_slice(), [Event::CommandInvoked(_, _, _, _, command, _, CommandSource::Palette(source)), Event::PaletteDismissed(_, _, _, _, PaletteDismissal::Selected(selected))] if command == "run" && selected == "run" && *source == node(50))
    );
    remove(cx, handle, 50).await;
    mount(cx, handle, 51).await;
    events(transport);
    cx.update(|cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string("copy".into())));
    key(
        cx,
        handle,
        if cfg!(target_os = "macos") {
            "cmd-v"
        } else {
            "ctrl-v"
        },
    );
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert!(
        matches!(events(transport).as_slice(), [Event::PaletteDismissed(_, _, _, _, PaletteDismissal::Selected(id))] if id == "copy"),
        "activation must filter against the current query without an intervening frame"
    );
    remove(cx, handle, 51).await;
    #[cfg(target_os = "macos")]
    {
        mount(cx, handle, 52).await;
        let _ = accessible_with_role(cx, handle, "Command search", Some("AXComboBox"), false);
        frame(cx, handle).await;
        assert!(
            accessible_with_role(cx, handle, "Command search", Some("AXComboBox"), false)
                .unwrap()
                .enabled
        );
        assert!(
            !accessible(cx, handle, "Palette disabled", false)
                .unwrap()
                .enabled
        );
        handle
            .update(cx, |view, window, cx| {
                window.focus(&view.focus.borrow().handle(node(52)).unwrap(), cx)
            })
            .unwrap();
        accessible_request(
            cx,
            handle,
            "Command search",
            Some("AXComboBox"),
            AccessibilityRequest::Focus,
        )
        .unwrap();
        frame(cx, handle).await;
        handle
            .update(cx, |view, window, cx| {
                assert!(
                    view.palettes[&node(52)]
                        .query
                        .read(cx)
                        .focus_handle(cx)
                        .is_focused(window)
                )
            })
            .unwrap();
        accessible_request(
            cx,
            handle,
            "Command search",
            Some("AXComboBox"),
            AccessibilityRequest::SetValue("disabled"),
        )
        .unwrap();
        frame(cx, handle).await;
        events(transport);
        key(cx, handle, "enter");
        frame(cx, handle).await;
        assert!(
            events(transport).is_empty(),
            "disabled-only results cannot activate"
        );
        accessible_request(
            cx,
            handle,
            "Command search",
            Some("AXComboBox"),
            AccessibilityRequest::SetValue(""),
        )
        .unwrap();
        frame(cx, handle).await;
        events(transport);
        super::super::editor_test::native_text(cx, handle, "ru", true);
        frame(cx, handle).await;
        key(cx, handle, "enter");
        frame(cx, handle).await;
        assert!(events(transport).is_empty());
        handle
            .update(cx, |view, _, cx| {
                assert!(!view.palettes[&node(52)].closed);
                assert!(
                    view.palettes[&node(52)]
                        .query
                        .read(cx)
                        .bridge_composition()
                        .is_some()
                );
            })
            .unwrap();
        super::super::editor_test::native_text(cx, handle, "run", false);
        frame(cx, handle).await;
        assert_eq!(
            accessible(cx, handle, "Palette run", true).unwrap().role,
            "AXStaticText"
        );
        frame(cx, handle).await;
        assert!(
            matches!(events(transport).as_slice(), [Event::CommandInvoked(_, _, _, _, command, _, _), Event::PaletteDismissed(_, _, _, _, PaletteDismissal::Selected(selected))] if command == "run" && selected == "run")
        );
        assert!(focused(cx, handle, node(4)));
        remove(cx, handle, 52).await;
        println!(
            "GPUIO_PALETTE_MACOS_OK: editable-combo/disabled semantics, native marked/committed text and AX activation/restoration"
        );
    }
    #[cfg(not(target_os = "macos"))]
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(52), Kind::Text, "".into(), None),
            Op::Remove(node(52)),
        ],
    );
    large_palette(cx, handle, transport).await;
    surfaces(cx, handle, transport).await;
    updates(cx, handle, transport).await;
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(47), 0, 1, vec![]),
            Op::SetRoot(Some(node(0))),
            Op::Remove(node(47)),
        ],
    );
    frame(cx, handle).await;
    println!(
        "GPUIO_PALETTE_NATIVE_OK: native query filtering, disabled navigation, modal focus/restoration, document Copy and command-before-dismissal ordering"
    );
}
