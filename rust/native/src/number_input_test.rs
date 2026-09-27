//! Actual native InputState checks, separate from the deterministic policy model.
use super::super::{
    editor_test::{frame as editor_frame, key, native_text},
    stop_application,
};
use super::*;
use crate::session::Session;
use gpuio_protocol::numeric::Domain;
use gpuio_protocol::v1::{Length, Style};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
#[cfg(target_os = "macos")]
#[path = "number_input_accessibility_test.rs"]
mod accessibility_test;
#[cfg(feature = "native-image-tests")]
#[path = "number_input_appearance_test.rs"]
mod appearance_test;
#[path = "number_input_policy_test.rs"]
mod policy_test;
#[path = "number_input_repeat_test.rs"]
mod repeat_test;
#[path = "number_input_workload_test.rs"]
mod workload_test;

async fn frame(cx: &mut AsyncApp, handle: WindowHandle<View>) {
    for attempt in 0..6 {
        let timeout = cx
            .background_executor()
            .timer(std::time::Duration::from_millis(500));
        let painted = futures_lite::future::or(
            async {
                editor_frame(cx, handle).await;
                true
            },
            async {
                timeout.await;
                false
            },
        )
        .await;
        if painted {
            return;
        }
        let active = handle
            .update(cx, |_, window, cx| {
                window.refresh();
                cx.notify();
                window.is_window_active()
            })
            .unwrap();
        eprintln!("NUMBER_INPUT_FRAME_RETRY attempt={attempt} active={active}");
    }
    panic!("numeric native window did not acknowledge painted frames within 3 seconds");
}

fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn config() -> n::Config {
    n::Config {
        domain: Domain::new(-2., 8., 0.5).unwrap(),
        label: "Temperature".into(),
        placeholder: "A number".into(),
        increment_label: "Increase temperature".into(),
        decrement_label: "Decrease temperature".into(),
        step_controls: n::StepControls::Sides,
        allow_empty: false,
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
            v.list_actions(&applied.lists, w, cx);
            cx.notify();
        })
        .unwrap();
}
fn command(cx: &mut AsyncApp, handle: WindowHandle<View>, command: n::Command) -> n::Response {
    handle
        .update(cx, |v, w, cx| v.numbers[&node(1)].command(&command, w, cx))
        .unwrap()
}
fn snapshot(cx: &mut AsyncApp, handle: WindowHandle<View>) -> n::Snapshot {
    let n::Response::Applied(s) = command(cx, handle, n::Command::ReadSnapshot) else {
        panic!("numeric owner failed read");
    };
    s
}
fn replace(cx: &mut AsyncApp, handle: WindowHandle<View>, text: &str) -> n::Snapshot {
    let n::Response::Applied(s) = command(
        cx,
        handle,
        n::Command::ReplaceDraft {
            text: text.into(),
            selection: n::SelectionPolicy::End,
            undo: n::UndoPolicy::Record,
            if_revision: None,
        },
    ) else {
        panic!("native replacement failed");
    };
    s
}
fn events(transport: &Transport) -> Vec<n::Event> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::NumberInputEvent(_, _, _, _, event) => Some(event),
            Event::EditorEvent(..) | Event::Press(..) => {
                panic!("numeric input leaked generic/editor events")
            }
            _ => None,
        })
        .collect()
}

async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(0),
                Kind::VirtualList,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetListConfig(
                node(0),
                gpuio_protocol::list::Config {
                    estimated_height: 40.,
                    overscan: 0.,
                    max_active: 4,
                    scroll_policy: gpuio_protocol::list::ScrollPolicy::KeepPosition,
                    scrollbar: true,
                    managed: true,
                },
            ),
            Op::SetListOrder(
                node(0),
                gpuio_protocol::list::Order {
                    revision: 1,
                    runs: vec![gpuio_protocol::list::IdRun { first: 1, count: 1 }],
                },
            ),
            Op::SetStyle(
                node(0),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(320.)),
                    Field::Height(Length::Px(120.)),
                ])],
            ),
            Op::Create(
                node(1),
                Kind::NumberInput,
                "".into(),
                Some(HandlerId::from_parts(1, 1).unwrap()),
            ),
            Op::SetNumberInput(node(1), config(), n::Value::Number(1.5)),
            Op::SetStyle(
                node(1),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(300.)),
                    Field::Height(Length::Px(40.)),
                ])],
            ),
            Op::SetListRows(
                node(0),
                vec![gpuio_protocol::list::Row {
                    id: 1,
                    node: node(1),
                }],
            ),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    frame(cx, handle).await;
    assert!(snapshot(cx, handle).focused, "autofocus initial state");
    assert!(
        events(transport)
            .iter()
            .any(|e| matches!(e, n::Event::Observed(s) if s.focused && s.draft == "1.5"))
    );
    assert!(
        handle.update(cx, |v, _, _| v.editors.is_empty()).unwrap(),
        "one native numeric owner only"
    );
    handle
        .update(cx, |v, w, cx| {
            let pins = v.list_pins(w, cx);
            assert_eq!(
                pins,
                vec![gpuio_protocol::list::Retained {
                    node: node(0),
                    rows: vec![1]
                }]
            );
            let base = v.session.borrow().tree(v.id).unwrap().revision();
            let result = v.session.borrow_mut().apply_guarded(
                &Transaction {
                    window: v.id,
                    base,
                    revision: base + 1,
                    operations: vec![
                        Op::Remove(node(1)),
                        Op::Splice(node(0), 0, 1, vec![]),
                        Op::SetListRows(node(0), vec![]),
                    ],
                },
                &pins,
            );
            assert!(
                matches!(result, Err(crate::tree::ApplyFailure::Retained(_))),
                "focused numeric editor must pin its virtual row"
            );
            assert_eq!(v.session.borrow().tree(v.id).unwrap().revision(), base);
        })
        .unwrap();

    replace(cx, handle, "-");
    key(cx, handle, "enter");
    let s = snapshot(cx, handle);
    assert_eq!(s.draft, "-");
    assert_eq!(s.committed, n::Value::Number(1.5));
    assert!(
        events(transport)
            .iter()
            .any(|e| matches!(e, n::Event::Rejected(n::Rejection::Incomplete, _)))
    );
    key(cx, handle, "escape");
    assert_eq!(snapshot(cx, handle).draft, "1.5");
    replace(cx, handle, "99");
    key(cx, handle, "enter");
    assert_eq!(snapshot(cx, handle).draft, "8");
    key(cx, handle, "down");
    assert_eq!(snapshot(cx, handle).committed, n::Value::Number(7.5));
    key(cx, handle, "up");
    assert_eq!(snapshot(cx, handle).committed, n::Value::Number(8.));
    let semantic = events(transport);
    assert!(
        semantic
            .iter()
            .filter(|e| matches!(e, n::Event::Committed(n::Source::Keyboard, _)))
            .count()
            >= 3
    );

    // Native undo changes draft/history only; committed application value stays put.
    replace(cx, handle, "2.25");
    assert!(matches!(
        command(cx, handle, n::Command::Undo),
        n::Response::Applied(_)
    ));
    assert_eq!(snapshot(cx, handle).draft, "8");
    assert!(matches!(
        command(cx, handle, n::Command::Redo),
        n::Response::Applied(_)
    ));
    assert_eq!(snapshot(cx, handle).draft, "2.25");
    assert_eq!(snapshot(cx, handle).committed, n::Value::Number(8.));
    let before = snapshot(cx, handle);
    native_text(cx, handle, "0", false);
    assert_eq!(snapshot(cx, handle).draft, "2.250");
    assert_eq!(
        command(
            cx,
            handle,
            n::Command::ReplaceDraft {
                text: "1".into(),
                selection: n::SelectionPolicy::End,
                undo: n::UndoPolicy::Record,
                if_revision: Some(before.revision),
            }
        ),
        n::Response::Failed(n::Error::StaleRevision)
    );
    let primary = if cfg!(target_os = "macos") {
        "cmd"
    } else {
        "ctrl"
    };
    key(cx, handle, &format!("{primary}-a"));
    key(cx, handle, &format!("{primary}-c"));
    assert_eq!(
        cx.update(|cx| cx.read_from_clipboard().unwrap().text()),
        Some("2.250".into())
    );
    cx.update(|cx| cx.write_to_clipboard(ClipboardItem::new_string("1e-".into())));
    key(cx, handle, &format!("{primary}-v"));
    assert_eq!(snapshot(cx, handle).draft, "1e-");

    // Retained config updates preserve the native editing session and selection.
    let prior = snapshot(cx, handle);
    apply(
        cx,
        handle,
        vec![Op::SetNumberInput(
            node(1),
            n::Config {
                domain: Domain::new(0., 4., 1.).unwrap(),
                step_controls: n::StepControls::Stacked,
                ..config()
            },
            n::Value::Empty,
        )],
    );
    let after = snapshot(cx, handle);
    assert_eq!(after.draft, prior.draft);
    assert_eq!(after.selection, prior.selection);
    assert_eq!(after.committed, n::Value::Number(4.));
    assert!(
        events(transport)
            .iter()
            .any(|e| matches!(e, n::Event::Observed(s) if s.domain.max() == 4.))
    );
    frame(cx, handle).await;

    #[cfg(target_os = "macos")]
    {
        replace(cx, handle, "");
        frame(cx, handle).await;
        native_text(cx, handle, "に", true);
        let composing = snapshot(cx, handle);
        assert!(composing.composition.is_some());
        assert_eq!(
            command(cx, handle, n::Command::Commit),
            n::Response::Failed(n::Error::Composing)
        );
        assert_eq!(
            command(cx, handle, n::Command::Cancel),
            n::Response::Failed(n::Error::Composing)
        );
        key(cx, handle, "enter");
        assert_eq!(snapshot(cx, handle).draft, composing.draft);
        key(cx, handle, "escape");
        assert!(snapshot(cx, handle).composition.is_none());
        assert_eq!(snapshot(cx, handle).draft, composing.draft);
        key(cx, handle, "escape");
        assert_eq!(snapshot(cx, handle).draft, "4");
        events(transport);
    }
    apply(
        cx,
        handle,
        vec![Op::SetNumberInput(
            node(1),
            n::Config {
                read_only: true,
                ..config()
            },
            n::Value::Empty,
        )],
    );
    frame(cx, handle).await;
    let prior = snapshot(cx, handle);
    key(cx, handle, "up");
    assert_eq!(snapshot(cx, handle).committed, prior.committed);
    assert_eq!(
        command(cx, handle, n::Command::Step(Direction::Increase)),
        n::Response::Failed(n::Error::ReadOnly)
    );
    assert_eq!(
        replace(cx, handle, "3").draft,
        "3",
        "explicit replacement allowed read-only"
    );
    events(transport);
    #[cfg(target_os = "macos")]
    accessibility_test::exercise(cx, handle, transport).await;
    #[cfg(feature = "native-image-tests")]
    appearance_test::exercise(cx, handle, transport).await;
    policy_test::exercise(cx, handle, transport).await;
    repeat_test::exercise(cx, handle, transport).await;
    let (weak, input) = handle
        .update(cx, |v, _, _| {
            let instance = &v.numbers[&node(1)];
            (Rc::downgrade(&instance.owner), instance.state.downgrade())
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetRoot(None), Op::Remove(node(1)), Op::Remove(node(0))],
    );
    frame(cx, handle).await;
    assert!(
        handle
            .update(cx, |_, w, _| w.captured_hitbox().is_none())
            .unwrap()
    );
    assert!(weak.upgrade().is_none());
    assert!(input.upgrade().is_none());
    assert!(
        handle
            .update(cx, |v, _, _| v.numbers.is_empty()
                && v.session.borrow().retained_bytes() == 0)
            .unwrap()
    );
    workload_test::exercise(cx, handle, transport).await;
    eprintln!(
        "GPUIO_NUMBER_INPUT_NATIVE_OK: one editor, draft/commit/cancel, native keyboard/clipboard/history, stale guards, configuration, IME and disposal"
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
    let id = WindowId::from_parts(0, 1).unwrap();
    gpui_platform::application().run(move |cx| {
        gpui_base::init(cx);
        cx.set_quit_mode(QuitMode::Explicit);
        let clipboard = cx.read_from_clipboard();
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, id, "GPUIO numeric input test", 420., 220.)
            .unwrap();
        let handle = cx
            .open_window(
                WindowOptions {
                    inactive_frame_interval: None,
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(420.), px(220.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(id, session.clone(), transport.clone())),
            )
            .unwrap();
        cx.activate(true);
        cx.spawn(async move |cx| {
            let result = super::super::native_test::protect(exercise(cx, handle, &transport)).await;
            *task_failure.borrow_mut() = result.err();
            // Close the owned window before stopping AppKit. A titlebar zoom
            // menu can otherwise keep its nested tracking loop alive after all
            // component assertions have completed.
            let _ = handle.update(cx, |_, window, _| window.remove_window());
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
