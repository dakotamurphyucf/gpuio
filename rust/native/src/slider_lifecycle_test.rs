//! Real two-window activation, independent ownership and close during capture.
use super::*;

async fn activation(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, active: bool) {
    for _ in 0..100 {
        if window.update(cx, |_, w, _| w.is_window_active()).unwrap() == active {
            return;
        }
        cx.background_executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
    }
    panic!("native window activation did not become {active}");
}

async fn capture_loss(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    use gpuio_protocol::v1::Field as F;
    for ancestor_policy in [false, true] {
        let point = position(cx, handle, 0.2);
        mouse(cx, handle, point, true);
        assert!(snapshot(cx, handle).dragging.is_some());
        if ancestor_policy {
            apply(
                cx,
                handle,
                vec![Op::SetStyle(
                    node(0),
                    vec![Style::Fields(vec![F::PointerEvents(false)])],
                )],
            );
        } else {
            handle
                .update(cx, |_, w, _| {
                    w.release_pointer();
                    w.refresh();
                })
                .unwrap();
        }
        frame(cx, handle).await;
        assert!(snapshot(cx, handle).dragging.is_none());
        assert_eq!(snapshot(cx, handle).value, initial());
        assert!(
            events(transport)
                .iter()
                .any(|event| matches!(event, s::Event::Cancelled(s::CancelReason::Interrupted, _)))
        );
        mouse(cx, handle, point, false);
        assert_eq!(snapshot(cx, handle).value, initial());
        apply(cx, handle, vec![Op::SetStyle(node(0), vec![])]);
        frame(cx, handle).await;
    }
}

#[cfg(target_os = "macos")]
async fn minimized_idle(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    use objc2::{
        msg_send,
        runtime::{AnyObject, Bool},
    };
    let view = super::super::super::editor_test::native_view(cx, handle) as *mut AnyObject;
    let native: *mut AnyObject = unsafe { msg_send![view, window] };
    let point = position(cx, handle, 0.2);
    mouse(cx, handle, point, true);
    assert!(snapshot(cx, handle).dragging.is_some());
    handle.update(cx, |_, w, _| w.minimize_window()).unwrap();
    let mut minimized = false;
    for _ in 0..100 {
        let value: Bool = unsafe { msg_send![native, isMiniaturized] };
        if value.as_bool() {
            minimized = true;
            break;
        }
        cx.background_executor()
            .timer(std::time::Duration::from_millis(10))
            .await;
    }
    assert!(minimized);
    activation(cx, handle, false).await;
    assert!(snapshot(cx, handle).dragging.is_none());
    assert_eq!(snapshot(cx, handle).value, initial());
    cx.background_executor()
        .timer(std::time::Duration::from_millis(150))
        .await;
    let count = handle.update(cx, |v, _, _| v.render_count).unwrap();
    cx.background_executor()
        .timer(std::time::Duration::from_millis(150))
        .await;
    assert_eq!(handle.update(cx, |v, _, _| v.render_count).unwrap(), count);
    unsafe {
        let _: () = msg_send![native, deminiaturize:std::ptr::null_mut::<AnyObject>()];
    }
    handle.update(cx, |_, w, _| w.activate_window()).unwrap();
    activation(cx, handle, true).await;
    frame(cx, handle).await;
    mouse(cx, handle, point, false);
    assert_eq!(snapshot(cx, handle).value, initial());
    assert!(events(transport).iter().any(|event| matches!(
        event,
        s::Event::Cancelled(s::CancelReason::WindowInactive, _)
    )));
    eprintln!(
        "GPUIO_SLIDER_MINIMIZED_OK: held drag cancelled, no settled hidden frames, restore and late-release suppression"
    );
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    first: WindowHandle<View>,
    transport: &Transport,
) {
    // Activation is queued by the platform. Do not enqueue a redundant request
    // that could run after the second window opens and steal activation back.
    if !first.update(cx, |_, w, _| w.is_window_active()).unwrap() {
        first.update(cx, |_, w, _| w.activate_window()).unwrap();
        activation(cx, first, true).await;
    }
    frame(cx, first).await;
    capture_loss(cx, first, transport).await;
    #[cfg(target_os = "macos")]
    minimized_idle(cx, first, transport).await;
    let point = position(cx, first, 0.2);
    mouse(cx, first, point, true);
    assert!(snapshot(cx, first).dragging.is_some());
    let (session, shared_transport) = first
        .update(cx, |v, _, _| (v.session.clone(), v.transport.clone()))
        .unwrap();
    let other_id = WindowId::from_parts(1, 1).unwrap();
    session
        .borrow_mut()
        .open(2, other_id, "Slider independent owner", 240., 120.)
        .unwrap();
    let second = cx.update(|cx| {
        cx.open_window(
            WindowOptions {
                focus: true,
                show: true,
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(240.), px(120.)),
                    cx,
                ))),
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(other_id, session.clone(), shared_transport.clone())),
        )
        .unwrap()
    });
    activation(cx, first, false).await;
    activation(cx, second, true).await;
    assert_eq!(snapshot(cx, first).value, initial());
    assert!(snapshot(cx, first).dragging.is_none());
    assert!(
        first
            .update(cx, |_, w, _| w.captured_hitbox().is_none())
            .unwrap()
    );
    assert!(events(transport).iter().any(|event| matches!(
        event,
        s::Event::Cancelled(s::CancelReason::WindowInactive, _)
    )));
    // Reuse node/handler slots in another window; routing must remain separate.
    apply(
        cx,
        second,
        vec![
            Op::Create(node(0), Kind::Container, "".into(), None),
            Op::Create(node(1), Kind::Slider, "".into(), Some(handler(1))),
            Op::SetSlider(node(1), config(), initial()),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    frame(cx, second).await;
    let weak = second
        .update(cx, |v, w, cx| {
            let owner = &v.sliders[&node(1)];
            w.focus(&owner.borrow().focus[0].1, cx);
            Rc::downgrade(owner)
        })
        .unwrap();
    frame(cx, second).await;
    key(cx, second, "right");
    assert_eq!(
        snapshot(cx, second).value,
        s::Value::Range {
            lower: 2.5,
            upper: 7.
        }
    );
    assert_eq!(snapshot(cx, first).value, initial());
    mouse(cx, first, point, false);
    assert_eq!(snapshot(cx, first).value, initial());
    let other_point = position(cx, second, 0.7);
    mouse(cx, second, other_point, true);
    assert!(snapshot(cx, second).dragging.is_some());
    second
        .update(cx, |v, w, _| {
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
        })
        .unwrap();
    assert!(second.update(cx, |_, _, _| ()).is_err());
    first.update(cx, |_, w, _| w.activate_window()).unwrap();
    activation(cx, first, true).await;
    frame(cx, first).await;
    assert!(
        weak.upgrade().is_none(),
        "closed slider state retained by native callbacks"
    );
    let point = position(cx, first, 0.4);
    mouse(cx, first, point, true);
    mouse(cx, first, point, false);
    assert_eq!(snapshot(cx, first).value, initial());
    events(transport);
    first
        .update(cx, |v, w, cx| w.focus(v.root_focus.as_ref().unwrap(), cx))
        .unwrap();
    frame(cx, first).await;
    eprintln!(
        "GPUIO_SLIDER_WINDOWS_OK: deactivation cancellation, independent same-slot owners, close during drag, callback disposal and surviving-window input"
    );
}
