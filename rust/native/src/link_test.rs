//! Production-window composed links: renderer, focus ordering and queued input.
//! Dispatched GPUI input is distinct from the public gallery's AppKit automation.
use super::*;
use gpuio_protocol::link::Config;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
#[path = "link_nonstop_test.rs"]
mod nonstop_test;
#[path = "link_scroll_test.rs"]
mod scroll_test;

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn config(slot: i64, tab_index: i64) -> Config {
    Config {
        label: format!("Guide {slot} 世界"),
        disabled: false,
        tab_stop: true,
        tab_index,
    }
}
fn apply(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, operations: Vec<Op>) {
    handle
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let transaction = Transaction {
                window: view.id,
                base,
                revision: base + 1,
                operations,
            };
            let result = view
                .session
                .borrow_mut()
                .apply(&transaction)
                .unwrap_or_else(|error| panic!("{error:?}: {transaction:?}"));
            view.update_editors(&result.dirty, window, cx);
            cx.notify();
        })
        .unwrap();
}
fn draw(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
}
fn focus(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, slot: i64) {
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&id(slot)].focus, cx)
        })
        .unwrap();
}
#[track_caller]
fn focused(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, slot: i64) {
    assert!(
        handle
            .update(cx, |view, window, _| view.buttons[&id(slot)]
                .focus
                .is_focused(window))
            .unwrap(),
        "expected focused link {slot}"
    );
}
fn presses(transport: &Transport) -> Vec<NodeId> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| match event {
            Event::Press(_, node, _, _) => Some(node),
            _ => None,
        })
        .collect()
}
fn key(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, key: &str) {
    super::editor_test::key(cx, handle, key);
    draw(cx, handle);
}
fn click(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, slot: i64) {
    let center = handle
        .update(cx, |view, _, _| {
            view.probes.borrow()[&id(slot)].bounds.center()
        })
        .unwrap();
    super::native_test::move_mouse(cx, handle, center, false);
    draw(cx, handle);
    super::native_test::mouse(cx, handle, center, true);
    super::native_test::mouse(cx, handle, center, false);
    draw(cx, handle);
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
enum AxAction {
    Inspect,
    Focus,
    Press,
}

#[cfg(target_os = "macos")]
fn accessible(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    slot: i64,
    action: AxAction,
) -> Option<bool> {
    use objc2::{
        msg_send,
        runtime::{AnyObject, Bool},
    };
    use objc2_foundation::NSString;
    unsafe fn visit(
        object: *mut AnyObject,
        label: &str,
        action: AxAction,
        depth: usize,
    ) -> Option<bool> {
        if object.is_null() || depth > 32 {
            return None;
        }
        unsafe {
            let title: *mut NSString = msg_send![object, accessibilityTitle];
            let role: *mut NSString = msg_send![object, accessibilityRole];
            if !title.is_null()
                && !role.is_null()
                && (*title).to_string() == label
                && (*role).to_string() == "AXLink"
            {
                let enabled: Bool = msg_send![object, isAccessibilityEnabled];
                match action {
                    AxAction::Inspect => (),
                    AxAction::Focus => {
                        let _: () = msg_send![object,setAccessibilityFocused:true];
                    }
                    AxAction::Press => {
                        let _: Bool = msg_send![object, accessibilityPerformPress];
                    }
                }
                return Some(enabled.as_bool());
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if children.is_null() {
                return None;
            }
            let count: usize = msg_send![children, count];
            assert!(count < 128);
            for index in 0..count {
                let child: *mut AnyObject = msg_send![children,objectAtIndex:index];
                if let Some(found) = visit(child, label, action, depth + 1) {
                    return Some(found);
                }
            }
            None
        }
    }
    // Query outside a View update: AX can synchronously re-enter the root entity.
    let view = super::editor_test::native_view(cx, handle) as *mut AnyObject;
    unsafe {
        let window: *mut AnyObject = msg_send![view, window];
        let content: *mut AnyObject = msg_send![window, contentView];
        visit(content, &config(slot, 0).label, action, 0)
    }
}
async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let mut operations = vec![
        Op::Create(id(0), Kind::Container, String::new(), None),
        // Inherited selection must not create extra text focus owners inside links.
        Op::SetStyle(
            id(0),
            vec![Style::Fields(vec![
                Field::Display(1),
                Field::Direction(1),
                Field::UserSelect(true),
            ])],
        ),
    ];
    for (slot, index) in [(1, 10), (2, 30), (3, 20), (4, 20)] {
        operations.extend([
            Op::Create(
                id(slot),
                Kind::Link,
                String::new(),
                Some(gpuio_protocol::HandlerId::from_parts(slot, 1).unwrap()),
            ),
            Op::SetLink(id(slot), config(slot, index)),
            Op::SetStyle(
                id(slot),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(260.)),
                    Field::Height(Length::Px(42.)),
                    Field::Background(Fill::Solid(Color::Rgba(0x26354aff))),
                ])],
            ),
            Op::Create(
                id(slot + 5),
                Kind::Text,
                format!("Visible guide {slot} 👩🏽‍💻"),
                None,
            ),
            Op::Splice(id(slot), 0, 0, vec![id(slot + 5)]),
        ]);
    }
    operations.extend([
        Op::Create(
            id(5),
            Kind::Button,
            "Ordinary button".into(),
            Some(gpuio_protocol::HandlerId::from_parts(5, 1).unwrap()),
        ),
        Op::Splice(id(0), 0, 0, vec![id(1), id(2), id(3), id(4), id(5)]),
        Op::SetRoot(Some(id(0))),
    ]);
    // Allocate contiguous generational slots before configuring/attaching them.
    operations.sort_by_key(|op| match op {
        Op::Create(node, ..) => (0, node.slot()),
        _ => (1, 0),
    });
    apply(cx, handle, operations);
    super::editor_test::frame(cx, handle).await;
    draw(cx, handle);
    let retained = handle
        .update(cx, |view, _, _| {
            assert!(
                view.selections.is_empty(),
                "link text inherits selection suppression"
            );
            view.buttons[&id(3)].focus.clone()
        })
        .unwrap();
    #[cfg(target_os = "macos")]
    {
        accessible(cx, handle, 1, AxAction::Inspect);
        super::editor_test::frame(cx, handle).await;
        for slot in 1..=4 {
            assert_eq!(accessible(cx, handle, slot, AxAction::Inspect), Some(true));
        }
        accessible(cx, handle, 3, AxAction::Focus);
        super::editor_test::frame(cx, handle).await;
        focused(cx, handle, 3);
        assert!(presses(transport).is_empty());
        accessible(cx, handle, 3, AxAction::Press);
        super::editor_test::frame(cx, handle).await;
        assert_eq!(presses(transport), vec![id(3)]);
    }
    focus(cx, handle, 5);
    draw(cx, handle);
    for slot in [1, 3, 4, 2, 5] {
        key(cx, handle, "tab");
        focused(cx, handle, slot);
    }
    for slot in [2, 4, 3, 1, 5] {
        key(cx, handle, "shift-tab");
        focused(cx, handle, slot);
    }
    assert!(presses(transport).is_empty(), "focus never activates");
    focus(cx, handle, 3);
    draw(cx, handle);
    for key_name in ["enter", "space"] {
        key(cx, handle, key_name);
        assert_eq!(
            presses(transport),
            vec![id(3)],
            "one activation from {key_name}"
        );
    }
    click(cx, handle, 8);
    assert_eq!(
        presses(transport),
        vec![id(3)],
        "clicking visible child activates root once"
    );
    apply(
        cx,
        handle,
        vec![
            Op::SetText(id(8), "Replacement content 世界".into()),
            Op::SetLink(id(3), config(3, -10)),
        ],
    );
    draw(cx, handle);
    focused(cx, handle, 3);
    assert!(
        handle
            .update(cx, |view, _, _| view.buttons[&id(3)].focus == retained)
            .unwrap()
    );
    key(cx, handle, "tab");
    focused(cx, handle, 5);
    apply(
        cx,
        handle,
        vec![Op::SetLink(
            id(1),
            Config {
                tab_stop: false,
                ..config(1, 10)
            },
        )],
    );
    draw(cx, handle);
    key(cx, handle, "tab");
    focused(cx, handle, 4);
    click(cx, handle, 6);
    focused(cx, handle, 1);
    assert_eq!(
        presses(transport),
        vec![id(1)],
        "Tab opt-out still receives pointer activation"
    );
    // A non-stop remains an ordering anchor after pointer/AX focus. Filtering
    // it out before finding current focus wrongly jumps to the first/last stop.
    key(cx, handle, "tab");
    focused(cx, handle, 4);
    focus(cx, handle, 1);
    draw(cx, handle);
    key(cx, handle, "shift-tab");
    focused(cx, handle, 5);
    #[cfg(target_os = "macos")]
    {
        accessible(cx, handle, 1, AxAction::Focus);
        super::editor_test::frame(cx, handle).await;
        focused(cx, handle, 1);
        key(cx, handle, "tab");
        focused(cx, handle, 4);
    }
    // Changing the policy while focused preserves the native handle and anchor.
    focus(cx, handle, 4);
    apply(
        cx,
        handle,
        vec![Op::SetLink(
            id(4),
            Config {
                tab_stop: false,
                ..config(4, 20)
            },
        )],
    );
    draw(cx, handle);
    focused(cx, handle, 4);
    key(cx, handle, "tab");
    focused(cx, handle, 2);
    focus(cx, handle, 4);
    draw(cx, handle);
    key(cx, handle, "shift-tab");
    focused(cx, handle, 5);
    assert!(presses(transport).is_empty(), "traversal never activates");
    apply(cx, handle, vec![Op::SetLink(id(4), config(4, 20))]);
    apply(
        cx,
        handle,
        vec![Op::SetLink(
            id(1),
            Config {
                disabled: true,
                ..config(1, 10)
            },
        )],
    );
    draw(cx, handle);
    click(cx, handle, 6);
    assert!(presses(transport).is_empty());
    #[cfg(target_os = "macos")]
    {
        assert_eq!(accessible(cx, handle, 1, AxAction::Press), Some(false));
        accessible(cx, handle, 1, AxAction::Focus);
        super::editor_test::frame(cx, handle).await;
        assert!(presses(transport).is_empty());
        assert!(
            !handle
                .update(cx, |view, window, _| view.buttons[&id(1)]
                    .focus
                    .is_focused(window))
                .unwrap()
        );
    }
    apply(cx, handle, vec![Op::SetLink(id(1), config(1, 10))]);
    draw(cx, handle);
    click(cx, handle, 6);
    assert_eq!(presses(transport), vec![id(1)]);
    // A trap moves existing keyed links without replacing their native handles.
    focus(cx, handle, 1);
    apply(
        cx,
        handle,
        vec![
            Op::Create(id(10), Kind::FocusScope, String::new(), None),
            Op::SetFocusScope(
                id(10),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Splice(id(0), 1, 3, vec![id(10)]),
            Op::Splice(id(10), 0, 0, vec![id(2), id(3), id(4)]),
        ],
    );
    super::editor_test::frame(cx, handle).await;
    draw(cx, handle);
    focused(cx, handle, 3);
    for slot in [4, 2, 3] {
        key(cx, handle, "tab");
        focused(cx, handle, slot);
    }
    key(cx, handle, "shift-tab");
    focused(cx, handle, 2);
    nonstop_test::exercise_trap(cx, handle, transport);
    click(cx, handle, 6);
    assert!(
        presses(transport).is_empty(),
        "outside pointer blocked during trap"
    );
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id(4),
            vec![Style::Fields(vec![Field::Inert(true)])],
        )],
    );
    draw(cx, handle);
    key(cx, handle, "shift-tab");
    focused(cx, handle, 3);
    apply(
        cx,
        handle,
        vec![
            Op::Splice(id(10), 0, 3, vec![]),
            Op::Splice(id(0), 1, 1, vec![id(2), id(3), id(4)]),
            Op::Remove(id(10)),
        ],
    );
    super::editor_test::frame(cx, handle).await;
    draw(cx, handle);
    focused(cx, handle, 1);
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id(3),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    draw(cx, handle);
    key(cx, handle, "tab");
    focused(cx, handle, 2);
    scroll_test::exercise(cx, handle, transport).await;
    let mut remove = vec![Op::SetRoot(None)];
    remove.extend((6..=9).map(|slot| Op::Remove(id(slot))));
    remove.extend((0..=5).map(|slot| Op::Remove(id(slot))));
    apply(cx, handle, remove);
    draw(cx, handle);
    handle
        .update(cx, |view, _, _| {
            assert!(view.buttons.is_empty());
            assert!(view.selections.is_empty());
            assert_eq!(view.session.borrow().retained_bytes(), 0);
        })
        .unwrap();
    eprintln!(
        "GPUIO_COMPOSED_LINK_OK: content activation, signed stable Tab order, opt-out, disabled recovery, native identity, trapped/inert/hidden navigation and disposal (GPUI-dispatched input)"
    );
}

pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        gpui_base::init(cx);
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        let window_id = WindowId::from_parts(0, 1).unwrap();
        session
            .borrow_mut()
            .open(1, window_id, "Composed link check", 340., 340.)
            .unwrap();
        let handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(340.), px(340.)),
                        cx,
                    ))),
                    focus: true,
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(window_id, session, transport.clone())),
            )
            .unwrap();
        cx.activate(true);
        cx.spawn(async move |cx| {
            let result = native_test::protect(exercise(cx, handle, &transport)).await;
            *task_failure.borrow_mut() = result.err();
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            cx.update(stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
