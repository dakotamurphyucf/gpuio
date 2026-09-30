use super::*;
use gpuio_extension_sdk as sdk;
use gpuio_protocol::extension::{Config, Payload, Schema, Signal};
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

static MOUNTS: AtomicUsize = AtomicUsize::new(0);
static UPDATES: AtomicUsize = AtomicUsize::new(0);
static COMMANDS: AtomicUsize = AtomicUsize::new(0);
static UNMOUNTS: AtomicUsize = AtomicUsize::new(0);
static SINK: Mutex<Option<sdk::EventSink>> = Mutex::new(None);
struct Factory;
struct Component {
    value: u8,
}
impl sdk::Factory for Factory {
    fn descriptor(&self) -> sdk::Descriptor {
        sdk::Descriptor {
            name: "test.counter",
            version: 1,
            fingerprint: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            sdk_version: sdk::SDK_VERSION,
            gpui_revision: sdk::GPUI_REVISION,
            max_properties: 1,
            max_command: 1,
            max_event: 1,
        }
    }
    fn validate_properties(&self, bytes: &[u8]) -> Result<(), sdk::Error> {
        if bytes.len() == 1 {
            Ok(())
        } else {
            Err(sdk::Error::InvalidProperties)
        }
    }
    fn validate_command(&self, bytes: &[u8]) -> Result<(), sdk::Error> {
        self.validate_properties(bytes)
    }
    fn mount(
        &self,
        bytes: &[u8],
        cx: &mut sdk::Context<'_>,
    ) -> Result<Box<dyn sdk::Component>, sdk::Error> {
        MOUNTS.fetch_add(1, Ordering::SeqCst);
        *SINK.lock().unwrap() = Some(cx.events.clone());
        Ok(Box::new(Component { value: bytes[0] }))
    }
}
impl sdk::Component for Component {
    fn update(&mut self, bytes: &[u8], cx: &mut sdk::Context<'_>) -> Result<(), sdk::Error> {
        UPDATES.fetch_add(1, Ordering::SeqCst);
        self.value = bytes[0];
        *SINK.lock().unwrap() = Some(cx.events.clone());
        Ok(())
    }
    fn command(&mut self, bytes: &[u8], _: &mut sdk::Context<'_>) -> Result<(), sdk::Error> {
        COMMANDS.fetch_add(1, Ordering::SeqCst);
        assert_ne!(bytes[0], 99, "contained extension command panic");
        self.value = bytes[0];
        Ok(())
    }
    fn render(&mut self, cx: &mut sdk::Context<'_>) -> Result<gpui::AnyElement, sdk::Error> {
        let sink = cx.events.clone();
        let press = sink.clone();
        let focus = cx.focus.clone();
        let value = self.value;
        Ok(div()
            .id("counter-button")
            .size_full()
            .track_focus(&focus)
            .role(gpui::Role::Button)
            .aria_label("Extension increment")
            .child(format!("Count {value}"))
            .on_a11y_action(gpui::AccessibleAction::Click, move |_, _, _| {
                let _ = press.guard(|| press.emit(vec![value + 1]));
            })
            .on_key_down(move |event, _, _| {
                if event.keystroke.key == "space" {
                    let _ = sink.guard(|| sink.emit(vec![value + 1]));
                }
            })
            .into_any_element())
    }
    fn unmount(&mut self) {
        UNMOUNTS.fetch_add(1, Ordering::SeqCst);
    }
}
pub(super) fn install() {
    crate::extensions::install([Arc::new(Factory) as Arc<dyn sdk::Factory>]).unwrap();
}
fn config(value: u8) -> Config {
    Config {
        schema: Schema {
            name: "test.counter".into(),
            version: 1,
            fingerprint: "a".repeat(64),
        },
        generation: 1,
        label: "Counter extension".into(),
        disabled: false,
        properties: Payload(vec![value]),
        command: None,
    }
}
fn signals(transport: &Transport) -> Vec<Signal> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::ExtensionEvent(_, _, _, _, _, signal) => Some(signal),
            _ => None,
        })
        .collect()
}
fn sink() -> sdk::EventSink {
    SINK.lock().unwrap().as_ref().unwrap().clone()
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    let mut config = config(7);
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(5),
                Kind::Extension,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(5, 1).unwrap()),
            ),
            Op::SetExtension(node(5), config.clone()),
            Op::SetStyle(
                node(5),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(300.)),
                    Field::Height(Length::Px(45.)),
                ])],
            ),
            Op::Splice(node(0), 4, 0, vec![node(5)]),
        ],
    );
    frame(cx, handle).await;
    assert_eq!(MOUNTS.load(Ordering::SeqCst), 1);
    assert_eq!(signals(transport), vec![Signal::Mounted]);
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.editors[&node(4)].focus_handle(cx), cx)
        })
        .unwrap();
    key(cx, handle, "tab");
    handle
        .update(cx, |view, window, _| {
            assert!(view.extensions[&node(5)].focus.is_focused(window))
        })
        .unwrap();
    key(cx, handle, "space");
    assert_eq!(signals(transport), vec![Signal::Data(Payload(vec![8]))]);
    #[cfg(target_os = "macos")]
    {
        let _ = accessible(cx, handle, "Extension increment", false);
        frame(cx, handle).await;
        let semantics =
            accessible_with_role(cx, handle, "Extension increment", Some("AXButton"), true)
                .expect("native extension accessibility button");
        assert!(semantics.enabled);
        frame(cx, handle).await;
        assert_eq!(signals(transport), vec![Signal::Data(Payload(vec![8]))]);
    }
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(5),
            vec![Style::Fields(vec![Field::PointerEvents(false)])],
        )],
    );
    assert_eq!(
        sink().guard_pointer::<()>(|| panic!("pointer-disabled callback ran")),
        Err(sdk::Error::Hidden)
    );
    assert_eq!(sink().guard(|| Ok(())), Ok(()));
    frame(cx, handle).await;
    key(cx, handle, "space");
    assert_eq!(signals(transport), vec![Signal::Data(Payload(vec![8]))]);
    let parent_style = handle
        .update(cx, |view, _, _| {
            view.session
                .borrow()
                .tree(view.id)
                .unwrap()
                .get(node(0))
                .unwrap()
                .style
                .clone()
        })
        .unwrap();
    let retained_sink = sink();
    let mut disabled_parent = parent_style.to_vec();
    disabled_parent.push(Style::Fields(vec![Field::Disabled(true)]));
    apply(cx, handle, vec![Op::SetStyle(node(0), disabled_parent)]);
    assert_eq!(
        retained_sink.guard::<()>(|| panic!("disabled extension callback ran")),
        Err(sdk::Error::Hidden)
    );
    assert_eq!(retained_sink.emit(vec![8]), Err(sdk::Error::Hidden));
    frame(cx, handle).await;
    handle
        .update(cx, |view, window, _| {
            assert!(!view.extensions[&node(5)].focus.is_focused(window));
        })
        .unwrap();
    key(cx, handle, "space");
    #[cfg(target_os = "macos")]
    {
        let semantics = accessible_request(
            cx,
            handle,
            "Extension increment",
            Some("AXButton"),
            AccessibilityRequest::PressRejected,
        )
        .expect("disabled extension remains accessible");
        assert!(!semantics.enabled);
        frame(cx, handle).await;
    }
    assert!(signals(transport).is_empty());
    apply(
        cx,
        handle,
        vec![Op::SetStyle(node(0), parent_style.to_vec())],
    );
    frame(cx, handle).await;
    assert_eq!(retained_sink.guard(|| Ok(())), Ok(()));
    assert_eq!(MOUNTS.load(Ordering::SeqCst), 1);
    assert_eq!(UNMOUNTS.load(Ordering::SeqCst), 0);
    handle
        .update(cx, |view, window, cx| {
            assert!(!view.extensions[&node(5)].focus.is_focused(window));
            window.focus(&view.editors[&node(4)].focus_handle(cx), cx);
        })
        .unwrap();
    key(cx, handle, "tab");
    key(cx, handle, "space");
    assert_eq!(signals(transport), vec![Signal::Data(Payload(vec![8]))]);
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            node(5),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    assert_eq!(sink().emit(vec![8]), Err(sdk::Error::Hidden));
    apply(cx, handle, vec![Op::SetStyle(node(5), vec![])]);
    let old = sink();
    config.properties = Payload(vec![10]);
    apply(cx, handle, vec![Op::SetExtension(node(5), config.clone())]);
    assert_eq!(old.emit(vec![8]), Err(sdk::Error::Closed));
    assert_eq!(UPDATES.load(Ordering::SeqCst), 1);
    // Both commands execute even without an intervening rendered frame.
    for sequence in 1..=2 {
        config.command = Some(gpuio_protocol::extension::Command {
            sequence,
            payload: Payload(vec![sequence as u8]),
        });
        apply(cx, handle, vec![Op::SetExtension(node(5), config.clone())]);
    }
    frame(cx, handle).await;
    frame(cx, handle).await;
    assert_eq!(COMMANDS.load(Ordering::SeqCst), 2);
    assert_eq!(
        signals(transport),
        vec![Signal::CommandCompleted(1), Signal::CommandCompleted(2)]
    );
    config.disabled = true;
    apply(cx, handle, vec![Op::SetExtension(node(5), config.clone())]);
    assert_eq!(sink().emit(vec![1]), Err(sdk::Error::Hidden));
    config.disabled = false;
    config.generation = 2;
    config.command = None;
    apply(cx, handle, vec![Op::SetExtension(node(5), config.clone())]);
    assert_eq!(MOUNTS.load(Ordering::SeqCst), 2);
    assert_eq!(UNMOUNTS.load(Ordering::SeqCst), 1);
    assert_eq!(signals(transport), vec![Signal::Mounted]);
    config.command = Some(gpuio_protocol::extension::Command {
        sequence: 1,
        payload: Payload(vec![99]),
    });
    apply(cx, handle, vec![Op::SetExtension(node(5), config.clone())]);
    assert_eq!(
        signals(transport),
        vec![Signal::Failed(gpuio_protocol::extension::Error::Panicked)]
    );
    assert_eq!(UNMOUNTS.load(Ordering::SeqCst), 2);
    frame(cx, handle).await;
    assert_eq!(COMMANDS.load(Ordering::SeqCst), 3);
    // Failed instances recover only through an explicit new generation.
    config.generation = 3;
    config.command = None;
    apply(cx, handle, vec![Op::SetExtension(node(5), config)]);
    let late = sink();
    apply(
        cx,
        handle,
        vec![Op::Splice(node(0), 4, 1, vec![]), Op::Remove(node(5))],
    );
    assert_eq!(UNMOUNTS.load(Ordering::SeqCst), 3);
    assert_eq!(late.emit(vec![1]), Err(sdk::Error::Closed));
    assert!(
        handle
            .update(cx, |view, _, _| view.extensions.is_empty())
            .unwrap()
    );
    let baseline = handle
        .update(cx, |view, _, _| {
            view.session
                .borrow()
                .tree(view.id)
                .unwrap()
                .retained_bytes()
        })
        .unwrap();
    for generation in 2..=16 {
        let id = NodeId::from_parts(5, generation).unwrap();
        apply(
            cx,
            handle,
            vec![
                Op::Create(
                    id,
                    Kind::Extension,
                    "".into(),
                    Some(gpuio_protocol::HandlerId::from_parts(5, generation).unwrap()),
                ),
                Op::SetExtension(id, self::config(1)),
                Op::Splice(node(0), 4, 0, vec![id]),
            ],
        );
        let late = sink();
        apply(
            cx,
            handle,
            vec![Op::Splice(node(0), 4, 1, vec![]), Op::Remove(id)],
        );
        assert_eq!(late.emit(vec![1]), Err(sdk::Error::Closed));
        assert_eq!(
            handle
                .update(cx, |view, _, _| view
                    .session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .retained_bytes())
                .unwrap(),
            baseline
        );
    }
    assert_eq!(
        MOUNTS.load(Ordering::SeqCst),
        UNMOUNTS.load(Ordering::SeqCst)
    );
    let id = NodeId::from_parts(5, 17).unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                id,
                Kind::Extension,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(5, 17).unwrap()),
            ),
            Op::SetExtension(id, self::config(1)),
            Op::Splice(node(0), 4, 0, vec![id]),
        ],
    );
    let after_close = sink();
    handle
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    assert_eq!(after_close.emit(vec![1]), Err(sdk::Error::Closed));
    assert_eq!(
        MOUNTS.load(Ordering::SeqCst),
        UNMOUNTS.load(Ordering::SeqCst)
    );
    eprintln!(
        "native extension lifecycle, keyboard focus, async events, commands, reset and panic containment passed"
    );
}
