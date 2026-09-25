//! macOS OS delegate paths, alongside the public OCaml multi-window test.
use super::*;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
fn os_close(view: usize) {
    use objc2::{msg_send, runtime::AnyObject};
    unsafe {
        let window: *mut AnyObject = msg_send![view as *mut AnyObject, window];
        let _: () = msg_send![window,performClose:std::ptr::null::<AnyObject>()];
    }
}
fn os_reopen() {
    use objc2::{msg_send, runtime::AnyObject};
    unsafe {
        let app: *mut AnyObject = msg_send![objc2::class!(NSApplication), sharedApplication];
        let delegate: *mut AnyObject = msg_send![app, delegate];
        // GPUI's pinned delegate method has a void Objective-C encoding.
        let _: () = msg_send![delegate,applicationShouldHandleReopen:app, hasVisibleWindows:false];
    }
}
fn events(transport: &Transport) -> Vec<Event> {
    transport.mailbox.lock().unwrap().drain(256)
}
async fn exercise(
    cx: &mut gpui::AsyncApp,
    windows: &[WindowHandle<View>],
    transport: &Transport,
    platform: &Rc<dyn gpui::Platform>,
) {
    let first = windows[0];
    let second = windows[1];
    events(transport);
    let native = editor_test::native_view(cx, first);
    os_close(native);
    let id = WindowId::from_parts(0, 1).unwrap();
    assert!(
        events(transport)
            .iter()
            .any(|event| matches!(event,Event::CloseRequested(window) if *window==id))
    );
    assert_eq!(
        cx.update(|cx| cx.windows().len()),
        2,
        "native close must await OCaml decision"
    );
    os_close(native);
    os_close(native);
    assert_eq!(
        events(transport)
            .iter()
            .filter(|event| matches!(event, Event::CloseRequested(_)))
            .count(),
        1,
        "repeat close coalesces"
    );
    platform.quit();
    let mut saw_quit = false;
    for _ in 0..100 {
        cx.background_executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
        if events(transport)
            .iter()
            .any(|event| matches!(event, Event::QuitRequested))
        {
            saw_quit = true;
            break;
        }
    }
    assert!(
        saw_quit,
        "OS terminate must request an asynchronous decision"
    );
    assert_eq!(
        cx.update(|cx| cx.windows().len()),
        2,
        "quit has not been approved"
    );
    first
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    assert_eq!(cx.update(|cx| cx.windows().len()), 1);
    let response = second
        .update(cx, |view, window, _| {
            window_host::command(
                view,
                &gpuio_protocol::window::Command::SetTitle("Surviving window".into()),
                window,
            )
        })
        .unwrap();
    assert!(
        matches!(response,gpuio_protocol::window::Response::Observed(snapshot) if snapshot.title=="Surviving window")
    );
    second
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    assert!(
        cx.update(|cx| cx.windows().is_empty()),
        "explicit quit policy permits zero windows"
    );
    os_reopen();
    assert!(
        events(transport)
            .iter()
            .any(|event| matches!(event, Event::ReopenRequested)),
        "native reopen is forwarded with zero windows"
    );
    eprintln!(
        "GPUIO_NATIVE_WINDOW_OK: actual macOS performClose and terminate callbacks deferred; repeated requests coalesced; independent survivor; explicit zero-window policy and OS reopen"
    );
}
pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _read = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let write = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::with_options(write.as_raw_fd(), false).unwrap());
    let platform = gpui_platform::current_platform(false);
    let application = gpui::Application::with_platform(platform.clone());
    let reopen_transport = transport.clone();
    application.on_reopen(move |_| window_host::control(&reopen_transport, Event::ReopenRequested));
    application.run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        gpui_base::init(cx);
        window_macos::install(cx, transport.clone());
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        let windows = (0..2)
            .map(|slot| {
                let id = WindowId::from_parts(slot, 1).unwrap();
                session
                    .borrow_mut()
                    .open(slot + 1, id, "Window test", 600., 360.)
                    .unwrap();
                cx.open_window(
                    WindowOptions {
                        focus: false,
                        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                            None,
                            size(px(600.), px(360.)),
                            cx,
                        ))),
                        ..Default::default()
                    },
                    |window, cx| {
                        cx.new(|cx| {
                            let view = View::new(id, session.clone(), transport.clone());
                            window_host::watch(&view, window, cx);
                            view
                        })
                    },
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        cx.spawn(async move |cx| {
            let result = native_test::protect(exercise(cx, &windows, &transport, &platform)).await;
            for handle in windows {
                let _ = handle.update(cx, |_, window, _| window.remove_window());
            }
            *task_failure.borrow_mut() = result.err();
            cx.update(stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
