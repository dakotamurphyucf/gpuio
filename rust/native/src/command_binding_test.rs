//! Actual-window query routing, child-only changes, coalescing and retirement.
use super::*;
#[path = "command_binding_context_test.rs"]
mod contexts;
#[path = "command_binding_retention_test.rs"]
mod retention;
#[path = "command_binding_windows_test.rs"]
mod windows;
use gpuio_protocol::{HandlerId, command_binding as binding};
use std::{
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    time::Duration,
};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn handler(slot: i64, generation: i64) -> HandlerId {
    HandlerId::from_parts(slot, generation).unwrap()
}
fn id() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn query(context: binding::Context) -> binding::Config {
    binding::Config {
        targets: match context {
            binding::Context::Here => vec![binding::Target::Command("run".into())],
            binding::Context::NativeContext(_) => {
                vec![binding::Target::NativeAction(NativeCommand::Copy)]
            }
            _ => vec![
                binding::Target::Command("run".into()),
                binding::Target::NativeAction(NativeCommand::Copy),
            ],
        },
        context,
    }
}
fn command(enabled: bool) -> CommandConfig {
    CommandConfig {
        id: "run".into(),
        generation: if enabled { 1 } else { 2 },
        label: "Run".into(),
        enabled,
        checked: None,
        target: CommandTarget::Callback,
        shortcuts: vec![Shortcut {
            key: "k".into(),
            modifiers: vec![ShortcutModifier::Primary],
            priority: ShortcutPriority::Override,
            text_input: ShortcutTextInput::ModifiedOnly,
            during_composition: false,
        }],
    }
}
fn apply(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, operations: Vec<Op>) {
    handle
        .update(cx, |view, window, cx| {
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
            view.list_actions(&applied.lists, window, cx);
            cx.notify();
        })
        .unwrap();
}
async fn frame(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    super::editor_test::frame(cx, handle).await;
}
fn observations(transport: &Transport) -> BTreeMap<NodeId, binding::Observation> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| match event {
            Event::CommandBindingObserved(_, node, _, _, observation) => Some((node, observation)),
            _ => None,
        })
        .collect()
}
fn registry(sample: &binding::Observation) -> binding::Disposition {
    let binding::State::Ready(entries) = &sample.state else {
        panic!("expected ready: {sample:?}");
    };
    let binding::Entry::Registry { candidates, .. } = &entries[0] else {
        panic!("missing registry: {sample:?}");
    };
    candidates[0].disposition.clone()
}
async fn exercise(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, transport: &Transport) {
    apply(
        cx,
        window,
        vec![
            Op::Create(node(0), Kind::CommandScope, "".into(), Some(handler(0, 1))),
            Op::SetCommands(node(0), vec![command(true)]),
            Op::SetStyle(
                node(0),
                vec![Style::Direction(1), Style::Gap(12.), Style::Padding(16.)],
            ),
            Op::Create(node(1), Kind::Container, "".into(), Some(handler(1, 1))),
            Op::SetCommandBinding(node(1), Some(query(binding::Context::Here))),
            Op::Create(node(2), Kind::Input, "draft".into(), Some(handler(2, 1))),
            Op::SetEditor(
                node(2),
                EditorConfig {
                    label: "Binding editor".into(),
                    placeholder: "".into(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: false,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::SetStyle(
                node(2),
                vec![
                    Style::Width(Length::Px(280.)),
                    Style::Height(Length::Px(36.)),
                ],
            ),
            Op::Splice(node(1), 0, 0, vec![node(2)]),
            Op::Create(node(3), Kind::Container, "".into(), Some(handler(3, 1))),
            Op::SetCommandBinding(node(3), Some(query(binding::Context::Focused))),
            Op::Create(
                node(4),
                Kind::Button,
                "Focus here".into(),
                Some(handler(4, 1)),
            ),
            Op::Splice(node(3), 0, 0, vec![node(4)]),
            Op::Create(node(5), Kind::Container, "".into(), Some(handler(5, 1))),
            Op::SetCommandBinding(
                node(5),
                Some(query(binding::Context::NativeContext(
                    "Input && bad".into(),
                ))),
            ),
            Op::Create(node(6), Kind::Text, "Native query".into(), None),
            Op::Splice(node(5), 0, 0, vec![node(6)]),
            Op::Splice(node(0), 0, 0, vec![node(1), node(3), node(5)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    frame(cx, window).await;
    let first = observations(transport);
    assert_eq!(first.len(), 3, "mounted first observations: {first:?}");
    assert_eq!(registry(&first[&node(1)]), binding::Disposition::Declared);
    assert_eq!(registry(&first[&node(3)]), binding::Disposition::Override);
    assert_eq!(first[&node(5)].state, binding::State::InvalidContext);
    window
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&node(4)].focus, cx)
        })
        .unwrap();
    frame(cx, window).await;
    let button = observations(transport);
    assert!(
        button.is_empty(),
        "moving between non-editor contexts leaves the same binding result"
    );
    window
        .update(cx, |view, window, cx| {
            window.focus(&view.editors[&node(2)].focus_handle(cx), cx);
        })
        .unwrap();
    frame(cx, window).await;
    let focused = observations(transport);
    let binding::State::Ready(entries) = &focused[&node(3)].state else {
        panic!("focused query");
    };
    assert!(
        matches!(&entries[1], binding::Entry::NativeBinding { strokes, disposition: binding::Disposition::Widget } if !strokes.is_empty())
    );
    let editor_identity = window
        .update(cx, |view, _, cx| view.editors[&node(2)].focus_handle(cx))
        .unwrap();
    // A native editor changes composition while the retained tree revision stays fixed.
    #[cfg(target_os = "macos")]
    {
        let revision = window
            .update(cx, |view, _, _| {
                view.session.borrow().tree(view.id).unwrap().revision()
            })
            .unwrap();
        super::editor_test::native_text(cx, window, "に", true);
        // Wait for child-driven paint, without window.refresh or parent notify.
        cx.background_executor()
            .timer(Duration::from_millis(200))
            .await;
        let changed = observations(transport);
        assert_eq!(
            registry(&changed[&node(3)]),
            binding::Disposition::Unavailable(binding::Suppression::Composition)
        );
        assert!(
            !changed.contains_key(&node(1)),
            "hypothetical Here is unaffected by editor composition"
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
            revision
        );
        super::editor_test::native_text(cx, window, "日本", false);
        cx.background_executor()
            .timer(Duration::from_millis(200))
            .await;
        let changed = observations(transport);
        assert_eq!(registry(&changed[&node(3)]), binding::Disposition::Override);
    }
    apply(
        cx,
        window,
        vec![Op::SetCommands(node(0), vec![command(false)])],
    );
    frame(cx, window).await;
    // Don't drain: the following sample must replace the pending disabled sample.
    apply(
        cx,
        window,
        vec![Op::SetCommands(node(0), vec![command(true)])],
    );
    frame(cx, window).await;
    let changed = observations(transport);
    assert_eq!(changed.len(), 2);
    assert_eq!(registry(&changed[&node(3)]), binding::Disposition::Override);
    apply(
        cx,
        window,
        vec![
            Op::Bind(node(5), Some(handler(5, 2))),
            Op::SetCommandBinding(
                node(5),
                Some(query(binding::Context::NativeContext("Input".into()))),
            ),
        ],
    );
    frame(cx, window).await;
    let changed = observations(transport);
    assert!(
        matches!(&changed[&node(5)].state, binding::State::Ready(entries)
        if matches!(&entries[0], binding::Entry::NativeBinding { disposition: binding::Disposition::Declared, .. }))
    );
    assert_eq!(
        changed[&node(5)].epoch,
        1,
        "configuration replacement resets native epoch"
    );
    let editor_query = query(binding::Context::Editor(id(), node(2)));
    apply(
        cx,
        window,
        vec![
            Op::Bind(node(5), Some(handler(5, 3))),
            Op::SetCommandBinding(node(5), Some(editor_query)),
        ],
    );
    frame(cx, window).await;
    let changed = observations(transport);
    assert_eq!(registry(&changed[&node(5)]), binding::Disposition::Declared);
    assert_eq!(
        window
            .update(cx, |view, _, cx| view.editors[&node(2)].focus_handle(cx))
            .unwrap(),
        editor_identity
    );
    apply(
        cx,
        window,
        vec![Op::SetStyle(
            node(3),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    frame(cx, window).await;
    assert_eq!(
        observations(transport)[&node(3)].state,
        binding::State::Suspended
    );
    apply(cx, window, vec![Op::SetStyle(node(3), vec![])]);
    frame(cx, window).await;
    assert_eq!(
        registry(&observations(transport)[&node(3)]),
        binding::Disposition::Override
    );
    contexts::exercise(cx, window, transport).await;
    retention::exercise(cx, window, transport).await;
    windows::exercise(cx, window, transport).await;
    apply(
        cx,
        window,
        vec![Op::Splice(node(1), 0, 1, vec![]), Op::Remove(node(2))],
    );
    frame(cx, window).await;
    assert_eq!(
        observations(transport)[&node(5)].state,
        binding::State::ContextGone
    );
    frame(cx, window).await;
    assert!(
        observations(transport).is_empty(),
        "unchanged forced repaint emits nothing"
    );
    cx.background_executor()
        .timer(Duration::from_millis(200))
        .await;
    assert!(
        observations(transport).is_empty(),
        "idle query does not schedule itself"
    );
    apply(
        cx,
        window,
        vec![Op::SetCommands(node(0), vec![command(false)])],
    );
    frame(cx, window).await;
    apply(
        cx,
        window,
        vec![
            Op::Bind(node(1), None),
            Op::SetCommandBinding(node(1), None),
            Op::Bind(node(3), None),
            Op::SetCommandBinding(node(3), None),
            Op::Bind(node(5), None),
            Op::SetCommandBinding(node(5), None),
        ],
    );
    assert!(
        observations(transport).is_empty(),
        "retiring owners removes undrained observations"
    );
    window
        .update(cx, |view, window, _| {
            assert!(view.binding_queries.is_empty());
            window.remove_window();
        })
        .unwrap();
    eprintln!(
        "GPUIO_COMMAND_BINDING_NATIVE_OK: mounted registry/widget queries, child-only composition, stable editor, context/config/epoch, coalescing, visibility and disposal"
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
            .open(1, id(), "GPUIO binding query test", 420., 260.)
            .unwrap();
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(420.), px(260.)),
                        cx,
                    ))),
                    focus: true,
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(id(), session.clone(), transport.clone())),
            )
            .unwrap();
        cx.activate(true);
        cx.spawn(async move |cx| {
            let result = super::native_test::protect(exercise(cx, window, &transport)).await;
            *task_failure.borrow_mut() = result.err();
            cx.update(super::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
