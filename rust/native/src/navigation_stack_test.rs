//! Mounted routes: real focus/input and retained owners, not only timeline samples.
use super::*;

fn config(selected: i64) -> gpuio_protocol::navigation_stack::Config {
    gpuio_protocol::navigation_stack::Config {
        selected: Some(selected),
        retain: true,
        motion: gpuio_protocol::navigation_stack::Motion::Slide,
        duration_ms: 400,
    }
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
    editor: EditorConfig,
) {
    let mut operations = Vec::new();
    for (id, kind, label) in [
        (54, Kind::NavigationStack, "Route viewport"),
        (55, Kind::Panel, "First route"),
        (56, Kind::Input, "First draft"),
        (57, Kind::Button, "First action"),
        (58, Kind::Panel, "Second route"),
        (59, Kind::Input, "Second draft"),
        (60, Kind::Button, "Second action"),
        (61, Kind::Panel, "Third route"),
        (62, Kind::Button, "Third action"),
    ] {
        let handler = matches!(kind, Kind::Input | Kind::Button)
            .then(|| gpuio_protocol::HandlerId::from_parts(id, 1).unwrap());
        operations.push(Op::Create(node(id), kind, label.into(), handler));
        if kind == Kind::Button {
            operations.push(Op::SetControl(node(id), Control::Button(false)));
        }
    }
    for (id, label) in [(56, "First draft"), (59, "Second draft")] {
        let mut config = editor.clone();
        config.label = label.into();
        operations.push(Op::SetEditor(node(id), config));
    }
    operations.extend([
        Op::SetNavigationStack(node(54), config(0)),
        Op::SetStyle(
            node(54),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(320.)),
                Field::Height(Length::Px(240.)),
            ])],
        ),
        Op::SetStyle(
            node(55),
            vec![Style::Fields(vec![
                Field::Width(Length::Percent(100.)),
                Field::Height(Length::Percent(100.)),
                Field::Background(Fill::Solid(Color::Rgba(0xe13599ff))),
            ])],
        ),
        Op::SetStyle(
            node(58),
            vec![Style::Fields(vec![
                Field::Width(Length::Percent(100.)),
                Field::Height(Length::Percent(100.)),
                Field::Background(Fill::Solid(Color::Rgba(0x2563ebff))),
            ])],
        ),
        Op::Splice(node(55), 0, 0, vec![node(56), node(57)]),
        Op::Splice(node(58), 0, 0, vec![node(59), node(60)]),
        Op::Splice(node(61), 0, 0, vec![node(62)]),
        Op::Splice(node(54), 0, 0, vec![node(55), node(58), node(61)]),
        Op::SetRoot(Some(node(54))),
    ]);
    apply(cx, handle, operations);
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(56)),
        "first route receives initial focus"
    );
    let first_handle = handle
        .update(cx, |v, _, cx| v.editors[&node(56)].focus_handle(cx))
        .unwrap();
    handle
        .update(cx, |v, w, cx| w.focus(&v.buttons[&node(57)].focus, cx))
        .unwrap();
    frame(cx, handle).await;
    let mut slow = config(1);
    slow.duration_ms = 2000;
    apply(cx, handle, vec![Op::SetNavigationStack(node(54), slow)]);
    frame(cx, handle).await;
    // A slow transition leaves a wide interval for checking both actual GPU pages.
    cx.background_executor()
        .timer(std::time::Duration::from_millis(400))
        .await;
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(59)),
        "destination first control receives focus"
    );
    handle
        .update(cx, |v, w, cx| {
            assert!(!v.focus.borrow().visible(node(56)));
            assert!(first_handle == v.editors[&node(56)].focus_handle(cx));
            assert!(matches!(
                v.editors
                    .get_mut(&node(56))
                    .unwrap()
                    .command(&EditorCommand::Focus, w, cx),
                EditorResult::Failed(EditorError::FocusBlocked)
            ));
            assert!(
                v.buttons.contains_key(&node(57)),
                "inactive controls retain identity"
            );
        })
        .unwrap();
    #[cfg(feature = "native-image-tests")]
    handle
        .update(cx, |v, w, _| {
            let image = w.render_to_image().unwrap();
            let bounds = v.probes.borrow()[&node(54)].bounds;
            let scale = w.scale_factor();
            for (x, expected) in [
                (bounds.left() + px(4.), [0xe1, 0x35, 0x99, 0xff]),
                (bounds.right() - px(4.), [0x25, 0x63, 0xeb, 0xff]),
            ] {
                let pixel = image.get_pixel(
                    (f32::from(x) * scale) as u32,
                    (f32::from(bounds.top() + px(200.)) * scale) as u32,
                );
                assert_eq!(
                    pixel.0, expected,
                    "incoming and inert outgoing pages paint together"
                );
            }
        })
        .unwrap();
    #[cfg(target_os = "macos")]
    {
        let _ = accessible_with_role(cx, handle, "Second action", Some("AXButton"), false);
        frame(cx, handle).await;
        assert!(
            accessible_with_role(cx, handle, "First action", Some("AXButton"), false).is_none()
        );
        assert!(
            accessible_with_role(cx, handle, "Second action", Some("AXButton"), false).is_some()
        );
    }
    // Return before the slide completes; remembered focus should beat the first editor.
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(node(54), config(0))],
    );
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(57)),
        "back restores last focused control"
    );
    let _ = presses(transport);
    key(cx, handle, "enter");
    assert_eq!(presses(transport), [node(57)]);
    // An ineligible remembered destination falls back to the first eligible control.
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(node(54), config(1))],
    );
    frame(cx, handle).await;
    apply(
        cx,
        handle,
        vec![
            Op::SetControl(node(57), Control::Button(true)),
            Op::SetNavigationStack(node(54), config(0)),
        ],
    );
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(56)));
    // Remove the current route, selecting a replacement at its position atomically.
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(54), 0, 1, vec![]),
            Op::Remove(node(55)),
            Op::Remove(node(56)),
            Op::Remove(node(57)),
            Op::SetNavigationStack(node(54), config(0)),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |v, _, _| assert!(!v.editors.contains_key(&node(56))))
        .unwrap();
    assert!(focused(cx, handle, node(59)));
    // Unmount policy removes every inactive descendant in the admitted transaction.
    let mut unmount = config(1);
    unmount.retain = false;
    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(58), 0, 2, vec![]),
            Op::Remove(node(59)),
            Op::Remove(node(60)),
            Op::SetNavigationStack(node(54), unmount),
        ],
    );
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(62)));
    handle
        .update(cx, |v, _, _| assert!(v.editors.is_empty()))
        .unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(500))
        .await;
    let idle = handle.update(cx, |v, _, _| v.render_count).unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(180))
        .await;
    assert_eq!(
        handle.update(cx, |v, _, _| v.render_count).unwrap(),
        idle,
        "settled navigation is idle"
    );
    // Explicit hidden and reduced-motion policies settle without background polling.
    let mut hidden_config = config(0);
    hidden_config.duration_ms = 2000;
    apply(
        cx,
        handle,
        vec![
            Op::SetNavigationStack(node(54), hidden_config),
            Op::SetStyle(node(54), vec![Style::Fields(vec![Field::Display(3)])]),
        ],
    );
    frame(cx, handle).await;
    let hidden_idle = handle.update(cx, |v, _, _| v.render_count).unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(180))
        .await;
    assert_eq!(
        handle.update(cx, |v, _, _| v.render_count).unwrap(),
        hidden_idle
    );
    cx.update(|cx| cx.set_reduce_motion(true));
    apply(
        cx,
        handle,
        vec![
            Op::SetNavigationStack(node(54), config(1)),
            Op::SetStyle(
                node(54),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(320.)),
                    Field::Height(Length::Px(240.)),
                ])],
            ),
        ],
    );
    frame(cx, handle).await;
    let reduced_idle = handle.update(cx, |v, _, _| v.render_count).unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(180))
        .await;
    assert_eq!(
        handle.update(cx, |v, _, _| v.render_count).unwrap(),
        reduced_idle
    );
    cx.update(|cx| cx.set_reduce_motion(false));
    // Dispose in the middle of a newly requested run.
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(node(54), config(0))],
    );
    frame(cx, handle).await;
    apply(
        cx,
        handle,
        vec![
            Op::SetRoot(None),
            Op::Remove(node(54)),
            Op::Remove(node(58)),
            Op::Remove(node(61)),
            Op::Remove(node(62)),
        ],
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
        "GPUIO_NAVIGATION_STACK_NATIVE_OK: mounted transitions, back focus, disabled fallback, immediate replacement/unmount, AX gating, idle and disposal"
    );
}
