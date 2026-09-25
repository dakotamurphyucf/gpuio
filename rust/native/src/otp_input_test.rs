//! Real retained field, native keyboard dispatch and macOS NSTextInputClient.
use super::super::{
    editor_test::{frame, key, native_text},
    stop_application,
};
use super::*;
use crate::session::Session;
use std::{
    cell::RefCell,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    rc::Rc,
};
#[cfg(target_os = "macos")]
#[path = "otp_input_accessibility_test.rs"]
mod accessibility;

fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn handler() -> HandlerId {
    HandlerId::from_parts(0, 1).unwrap()
}
fn config() -> o::Config {
    o::Config {
        policy: o::Policy::new(6, o::Alphabet::Digits).unwrap(),
        label: "Verification code".into(),
        masked: false,
        disabled: false,
        read_only: false,
        auto_focus: true,
    }
}
fn apply(cx: &mut AsyncApp, handle: WindowHandle<View>, operations: Vec<Op>) {
    handle
        .update(cx, |v, w, cx| {
            let base = v.session.borrow().tree(v.id).unwrap().revision();
            let applied = v
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: v.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            v.update_editors(&applied.dirty, w, cx);
            cx.notify();
        })
        .unwrap();
}
fn snapshot(cx: &mut AsyncApp, handle: WindowHandle<View>) -> o::Snapshot {
    handle
        .update(cx, |v, _, cx| {
            v.otps[&node()].state.read(cx).model.snapshot()
        })
        .unwrap()
}
fn command(cx: &mut AsyncApp, handle: WindowHandle<View>, command: o::Command) -> o::Response {
    handle
        .update(cx, |v, w, cx| v.otps[&node()].command(&command, w, cx))
        .unwrap()
}
fn events(transport: &Transport) -> Vec<o::Event> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::OtpInputEvent(_, _, _, _, event) => Some(event),
            Event::Press(..) | Event::EditorEvent(..) => panic!("OTP leaked generic input event"),
            _ => None,
        })
        .collect()
}
fn clipboard(cx: &mut AsyncApp, text: &str) {
    cx.update(|cx| cx.write_to_clipboard(ClipboardItem::new_string(text.into())));
}
#[cfg(target_os = "macos")]
const SELECT_ALL: &str = "cmd-a";
#[cfg(not(target_os = "macos"))]
const SELECT_ALL: &str = "ctrl-a";
#[cfg(target_os = "macos")]
const PASTE: &str = "cmd-v";
#[cfg(not(target_os = "macos"))]
const PASTE: &str = "ctrl-v";
#[cfg(target_os = "macos")]
const COPY: &str = "cmd-c";
#[cfg(not(target_os = "macos"))]
const COPY: &str = "ctrl-c";

async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(), Kind::OtpInput, "".into(), Some(handler())),
            Op::SetOtpInput(node(), config(), "12".into()),
            Op::SetRoot(Some(node())),
        ],
    );
    frame(cx, handle).await;
    frame(cx, handle).await;
    assert!(snapshot(cx, handle).focused);
    let initial = events(transport);
    assert!(
        initial
            .iter()
            .any(|event| matches!(event, o::Event::Observed(_)))
    );
    assert!(
        !initial
            .iter()
            .any(|event| matches!(event, o::Event::Complete(_)))
    );
    // Platform insertion, full-width normalization and ordered completion.
    native_text(cx, handle, "３４５６", false);
    frame(cx, handle).await;
    assert_eq!(snapshot(cx, handle).value, "123456");
    let complete = events(transport);
    assert!(
        matches!(complete.as_slice(),[o::Event::Changed(a),o::Event::Complete(b)] if b.revision==a.revision+1 && b.value=="123456")
    );
    // Shift navigation, replacement and native undo/redo through GPUI key map.
    key(cx, handle, "shift-left");
    assert_eq!(
        snapshot(cx, handle).selection,
        o::Selection { anchor: 6, head: 5 }
    );
    native_text(cx, handle, "7", false);
    assert_eq!(snapshot(cx, handle).value, "123457");
    key(
        cx,
        handle,
        if cfg!(target_os = "macos") {
            "cmd-z"
        } else {
            "ctrl-z"
        },
    );
    assert_eq!(snapshot(cx, handle).value, "123456");
    key(
        cx,
        handle,
        if cfg!(target_os = "macos") {
            "cmd-shift-z"
        } else {
            "ctrl-shift-z"
        },
    );
    assert_eq!(snapshot(cx, handle).value, "123457");
    events(transport);
    key(cx, handle, SELECT_ALL);
    clipboard(cx, "9 8-7\n6\t5 4");
    key(cx, handle, PASTE);
    assert_eq!(snapshot(cx, handle).value, "987654");
    key(cx, handle, SELECT_ALL);
    clipboard(cx, "sentinel");
    key(cx, handle, COPY);
    assert_eq!(
        cx.update(|cx| cx.read_from_clipboard().unwrap().text().unwrap()),
        "987654"
    );
    // Bad complete input preserves the checkpoint; no valid prefix is accepted.
    native_text(cx, handle, "12x", false);
    assert_eq!(snapshot(cx, handle).value, "987654");
    assert!(
        events(transport)
            .iter()
            .any(|event| matches!(event,o::Event::Rejected(_,s) if s.value=="987654"))
    );
    #[cfg(target_os = "macos")]
    {
        key(cx, handle, SELECT_ALL);
        frame(cx, handle).await;
        native_text(cx, handle, "１２", true);
        frame(cx, handle).await;
        let s = snapshot(cx, handle);
        assert_eq!(s.value, "987654");
        assert_eq!(s.draft, "１２");
        assert!(s.composition.is_some());
        // The native command adapter must not overwrite or finalize an OS preedit.
        events(transport);
        assert_eq!(
            command(cx, handle, o::Command::ReadSnapshot),
            o::Response::Applied(s.clone())
        );
        for blocked in [
            o::Command::Replace {
                value: "34".into(),
                selection: o::SelectionPolicy::End,
                undo: o::UndoPolicy::Record,
                if_revision: None,
            },
            o::Command::Clear {
                undo: o::UndoPolicy::Reset,
                if_revision: None,
            },
            o::Command::Select(o::Selection { anchor: 0, head: 0 }),
            o::Command::Undo,
            o::Command::Redo,
        ] {
            assert_eq!(
                command(cx, handle, blocked),
                o::Response::Failed(o::Error::Composing)
            );
            assert_eq!(snapshot(cx, handle), s);
        }
        assert!(events(transport).is_empty());
        // The actual painted geometry answers candidate/point queries at exact boundaries.
        handle
            .update(cx, |v, w, cx| {
                v.otps[&node()].state.update(cx, |s, cx| {
                    let bounds = s.layout.as_ref().unwrap().bounds;
                    let candidate = s.bounds_for_range(1..2, bounds, w, cx).unwrap();
                    assert!(candidate.size.width > px(0.));
                    assert!(bounds.contains(&candidate.origin));
                    assert_eq!(s.text_for_range(0..2, &mut None, w, cx).unwrap(), "１２");
                    assert_eq!(s.selected_text_range(false, w, cx).unwrap().range, 2..2);
                })
            })
            .unwrap();
        // Rerendered seed never resets an active preedit or native history.
        let updated = o::Config {
            label: "Updated code".into(),
            ..config()
        };
        apply(
            cx,
            handle,
            vec![Op::SetOtpInput(node(), updated, "000000".into())],
        );
        assert_eq!(snapshot(cx, handle).draft, "１２");
        events(transport);
        let o::Response::Applied(cancelled) = command(cx, handle, o::Command::CancelComposition)
        else {
            panic!("cancel must succeed")
        };
        assert_eq!(cancelled.value, "987654");
        assert_eq!(cancelled.draft, cancelled.value);
        assert_eq!(cancelled.selection, o::Selection { anchor: 0, head: 6 });
        assert!(cancelled.composition.is_none());
        assert_eq!(events(transport), vec![o::Event::Observed(cancelled)]);
        frame(cx, handle).await;
        native_text(cx, handle, "１２", true);
        native_text(cx, handle, "１２３４５６", false);
        assert_eq!(snapshot(cx, handle).value, "123456");
        assert!(snapshot(cx, handle).composition.is_none());
        frame(cx, handle).await;
        key(cx, handle, SELECT_ALL);
        native_text(cx, handle, "👩‍👩‍👧‍👦", true);
        frame(cx, handle).await;
        native_text(cx, handle, "👩‍👩‍👧‍👦", false);
        assert_eq!(snapshot(cx, handle).value, "123456");
        assert!(snapshot(cx, handle).composition.is_none());
    }
    #[cfg(target_os = "macos")]
    {
        // AccessKit activates lazily when the platform first queries this window.
        accessibility::field(cx, handle, "Updated code", None);
        frame(cx, handle).await;
    }
    // Pointer hit testing uses the same geometry as the platform candidate query.
    apply(
        cx,
        handle,
        vec![Op::SetOtpInput(node(), config(), "".into())],
    );
    frame(cx, handle).await;
    let points = handle
        .update(cx, |v, w, cx| {
            v.otps[&node()].state.update(cx, |s, cx| {
                let bounds = s.layout.as_ref().unwrap().bounds;
                [2, 4].map(|offset| {
                    let range = s.bounds_for_range(offset..offset, bounds, w, cx).unwrap();
                    point(range.left(), range.center().y)
                })
            })
        })
        .unwrap();
    super::super::native_test::move_mouse(cx, handle, points[0], false);
    super::super::native_test::mouse(cx, handle, points[0], true);
    super::super::native_test::move_mouse(cx, handle, points[1], true);
    super::super::native_test::mouse(cx, handle, points[1], false);
    assert_eq!(
        snapshot(cx, handle).selection,
        o::Selection { anchor: 2, head: 4 }
    );
    // Capture survives a repaint and continues selection outside the field.
    super::super::native_test::mouse(cx, handle, points[0], true);
    frame(cx, handle).await;
    let outside = handle
        .update(cx, |v, w, cx| {
            assert!(w.captured_hitbox().is_some());
            let bounds = v.otps[&node()]
                .state
                .read(cx)
                .layout
                .as_ref()
                .unwrap()
                .bounds;
            point(bounds.right() + px(40.), bounds.center().y)
        })
        .unwrap();
    super::super::native_test::move_mouse(cx, handle, outside, true);
    super::super::native_test::mouse(cx, handle, outside, false);
    assert_eq!(
        snapshot(cx, handle).selection,
        o::Selection { anchor: 2, head: 6 }
    );
    handle
        .update(cx, |v, w, cx| {
            assert!(w.captured_hitbox().is_none());
            v.otps[&node()]
                .state
                .update(cx, |s, cx| s.set_selected_text_range(2..4, w, cx));
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetOtpInput(
            node(),
            o::Config {
                read_only: true,
                ..config()
            },
            "".into(),
        )],
    );
    frame(cx, handle).await;
    let readonly = snapshot(cx, handle);
    native_text(cx, handle, "9", false);
    key(cx, handle, "backspace");
    assert_eq!(snapshot(cx, handle).value, readonly.value);
    key(cx, handle, COPY);
    assert_eq!(
        cx.update(|cx| cx.read_from_clipboard().unwrap().text().unwrap()),
        readonly.value[2..4]
    );
    #[cfg(target_os = "macos")]
    {
        let accessible = accessibility::field(cx, handle, "Verification code", None)
            .expect("OTP accessibility field");
        assert!(accessible.enabled);
        assert_eq!(accessible.value.as_deref(), Some(readonly.value.as_str()));
        apply(
            cx,
            handle,
            vec![Op::SetOtpInput(node(), config(), "".into())],
        );
        frame(cx, handle).await;
        accessibility::field(cx, handle, "Verification code", Some("６５４３２１"));
        frame(cx, handle).await;
        assert_eq!(snapshot(cx, handle).value, "654321");
    }
    #[cfg(target_os = "macos")]
    {
        // A maximum preedit remains bounded and scrolls its caret inside narrow geometry.
        apply(
            cx,
            handle,
            vec![Op::SetStyle(
                node(),
                vec![gpuio_protocol::v1::Style::Width(
                    gpuio_protocol::v1::Length::Px(80.),
                )],
            )],
        );
        frame(cx, handle).await;
        key(cx, handle, SELECT_ALL);
        native_text(cx, handle, &"9".repeat(4096), true);
        frame(cx, handle).await;
        assert_eq!(snapshot(cx, handle).draft.len(), 4096);
        handle
            .update(cx, |v, w, cx| {
                v.otps[&node()].state.update(cx, |s, cx| {
                    let bounds = s.layout.as_ref().unwrap().bounds;
                    let caret = s.bounds_for_range(4096..4096, bounds, w, cx).unwrap();
                    assert!(bounds.contains(&caret.origin));
                    assert_eq!(s.character_index_for_point(caret.origin, w, cx), Some(4096));
                })
            })
            .unwrap();
        native_text(cx, handle, &"9".repeat(4097), true);
        assert_eq!(snapshot(cx, handle).draft.len(), 4096);
        native_text(cx, handle, "too-long", false);
        assert_eq!(snapshot(cx, handle).value, "654321");
        assert!(snapshot(cx, handle).composition.is_none());
        apply(cx, handle, vec![Op::SetStyle(node(), vec![])]);
    }
    // Masked clipboard operations do not replace the user's clipboard.
    apply(
        cx,
        handle,
        vec![Op::SetOtpInput(
            node(),
            o::Config {
                masked: true,
                ..config()
            },
            "".into(),
        )],
    );
    frame(cx, handle).await;
    key(cx, handle, SELECT_ALL);
    clipboard(cx, "protected");
    key(cx, handle, COPY);
    assert_eq!(
        cx.update(|cx| cx.read_from_clipboard().unwrap().text().unwrap()),
        "protected"
    );
    #[cfg(target_os = "macos")]
    {
        let accessible = accessibility::field(cx, handle, "Verification code", None)
            .expect("OTP accessibility field");
        assert_eq!(accessible.subrole.as_deref(), Some("AXSecureTextField"));
        assert_ne!(
            accessible.value.as_deref(),
            Some(snapshot(cx, handle).value.as_str())
        );
        assert!(
            accessible
                .value
                .as_deref()
                .is_none_or(|value| value.chars().all(|ch| ch == '•'))
        );
    }
    // Disable during native composition clears it and releases focus.
    #[cfg(target_os = "macos")]
    native_text(cx, handle, "９", true);
    apply(
        cx,
        handle,
        vec![Op::SetOtpInput(
            node(),
            o::Config {
                disabled: true,
                ..config()
            },
            "".into(),
        )],
    );
    frame(cx, handle).await;
    assert!(snapshot(cx, handle).composition.is_none());
    assert!(!snapshot(cx, handle).focused);
    let value = snapshot(cx, handle).value;
    native_text(cx, handle, "0", false);
    assert_eq!(snapshot(cx, handle).value, value);
    let weak = handle
        .update(cx, |v, _, _| v.otps[&node()].state.downgrade())
        .unwrap();
    apply(cx, handle, vec![Op::SetRoot(None), Op::Remove(node())]);
    frame(cx, handle).await;
    assert!(cx.update(|cx| {
        weak.upgrade()
            .map(|entity| entity.read(cx).model.snapshot())
            .is_none()
    }));
    let new_node = NodeId::from_parts(0, 2).unwrap();
    let new_handler = HandlerId::from_parts(0, 2).unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::Create(new_node, Kind::OtpInput, "".into(), Some(new_handler)),
            Op::SetOtpInput(
                new_node,
                o::Config {
                    policy: o::Policy::new(4, o::Alphabet::AsciiAlphanumeric).unwrap(),
                    ..config()
                },
                "".into(),
            ),
            Op::SetRoot(Some(new_node)),
        ],
    );
    frame(cx, handle).await;
    events(transport);
    handle
        .update(cx, |v, _, cx| {
            let snapshot = v.otps[&new_node].state.read(cx).model.snapshot();
            let revision = v.session.borrow().tree(v.id).unwrap().revision();
            for _ in 0..crate::mailbox::MAX_INPUT_EVENTS {
                assert!(transport.input(Event::OtpInputEvent(
                    v.id,
                    new_node,
                    new_handler,
                    revision,
                    o::Event::Observed(snapshot.clone())
                )));
            }
        })
        .unwrap();
    native_text(cx, handle, "Ab12", false);
    handle
        .update(cx, |v, _, cx| {
            assert!(!v.session.borrow().accepts_input(v.id));
            assert_eq!(
                v.otps[&new_node].state.read(cx).model.editor().value(),
                "Ab12"
            );
        })
        .unwrap();
    handle
        .update(cx, |v, w, cx| {
            assert_eq!(
                v.otps[&new_node].command(
                    &o::Command::Clear {
                        undo: o::UndoPolicy::Reset,
                        if_revision: None
                    },
                    w,
                    cx
                ),
                o::Response::Failed(o::Error::NativeFailure)
            );
        })
        .unwrap();
    native_text(cx, handle, "3", false);
    handle
        .update(cx, |v, _, cx| {
            assert_eq!(
                v.otps[&new_node].state.read(cx).model.editor().value(),
                "Ab12"
            )
        })
        .unwrap();
    assert!(
        !events(transport)
            .iter()
            .any(|event| matches!(event, o::Event::Changed(_) | o::Event::Complete(_)))
    );
    handle
        .update(cx, |v, w, cx| {
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
            cx.notify();
        })
        .unwrap();
    eprintln!(
        "GPUIO_OTP_NATIVE_OK: retained owner, native key map/clipboard, ordered completion, NSTextInputClient composition/normalization/rollback, candidate/clipped geometry, pointer capture, read-only, AppKit AX values/actions, mask, disable, disposal and terminal overload"
    );
}
fn open_test_window(cx: &mut App, transport: &Arc<Transport>) -> WindowHandle<View> {
    let id = WindowId::from_parts(0, 1).unwrap();
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, id, "GPUIO OTP input test", 350., 130.)
        .unwrap();
    cx.open_window(
        WindowOptions {
            inactive_frame_interval: None,
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(350.), px(130.)),
                cx,
            ))),
            ..Default::default()
        },
        |_, cx| cx.new(|_| View::new(id, session.clone(), transport.clone())),
    )
    .unwrap()
}
async fn command_pressure(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(), Kind::OtpInput, "".into(), Some(handler())),
            Op::SetOtpInput(node(), config(), "12".into()),
            Op::SetRoot(Some(node())),
        ],
    );
    frame(cx, handle).await;
    frame(cx, handle).await;
    events(transport);
    handle
        .update(cx, |v, _, cx| {
            let snapshot = v.otps[&node()].state.read(cx).model.snapshot();
            let revision = v.session.borrow().tree(v.id).unwrap().revision();
            for _ in 0..crate::mailbox::MAX_INPUT_EVENTS {
                assert!(transport.input(Event::OtpInputEvent(
                    v.id,
                    node(),
                    handler(),
                    revision,
                    o::Event::Observed(snapshot.clone())
                )));
            }
        })
        .unwrap();
    // A successful mutation whose observation cannot be admitted must not return Applied.
    assert_eq!(
        command(
            cx,
            handle,
            o::Command::Replace {
                value: "654321".into(),
                selection: o::SelectionPolicy::End,
                undo: o::UndoPolicy::Record,
                if_revision: None,
            }
        ),
        o::Response::Failed(o::Error::NativeFailure)
    );
    assert_eq!(snapshot(cx, handle).value, "654321");
    assert_eq!(
        command(
            cx,
            handle,
            o::Command::Clear {
                undo: o::UndoPolicy::Reset,
                if_revision: None
            }
        ),
        o::Response::Failed(o::Error::NativeFailure)
    );
    assert_eq!(snapshot(cx, handle).value, "654321");
    let drained = transport.mailbox.lock().unwrap().drain(256);
    assert!(
        drained
            .iter()
            .any(|event| matches!(event, Event::Overloaded(_)))
    );
    assert!(!drained.iter().any(|event| matches!(event,
        Event::OtpInputEvent(_,_,_,_,o::Event::Observed(snapshot)) if snapshot.value == "654321")));
    handle
        .update(cx, |v, w, cx| {
            assert!(!v.session.borrow().accepts_input(v.id));
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
            cx.notify();
        })
        .unwrap();
    eprintln!(
        "GPUIO_OTP_COMMAND_PRESSURE_OK: lost observation reports failure and blocks later mutation"
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
        gpui_base::init(cx);
        cx.set_quit_mode(QuitMode::Explicit);
        let clipboard = cx.read_from_clipboard();
        let handle = open_test_window(cx, &transport);
        cx.activate(true);
        cx.spawn(async move |cx| {
            let result = super::super::native_test::protect(async {
                exercise(cx, handle, &transport).await;
                let next = cx.update(|cx| open_test_window(cx, &transport));
                command_pressure(cx, next, &transport).await;
            })
            .await;
            *task_failure.borrow_mut() = result.err();
            cx.update(|cx| {
                if let Some(clipboard) = clipboard {
                    cx.write_to_clipboard(clipboard);
                }
                stop_application(cx);
            });
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
