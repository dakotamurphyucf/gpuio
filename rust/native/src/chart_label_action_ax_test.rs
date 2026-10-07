//! Real AppKit accessibility activation across a label exposure lifetime.
use super::*;
use objc2::{
    msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool},
};

pub(super) use crate::host::chart_view::test::native_ax::{press, target_named, target_role};

pub(super) async fn target(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
) -> Retained<AnyObject> {
    target_named(cx, handle, "Run radar action").await
}

pub(super) async fn delivered(cx: &mut gpui::AsyncApp, transport: &Transport) {
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
    panic!("native AX activation not delivered");
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: &SharedSession,
    transport: &Transport,
    data: &Data,
    absent: &Data,
) {
    let original = target(cx, handle).await;
    presses(transport);
    press(&original);
    delivered(cx, transport).await;
    let revision = cx.update(|cx| {
        press(&original);
        assert_eq!(
            presses(transport),
            0,
            "AX request must still be queued for this test"
        );
        publish(session, source, absent, cx);
        publish(session, source, data, cx)
    });
    // Let AppKit dispatch the queued request. The positive control above proves
    // this is an actionable native target, not an unsupported synthetic request.
    cx.background_executor()
        .timer(Duration::from_millis(100))
        .await;
    assert_eq!(
        presses(transport),
        0,
        "queued AX action cannot survive label hiding"
    );
    ready(cx, handle, revision, 0xff0000ff).await;
    let fresh = target(cx, handle).await;
    press(&fresh);
    delivered(cx, transport).await;
    unsafe {
        // The retired native object may refuse the action at the platform layer.
        // If it accepts the request, it still must not reach the fresh callback.
        let _: Bool = msg_send![&*original, accessibilityPerformPress];
    }
    cx.background_executor()
        .timer(Duration::from_millis(100))
        .await;
    assert_eq!(
        presses(transport),
        0,
        "old AX object cannot activate the replacement exposure"
    );
    press(&fresh);
    delivered(cx, transport).await;
    eprintln!(
        "GPUIO_RADAR_AX_ACTION_OK: real AppKit press, queued pre-hide action and retired AX object rejected, fresh semantic target recovers"
    );
}
