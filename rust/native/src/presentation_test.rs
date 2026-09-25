//! Native presentation semantics; expanded with the OCH-33 component families.
use super::*;
#[path = "loading_test.rs"]
mod loading_test;
use gpuio_protocol::accessibility::{Config, Field, Live, Role};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
fn node(n: i64) -> NodeId {
    NodeId::from_parts(n, 1).unwrap()
}
fn wid() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn handler(n: i64) -> gpuio_protocol::HandlerId {
    gpuio_protocol::HandlerId::from_parts(n, 1).unwrap()
}
fn field(error: Option<&str>, required: bool) -> Config {
    Config {
        role: None,
        label: None,
        description: None,
        live: Live::Off,
        field: Some(Field {
            label: "Email address".into(),
            help: Some("Kept private".into()),
            error: error.map(str::to_owned),
            required,
        }),
    }
}
fn apply(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, operations: Vec<Op>) {
    handle
        .update(cx, |v, w, cx| {
            let base = v.session.borrow().tree(v.id).unwrap().revision();
            let result = v
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: v.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            v.update_editors(&result.dirty, w, cx);
            v.list_actions(&result.lists);
            cx.notify();
        })
        .unwrap();
}
async fn frame(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    super::editor_test::frame(cx, handle).await;
    super::editor_test::frame(cx, handle).await;
}
fn snapshot(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> EditorSnapshot {
    handle
        .update(cx, |v, w, cx| v.editors[&node(1)].snapshot(w, cx))
        .unwrap()
}
#[cfg(target_os = "macos")]
#[derive(Debug, PartialEq)]
struct Accessible {
    role: String,
    help: Option<String>,
    required: bool,
}
#[cfg(target_os = "macos")]
fn accessible(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    label: &str,
    press: bool,
) -> Option<Accessible> {
    use objc2::{
        msg_send,
        runtime::{AnyObject, Bool},
    };
    use objc2_foundation::NSString;
    unsafe fn visit(
        object: *mut AnyObject,
        label: &str,
        press: bool,
        depth: usize,
    ) -> Option<Accessible> {
        if object.is_null() || depth > 20 {
            return None;
        }
        unsafe {
            let title: *mut NSString = msg_send![object, accessibilityTitle];
            let role: *mut NSString = msg_send![object, accessibilityRole];
            if !title.is_null()
                && (*title).to_string() == label
                && !role.is_null()
                && ["AXTextField", "AXLink", "AXProgressIndicator"]
                    .contains(&(*role).to_string().as_str())
            {
                let help: *mut NSString = msg_send![object, accessibilityHelp];
                let required: Bool = msg_send![object, isAccessibilityRequired];
                if press {
                    let _: () = msg_send![object,setAccessibilityFocused:true];
                    let activated: Bool = msg_send![object, accessibilityPerformPress];
                    assert!(activated.as_bool());
                }
                return Some(Accessible {
                    role: (*role).to_string(),
                    help: help.as_ref().map(|s| s.to_string()),
                    required: required.as_bool(),
                });
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if children.is_null() {
                return None;
            }
            let count: usize = msg_send![children, count];
            assert!(count < 128);
            for index in 0..count {
                let child: *mut AnyObject = msg_send![children,objectAtIndex:index];
                if let Some(found) = visit(child, label, press, depth + 1) {
                    return Some(found);
                }
            }
            None
        }
    }
    let address = super::editor_test::native_view(cx, handle) as *mut AnyObject;
    unsafe {
        let window: *mut AnyObject = msg_send![address, window];
        let content: *mut AnyObject = msg_send![window, contentView];
        visit(content, label, press, 0)
    }
}
async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(0), Kind::Container, "".into(), None),
            Op::Create(node(1), Kind::Input, "".into(), Some(handler(1))),
            Op::SetEditor(
                node(1),
                EditorConfig {
                    label: "Original editor".into(),
                    placeholder: "".into(),
                    disabled: false,
                    read_only: false,
                    submit_on_enter: true,
                    auto_focus: true,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::SetStyle(
                node(1),
                vec![
                    Style::Width(Length::Px(300.)),
                    Style::Height(Length::Px(40.)),
                ],
            ),
            Op::SetAccessibility(node(1), Some(field(Some("Required"), true))),
            Op::Create(
                node(2),
                Kind::Button,
                "Privacy policy".into(),
                Some(handler(2)),
            ),
            Op::SetControl(node(2), Control::Button(false)),
            Op::SetAccessibility(
                node(2),
                Some(Config {
                    role: Some(Role::Link),
                    label: Some("Privacy policy".into()),
                    description: None,
                    live: Live::Off,
                    field: None,
                }),
            ),
            Op::Splice(node(0), 0, 0, vec![node(1), node(2)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    {
        accessible(cx, handle, "Email address", false);
        frame(cx, handle).await;
        assert_eq!(
            accessible(cx, handle, "Email address", false),
            Some(Accessible {
                role: "AXTextField".into(),
                help: Some("Kept private\nRequired".into()),
                required: true
            })
        );
    }
    super::editor_test::native_text(cx, handle, "retained 👩🏽‍💻", false);
    #[cfg(target_os = "macos")]
    super::editor_test::native_text(cx, handle, "仮", true);
    let before = snapshot(cx, handle);
    assert!(before.focused);
    let alive = handle
        .update(cx, |v, _, _| v.editors[&node(1)].liveness_probe())
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetAccessibility(node(1), Some(field(None, false)))],
    );
    frame(cx, handle).await;
    assert_eq!(
        snapshot(cx, handle),
        before,
        "semantic update preserves text, revision, selection, focus and composition"
    );
    assert!(alive());
    #[cfg(target_os = "macos")]
    assert_eq!(
        accessible(cx, handle, "Email address", false),
        Some(Accessible {
            role: "AXTextField".into(),
            help: Some("Kept private".into()),
            required: false
        })
    );
    apply(cx, handle, vec![Op::SetAccessibility(node(1), None)]);
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle), before);
    #[cfg(target_os = "macos")]
    {
        assert!(accessible(cx, handle, "Email address", false).is_none());
        assert_eq!(
            accessible(cx, handle, "Original editor", false),
            Some(Accessible {
                role: "AXTextField".into(),
                help: None,
                required: false
            })
        );
    }
    super::editor_test::native_text(cx, handle, "確", false);
    assert!(snapshot(cx, handle).composition.is_none());
    transport.mailbox.lock().unwrap().drain(256);
    #[cfg(target_os = "macos")]
    {
        let found = accessible(cx, handle, "Privacy policy", true).unwrap();
        assert_eq!(found.role, "AXLink");
        frame(cx, handle).await;
        assert_eq!(
            transport
                .mailbox
                .lock()
                .unwrap()
                .drain(256)
                .iter()
                .filter(|e| matches!(e,Event::Press(_,id,_,_) if *id==node(2)))
                .count(),
            1
        );
    }
    handle
        .update(cx, |v, w, cx| w.focus(&v.buttons[&node(2)].focus, cx))
        .unwrap();
    super::editor_test::key(cx, handle, "enter");
    assert_eq!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .iter()
            .filter(|e| matches!(e,Event::Press(_,id,_,_) if *id==node(2)))
            .count(),
        1
    );
    apply(
        cx,
        handle,
        vec![
            Op::SetRoot(None),
            Op::Remove(node(2)),
            Op::Remove(node(1)),
            Op::Remove(node(0)),
        ],
    );
    frame(cx, handle).await;
    assert!(!alive());
    loading_test::exercise(cx, handle).await;
    handle
        .update(cx, |v, w, _| {
            assert_eq!(v.session.borrow().retained_bytes(), 0);
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
        })
        .unwrap();
    eprintln!(
        "GPUIO_PRESENTATION_SEMANTICS_OK: semantic-only updates/reset preserve editor state and native link activation; disposal releases metadata/editor"
    );
    #[cfg(target_os = "macos")]
    eprintln!(
        "GPUIO_PRESENTATION_AX_OK: live AppKit field label/help/error/required, AXLink activation and IME-preserving metadata changes"
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
        let motion_watch = crate::motion_preference::init(cx);
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, wid(), "Presentation semantics", 360., 220.)
            .unwrap();
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(360.), px(220.)),
                        cx,
                    ))),
                    focus: true,
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(wid(), session.clone(), transport.clone())),
            )
            .unwrap();
        cx.activate(true);
        cx.spawn(async move |cx| {
            let result = super::native_test::protect(exercise(cx, window, &transport)).await;
            *task_failure.borrow_mut() = result.err();
            drop(motion_watch);
            cx.update(super::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
