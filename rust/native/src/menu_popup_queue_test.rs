//! Real AppKit scheduling, with invalidation before yielding the GPUI borrow.
//! A native-tests-only counter distinguishes skipped tracking from a brief popup.
use super::*;
use crate::{host, session::Session, transport::Transport};
use gpui::{WindowBounds, WindowHandle, WindowOptions, size};
use gpuio_protocol::{HandlerId, WindowId, menu_command};
use std::{
    os::fd::AsRawFd,
    os::unix::net::UnixStream,
    time::{Duration, Instant},
};

fn node(generation: i64) -> NodeId {
    NodeId::from_parts(0, generation).unwrap()
}
fn observer(generation: i64) -> HandlerId {
    HandlerId::from_parts(0, generation).unwrap()
}
fn config(label: &str) -> MenuConfig {
    MenuConfig {
        presentation: MenuPresentation::PlatformContext,
        menus: vec![MenuDefinition {
            label: label.into(),
            disabled: false,
            items: vec![MenuItem::Label("Queued popup lifecycle".into())],
        }],
    }
}
fn apply(view: &mut View, window: &mut Window, cx: &mut Context<View>, operations: Vec<Op>) {
    let base = view.session.borrow().tree(view.id).unwrap().revision();
    let applied = view
        .session
        .borrow_mut()
        .apply(&Transaction {
            window: view.id,
            base,
            revision: base + 1,
            operations,
        })
        .unwrap();
    view.update_editors(&applied.dirty, window, cx);
    cx.notify();
}
fn show(
    view: &mut View,
    window: &mut Window,
    cx: &mut Context<View>,
    owner: NodeId,
    generation: i64,
) {
    assert!(window.is_window_active());
    assert_eq!(
        view.menu_command(
            owner,
            observer(generation),
            &menu_command::Command::Show(menu_command::Position { x: 30., y: 60. }),
            window,
            cx
        ),
        menu_command::Response::Applied
    );
    assert!(popup::busy(), "admission retains a pending lease");
}
async fn retired(cx: &mut gpui::AsyncApp) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while popup::busy() {
        assert!(
            Instant::now() < deadline,
            "queued popup lease did not retire"
        );
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
}
async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    let owner = node(1);
    handle
        .update(cx, |view, window, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(owner, Kind::Menu, "".into(), Some(observer(1))),
                    Op::SetMenu(owner, config("Original")),
                    Op::Create(
                        NodeId::from_parts(1, 1).unwrap(),
                        Kind::Text,
                        "Native popup scheduling qualification".into(),
                        None,
                    ),
                    Op::Splice(owner, 0, 0, vec![NodeId::from_parts(1, 1).unwrap()]),
                    Op::SetRoot(Some(owner)),
                ],
            );
            window.activate_window();
        })
        .unwrap();
    host::editor_test::frame(cx, handle).await;
    let before = popup::tracking_calls();
    // Each invalidation occurs in the same update as Show. The CFRunLoop block
    // cannot run until this borrow returns; there is no guessed sleep window.
    handle
        .update(cx, |view, window, cx| {
            show(view, window, cx, owner, 1);
            assert_eq!(
                view.menu_command(
                    owner,
                    observer(1),
                    &menu_command::Command::Close,
                    window,
                    cx
                ),
                menu_command::Response::Applied
            );
            assert_eq!(popup::tracking_calls(), before);
        })
        .unwrap();
    retired(cx).await;
    assert_eq!(popup::tracking_calls(), before);
    eprintln!("POPUP_QUEUED_CLOSE_OK");

    handle
        .update(cx, |view, window, cx| {
            show(view, window, cx, owner, 1);
            apply(view, window, cx, vec![Op::Bind(owner, Some(observer(2)))]);
        })
        .unwrap();
    retired(cx).await;
    assert_eq!(popup::tracking_calls(), before);
    host::editor_test::frame(cx, handle).await;
    eprintln!("POPUP_QUEUED_OBSERVER_REPLACEMENT_OK");

    handle
        .update(cx, |view, window, cx| {
            show(view, window, cx, owner, 2);
            apply(
                view,
                window,
                cx,
                vec![Op::SetMenu(owner, config("Replacement"))],
            );
        })
        .unwrap();
    retired(cx).await;
    assert_eq!(popup::tracking_calls(), before);
    host::editor_test::frame(cx, handle).await;
    eprintln!("POPUP_QUEUED_DEFINITION_REPLACEMENT_OK");

    for field in [Field::Display(3), Field::Disabled(true)] {
        handle
            .update(cx, |view, window, cx| {
                show(view, window, cx, owner, 2);
                apply(
                    view,
                    window,
                    cx,
                    vec![Op::SetStyle(owner, vec![Style::Fields(vec![field])])],
                );
            })
            .unwrap();
        retired(cx).await;
        assert_eq!(popup::tracking_calls(), before);
        handle
            .update(cx, |view, window, cx| {
                apply(view, window, cx, vec![Op::SetStyle(owner, vec![])]);
            })
            .unwrap();
        host::editor_test::frame(cx, handle).await;
    }
    eprintln!("POPUP_QUEUED_HIDDEN_DISABLED_OK");

    handle
        .update(cx, |view, window, cx| {
            show(view, window, cx, owner, 2);
            apply(
                view,
                window,
                cx,
                vec![
                    Op::SetRoot(None),
                    Op::Splice(owner, 0, 1, vec![]),
                    Op::Remove(NodeId::from_parts(1, 1).unwrap()),
                    Op::Remove(owner),
                ],
            );
        })
        .unwrap();
    retired(cx).await;
    assert_eq!(
        popup::tracking_calls(),
        before,
        "detached root entered native tracking"
    );
    let owner = node(2);
    handle
        .update(cx, |view, window, cx| {
            let child = NodeId::from_parts(1, 2).unwrap();
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(owner, Kind::Menu, "".into(), Some(observer(2))),
                    Op::SetMenu(owner, config("Remounted")),
                    Op::Create(child, Kind::Text, "Remounted popup owner".into(), None),
                    Op::Splice(owner, 0, 0, vec![child]),
                    Op::SetRoot(Some(owner)),
                ],
            );
        })
        .unwrap();
    host::editor_test::frame(cx, handle).await;
    eprintln!("POPUP_QUEUED_OWNER_REMOVAL_REMOUNT_OK");

    // Positive control: a current lease must really enter AppKit. Close only
    // after the native entry counter advances, using the normal foreground
    // command path during the nested tracking loop.
    let closer = cx.spawn(async move |cx| {
        let deadline = Instant::now() + Duration::from_secs(5);
        while popup::tracking_calls() == before {
            assert!(
                Instant::now() < deadline,
                "current popup never entered AppKit"
            );
            cx.background_executor()
                .timer(Duration::from_millis(10))
                .await;
        }
        handle
            .update(cx, |view, window, cx| {
                assert_eq!(
                    view.menu_command(
                        owner,
                        observer(2),
                        &menu_command::Command::Close,
                        window,
                        cx
                    ),
                    menu_command::Response::Applied
                );
            })
            .unwrap();
    });
    handle
        .update(cx, |view, window, cx| show(view, window, cx, owner, 2))
        .unwrap();
    retired(cx).await;
    closer.await;
    assert_eq!(popup::tracking_calls(), before + 1);
    eprintln!("POPUP_QUEUED_CURRENT_RECOVERY_OK");

    handle
        .update(cx, |view, window, cx| {
            show(view, window, cx, owner, 2);
            view.session.borrow_mut().close(view.id).unwrap();
            window.remove_window();
        })
        .unwrap();
    retired(cx).await;
    assert_eq!(popup::tracking_calls(), before + 1);
    eprintln!("POPUP_QUEUED_WINDOW_CLOSE_OK");
}

pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        gpui_base::init(cx);
        let session = Rc::new(RefCell::new(Session::default()));
        let id = WindowId::from_parts(0, 1).unwrap();
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, id, "GPUIO queued popup test", 420., 180.)
            .unwrap();
        let handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(420.), px(180.)),
                        cx,
                    ))),
                    focus: true,
                    ..Default::default()
                },
                |window, cx| {
                    window.set_window_title("GPUIO queued popup test");
                    cx.new(|_| View::new(id, session, transport))
                },
            )
            .unwrap();
        cx.activate(true);
        cx.spawn(async move |cx| {
            let result = host::native_test::protect(exercise(cx, handle)).await;
            *task_failure.borrow_mut() = result.err();
            cx.update(host::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
    eprintln!("GPUIO_NATIVE_POPUP_QUEUE_OK");
}
