//! Actual AppKit actions must not survive a retained inspection exposure.
use super::*;
use crate::host::chart_view::test::native_ax;
use objc2::{
    msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool},
};

async fn target(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> Retained<AnyObject> {
    native_ax::target_named(cx, handle, "Inspect action").await
}
async fn delivered(cx: &mut gpui::AsyncApp, transport: &Transport) {
    for _ in 0..100 {
        let count = presses(transport);
        if count > 0 {
            assert_eq!(count, 1);
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    panic!("inspection AppKit action was not delivered");
}
async fn rejected(cx: &mut gpui::AsyncApp, transport: &Transport) {
    cx.background_executor()
        .timer(Duration::from_millis(100))
        .await;
    assert_eq!(presses(transport), 0, "retired inspection action");
}
fn preview(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.charts[&id(1)].borrow().input.focus, cx)
        })
        .unwrap();
    key(cx, handle, "home");
    key(cx, handle, "enter");
    draw(cx, handle);
}
async fn resize(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, width: f32, shown: bool) {
    handle
        .update(cx, |_, window, _| {
            window.resize(gpui::size(px(width), px(240.)))
        })
        .unwrap();
    for _ in 0..100 {
        draw(cx, handle);
        if handle
            .update(cx, |view, window, _| {
                window.viewport_size().width == px(width)
                    && view.focus.borrow().allows(button()) == shown
                    && (shown || !view.buttons[&button()].focus.is_focused(window))
            })
            .unwrap()
        {
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    panic!("inspection clipping/focus did not settle: width={width}, shown={shown}");
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: &SharedSession,
    transport: &Transport,
    config: &Config,
) {
    let original = session.borrow().chart(source).unwrap().snapshot().unwrap();
    let mut absent = original.data().clone();
    let Contents::Pie(values) = &mut absent.contents else {
        panic!("pie fixture");
    };
    values.retain(|v| v.id != 7);
    for container in [Container::Card, Container::Overlay] {
        let mut config = config.clone();
        config.inspection_content[0].container = container;
        apply(cx, handle, vec![Op::SetChart(id(1), Box::new(config))]);
        preview(cx, handle);
        let old = target(cx, handle).await;
        presses(transport);
        native_ax::press(&old);
        delivered(cx, transport).await;
        let revision = cx.update(|cx| {
            native_ax::press(&old);
            assert_eq!(presses(transport), 0, "AppKit action still queued");
            publish(session, source, &absent, cx);
            handle
                .update(cx, |view, _, _| {
                    assert!(
                        !view.focus.borrow().allows(button()),
                        "retirement before paint"
                    );
                })
                .unwrap();
            publish(session, source, original.data(), cx)
        });
        rejected(cx, transport).await;
        ready(cx, handle, revision, 0xff0000ff).await;
        preview(cx, handle);
        let fresh = target(cx, handle).await;
        native_ax::press(&fresh);
        delivered(cx, transport).await;
        unsafe {
            let _: Bool = msg_send![&*old, accessibilityPerformPress];
        }
        rejected(cx, transport).await;
        native_ax::press(&fresh);
        delivered(cx, transport).await;
    }
    // The Overlay remains partly visible while its nested button is clipped.
    // No tree/source update accompanies the native resize.
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(
                wrapper(),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(180.)),
                    Field::Height(Length::Px(28.)),
                ])],
            ),
            Op::SetStyle(
                button(),
                vec![Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(120.)),
                    Field::Top(Length::Px(0.)),
                    Field::Width(Length::Px(40.)),
                    Field::Height(Length::Px(28.)),
                    Field::Background(Fill::Solid(Color::Rgba(0x00ff00ff))),
                ])],
            ),
        ],
    );
    preview(cx, handle);
    let old = target(cx, handle).await;
    presses(transport);
    native_ax::press(&old);
    delivered(cx, transport).await;
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&button()].focus, cx);
            assert!(view.buttons[&button()].focus.is_focused(window));
        })
        .unwrap();
    resize(cx, handle, 80., false).await;
    handle
        .update(cx, |view, _, _| {
            assert!(
                view.focus.borrow().allows(wrapper()),
                "outer wrapper still partly visible"
            );
        })
        .unwrap();
    resize(cx, handle, 240., true).await;
    unsafe {
        let _: Bool = msg_send![&*old, accessibilityPerformPress];
    }
    rejected(cx, transport).await;
    let fresh = target(cx, handle).await;
    native_ax::press(&fresh);
    delivered(cx, transport).await;
    eprintln!(
        "GPUIO_INSPECTION_AX_CLIP_OK: actual AppKit actions in Card/Overlay, prepaint source retirement rejects queued and retained AX actions, nested clipping clears focus and retires AX without a tree update, fresh actions recover"
    );
}
