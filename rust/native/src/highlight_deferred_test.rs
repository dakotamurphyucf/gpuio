//! Deferred surfaces must participate in the final native visibility sample.
use super::*;

fn text_style() -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(180.)),
        Field::Height(Length::Px(30.)),
        Field::FontSize(20.),
        Field::LineHeight(Length::Px(30.)),
        Field::Shrink(0.),
    ])]
}
fn hover_style() -> Vec<Style> {
    vec![
        Style::Fields(vec![Field::Background(Fill::Solid(Color::Rgba(
            0xffffffff,
        )))]),
        Style::State(2, vec![Field::Visibility(1)]),
    ]
}
fn pointer(cx: &mut AsyncApp, handle: WindowHandle<View>, position: gpui::Point<gpui::Pixels>) {
    crate::host::native_test::move_mouse(cx, handle, position, false);
    draw(cx, handle);
}
fn outside(cx: &mut AsyncApp, handle: WindowHandle<View>) {
    pointer(cx, handle, gpui::point(px(390.), px(290.)));
}
fn body(text: i64, nested: i64) -> Vec<Op> {
    vec![
        Op::Create(id(text), Kind::Text, "aaa aaa body".into(), None),
        Op::SetStyle(id(text), text_style()),
        Op::Create(
            id(nested),
            Kind::HighlightScope,
            "".into(),
            Some(handler(3)),
        ),
        Op::SetHighlightScope(id(nested), Config(vec![])),
        Op::Create(id(nested + 1), Kind::Text, "aaa excluded".into(), None),
        Op::SetStyle(id(nested + 1), text_style()),
        Op::Splice(id(nested), 0, 0, vec![id(nested + 1)]),
    ]
}
async fn hide_and_restore(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
    text: i64,
    nested: i64,
    remaining: i64,
    position: Option<gpui::Point<gpui::Pixels>>,
) {
    let (text_position, weak) = handle
        .update(cx, |view, _, _| {
            (
                view.probes.borrow()[&id(text)].bounds.center(),
                Rc::downgrade(&view.highlights[&id(nested)]),
            )
        })
        .unwrap();
    let painted = red_pixels(cx, handle);
    assert!(painted > 20, "deferred content paints a wash");
    pointer(cx, handle, position.unwrap_or(text_position));
    count(cx, handle, transport, remaining).await;
    assert!(
        weak.upgrade().is_none(),
        "unpainted nested scope is released"
    );
    let hidden = red_pixels(cx, handle);
    assert!(hidden < painted, "hidden panel loses its GPU wash");
    if remaining == 0 {
        assert_eq!(hidden, 0);
    } else {
        assert!(hidden > 20, "visible anchor retains its wash");
    }
    let retained = result(cx, handle);
    for _ in 0..4 {
        draw(cx, handle);
        pause(cx).await;
    }
    assert!(Arc::ptr_eq(&retained, &result(cx, handle)));
    assert!(
        observations(transport).is_empty(),
        "stable hidden panel is silent"
    );
    outside(cx, handle);
    count(cx, handle, transport, remaining + 2).await;
    assert!(red_pixels(cx, handle) > hidden);
    assert!(
        handle
            .update(cx, |view, _, _| view.highlights.contains_key(&id(nested)))
            .unwrap()
    );
}
fn tooltip_config(open: bool) -> TooltipConfig {
    TooltipConfig {
        label: "Searchable floating content".into(),
        width: 230.,
        open_state: TooltipOpenState::Controlled(open),
        disabled: false,
        hoverable: true,
        show_delay_ns: 0,
        hide_delay_ns: 0,
        skip_delay_ns: 0,
    }
}
async fn tooltip(
    cx: &mut AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
    owner: i64,
    kind: Kind,
) {
    outside(cx, handle);
    let mut ops = vec![
        Op::Create(id(owner), kind, "".into(), Some(handler(2))),
        Op::SetTooltip(id(owner), tooltip_config(false)),
        Op::SetPlacement(
            id(owner),
            Some(Placement {
                side: Side::Bottom,
                align: Align::Start,
                offset: 6.,
            }),
        ),
        Op::SetStyle(id(owner), hover_style()),
        Op::Create(id(owner + 1), Kind::Text, "aaa anchor".into(), None),
        Op::SetStyle(id(owner + 1), text_style()),
        Op::Create(id(owner + 2), Kind::Container, "".into(), None),
    ];
    ops.extend(body(owner + 3, owner + 4));
    ops.extend([
        Op::Splice(id(owner + 2), 0, 0, vec![id(owner + 3), id(owner + 4)]),
        Op::Splice(id(owner), 0, 0, vec![id(owner + 1), id(owner + 2)]),
        Op::Splice(id(1), 0, 0, vec![id(owner)]),
    ]);
    apply(cx, handle, ops);
    count(cx, handle, transport, 1).await;
    apply(
        cx,
        handle,
        vec![Op::SetTooltip(id(owner), tooltip_config(true))],
    );
    count(cx, handle, transport, 3).await;
    hide_and_restore(cx, handle, transport, owner + 3, owner + 4, 1, None).await;
    let weak = handle
        .update(cx, |view, _, _| {
            Rc::downgrade(&view.highlights[&id(owner + 4)])
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetTooltip(id(owner), tooltip_config(false))],
    );
    count(cx, handle, transport, 1).await;
    assert!(
        weak.upgrade().is_none(),
        "closing releases the floating scope"
    );
    apply(
        cx,
        handle,
        vec![Op::SetTooltip(id(owner), tooltip_config(true))],
    );
    count(cx, handle, transport, 3).await;
    let mut ops = vec![Op::Splice(id(1), 0, 1, vec![])];
    ops.extend((owner..=owner + 5).rev().map(|n| Op::Remove(id(n))));
    apply(cx, handle, ops);
    count(cx, handle, transport, 0).await;
}
async fn overlay(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let mut style = hover_style();
    style.push(Style::Fields(vec![Field::Height(Length::Px(90.))]));
    let mut ops = vec![
        Op::Create(id(14), Kind::FocusScope, "".into(), Some(handler(2))),
        Op::SetFocusScope(
            id(14),
            FocusScopeConfig {
                trap: true,
                auto_focus: false,
                restore_focus: false,
            },
        ),
        Op::SetOverlay(
            id(14),
            Some(OverlayConfig {
                kind: OverlayKind::Dialog,
                label: "Searchable dialog".into(),
                width: 230.,
                dismiss_on_escape: false,
                dismiss_on_outside_pointer: false,
            }),
        ),
        Op::SetStyle(id(14), style),
    ];
    ops.extend(body(15, 16));
    ops.extend([
        Op::Splice(id(14), 0, 0, vec![id(15), id(16)]),
        Op::Splice(id(1), 0, 0, vec![id(14)]),
    ]);
    apply(cx, handle, ops);
    count(cx, handle, transport, 2).await;
    hide_and_restore(cx, handle, transport, 15, 16, 0, None).await;
    let mut ops = vec![Op::Splice(id(1), 0, 1, vec![])];
    ops.extend((14..=17).rev().map(|n| Op::Remove(id(n))));
    apply(cx, handle, ops);
    count(cx, handle, transport, 0).await;
}
async fn toast(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let mut ops = vec![
        Op::Create(id(18), Kind::ToastStack, "".into(), None),
        Op::SetToastStack(
            id(18),
            ToastStackConfig {
                label: "Notifications".into(),
                corner: ToastCorner::BottomRight,
                width: 250.,
                max_visible: 1,
            },
        ),
        Op::Create(id(19), Kind::Toast, "".into(), Some(handler(2))),
        Op::SetToast(
            id(19),
            ToastConfig {
                label: "Notice".into(),
                close_label: "Dismiss".into(),
                timeout_ns: None,
                politeness: ToastPoliteness::Polite,
            },
        ),
        Op::SetStyle(id(19), hover_style()),
    ];
    ops.extend(body(20, 21));
    ops.extend([
        Op::Splice(id(19), 0, 0, vec![id(20), id(21)]),
        Op::Splice(id(18), 0, 0, vec![id(19)]),
        Op::Splice(id(1), 0, 0, vec![id(18)]),
    ]);
    apply(cx, handle, ops);
    count(cx, handle, transport, 2).await;
    hide_and_restore(cx, handle, transport, 20, 21, 0, None).await;
    // The stack owns state styles independently from each notification panel.
    let mut stack_style = hover_style();
    stack_style.push(Style::Padding(12.));
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(
                id(19),
                vec![Style::Fields(vec![Field::Background(Fill::Solid(
                    Color::Rgba(0xffffffff),
                ))])],
            ),
            Op::SetStyle(id(18), stack_style),
        ],
    );
    draw(cx, handle);
    pause(cx).await;
    observations(transport);
    // A notification occludes the stack beneath it. Exercise exposed padding.
    let position = handle
        .update(cx, |view, _, _| {
            let bounds = view.toasts[&id(19)].bounds.get();
            gpui::point(bounds.origin.x - px(6.), bounds.center().y)
        })
        .unwrap();
    hide_and_restore(cx, handle, transport, 20, 21, 0, Some(position)).await;
    let mut ops = vec![Op::Splice(id(1), 0, 1, vec![])];
    ops.extend((18..=22).rev().map(|n| Op::Remove(id(n))));
    apply(cx, handle, ops);
    count(cx, handle, transport, 0).await;
}

async fn context_menu(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let mut ops = vec![
        Op::Create(id(23), Kind::Menu, "".into(), None),
        Op::SetMenu(
            id(23),
            MenuConfig {
                presentation: MenuPresentation::Context,
                menus: vec![MenuDefinition {
                    label: "aaa metadata".into(),
                    disabled: false,
                    items: vec![MenuItem::Separator],
                }],
            },
        ),
        Op::SetStyle(id(23), hover_style()),
        Op::Create(id(24), Kind::Container, "".into(), None),
    ];
    ops.extend(body(25, 26));
    ops.extend([
        Op::Splice(id(24), 0, 0, vec![id(25), id(26)]),
        Op::Splice(id(23), 0, 0, vec![id(24)]),
        Op::Splice(id(1), 0, 0, vec![id(23)]),
    ]);
    apply(cx, handle, ops);
    count(cx, handle, transport, 2).await;
    hide_and_restore(cx, handle, transport, 25, 26, 0, None).await;
    let mut ops = vec![Op::Splice(id(1), 0, 1, vec![])];
    ops.extend((23..=27).rev().map(|n| Op::Remove(id(n))));
    apply(cx, handle, ops);
    count(cx, handle, transport, 0).await;
}

pub(super) async fn exercise(
    cx: &mut AsyncApp,
    session: &Rc<RefCell<Session>>,
    transport: &Arc<Transport>,
) {
    let window_id = WindowId::from_parts(0, 3).unwrap();
    session
        .borrow_mut()
        .open(4, window_id, "Deferred highlighting", 400., 300.)
        .unwrap();
    let bounds = cx.update(|cx| Bounds::centered(None, size(px(400.), px(300.)), cx));
    let handle = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                focus: false,
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(window_id, session.clone(), transport.clone())),
        )
        .unwrap();
    transport.mailbox.lock().unwrap().drain(128);
    let checked = crate::host::native_test::protect(async {
        apply(cx, handle, vec![
            Op::Create(id(0), Kind::Container, "".into(), None),
            Op::SetStyle(id(0), frame_style(Length::Percent(100.), 300.)),
            Op::Create(id(1), Kind::HighlightScope, "".into(), Some(handler(1))),
            Op::SetStyle(id(1), frame_style(Length::Percent(100.), 300.)),
            Op::SetHighlightScope(id(1), config(0xff0000ff)),
            Op::Splice(id(0), 0, 0, vec![id(1)]), Op::SetRoot(Some(id(0))),
        ]);
        count(cx, handle, transport, 0).await;
        tooltip(cx, handle, transport, 2, Kind::Tooltip).await;
        tooltip(cx, handle, transport, 8, Kind::HoverCard).await;
        overlay(cx, handle, transport).await;
        toast(cx, handle, transport).await;
        context_menu(cx, handle, transport).await;
        assert_eq!(red_pixels(cx, handle), 0);
        eprintln!("GPUIO_NATIVE_HIGHLIGHT_DEFERRED_OK: tooltip/hover-card anchor retention, controlled popup lifetime, dialog/toast/stack/context-menu hover visibility, stable counts/GPU paint and nested scope disposal");
    }).await;
    let _ = handle.update(cx, |_, window, _| window.remove_window());
    let _ = session.borrow_mut().close(window_id);
    if let Err(error) = checked {
        std::panic::resume_unwind(error);
    }
}
