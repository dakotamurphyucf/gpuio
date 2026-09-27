//! Composition and deferred popup ownership inside a carousel page.
use super::super::super::native_test::{mouse, move_mouse};
use super::*;
fn handler(slot: i64) -> gpuio_protocol::HandlerId {
    gpuio_protocol::HandlerId::from_parts(slot, 1).unwrap()
}
fn observed(transport: &Transport) -> Vec<Event> {
    transport.mailbox.lock().unwrap().drain(128)
}
fn no_carousel(events: &[Event]) -> bool {
    !events
        .iter()
        .any(|event| matches!(event, Event::CarouselRequested(..)))
}
fn focus_editor(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, id: i64) {
    handle
        .update(cx, |v, w, cx| {
            w.focus(&v.editors[&node(id)].focus_handle(cx), cx)
        })
        .unwrap();
}
fn snapshot(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, id: i64) -> EditorSnapshot {
    handle
        .update(cx, |v, w, cx| v.editors[&node(id)].snapshot(w, cx))
        .unwrap()
}
fn automatic(revision: i64, selected: i64) -> Config {
    let mut value = config(revision, selected, Axis::Horizontal, Direction::Direct);
    value.auto_advance_ms = Some(1000);
    value
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    editor: EditorConfig,
    transport: &Transport,
) {
    let mut ops = vec![];
    for (slot, kind, label) in [
        (477, Kind::Container, "Nested gallery fixture"),
        (478, Kind::Button, "Outside nested gallery"),
        (479, Kind::Carousel, "Composition gallery"),
        (480, Kind::NavigationStack, "Composition pages"),
        (481, Kind::Panel, "Editing page"),
        (482, Kind::Panel, "Destination page"),
        (483, Kind::Input, "Page draft"),
        (484, Kind::FocusScope, "Page popover"),
        (485, Kind::Input, "Popover draft"),
        (486, Kind::Button, "Page popover anchor"),
        (487, Kind::Button, "Destination action"),
    ] {
        let owns_handler = matches!(
            kind,
            Kind::Carousel | Kind::Input | Kind::Button | Kind::FocusScope
        );
        ops.push(Op::Create(
            node(slot),
            kind,
            label.into(),
            owns_handler.then(|| handler(slot)),
        ));
        if kind == Kind::Button {
            ops.push(Op::SetControl(node(slot), Control::Button(false)));
        }
        if kind == Kind::Input {
            ops.push(Op::SetEditor(
                node(slot),
                EditorConfig {
                    label: label.into(),
                    ..editor.clone()
                },
            ));
            ops.push(Op::SetStyle(
                node(slot),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(200.)),
                    Field::Height(Length::Px(36.)),
                ])],
            ));
        }
    }
    ops.extend([
        Op::SetCarousel(node(479), config(0, 0, Axis::Horizontal, Direction::Direct)),
        Op::SetNavigationStack(node(480), presentation(0, 200)),
        Op::SetStyle(
            node(479),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(320.)),
                Field::Height(Length::Px(200.)),
            ])],
        ),
        Op::SetStyle(
            node(480),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(320.)),
                Field::Height(Length::Px(200.)),
            ])],
        ),
        Op::SetFocusScope(
            node(484),
            FocusScopeConfig {
                trap: false,
                auto_focus: false,
                restore_focus: true,
            },
        ),
        Op::SetStyle(node(484), vec![Style::Fields(vec![Field::Display(3)])]),
        Op::Splice(node(484), 0, 0, vec![node(485)]),
        Op::Splice(node(481), 0, 0, vec![node(483), node(486), node(484)]),
        Op::Splice(node(482), 0, 0, vec![node(487)]),
        Op::Splice(node(480), 0, 0, vec![node(481), node(482)]),
        Op::Splice(node(479), 0, 0, vec![node(480)]),
        Op::Splice(node(477), 0, 0, vec![node(478), node(479)]),
        Op::SetRoot(Some(node(477))),
    ]);
    apply(cx, handle, ops);
    frame(cx, handle).await;
    focus_editor(cx, handle, 483);
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    {
        super::super::super::editor_test::native_text(cx, handle, "仮入力", true);
        frame(cx, handle).await;
        assert!(snapshot(cx, handle, 483).composition.is_some());
    }
    let before = snapshot(cx, handle, 483);
    let identity = handle
        .update(cx, |v, _, cx| v.editors[&node(483)].focus_handle(cx))
        .unwrap();
    observed(transport);
    apply(
        cx,
        handle,
        vec![
            Op::SetCarousel(node(479), config(1, 1, Axis::Horizontal, Direction::Next)),
            Op::SetNavigationStack(node(480), presentation(1, 200)),
        ],
    );
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(487)),
        "hiding marked input hands focus to destination"
    );
    handle
        .update(cx, |v, w, cx| {
            assert_eq!(identity, v.editors[&node(483)].focus_handle(cx));
            assert!(matches!(
                v.editors
                    .get_mut(&node(483))
                    .unwrap()
                    .command(&EditorCommand::Focus, w, cx),
                EditorResult::Failed(EditorError::FocusBlocked)
            ));
        })
        .unwrap();
    #[cfg(target_os = "macos")]
    super::super::super::editor_test::native_text(cx, handle, "must not reach hidden draft", false);
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle, 483).text, before.text);
    assert!(no_carousel(&observed(transport)));
    apply(
        cx,
        handle,
        vec![
            Op::SetCarousel(
                node(479),
                config(2, 0, Axis::Horizontal, Direction::Previous),
            ),
            Op::SetNavigationStack(node(480), presentation(0, 0)),
        ],
    );
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle, 483).text, before.text);

    handle
        .update(cx, |v, w, cx| w.focus(&v.buttons[&node(486)].focus, cx))
        .unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::SetCarousel(node(479), automatic(3, 0)),
            Op::SetStyle(
                node(484),
                vec![Style::Fields(vec![Field::Height(Length::Px(120.))])],
            ),
            Op::SetOverlay(
                node(484),
                Some(OverlayConfig {
                    kind: OverlayKind::Popover,
                    label: "Page popover".into(),
                    width: 360.,
                    dismiss_on_escape: true,
                    dismiss_on_outside_pointer: true,
                }),
            ),
        ],
    );
    frame(cx, handle).await;
    focus_editor(cx, handle, 485);
    let outside = handle
        .update(cx, |v, _, _| v.probes.borrow()[&node(478)].bounds.center())
        .unwrap();
    move_mouse(cx, handle, outside, false);
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(485)));
    assert!(
        !handle
            .update(cx, |v, _, _| v.carousels[&node(479)].borrow().has_timer())
            .unwrap(),
        "deferred child popup focus pauses carousel"
    );
    observed(transport);
    cx.background_executor()
        .timer(std::time::Duration::from_millis(1100))
        .await;
    assert!(no_carousel(&observed(transport)));
    #[cfg(target_os = "macos")]
    {
        super::super::super::editor_test::native_text(cx, handle, "編集中", true);
        frame(cx, handle).await;
        assert!(snapshot(cx, handle, 485).composition.is_some());
        observed(transport);
        key(cx, handle, "escape");
        frame(cx, handle).await;
        let events = observed(transport);
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, Event::OverlayDismissed(..))),
            "IME gets first Escape"
        );
        assert!(no_carousel(&events));
        assert!(snapshot(cx, handle, 485).composition.is_none());
    }
    key(cx, handle, "right");
    key(cx, handle, "home");
    assert!(
        no_carousel(&observed(transport)),
        "popup editor retains navigation keys"
    );
    key(cx, handle, "escape");
    frame(cx, handle).await;
    let events = observed(transport);
    assert!(events.iter().any(|event| matches!(event, Event::OverlayDismissed(_, id, _, _, Dismissal::Escape) if *id == node(484))));
    assert!(no_carousel(&events));
    // The controlled popover stays open until accepted closure. Pointer-only
    // interaction outside the carousel's allocated box is still contained use.
    handle
        .update(cx, |v, w, cx| w.focus(&v.buttons[&node(478)].focus, cx))
        .unwrap();
    frame(cx, handle).await;
    let point = handle
        .update(cx, |v, _, _| {
            let probes = v.probes.borrow();
            let popup = probes[&node(484)].bounds;
            let point = gpui::point(popup.right() - px(4.), popup.center().y);
            assert!(
                !probes[&node(479)].bounds.contains(&point),
                "fixture must hover the external popup area"
            );
            assert!(v.focus.borrow().surface_contains(node(479), point));
            point
        })
        .unwrap();
    move_mouse(cx, handle, point, false);
    frame(cx, handle).await;
    assert!(
        !handle
            .update(cx, |v, _, _| v.carousels[&node(479)].borrow().has_timer())
            .unwrap(),
        "deferred child popup hover pauses carousel"
    );
    observed(transport);
    cx.background_executor()
        .timer(std::time::Duration::from_millis(1100))
        .await;
    assert!(no_carousel(&observed(transport)));
    mouse(cx, handle, point, true);
    mouse(cx, handle, point, false);
    assert!(
        !observed(transport)
            .iter()
            .any(|event| matches!(event, Event::OverlayDismissed(..))),
        "clicking the deferred panel is inside the overlay"
    );
    move_mouse(cx, handle, outside, false);
    mouse(cx, handle, outside, true);
    mouse(cx, handle, outside, false);
    frame(cx, handle).await;
    assert!(observed(transport).iter().any(|event| matches!(
        event,
        Event::OverlayDismissed(_, id, _, _, Dismissal::OutsidePointer) if *id == node(484)
    )));
    assert!(
        handle
            .update(cx, |v, _, _| v.carousels[&node(479)].borrow().has_timer())
            .unwrap(),
        "leaving the popup resumes the automatic clock"
    );
    cx.background_executor()
        .timer(std::time::Duration::from_millis(1100))
        .await;
    let proposals = observed(transport)
        .into_iter()
        .filter(|event| matches!(event, Event::CarouselRequested(..)))
        .collect::<Vec<_>>();
    assert_eq!(proposals.len(), 1, "resumed clock proposes exactly once");
    focus_editor(cx, handle, 485);
    frame(cx, handle).await;
    let popup_before = snapshot(cx, handle, 485);
    apply(
        cx,
        handle,
        vec![
            Op::SetCarousel(node(479), automatic(4, 1)),
            Op::SetNavigationStack(node(480), presentation(1, 200)),
        ],
    );
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(487)));
    handle
        .update(cx, |v, w, cx| {
            assert!(
                v.focus.borrow().handle(node(484)).is_none(),
                "hidden page retires popup scope"
            );
            assert!(!v.focus.borrow().visible(node(485)));
            assert!(matches!(
                v.editors
                    .get_mut(&node(485))
                    .unwrap()
                    .command(&EditorCommand::Focus, w, cx),
                EditorResult::Failed(EditorError::FocusBlocked)
            ));
        })
        .unwrap();
    #[cfg(target_os = "macos")]
    super::super::super::editor_test::native_text(cx, handle, "blocked popup text", false);
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle, 485).text, popup_before.text);
    observed(transport);
    apply(
        cx,
        handle,
        std::iter::once(Op::SetRoot(None))
            .chain((477..=487).map(|slot| Op::Remove(node(slot))))
            .collect(),
    );
    frame(cx, handle).await;
    cx.background_executor()
        .timer(std::time::Duration::from_millis(1100))
        .await;
    assert!(no_carousel(&observed(transport)));
    handle
        .update(cx, |v, _, _| {
            assert!(v.carousels.is_empty() && v.navigation.is_empty() && v.editors.is_empty());
            assert_eq!(v.session.borrow().retained_bytes(), 0);
        })
        .unwrap();
    println!(
        "GPUIO_CAROUSEL_NESTED_OK: marked-text isolation, retained buffer/identity, destination focus, popup focus/hover auto pause, IME-first Escape, editor keys, controlled dismissal, hidden popup retirement and teardown"
    );
}
