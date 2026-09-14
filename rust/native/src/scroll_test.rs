//! Native wheel routing through the production view, with GPUI scroll handles
//! exposing the actual native container offsets. The composer uses its widget offset.
use super::*;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn window_id() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn handler(slot: i64) -> gpuio_protocol::HandlerId {
    gpuio_protocol::HandlerId::from_parts(slot, 1).unwrap()
}
fn dimensions(width: f64, height: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(height)),
        Field::Shrink(0.),
    ])]
}
fn scrolling(width: f64, height: f64, horizontal: bool) -> Vec<Style> {
    let mut style = dimensions(width, height);
    style.push(Style::Fields(vec![if horizontal {
        Field::OverflowX(3)
    } else {
        Field::OverflowY(3)
    }]));
    style
}
fn apply(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, operations: Vec<Op>) {
    window
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let tx = Transaction {
                window: view.id,
                base,
                revision: base + 1,
                operations,
            };
            let applied = view
                .session
                .borrow_mut()
                .apply(&tx)
                .unwrap_or_else(|error| panic!("{error:?}: {tx:?}"));
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
        .unwrap();
}
async fn frame(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    super::editor_test::frame(cx, window).await;
    super::editor_test::frame(cx, window).await;
}
fn offset(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    slot: i64,
) -> gpui::Point<gpui::Pixels> {
    window
        .update(cx, |view, _, _| view.scrolls[&node(slot)].handle.offset())
        .unwrap()
}
fn bounds(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, slot: i64) -> Bounds<gpui::Pixels> {
    window
        .update(cx, |view, _, _| view.probes.borrow()[&node(slot)].bounds)
        .unwrap()
}
async fn wheel(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    position: gpui::Point<gpui::Pixels>,
    x: f32,
    y: f32,
) {
    super::native_test::move_mouse(cx, window, position, false);
    window
        .update(cx, |_, window, cx| {
            window.dispatch_event(
                gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                    position,
                    delta: gpui::ScrollDelta::Pixels(gpui::point(px(x), px(y))),
                    touch_phase: gpui::TouchPhase::Started,
                    modifiers: Default::default(),
                }),
                cx,
            );
        })
        .unwrap();
    frame(cx, window).await;
}
async fn reset(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    window
        .update(cx, |view, window, _| {
            for handle in view.scrolls.values() {
                handle.handle.set_offset(Default::default());
            }
            window.refresh();
        })
        .unwrap();
    frame(cx, window).await;
}
fn initial() -> Vec<Op> {
    let mut ops = vec![];
    for (slot, kind, text, styles) in [
        (
            0,
            Kind::Container,
            String::new(),
            scrolling(440., 320., false),
        ),
        (
            1,
            Kind::Container,
            String::new(),
            scrolling(420., 160., false),
        ),
        (
            2,
            Kind::Container,
            String::new(),
            scrolling(400., 40., true),
        ),
        (
            3,
            Kind::Text,
            "let streaming_response = \"long code\"; ".repeat(50),
            dimensions(1600., 30.),
        ),
        (
            4,
            Kind::Text,
            "Transcript row\n".repeat(40),
            dimensions(400., 800.),
        ),
        (
            5,
            Kind::Textarea,
            "Composer line\n".repeat(30),
            dimensions(420., 96.),
        ),
        (
            6,
            Kind::Text,
            "Outer scroll extent\n".repeat(40),
            dimensions(420., 800.),
        ),
    ] {
        ops.push(Op::Create(
            node(slot),
            kind,
            text,
            (slot == 5).then_some(handler(slot)),
        ));
        ops.push(Op::SetStyle(node(slot), styles));
    }
    ops.extend([
        Op::SetEditor(
            node(5),
            EditorConfig {
                label: "Composer".into(),
                placeholder: "".into(),
                disabled: false,
                read_only: false,
                auto_focus: false,
                submit_on_enter: false,
                min_rows: 3,
                max_rows: 3,
            },
        ),
        Op::Splice(node(2), 0, 0, vec![node(3)]),
        Op::Splice(node(1), 0, 0, vec![node(2), node(4)]),
        Op::Splice(node(0), 0, 0, vec![node(1), node(5), node(6)]),
        Op::SetRoot(Some(node(0))),
    ]);
    ops
}
async fn exercise(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    frame(cx, window).await;
    let revision = window
        .update(cx, |view, _, _| {
            view.session.borrow().tree(view.id).unwrap().revision()
        })
        .unwrap();
    assert_eq!(offset(cx, window, 0), Default::default());
    let transcript = bounds(cx, window, 1).center();
    wheel(cx, window, transcript, 0., -60.).await;
    assert!(offset(cx, window, 1).y < px(0.), "transcript scrolls");
    assert_eq!(
        offset(cx, window, 0).y,
        px(0.),
        "inner transcript must not simultaneously scroll ancestor"
    );
    assert_eq!(
        window
            .update(cx, |view, _, _| view
                .session
                .borrow()
                .tree(view.id)
                .unwrap()
                .revision())
            .unwrap(),
        revision,
        "native scrolling requires no tree transaction"
    );
    let retained = window
        .update(cx, |view, _, _| Rc::downgrade(&view.scrolls[&node(1)]))
        .unwrap();
    let position = offset(cx, window, 1);
    apply(
        cx,
        window,
        vec![
            Op::SetStyle(node(1), scrolling(420., 160., false)),
            Op::SetText(node(4), "Updated transcript\n".repeat(40)),
        ],
    );
    frame(cx, window).await;
    assert_eq!(
        offset(cx, window, 1),
        position,
        "same-node updates retain native viewport"
    );
    window
        .update(cx, |view, _, _| {
            assert!(Rc::ptr_eq(
                &retained.upgrade().unwrap(),
                &view.scrolls[&node(1)]
            ))
        })
        .unwrap();
    reset(cx, window).await;
    let code = bounds(cx, window, 2).center();
    wheel(cx, window, code, -90., 0.).await;
    assert!(offset(cx, window, 2).x < px(0.));
    assert_eq!(
        offset(cx, window, 1),
        Default::default(),
        "horizontal code wheel does not scroll transcript"
    );
    assert_eq!(offset(cx, window, 0), Default::default());
    let code_x = offset(cx, window, 2).x;
    wheel(cx, window, code, 0., -40.).await;
    assert_eq!(
        offset(cx, window, 2).x,
        code_x,
        "vertical wheel is not converted into horizontal code motion"
    );
    assert!(
        offset(cx, window, 1).y < px(0.),
        "vertical wheel over code reaches transcript"
    );
    assert_eq!(offset(cx, window, 0).y, px(0.));
    reset(cx, window).await;
    window
        .update(cx, |view, window, _| {
            let handle = &view.scrolls[&node(1)].handle;
            handle.set_offset(gpui::point(px(0.), -handle.max_offset().y));
            window.refresh();
        })
        .unwrap();
    frame(cx, window).await;
    let limit = offset(cx, window, 1);
    wheel(cx, window, transcript, 0., -50.).await;
    assert_eq!(
        offset(cx, window, 1),
        limit,
        "inner viewport clamps at its bottom"
    );
    assert!(
        offset(cx, window, 0).y < px(0.),
        "wheel can reach ancestor at inner boundary"
    );
    reset(cx, window).await;
    let composer = bounds(cx, window, 5).center();
    let before = window
        .update(cx, |view, _, cx| view.editors[&node(5)].scroll_offset(cx))
        .unwrap();
    wheel(
        cx,
        window,
        composer,
        0.,
        if before.y < px(-30.) { 30. } else { -30. },
    )
    .await;
    let after = window
        .update(cx, |view, _, cx| view.editors[&node(5)].scroll_offset(cx))
        .unwrap();
    assert_ne!(before, after, "real composer viewport scrolls");
    assert_eq!(
        offset(cx, window, 0).y,
        px(0.),
        "composer consumes its own scrolling"
    );
    apply(
        cx,
        window,
        vec![
            Op::Create(node(7), Kind::Select, "".into(), Some(handler(7))),
            Op::SetChoice(
                node(7),
                ChoiceConfig {
                    label: "Choices".into(),
                    selected: Some("0".into()),
                    disabled: false,
                    items: (0..100)
                        .map(|i| ChoiceItem {
                            id: i.to_string(),
                            label: format!("Choice {i}"),
                            disabled: false,
                        })
                        .collect(),
                },
            ),
            Op::SetStyle(node(7), dimensions(180., 28.)),
            Op::Splice(node(0), 2, 0, vec![node(7)]),
        ],
    );
    frame(cx, window).await;
    window
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&node(7)].focus, cx)
        })
        .unwrap();
    super::editor_test::key(cx, window, "space");
    frame(cx, window).await;
    let popup = window
        .update(cx, |view, _, _| {
            view.selects[&node(7)]
                .borrow()
                .popup
                .borrow()
                .popup_bounds
                .get()
        })
        .unwrap();
    wheel(cx, window, popup.center(), 0., -90.).await;
    let popup_offset = |cx: &mut gpui::AsyncApp| {
        window
            .update(cx, |view, _, _| {
                view.selects[&node(7)]
                    .borrow()
                    .popup
                    .borrow()
                    .scroll_offset()
            })
            .unwrap()
    };
    assert!(popup_offset(cx).y < px(0.), "select popup scrolls");
    assert_eq!(
        offset(cx, window, 0).y,
        px(0.),
        "popup does not scroll underlying app"
    );
    wheel(cx, window, popup.center(), 0., -10000.).await;
    let popup_limit = popup_offset(cx);
    wheel(cx, window, popup.center(), 0., -90.).await;
    assert_eq!(popup_offset(cx), popup_limit);
    assert_eq!(
        offset(cx, window, 0).y,
        px(0.),
        "popup shields underlying app even at its boundary"
    );
    super::editor_test::key(cx, window, "escape");
    frame(cx, window).await;
    apply(
        cx,
        window,
        vec![
            Op::Create(node(8), Kind::FocusScope, "".into(), Some(handler(8))),
            Op::SetFocusScope(
                node(8),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::SetOverlay(
                node(8),
                Some(OverlayConfig {
                    kind: OverlayKind::Dialog,
                    label: "Scroll dialog".into(),
                    width: 240.,
                    dismiss_on_escape: true,
                    dismiss_on_outside_pointer: false,
                }),
            ),
            Op::Create(node(9), Kind::Container, "".into(), None),
            Op::SetStyle(node(9), scrolling(200., 96., false)),
            Op::Create(node(10), Kind::Text, "Dialog lines\n".repeat(30), None),
            Op::SetStyle(node(10), dimensions(180., 600.)),
            Op::Splice(node(9), 0, 0, vec![node(10)]),
            Op::Splice(node(8), 0, 0, vec![node(9)]),
            Op::Splice(node(0), 4, 0, vec![node(8)]),
        ],
    );
    frame(cx, window).await;
    let panel = bounds(cx, window, 9).center();
    wheel(cx, window, panel, 0., -45.).await;
    assert!(offset(cx, window, 9).y < px(0.));
    assert_eq!(offset(cx, window, 0).y, px(0.));
    wheel(cx, window, gpui::point(px(20.), px(30.)), 0., -60.).await;
    assert_eq!(
        offset(cx, window, 0).y,
        px(0.),
        "modal backdrop shields scroll beneath it"
    );
    // Removing the last scroll declaration releases the owner before another
    // frame, even though GPUI can still hold the previous frame's elements.
    let code_owner = window
        .update(cx, |view, _, _| Rc::downgrade(&view.scrolls[&node(2)]))
        .unwrap();
    apply(
        cx,
        window,
        vec![Op::SetStyle(node(2), dimensions(400., 40.))],
    );
    assert!(
        code_owner.upgrade().is_none(),
        "obsolete scroll owner is not retained by paint callbacks"
    );
    apply(
        cx,
        window,
        vec![
            Op::SetRoot(None),
            Op::Remove(node(10)),
            Op::Remove(node(9)),
            Op::Remove(node(8)),
            Op::Remove(node(7)),
            Op::Remove(node(6)),
            Op::Remove(node(5)),
            Op::Remove(node(4)),
            Op::Remove(node(3)),
            Op::Remove(node(2)),
            Op::Remove(node(1)),
            Op::Remove(node(0)),
        ],
    );
    assert!(
        retained.upgrade().is_none(),
        "unmount releases scroll owner immediately"
    );
    frame(cx, window).await;
    window
        .update(cx, |view, _, _| {
            assert!(view.editors.is_empty());
            assert!(view.scrolls.is_empty());
        })
        .unwrap();
    eprintln!(
        "GPUIO_NATIVE_SCROLL_OK: nested transcript, horizontal code axes, boundary routing, composer, select popup, modal shielding and disposal"
    );
}
pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _read = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let write = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(write.as_raw_fd()).unwrap());
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        gpui_base::init(cx);
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, window_id(), "Scroll routing", 480., 420.)
            .unwrap();
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(480.), px(420.)),
                        cx,
                    ))),
                    focus: false,
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(window_id(), session.clone(), transport.clone())),
            )
            .unwrap();
        cx.spawn(async move |cx| {
            let result = super::native_test::protect(async {
                apply(cx, window, initial());
                exercise(cx, window).await;
                window
                    .update(cx, |_, window, _| window.remove_window())
                    .unwrap();
            })
            .await;
            *task_failure.borrow_mut() = result.err();
            cx.update(super::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
