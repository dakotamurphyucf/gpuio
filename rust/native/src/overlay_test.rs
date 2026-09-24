//! Actual window tests for controlled overlays, deferred child popups and IME.
use super::*;
fn handler(slot: i64) -> gpuio_protocol::HandlerId {
    gpuio_protocol::HandlerId::from_parts(slot, 1).unwrap()
}
fn config(kind: OverlayKind, label: &str) -> OverlayConfig {
    OverlayConfig {
        kind,
        label: label.into(),
        width: 220.,
        dismiss_on_escape: true,
        dismiss_on_outside_pointer: true,
    }
}
fn button(slot: i64, label: &str) -> Vec<Op> {
    vec![
        Op::Create(node(slot), Kind::Button, label.into(), Some(handler(slot))),
        Op::SetControl(node(slot), Control::Button(false)),
        Op::SetStyle(
            node(slot),
            vec![Style::Fields(vec![
                Field::Height(Length::Px(28.)),
                Field::Width(Length::Px(160.)),
            ])],
        ),
    ]
}
fn panel(slot: i64, config: OverlayConfig) -> Vec<Op> {
    vec![
        Op::Create(node(slot), Kind::FocusScope, "".into(), Some(handler(slot))),
        Op::SetFocusScope(
            node(slot),
            FocusScopeConfig {
                trap: config.kind == OverlayKind::Dialog,
                auto_focus: true,
                restore_focus: true,
            },
        ),
        Op::SetOverlay(node(slot), Some(config)),
    ]
}
fn events(transport: &Transport) -> Vec<Event> {
    transport.mailbox.lock().unwrap().drain(128)
}
fn dismissals(transport: &Transport) -> Vec<(NodeId, Dismissal)> {
    events(transport)
        .into_iter()
        .filter_map(|event| match event {
            Event::OverlayDismissed(_, node, _, _, reason) => Some((node, reason)),
            _ => None,
        })
        .collect()
}
fn click(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, point: gpui::Point<gpui::Pixels>) {
    super::super::native_test::move_mouse(cx, handle, point, false);
    super::super::native_test::mouse(cx, handle, point, true);
    super::super::native_test::mouse(cx, handle, point, false);
}
fn bounds(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, slot: i64) -> Bounds<gpui::Pixels> {
    handle
        .update(cx, |view, _, _| view.probes.borrow()[&node(slot)].bounds)
        .unwrap()
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&node(1)].focus, cx)
        })
        .unwrap();
    frame(cx, handle).await;
    let dialog = config(OverlayKind::Dialog, "Settings dialog");
    let mut ops = panel(17, dialog.clone());
    ops.extend(button(18, "Dialog action"));
    ops.extend([
        Op::Create(node(19), Kind::Select, "".into(), Some(handler(19))),
        Op::SetChoice(
            node(19),
            ChoiceConfig {
                label: "Dialog choice".into(),
                selected: None,
                disabled: false,
                items: (0..3)
                    .map(|i| ChoiceItem {
                        id: i.to_string(),
                        label: format!("Option {i}"),
                        disabled: false,
                    })
                    .collect(),
            },
        ),
        Op::Create(node(20), Kind::Input, "".into(), Some(handler(20))),
        Op::SetEditor(
            node(20),
            EditorConfig {
                label: "Dialog editor".into(),
                placeholder: "".into(),
                disabled: false,
                read_only: false,
                auto_focus: false,
                submit_on_enter: false,
                min_rows: 1,
                max_rows: 1,
            },
        ),
        Op::Splice(node(17), 0, 0, vec![node(18), node(19), node(20)]),
        Op::Splice(node(0), 4, 0, vec![node(17)]),
    ]);
    apply(cx, handle, ops);
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(18)),
        "dialog enters its first child"
    );
    let panel_bounds = bounds(cx, handle, 17);
    assert!((panel_bounds.center().x - px(200.)).abs() < px(2.));
    assert!((panel_bounds.center().y - px(140.)).abs() < px(2.));
    #[cfg(target_os = "macos")]
    assert_eq!(
        accessible(cx, handle, "Settings dialog", false)
            .unwrap()
            .role,
        "AXWindow"
    );
    events(transport);
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(19)));
    key(cx, handle, "down");
    frame(cx, handle).await;
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert!(
        dismissals(transport).is_empty(),
        "first Escape closes child popup only"
    );
    key(cx, handle, "down");
    frame(cx, handle).await;
    let popup_point = handle
        .update(cx, |view, _, _| {
            let state = view.selects[&node(19)].borrow();
            let popup = state.popup.borrow();
            popup
                .option_probes
                .borrow()
                .values()
                .map(|probe| probe.bounds.center())
                .find(|point| !panel_bounds.contains(point))
                .expect("popup has an option outside dialog bounds")
        })
        .unwrap();
    events(transport);
    click(cx, handle, popup_point);
    frame(cx, handle).await;
    let output = events(transport);
    assert!(
        output
            .iter()
            .any(|event| matches!(event, Event::Choice(_, id, ..) if *id == node(19))),
        "choice remains interactive outside panel bounds: {output:?}"
    );
    assert!(
        !output
            .iter()
            .any(|event| matches!(event, Event::OverlayDismissed(..))),
        "choice click must not dismiss dialog"
    );
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert_eq!(dismissals(transport), [(node(17), Dismissal::Escape)]);
    assert!(
        focused(cx, handle, node(19)),
        "dismissal is intent until app closes"
    );
    click(cx, handle, gpui::point(px(3.), px(3.)));
    frame(cx, handle).await;
    assert_eq!(
        dismissals(transport),
        [(node(17), Dismissal::OutsidePointer)]
    );
    assert!(
        focused(cx, handle, node(19)),
        "backdrop cannot focus underlying control"
    );
    key(cx, handle, "tab");
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(20)));
    #[cfg(target_os = "macos")]
    {
        super::super::editor_test::native_text(cx, handle, "に", true);
        frame(cx, handle).await;
        events(transport);
        key(cx, handle, "escape");
        frame(cx, handle).await;
        assert!(
            dismissals(transport).is_empty(),
            "composition cancellation must not dismiss dialog"
        );
        let composing = handle
            .update(cx, |view, window, cx| {
                view.editors[&node(20)]
                    .snapshot(window, cx)
                    .composition
                    .is_some()
            })
            .unwrap();
        assert!(!composing);
    }
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert_eq!(
        dismissals(transport),
        [(node(17), Dismissal::Escape)],
        "editor Escape bubbles after its own handling"
    );
    let mut nested = panel(21, config(OverlayKind::Dialog, "Nested dialog"));
    nested.extend(button(22, "Nested action"));
    nested.extend([
        Op::Splice(node(21), 0, 0, vec![node(22)]),
        Op::Splice(node(17), 3, 0, vec![node(21)]),
    ]);
    apply(cx, handle, nested);
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(22)));
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert_eq!(dismissals(transport), [(node(21), Dismissal::Escape)]);
    click(cx, handle, gpui::point(px(3.), px(3.)));
    frame(cx, handle).await;
    assert_eq!(
        dismissals(transport),
        [(node(21), Dismissal::OutsidePointer)]
    );
    apply(
        cx,
        handle,
        vec![
            Op::Remove(node(22)),
            Op::Remove(node(21)),
            Op::Splice(node(17), 3, 1, vec![]),
        ],
    );
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(20)),
        "nested close restores editor"
    );
    apply(
        cx,
        handle,
        vec![
            Op::Remove(node(18)),
            Op::Remove(node(19)),
            Op::Remove(node(20)),
            Op::Remove(node(17)),
            Op::Splice(node(0), 4, 1, vec![]),
        ],
    );
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(1)), "dialog close restores opener");
    // The permanent anchor owns layout; the open panel contributes no height.
    let mut anchor = vec![
        Op::Create(node(23), Kind::Container, "".into(), None),
        Op::SetStyle(
            node(23),
            vec![Style::Fields(vec![
                Field::Position(1),
                Field::Left(Length::Px(15.)),
                Field::Top(Length::Px(15.)),
                Field::Width(Length::Px(160.)),
                Field::Height(Length::Px(28.)),
            ])],
        ),
    ];
    anchor.extend(button(24, "Popover opener"));
    anchor.extend([
        Op::Splice(node(23), 0, 0, vec![node(24)]),
        Op::Splice(node(0), 4, 0, vec![node(23)]),
    ]);
    apply(cx, handle, anchor);
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&node(24)].focus, cx)
        })
        .unwrap();
    frame(cx, handle).await;
    let mut popover = panel(25, config(OverlayKind::Popover, "Details popover"));
    popover.extend(button(26, "Popover action"));
    popover.extend([
        Op::Splice(node(25), 0, 0, vec![node(26)]),
        Op::Splice(node(23), 1, 0, vec![node(25)]),
    ]);
    apply(cx, handle, popover);
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(26)));
    let anchor = bounds(cx, handle, 23);
    let panel = bounds(cx, handle, 25);
    assert_eq!(anchor.size.height, px(28.));
    // Canvas probes begin inside the panel's one-pixel border.
    assert!((panel.top() - anchor.bottom()).abs() <= px(1.));
    assert!((panel.left() - anchor.left()).abs() <= px(1.));
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(23),
            vec![Style::Fields(vec![
                Field::Position(1),
                Field::Left(Length::Px(100.)),
                Field::Top(Length::Px(45.)),
                Field::Width(Length::Px(160.)),
                Field::Height(Length::Px(28.)),
            ])],
        )],
    );
    frame(cx, handle).await;
    let anchor = bounds(cx, handle, 23);
    let panel = bounds(cx, handle, 25);
    // Canvas probes begin inside the panel's one-pixel border.
    assert!((panel.top() - anchor.bottom()).abs() <= px(1.));
    assert!((panel.left() - anchor.left()).abs() <= px(1.));
    apply(
        cx,
        handle,
        vec![Op::SetPlacement(
            node(25),
            Some(Placement {
                side: Side::Top,
                align: Align::End,
                offset: 6.,
            }),
        )],
    );
    frame(cx, handle).await;
    let flipped = bounds(cx, handle, 25);
    assert!(
        (flipped.top() - anchor.bottom() - px(6.)).abs() <= px(1.),
        "top preference flips below when space is insufficient"
    );
    assert!(
        (flipped.right() - anchor.right()).abs() <= px(1.),
        "end alignment follows current width"
    );
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(23),
            vec![Style::Fields(vec![
                Field::Position(1),
                Field::Left(Length::Px(100.)),
                Field::Top(Length::Px(150.)),
                Field::Width(Length::Px(160.)),
                Field::Height(Length::Px(28.)),
            ])],
        )],
    );
    frame(cx, handle).await;
    let anchor = bounds(cx, handle, 23);
    let above = bounds(cx, handle, 25);
    assert!(
        (anchor.top() - above.bottom() - px(6.)).abs() <= px(1.),
        "available top placement uses the requested gap"
    );
    assert!((above.right() - anchor.right()).abs() <= px(1.));
    assert!(
        focused(cx, handle, node(26)),
        "placement edits retain focus"
    );
    events(transport);
    key(cx, handle, "escape");
    frame(cx, handle).await;
    assert_eq!(dismissals(transport), [(node(25), Dismissal::Escape)]);
    apply(
        cx,
        handle,
        vec![
            Op::Remove(node(26)),
            Op::Remove(node(25)),
            Op::Splice(node(23), 1, 1, vec![]),
        ],
    );
    frame(cx, handle).await;
    assert!(focused(cx, handle, node(24)));
    apply(
        cx,
        handle,
        vec![
            Op::Remove(node(24)),
            Op::Remove(node(23)),
            Op::Splice(node(0), 4, 1, vec![]),
        ],
    );
    frame(cx, handle).await;
    handle
        .update(cx, |view, _, _| {
            for id in [17, 21, 25] {
                assert!(view.focus.borrow().handle(node(id)).is_none());
            }
        })
        .unwrap();
    println!(
        "GPUIO_OVERLAYS_OK: dialog/popover geometry, nested focus/restoration, controlled dismissal, child popup hit routing, Escape/IME and cleanup"
    );
}
