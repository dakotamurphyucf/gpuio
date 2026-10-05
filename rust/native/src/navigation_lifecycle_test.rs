//! Nested route/focus lifetimes and bounded mounted history in a real window.
use super::*;

fn config(selected: i64) -> gpuio_protocol::navigation_stack::Config {
    gpuio_protocol::navigation_stack::Config {
        selected: Some(selected),
        retain: true,
        motion: gpuio_protocol::navigation_stack::Motion::Immediate,
        duration_ms: 0,
    }
}
fn hidden(id: i64) -> Op {
    Op::SetStyle(node(id), vec![Style::Fields(vec![Field::Display(3)])])
}
fn extent(id: i64) -> Op {
    Op::SetStyle(
        node(id),
        vec![Style::Fields(vec![
            Field::Width(Length::Percent(100.)),
            Field::Height(Length::Percent(100.)),
        ])],
    )
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
    mut editor: EditorConfig,
) {
    let mut operations = Vec::new();
    for (id, kind, label) in [
        (63, Kind::NavigationStack, "Outer routes"),
        (64, Kind::Panel, "Nested route"),
        (65, Kind::NavigationStack, "Inner routes"),
        (66, Kind::Panel, "Inner first"),
        (67, Kind::Button, "Inner action"),
        (68, Kind::Panel, "Inner editor"),
        (69, Kind::Input, "Nested draft"),
        (70, Kind::Panel, "Outer second"),
        (71, Kind::Button, "Outer action"),
        (72, Kind::FocusScope, ""),
        (73, Kind::Button, "Modal action"),
        (74, Kind::Container, ""),
    ] {
        let handler = matches!(kind, Kind::Button | Kind::Input | Kind::FocusScope)
            .then(|| gpuio_protocol::HandlerId::from_parts(id, 1).unwrap());
        operations.push(Op::Create(node(id), kind, label.into(), handler));
        if kind == Kind::Button {
            operations.push(Op::SetControl(node(id), Control::Button(false)));
        }
    }
    editor.label = "Nested draft".into();
    operations.extend([
        Op::SetEditor(node(69), editor.clone()),
        Op::SetNavigationStack(node(63), config(0)),
        Op::SetNavigationStack(node(65), config(0)),
        extent(63),
        extent(64),
        extent(65),
        extent(66),
        extent(68),
        extent(70),
        extent(74),
        Op::Splice(node(66), 0, 0, vec![node(67)]),
        Op::Splice(node(68), 0, 0, vec![node(69)]),
        Op::Splice(node(65), 0, 0, vec![node(66), node(68)]),
        Op::Splice(node(64), 0, 0, vec![node(65)]),
        Op::Splice(node(70), 0, 0, vec![node(71)]),
        Op::Splice(node(63), 0, 0, vec![node(64), node(70)]),
        Op::SetFocusScope(
            node(72),
            FocusScopeConfig {
                trap: true,
                auto_focus: true,
                restore_focus: true,
            },
        ),
        Op::SetOverlay(
            node(72),
            Some(OverlayConfig {
                kind: OverlayKind::Dialog,
                label: "Navigation modal".into(),
                width: 220.,
                dismiss_on_escape: true,
                dismiss_on_outside_pointer: true,
            }),
        ),
        Op::Splice(node(72), 0, 0, vec![node(73)]),
        hidden(72),
        Op::Splice(node(74), 0, 0, vec![node(63), node(72)]),
        Op::SetRoot(Some(node(74))),
    ]);
    apply(cx, handle, operations);
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(67)));
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(node(65), config(1))],
    );
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(69)));
    #[cfg(target_os = "macos")]
    {
        super::super::editor_test::native_text(cx, handle, "日本", true);
        frame(cx, handle).await;
        handle
            .update(cx, |v, w, cx| {
                assert!(v.editors[&node(69)].snapshot(w, cx).composition.is_some())
            })
            .unwrap();
    }
    let original_handle = handle
        .update(cx, |v, _, cx| v.editors[&node(69)].focus_handle(cx))
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(node(63), config(1))],
    );
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(71)));
    let before = handle
        .update(cx, |v, w, cx| v.editors[&node(69)].snapshot(w, cx))
        .unwrap();
    #[cfg(target_os = "macos")]
    super::super::editor_test::native_text(cx, handle, "must not reach hidden editor", false);
    handle
        .update(cx, |v, w, cx| {
            assert!(!v.focus.borrow().allows(node(69)));
            assert_eq!(v.editors[&node(69)].snapshot(w, cx).text, before.text);
            assert!(original_handle == v.editors[&node(69)].focus_handle(cx));
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(node(63), config(0))],
    );
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(69)),
        "outer back restores nested editor"
    );
    // A separate modal has precedence while a route changes behind it.
    apply(cx, handle, vec![Op::SetStyle(node(72), vec![])]);
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(73)));
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(node(63), config(1))],
    );
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(73)),
        "navigation cannot steal modal focus"
    );
    transport.mailbox.lock().unwrap().drain(128);
    key(cx, handle, "escape");
    assert!(transport.mailbox.lock().unwrap().drain(128).iter().any(|event|
        matches!(event, Event::OverlayDismissed(_, id, _, _, Dismissal::Escape) if *id == node(72))));
    assert!(
        focused(cx, handle, node(73)),
        "dismiss request alone does not release modal"
    );
    apply(cx, handle, vec![hidden(72)]);
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(71)),
        "closing modal focuses the current destination"
    );
    // Move the same retained modal into a page. Leaving that page retires its scope.
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(74), 1, 1, vec![]),
            Op::Splice(node(64), 1, 0, vec![node(72)]),
            Op::SetNavigationStack(node(63), config(0)),
            Op::SetStyle(node(72), vec![]),
        ],
    );
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(73)));
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(node(63), config(1))],
    );
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(71)),
        "outgoing modal cannot retain a trap"
    );
    handle
        .update(cx, |v, _, _| {
            assert!(v.focus.borrow().handle(node(72)).is_none())
        })
        .unwrap();
    transport.mailbox.lock().unwrap().drain(128);
    key(cx, handle, "escape");
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(event, Event::OverlayDismissed(..)))
    );
    apply(
        cx,
        handle,
        std::iter::once(Op::SetRoot(None))
            .chain((63..=74).map(|id| Op::Remove(node(id))))
            .collect(),
    );
    frame(cx, handle).await;
    handle
        .update(cx, |v, _, _| {
            assert!(v.navigation.is_empty());
            assert!(v.editors.is_empty());
            assert_eq!(v.session.borrow().retained_bytes(), 0);
        })
        .unwrap();
    println!(
        "GPUIO_NAVIGATION_NESTED_OK: nested history, IME isolation, higher modal precedence, Escape ordering, outgoing modal retirement and disposal"
    );
    workload(cx, handle, transport, editor).await;
}

async fn workload(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
    editor: EditorConfig,
) {
    let page = |index: i64| 76 + index * 3;
    let button = |index: i64| page(index) + 1;
    let input = |index: i64| page(index) + 2;
    let mut operations = vec![
        Op::Create(
            node(75),
            Kind::NavigationStack,
            "Large history".into(),
            None,
        ),
        Op::SetNavigationStack(node(75), config(0)),
        extent(75),
    ];
    for index in 0..128 {
        let mut editor = editor.clone();
        editor.label = format!("Draft {index}");
        operations.extend([
            Op::Create(
                node(page(index)),
                Kind::Panel,
                format!("Page {index}"),
                None,
            ),
            Op::Create(
                node(button(index)),
                Kind::Button,
                format!("Action {index}"),
                Some(gpuio_protocol::HandlerId::from_parts(button(index), 1).unwrap()),
            ),
            Op::Create(
                node(input(index)),
                Kind::Input,
                format!("Stable draft {index}"),
                Some(gpuio_protocol::HandlerId::from_parts(input(index), 1).unwrap()),
            ),
            Op::SetControl(node(button(index)), Control::Button(false)),
            Op::SetEditor(node(input(index)), editor),
            Op::SetStyle(
                node(page(index)),
                vec![Style::Fields(vec![
                    Field::Width(Length::Percent(100.)),
                    Field::Height(Length::Percent(100.)),
                    Field::Background(Fill::Solid(Color::Rgba(0x17314fff))),
                ])],
            ),
            Op::Splice(
                node(page(index)),
                0,
                0,
                vec![node(button(index)), node(input(index))],
            ),
        ]);
    }
    operations.extend([
        Op::Splice(
            node(75),
            0,
            0,
            (0..128).map(|index| node(page(index))).collect(),
        ),
        Op::SetRoot(Some(node(75))),
    ]);
    // The established editor quota reserves 8 MiB per native input. Verify the
    // oversized proposal rolls back, then mount 128 pages with four editor pages
    // and ordinary readonly content on the others, within the unchanged quota.
    handle
        .update(cx, |v, _, _| {
            let base = v.session.borrow().tree(v.id).unwrap().revision();
            let result = v.session.borrow_mut().apply(&Transaction {
                window: v.id,
                base,
                revision: base + 1,
                operations: operations.clone(),
            });
            assert_eq!(result, Err(ErrorCode::LimitExceeded));
            assert_eq!(v.session.borrow().tree(v.id).unwrap().revision(), base);
            assert_eq!(v.session.borrow().retained_bytes(), 0);
            assert!(v.editors.is_empty());
        })
        .unwrap();
    let operations = operations
        .into_iter()
        .filter_map(|op| match op {
            Op::Create(id, Kind::Input, text, _) if ((id.slot() as i64 - 78) / 3) % 32 != 0 => {
                Some(Op::Create(id, Kind::Text, text, None))
            }
            Op::SetEditor(id, _) if ((id.slot() as i64 - 78) / 3) % 32 != 0 => None,
            op => Some(op),
        })
        .collect();
    apply(cx, handle, operations);
    frame(cx, handle).await;
    let retained = handle
        .update(cx, |v, _, _| v.session.borrow().retained_bytes())
        .unwrap();
    let identities = handle
        .update(cx, |v, _, cx| {
            (0..128)
                .map(|index| {
                    (
                        v.buttons[&node(button(index))].focus.clone(),
                        v.editors
                            .get(&node(input(index)))
                            .map(|editor| editor.focus_handle(cx)),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap();
    let mut latencies = Vec::new();
    for index in 0..128 {
        let started = std::time::Instant::now();
        apply(
            cx,
            handle,
            vec![Op::SetNavigationStack(node(75), config(index))],
        );
        frame(cx, handle).await;
        latencies.push(started.elapsed().as_micros());
        assert!(
            focused(cx, handle, node(button(index))),
            "focus at page {index}"
        );
        transport.mailbox.lock().unwrap().drain(128);
        key(cx, handle, "enter");
        assert_eq!(presses(transport), [node(button(index))]);
        handle
            .update(cx, |v, w, cx| {
                assert_eq!(v.editors.len(), 4);
                assert_eq!(v.buttons.len(), 128);
                assert_eq!(v.navigation.len(), 1);
                let (owners, remembered, pending) = v.focus.borrow().navigation_test_stats();
                assert_eq!(owners, 1);
                assert!(remembered <= 128 && pending <= 1);
                assert_eq!(v.session.borrow().retained_bytes(), retained);
                if let Some(editor) = v.editors.get(&node(input(index))) {
                    assert_eq!(editor.snapshot(w, cx).text, format!("Stable draft {index}"));
                }
                for (index, (button_handle, editor_handle)) in identities.iter().enumerate() {
                    assert!(button_handle == &v.buttons[&node(button(index as i64))].focus);
                    if let Some(editor_handle) = editor_handle {
                        assert!(
                            editor_handle
                                == &v.editors[&node(input(index as i64))].focus_handle(cx)
                        );
                    }
                }
            })
            .unwrap();
    }
    let mut sliding = config(0);
    sliding.motion = gpuio_protocol::navigation_stack::Motion::Slide;
    sliding.duration_ms = 2000;
    apply(cx, handle, vec![Op::SetNavigationStack(node(75), sliding)]);
    frame(cx, handle).await;
    for (width, height) in [(300., 180.), (500., 320.), (400., 280.)] {
        handle
            .update(cx, |_, w, _| w.resize(gpui::size(px(width), px(height))))
            .unwrap();
        // A GPUI frame can precede AppKit's asynchronous resize notification.
        // Wait for the actual viewport, then paint and test the child geometry.
        let expected = gpui::size(px(width), px(height));
        let mut resized = false;
        for _ in 0..200 {
            resized = handle
                .update(cx, |_, window, _| window.viewport_size() == expected)
                .unwrap();
            if resized {
                break;
            }
            cx.background_executor()
                .timer(std::time::Duration::from_millis(10))
                .await;
        }
        assert!(
            resized,
            "navigation native viewport did not resize to {expected:?}"
        );
        frame(cx, handle).await;
        handle
            .update(cx, |v, w, _| {
                assert_eq!(
                    v.probes.borrow()[&node(button(127))].bounds.size.width,
                    px(width)
                );
                let incoming = v.probes.borrow()[&node(page(0))].bounds;
                let outgoing = v.probes.borrow()[&node(page(127))].bounds;
                assert_eq!(incoming.size.width, px(width));
                assert!(
                    (f32::from(outgoing.left() - incoming.left()) - width).abs() <= 1.,
                    "sliding offsets adapt to the new assigned width"
                );
                #[cfg(feature = "native-image-tests")]
                {
                    let image = w.render_to_image().unwrap();
                    let scale = w.scale_factor();
                    assert_eq!(
                        image
                            .get_pixel(
                                ((width - 4.) * scale) as u32,
                                ((height - 4.) * scale) as u32
                            )
                            .0,
                        [0x17, 0x31, 0x4f, 0xff],
                        "selected page fills resized viewport"
                    );
                }
                #[cfg(not(feature = "native-image-tests"))]
                let _ = w;
            })
            .unwrap();
    }
    // Empty inactive pages preserve history positions but release all their widgets.
    let mut unmount = config(0);
    unmount.retain = false;
    let mut operations = vec![Op::SetNavigationStack(node(75), unmount)];
    for index in 1..128 {
        operations.extend([
            Op::Splice(node(page(index)), 0, 2, vec![]),
            Op::Remove(node(button(index))),
            Op::Remove(node(input(index))),
        ]);
    }
    apply(cx, handle, operations);
    frame(cx, handle).await;
    handle
        .update(cx, |v, _, _| {
            assert_eq!(v.editors.len(), 1);
            assert_eq!(v.buttons.len(), 1);
            assert!(v.session.borrow().retained_bytes() < retained);
        })
        .unwrap();
    drop(identities);
    let mut operations = vec![
        Op::SetRoot(None),
        Op::Remove(node(75)),
        Op::Remove(node(button(0))),
        Op::Remove(node(input(0))),
    ];
    operations.extend((0..128).map(|index| Op::Remove(node(page(index)))));
    apply(cx, handle, operations);
    frame(cx, handle).await;
    handle
        .update(cx, |v, _, _| {
            assert!(v.editors.is_empty() && v.buttons.is_empty() && v.navigation.is_empty());
            assert_eq!(v.focus.borrow().navigation_test_stats(), (0, 0, 0));
            assert_eq!(v.session.borrow().retained_bytes(), 0);
        })
        .unwrap();
    latencies.sort_unstable();
    println!(
        "GPUIO_NAVIGATION_WORKLOAD_OK: 128 pages/buttons with four editor pages, full traversal and input, stable ownership/accounting, resize GPU, Unmount release; apply-through-two-frame-wait median={}us p95={}us (includes display scheduling)",
        latencies[64], latencies[121]
    );
}
