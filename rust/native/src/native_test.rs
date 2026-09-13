//! Real-window validation; compiled only with the explicit native-tests feature.
use super::*;
use gpui::{MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PlatformInput, point};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
#[derive(Clone, Copy)]
pub(super) struct Probe {
    pub bounds: gpui::Bounds<gpui::Pixels>,
    pub color: gpui::Hsla,
}
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn styles(pointer: bool) -> Vec<Style> {
    vec![
        Style::Fields(vec![
            Field::Width(Length::Px(140.)),
            Field::Height(Length::Px(60.)),
            Field::Foreground(Color::Rgba(0x111111ff)),
            Field::PointerEvents(pointer),
        ]),
        Style::State(1, vec![Field::Foreground(Color::Rgba(0x444444ff))]),
        Style::HoverBackground(Color::Rgba(0xeeeeeeff)),
        Style::State(2, vec![Field::Foreground(Color::Rgba(0x999999ff))]),
        Style::State(2, vec![Field::Foreground(Color::Rgba(0x222222ff))]),
        Style::State(3, vec![Field::Foreground(Color::Rgba(0x333333ff))]),
    ]
}
async fn probe(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    node: NodeId,
    expected: Option<u32>,
) -> Probe {
    for _ in 0..300 {
        let found = window
            .update(cx, |view, _, _| view.probes.borrow().get(&node).copied())
            .unwrap();
        if let Some(found) = found
            && expected.is_none_or(|color| found.color == rgba(color).into())
        {
            return found;
        }
        cx.background_executor()
            .timer(std::time::Duration::from_millis(16))
            .await;
    }
    eprintln!(
        "window status: {:?}",
        window
            .update(cx, |view, w, _| (
                w.is_window_active(),
                w.is_window_hovered(),
                w.mouse_position(),
                view.probes.borrow().get(&node).map(|p| (p.bounds, p.color))
            ))
            .unwrap()
    );
    panic!("native node {node:?} did not paint expected color {expected:?}");
}
pub(super) fn move_mouse(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    position: gpui::Point<gpui::Pixels>,
    pressed: bool,
) {
    cx.update_window(window.into(), |_, window, cx| {
        window.dispatch_event(
            PlatformInput::MouseMove(MouseMoveEvent {
                position,
                pressed_button: pressed.then_some(MouseButton::Left),
                modifiers: Default::default(),
            }),
            cx,
        )
    })
    .unwrap();
}
pub(super) fn mouse(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    position: gpui::Point<gpui::Pixels>,
    down: bool,
) {
    cx.update_window(window.into(), |_, window, cx| {
        let event = if down {
            PlatformInput::MouseDown(MouseDownEvent {
                position,
                button: MouseButton::Left,
                modifiers: Default::default(),
                click_count: 1,
                first_mouse: false,
            })
        } else {
            PlatformInput::MouseUp(MouseUpEvent {
                position,
                button: MouseButton::Left,
                modifiers: Default::default(),
                click_count: 1,
            })
        };
        window.dispatch_event(event, cx);
    })
    .unwrap();
}
fn key(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, key: &str, down: bool) {
    let keystroke = gpui::Keystroke::parse(key).unwrap();
    cx.update_window(window.into(), |_, window, cx| {
        let event = if down {
            PlatformInput::KeyDown(gpui::KeyDownEvent {
                keystroke,
                is_held: false,
                prefer_character_input: false,
            })
        } else {
            PlatformInput::KeyUp(gpui::KeyUpEvent { keystroke })
        };
        window.dispatch_event(event, cx);
    })
    .unwrap();
}
fn presses(transport: &Transport) -> usize {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .iter()
        .filter(|e| matches!(e, Event::Press(..)))
        .count()
}

pub(super) async fn protect<F: std::future::Future>(
    future: F,
) -> Result<F::Output, Box<dyn std::any::Any + Send>> {
    let mut future = std::pin::pin!(future);
    std::future::poll_fn(|cx| {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| future.as_mut().poll(cx))) {
            Ok(poll) => poll.map(Ok),
            Err(error) => std::task::Poll::Ready(Err(error)),
        }
    })
    .await
}
async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    session: SharedSession,
    transport: Arc<Transport>,
    window_id: WindowId,
    handler: gpuio_protocol::HandlerId,
    text: &str,
) {
    cx.background_executor()
        .timer(std::time::Duration::from_millis(200))
        .await;
    probe(cx, window, id(1), None).await;
    move_mouse(cx, window, point(px(-20.), px(-20.)), false);
    let button = probe(cx, window, id(1), Some(0x111111ff)).await;
    assert_eq!(button.bounds.size.width, px(140.));
    assert_eq!(button.bounds.size.height, px(60.));
    let label = probe(cx, window, id(2), None).await;
    assert!(label.bounds.left() > button.bounds.right());
    let position = button.bounds.origin + point(px(10.), px(10.));
    move_mouse(cx, window, position, false);
    probe(cx, window, id(1), Some(0x222222ff)).await;
    mouse(cx, window, position, true);
    probe(cx, window, id(1), Some(0x333333ff)).await;
    mouse(cx, window, position, false);
    assert_eq!(presses(&transport), 1);
    move_mouse(cx, window, point(px(-20.), px(-20.)), false);
    probe(cx, window, id(1), Some(0x444444ff)).await;
    key(cx, window, "enter", true);
    assert_eq!(presses(&transport), 0);
    key(cx, window, "enter", false);
    assert_eq!(presses(&transport), 1);
    key(cx, window, "space", true);
    assert_eq!(presses(&transport), 0);
    key(cx, window, "space", false);
    assert_eq!(presses(&transport), 1);
    session
        .borrow_mut()
        .apply(&Transaction {
            window: window_id,
            base: 1,
            revision: 2,
            operations: vec![Op::SetStyle(id(1), styles(false))],
        })
        .unwrap();
    window.update(cx, |_, _, cx| cx.notify()).unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(60))
        .await;
    move_mouse(cx, window, position, false);
    mouse(cx, window, position, true);
    mouse(cx, window, position, false);
    assert_eq!(presses(&transport), 0);
    key(cx, window, "enter", true);
    key(cx, window, "enter", false);
    assert_eq!(presses(&transport), 1);
    key(cx, window, "tab", true);
    assert!(
        window
            .update(cx, |view, w, _| view.selections[&id(2)]
                .borrow()
                .focus
                .is_focused(w))
            .unwrap()
    );
    key(cx, window, "shift-tab", true);
    assert!(
        window
            .update(cx, |view, w, _| view.buttons[&id(1)].focus.is_focused(w))
            .unwrap()
    );
    let saved_clipboard = cx.update(|cx| cx.read_from_clipboard());
    window
        .update(cx, |view, window, cx| {
            window.focus(&view.selections[&id(2)].borrow().focus, cx)
        })
        .unwrap();
    key(cx, window, "secondary-a", true);
    key(cx, window, "secondary-c", true);
    let copied = cx.update(|cx| cx.read_from_clipboard().and_then(|item| item.text()));
    cx.update(|cx| {
        cx.write_to_clipboard(
            saved_clipboard.unwrap_or_else(|| gpui::ClipboardItem::new_string(String::new())),
        )
    });
    assert_eq!(copied.as_deref(), Some(text));
    let old_focus = window
        .update(cx, |view, _, _| view.buttons[&id(1)].focus.clone())
        .unwrap();
    let replacement = NodeId::from_parts(1, 2).unwrap();
    session
        .borrow_mut()
        .apply(&Transaction {
            window: window_id,
            base: 2,
            revision: 3,
            operations: vec![
                Op::Remove(id(1)),
                Op::Create(replacement, Kind::Button, "New".into(), Some(handler)),
                Op::Splice(id(0), 0, 1, vec![replacement]),
            ],
        })
        .unwrap();
    window.update(cx, |_, _, cx| cx.notify()).unwrap();
    let next = probe(cx, window, replacement, Some(0x555555ff)).await;
    assert_ne!(next.bounds.size.width, px(140.));
    window
        .update(cx, |view, _, _| {
            assert!(!view.buttons.contains_key(&id(1)));
            assert_ne!(old_focus, view.buttons[&replacement].focus);
        })
        .unwrap();
    assert!(
        session
            .borrow()
            .press(window_id, id(1), handler, 1)
            .is_none()
    );
    eprintln!(
        "NATIVE_VIEW_PASS grid=true hover=true pressed=true focus=true keyboard=true tab=true pointer_policy=true selection_copy=true replacement=true reset=true"
    );
}
pub fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let window_id = WindowId::from_parts(0, 1).unwrap();
    let handler = gpuio_protocol::HandlerId::from_parts(0, 1).unwrap();
    let text = "Hello 👨‍👩‍👧‍👦\nNative selection";
    gpui_platform::application().run(move |cx| {
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, window_id, "Native style test", 500., 300.)
            .unwrap();
        session
            .borrow_mut()
            .apply(&Transaction {
                window: window_id,
                base: 0,
                revision: 1,
                operations: vec![
                    Op::Create(id(0), Kind::Container, "".into(), None),
                    Op::Create(id(1), Kind::Button, "Run".into(), Some(handler)),
                    Op::Create(id(2), Kind::Text, text.into(), None),
                    Op::SetStyle(
                        id(0),
                        vec![Style::Fields(vec![
                            Field::Display(2),
                            Field::GridColumns(2),
                            Field::Width(Length::Px(460.)),
                            Field::ColumnGap(Length::Px(10.)),
                            Field::Foreground(Color::Rgba(0x555555ff)),
                        ])],
                    ),
                    Op::SetStyle(id(1), styles(true)),
                    Op::SetStyle(
                        id(2),
                        vec![Style::Fields(vec![
                            Field::UserSelect(true),
                            Field::SelectionColor(Color::Rgba(0xff880080)),
                        ])],
                    ),
                    Op::Splice(id(0), 0, 0, vec![id(1), id(2)]),
                    Op::SetRoot(Some(id(0))),
                ],
            })
            .unwrap();
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(500.), px(300.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(window_id, session.clone(), transport.clone())),
            )
            .unwrap();
        cx.activate(true);
        window
            .update(cx, |_, window, _| window.activate_window())
            .unwrap();
        cx.spawn(async move |cx| {
            let result = protect(exercise(
                cx, window, session, transport, window_id, handler, text,
            ))
            .await;
            *task_failure.borrow_mut() = result.err();
            cx.update(stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
