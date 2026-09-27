//! Actual activation loss and independent same-node identities across windows.
use super::*;
use std::time::Duration;

async fn active(cx: &mut AsyncApp, handle: WindowHandle<View>, expected: bool) {
    for _ in 0..100 {
        if handle
            .update(cx, |_, w, _| w.is_window_active() == expected)
            .unwrap()
        {
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(20))
            .await;
    }
    panic!("native window activation did not become {expected}");
}

pub(super) async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    handle.update(cx, |_, w, _| w.activate_window()).unwrap();
    active(cx, handle, true).await;
    assert!(matches!(
        command(cx, handle, o::Command::Focus),
        o::Response::Applied(_)
    ));
    frame(cx, handle).await;
    key(cx, handle, SELECT_ALL);
    #[cfg(target_os = "macos")]
    {
        native_text(cx, handle, "１２", true);
        assert!(snapshot(cx, handle).composition.is_some());
    }
    let original = snapshot(cx, handle).value;
    let (session, transport_clone) = handle
        .update(cx, |v, _, _| (v.session.clone(), v.transport.clone()))
        .unwrap();
    let other_id = WindowId::from_parts(1, 1).unwrap();
    session
        .borrow_mut()
        .open(2, other_id, "Independent verification code", 300., 120.)
        .unwrap();
    let other = cx.update(|cx| {
        cx.open_window(
            WindowOptions {
                inactive_frame_interval: None,
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(300.), px(120.)),
                    cx,
                ))),
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(other_id, session.clone(), transport_clone.clone())),
        )
        .unwrap()
    });
    active(cx, handle, false).await;
    assert!(snapshot(cx, handle).composition.is_none());
    assert_eq!(snapshot(cx, handle).value, original);
    apply(
        cx,
        other,
        vec![
            Op::Create(node(), Kind::OtpInput, "".into(), Some(handler())),
            Op::SetOtpInput(node(), config(), "56".into()),
            Op::SetRoot(Some(node())),
        ],
    );
    frame(cx, other).await;
    events(transport);
    native_text(cx, other, "７８９０", false);
    assert_eq!(snapshot(cx, other).value, "567890");
    assert_eq!(snapshot(cx, handle).value, original);
    let output = transport.mailbox.lock().unwrap().drain(128);
    assert!(matches!(output.as_slice(),[
        Event::OtpInputEvent(w1,_,_,_,o::Event::Changed(_)),
        Event::OtpInputEvent(w2,_,_,_,o::Event::Complete(_))
    ] if *w1==other_id && *w2==other_id));
    frame(cx, other).await;
    let position = other
        .update(cx, |v, _, cx| {
            v.otps[&node()]
                .state
                .read(cx)
                .layout
                .as_ref()
                .unwrap()
                .bounds
                .center()
        })
        .unwrap();
    super::super::super::native_test::move_mouse(cx, other, position, false);
    super::super::super::native_test::mouse(cx, other, position, true);
    #[cfg(target_os = "macos")]
    native_text(cx, other, "９", true);
    let weak = other
        .update(cx, |v, w, _| {
            assert!(w.captured_hitbox().is_some());
            v.otps[&node()].state.downgrade()
        })
        .unwrap();
    other
        .update(cx, |v, w, _| {
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
        })
        .unwrap();
    handle.update(cx, |_, w, _| w.activate_window()).unwrap();
    active(cx, handle, true).await;
    frame(cx, handle).await;
    assert!(weak.upgrade().is_none());
    assert!(session.borrow().tree(other_id).is_none());
    assert_eq!(snapshot(cx, handle).value, original);
    events(transport);
    eprintln!(
        "GPUIO_OTP_WINDOWS_OK: actual deactivation cancels preedit, independent equal-node routes, close during capture/composition, owner disposal and surviving window state"
    );
}
