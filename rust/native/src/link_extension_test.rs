//! Package-owned nested Tab navigation hands off to host ordering at boundaries.
use super::{content_test::frame, *};
use gpuio_extension_sdk as sdk;
use gpuio_protocol::{
    extension::{Payload, Schema, Signal},
    input,
};

thread_local! {
    static CHILDREN: RefCell<Option<[gpui::FocusHandle; 2]>> = const { RefCell::new(None) };
}
struct Factory;
struct Component {
    children: [gpui::FocusHandle; 2],
}
impl sdk::Factory for Factory {
    fn descriptor(&self) -> sdk::Descriptor {
        sdk::Descriptor {
            name: "test.link_group",
            version: 1,
            fingerprint: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
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
        _: &[u8],
        cx: &mut sdk::Context<'_>,
    ) -> Result<Box<dyn sdk::Component>, sdk::Error> {
        let children = std::array::from_fn(|_| cx.app.focus_handle().tab_stop(true));
        CHILDREN.with(|state| *state.borrow_mut() = Some(children.clone()));
        Ok(Box::new(Component { children }))
    }
}
impl sdk::Component for Component {
    fn update(&mut self, _: &[u8], _: &mut sdk::Context<'_>) -> Result<(), sdk::Error> {
        Ok(())
    }
    fn command(&mut self, _: &[u8], _: &mut sdk::Context<'_>) -> Result<(), sdk::Error> {
        Ok(())
    }
    fn render(&mut self, cx: &mut sdk::Context<'_>) -> Result<gpui::AnyElement, sdk::Error> {
        let primary = cx.focus.clone();
        let children = self.children.clone();
        let sink = cx.events.clone();
        let disabled = sink.check().is_err();
        Ok(div()
            .id("extension-group")
            .size_full()
            .track_focus(&cx.focus)
            .role(gpui::Role::Group)
            .aria_label("Package focus group")
            .children(self.children.iter().enumerate().map(|(index, focus)| {
                div()
                    .id(("extension-child", index))
                    .h(px(24.))
                    .track_focus(focus)
                    .role(gpui::Role::Button)
                    .aria_label(format!("Package child {index}"))
                    .a11y_synthetic_children(move |builder| {
                        if disabled {
                            builder.parent_node().set_disabled();
                        }
                    })
                    .child(format!("Child {index}"))
            }))
            .on_key_down(move |event, window, app| {
                if event.keystroke.key != "tab" {
                    return;
                }
                // The package owns its internal sequence. Boundary events bubble
                // to the host; the host must not replace this policy.
                let target = if event.keystroke.modifiers.shift {
                    if children[1].is_focused(window) {
                        Some(&children[0])
                    } else if children[0].is_focused(window) {
                        Some(&primary)
                    } else {
                        None
                    }
                } else if primary.is_focused(window) {
                    Some(&children[0])
                } else if children[0].is_focused(window) {
                    Some(&children[1])
                } else {
                    None
                };
                if let Some(target) = target {
                    let _ = sink.guard(|| {
                        window.focus(target, app);
                        app.stop_propagation();
                        Ok(())
                    });
                }
            })
            .into_any_element())
    }
    fn unmount(&mut self) {
        CHILDREN.with(|state| state.borrow_mut().take());
    }
}
pub(super) fn install() {
    crate::extensions::install([Arc::new(Factory) as Arc<dyn sdk::Factory>]).unwrap();
}
fn extension_config() -> gpuio_protocol::extension::Config {
    gpuio_protocol::extension::Config {
        schema: Schema {
            name: "test.link_group".into(),
            version: 1,
            fingerprint: "b".repeat(64),
        },
        generation: 1,
        label: "Nested extension".into(),
        disabled: false,
        properties: Payload(vec![0]),
        command: None,
    }
}
#[track_caller]
fn extension_focused(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, child: Option<usize>) {
    assert!(
        handle
            .update(cx, |view, window, _| match child {
                None => view.extensions[&id(32)].focus.is_focused(window),
                Some(index) => CHILDREN
                    .with(|state| state.borrow().as_ref().unwrap()[index].is_focused(window)),
            })
            .unwrap(),
        "expected extension focus {child:?}"
    );
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    let mut ops = vec![
        Op::Create(
            id(31),
            Kind::Link,
            String::new(),
            Some(gpuio_protocol::HandlerId::from_parts(31, 1).unwrap()),
        ),
        Op::Create(
            id(32),
            Kind::Extension,
            String::new(),
            Some(gpuio_protocol::HandlerId::from_parts(32, 1).unwrap()),
        ),
        Op::Create(
            id(33),
            Kind::Link,
            String::new(),
            Some(gpuio_protocol::HandlerId::from_parts(33, 1).unwrap()),
        ),
        Op::Create(id(34), Kind::Text, "Before group".into(), None),
        Op::Create(id(35), Kind::Text, "After group".into(), None),
        Op::Create(id(36), Kind::FocusScope, String::new(), None),
        Op::Create(
            id(37),
            Kind::InputRegion,
            String::new(),
            Some(gpuio_protocol::HandlerId::from_parts(37, 1).unwrap()),
        ),
        Op::SetInputRegion(
            id(37),
            input::Config {
                label: "Outer focus owner".into(),
                disabled: false,
                focus: input::Focus::Click,
                subscriptions: vec![input::Subscription {
                    kind: input::Kind::MouseDown,
                    phase: input::Phase::Bubble,
                    policy: input::Policy::Observe,
                }],
            },
        ),
        Op::SetLink(id(31), config(31, -2)),
        Op::SetLink(id(33), config(33, 2)),
        Op::SetExtension(id(32), extension_config()),
        Op::SetFocusScope(
            id(36),
            FocusScopeConfig {
                trap: true,
                auto_focus: true,
                restore_focus: true,
            },
        ),
        Op::Splice(id(31), 0, 0, vec![id(34)]),
        Op::Splice(id(33), 0, 0, vec![id(35)]),
        Op::Splice(id(37), 0, 0, vec![id(31), id(32), id(33)]),
        Op::Splice(id(36), 0, 0, vec![id(37)]),
        Op::Splice(id(0), 0, 0, vec![id(36)]),
    ];
    for slot in [31, 32, 33, 36, 37] {
        ops.push(Op::SetStyle(
            id(slot),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(300.)),
                Field::Height(Length::Px(if slot >= 36 { 170. } else { 50. })),
                Field::Display(1),
                Field::Direction(1),
                Field::Shrink(0.),
            ])],
        ));
    }
    apply(cx, handle, ops);
    frame(cx, handle).await;
    let revision = handle
        .update(cx, |view, _, _| {
            view.session.borrow().tree(view.id).unwrap().revision()
        })
        .unwrap();
    let window = WindowId::from_parts(0, 1).unwrap();
    assert_eq!(
        transport.mailbox.lock().unwrap().drain(256),
        vec![
            Event::ExtensionEvent(
                window,
                id(32),
                gpuio_protocol::HandlerId::from_parts(32, 1).unwrap(),
                revision,
                1,
                Signal::Mounted
            ),
            Event::Rendered(window, revision),
        ]
    );
    focused(cx, handle, 31);
    key(cx, handle, "tab");
    extension_focused(cx, handle, None);
    key(cx, handle, "tab");
    extension_focused(cx, handle, Some(0));
    key(cx, handle, "tab");
    extension_focused(cx, handle, Some(1));
    key(cx, handle, "tab");
    focused(cx, handle, 33);
    key(cx, handle, "tab");
    focused(cx, handle, 31);
    key(cx, handle, "shift-tab");
    focused(cx, handle, 33);
    key(cx, handle, "shift-tab");
    extension_focused(cx, handle, None);
    key(cx, handle, "shift-tab");
    focused(cx, handle, 31);
    for _ in 0..3 {
        key(cx, handle, "tab");
    }
    extension_focused(cx, handle, Some(1));
    handle
        .update(cx, |view, window, cx| {
            assert_eq!(
                view.focus.borrow().focused_node(window, cx),
                Some(id(32)),
                "nearest extension owner wins over outer input region"
            );
        })
        .unwrap();
    let mut config = extension_config();
    config.properties = Payload(vec![1]);
    apply(cx, handle, vec![Op::SetExtension(id(32), config)]);
    frame(cx, handle).await;
    extension_focused(cx, handle, Some(1));
    key(cx, handle, "shift-tab");
    extension_focused(cx, handle, Some(0));
    key(cx, handle, "shift-tab");
    extension_focused(cx, handle, None);
    key(cx, handle, "shift-tab");
    focused(cx, handle, 31);
    for _ in 0..3 {
        key(cx, handle, "tab");
    }
    extension_focused(cx, handle, Some(1));
    content_test::events(cx, handle, transport, true, None);
    // A nested modal restores the actual package leaf, not only its primary.
    apply(
        cx,
        handle,
        vec![
            Op::Create(id(38), Kind::FocusScope, String::new(), None),
            Op::Create(
                id(39),
                Kind::Button,
                "Modal action".into(),
                Some(gpuio_protocol::HandlerId::from_parts(39, 1).unwrap()),
            ),
            Op::SetFocusScope(
                id(38),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Splice(id(38), 0, 0, vec![id(39)]),
            Op::Splice(id(36), 1, 0, vec![id(38)]),
        ],
    );
    frame(cx, handle).await;
    focused(cx, handle, 39);
    content_test::events(cx, handle, transport, true, None);
    apply(
        cx,
        handle,
        vec![
            Op::Splice(id(36), 1, 1, vec![]),
            Op::Remove(id(39)),
            Op::Remove(id(38)),
        ],
    );
    frame(cx, handle).await;
    extension_focused(cx, handle, Some(1));
    content_test::events(cx, handle, transport, true, None);
    let mut disabled = extension_config();
    disabled.disabled = true;
    apply(cx, handle, vec![Op::SetExtension(id(32), disabled)]);
    frame(cx, handle).await;
    content_test::events(cx, handle, transport, true, None);
    handle
        .update(cx, |view, window, cx| {
            let child = CHILDREN.with(|state| state.borrow().as_ref().unwrap()[1].clone());
            assert!(!child.is_focused(window));
            assert!(
                !view.focus.borrow().can_focus(&child, window),
                "outer eligible owner must not admit disabled child"
            );
            assert_ne!(view.focus.borrow().focused_node(window, cx), Some(id(32)));
        })
        .unwrap();
    focus(cx, handle, 31);
    key(cx, handle, "tab");
    focused(cx, handle, 33);
    apply(
        cx,
        handle,
        vec![Op::SetExtension(id(32), extension_config())],
    );
    frame(cx, handle).await;
    content_test::events(cx, handle, transport, true, None);
    key(cx, handle, "shift-tab");
    extension_focused(cx, handle, None);
    key(cx, handle, "tab");
    extension_focused(cx, handle, Some(0));
    let mut remove = vec![Op::Splice(id(0), 0, 1, vec![])];
    remove.extend((31..=37).map(|slot| Op::Remove(id(slot))));
    apply(cx, handle, remove);
    content_test::idle(cx, handle).await;
    content_test::events(cx, handle, transport, true, None);
    CHILDREN.with(|state| assert!(state.borrow().is_none()));
    eprintln!(
        "GPUIO_LINK_EXTENSION_OK: package Tab boundaries, signed host ordering, reverse/wrap, nearest owner, retained updates, modal leaf restoration, disabled fencing and disposal"
    );
}
