//! Native command registry routes inside retained inspection content.
use super::*;
use gpuio_protocol::v1::{CommandConfig, CommandSource, CommandTarget};

fn wrapper() -> NodeId {
    NodeId::from_parts(2, 10).unwrap()
}
fn scope() -> NodeId {
    NodeId::from_parts(3, 10).unwrap()
}
fn button() -> NodeId {
    NodeId::from_parts(4, 1).unwrap()
}
fn handler() -> HandlerId {
    HandlerId::from_parts(910, 1).unwrap()
}
fn invocations(transport: &Transport) -> usize {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(1024)
        .into_iter()
        .filter(|event| match event {
            Event::CommandInvoked(_, owner, observer, _, command, generation, origin) => {
                assert_eq!(*owner, scope());
                assert_eq!(*observer, handler());
                assert_eq!(command, "inspect");
                assert_eq!(*generation, 1);
                assert_eq!(*origin, CommandSource::Button(button()));
                true
            }
            Event::Press(_, node, ..) if *node == button() => panic!("command emitted plain Press"),
            _ => false,
        })
        .count()
}
fn preview(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> gpui::Point<gpui::Pixels> {
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.charts[&id(1)].borrow().input.focus, cx)
        })
        .unwrap();
    key(cx, handle, "home");
    key(cx, handle, "enter");
    draw(cx, handle);
    handle
        .update(cx, |view, _, _| {
            assert!(view.focus.borrow().allows(button()));
            view.probes.borrow()[&button()].bounds.center()
        })
        .unwrap()
}
fn click(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, point: gpui::Point<gpui::Pixels>) {
    move_mouse(cx, handle, point, false);
    mouse(cx, handle, point, true);
    mouse(cx, handle, point, false);
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: &SharedSession,
    transport: &Transport,
) {
    let original = session.borrow().chart(source).unwrap().snapshot().unwrap();
    let mut absent = original.data().clone();
    let Contents::Pie(values) = &mut absent.contents else {
        panic!("pie source");
    };
    values.retain(|v| v.id != 7);
    let mut chart = config(source, 0xff0000ff);
    chart.style.inspection.card.width = 160.;
    chart.inspection_content = vec![Entry {
        target: Some(Target::Slice(7)),
        container: Container::Card,
    }];
    apply(
        cx,
        handle,
        vec![
            Op::SetChart(id(1), Box::new(chart.clone())),
            Op::Create(wrapper(), Kind::Container, "".into(), None),
            Op::Create(scope(), Kind::CommandScope, "".into(), Some(handler())),
            Op::SetCommands(
                scope(),
                vec![CommandConfig {
                    id: "inspect".into(),
                    generation: 1,
                    label: "Run inspection command".into(),
                    enabled: true,
                    checked: None,
                    shortcuts: vec![],
                    target: CommandTarget::Callback,
                }],
            ),
            Op::Create(button(), Kind::CommandButton, "".into(), None),
            Op::SetCommandRef(button(), "inspect".into()),
            Op::SetStyle(
                button(),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(120.)),
                    Field::Height(Length::Px(28.)),
                ])],
            ),
            Op::Splice(scope(), 0, 0, vec![button()]),
            Op::Splice(wrapper(), 0, 0, vec![scope()]),
            Op::Splice(id(1), 0, 0, vec![wrapper()]),
        ],
    );
    ready(cx, handle, original.revision(), 0xff0000ff).await;
    for container in [Container::Card, Container::Overlay] {
        chart.inspection_content[0].container = container;
        apply(
            cx,
            handle,
            vec![Op::SetChart(id(1), Box::new(chart.clone()))],
        );
        let point = preview(cx, handle);
        invocations(transport);
        click(cx, handle, point);
        assert_eq!(invocations(transport), 1, "fresh native command button");
        mouse(cx, handle, point, true);
        let revision = cx.update(|cx| publish(session, source, original.data(), cx));
        mouse(cx, handle, point, false);
        assert_eq!(
            invocations(transport),
            1,
            "same target preserves command gesture"
        );
        ready(cx, handle, revision, 0xff0000ff).await;
        for paint in [false, true] {
            let point = preview(cx, handle);
            move_mouse(cx, handle, point, false);
            mouse(cx, handle, point, true);
            let revision = cx.update(|cx| {
                publish(session, source, &absent, cx);
                handle
                    .update(cx, |view, _, _| {
                        assert!(!view.focus.borrow().allows(button()))
                    })
                    .unwrap();
                publish(session, source, original.data(), cx)
            });
            if paint {
                ready(cx, handle, revision, 0xff0000ff).await;
                preview(cx, handle);
            }
            mouse(cx, handle, point, false);
            assert_eq!(
                invocations(transport),
                0,
                "retired gesture; replacement paint={paint}"
            );
            ready(cx, handle, revision, 0xff0000ff).await;
            let point = preview(cx, handle);
            click(cx, handle, point);
            assert_eq!(invocations(transport), 1, "fresh command recovers");
        }
        #[cfg(target_os = "macos")]
        accessibility(
            cx,
            handle,
            source,
            session,
            transport,
            original.data(),
            &absent,
        )
        .await;
    }
    apply(
        cx,
        handle,
        vec![
            Op::SetChart(id(1), Box::new(config(source, 0xff0000ff))),
            Op::Splice(id(1), 0, 1, vec![]),
            Op::Remove(button()),
            Op::Remove(scope()),
            Op::Remove(wrapper()),
        ],
    );
    handle
        .update(cx, |view, _, _| {
            assert!(!view.buttons.contains_key(&button()));
            assert!(
                view.session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .get(scope())
                    .is_none()
            );
        })
        .unwrap();
    eprintln!(
        "GPUIO_INSPECTION_COMMAND_OK: Card/Overlay native pointer commands, exact route identity, same-target preservation, retirement before/after paint, recovery and teardown; AppKit queued action exercised={}",
        cfg!(target_os = "macos")
    );
}

#[cfg(target_os = "macos")]
async fn accessibility(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: &SharedSession,
    transport: &Transport,
    data: &Data,
    absent: &Data,
) {
    use crate::host::chart_view::test::native_ax;
    let old = native_ax::target_named(cx, handle, "Run inspection command").await;
    invocations(transport);
    native_ax::press(&old);
    delivered(cx, transport).await;
    let revision = cx.update(|cx| {
        native_ax::press(&old);
        assert_eq!(invocations(transport), 0, "AX command still queued");
        publish(session, source, absent, cx);
        publish(session, source, data, cx)
    });
    cx.background_executor()
        .timer(Duration::from_millis(100))
        .await;
    assert_eq!(invocations(transport), 0, "queued AX command retired");
    ready(cx, handle, revision, 0xff0000ff).await;
    preview(cx, handle);
    let fresh = native_ax::target_named(cx, handle, "Run inspection command").await;
    native_ax::press(&fresh);
    delivered(cx, transport).await;
    unsafe {
        let _: objc2::runtime::Bool = objc2::msg_send![&*old, accessibilityPerformPress];
    }
    cx.background_executor()
        .timer(Duration::from_millis(100))
        .await;
    assert_eq!(invocations(transport), 0, "retired AX command object");
    native_ax::press(&fresh);
    delivered(cx, transport).await;
}
#[cfg(target_os = "macos")]
async fn delivered(cx: &mut gpui::AsyncApp, transport: &Transport) {
    for _ in 0..100 {
        let count = invocations(transport);
        if count > 0 {
            assert_eq!(count, 1);
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    panic!("native inspection command was not delivered");
}
