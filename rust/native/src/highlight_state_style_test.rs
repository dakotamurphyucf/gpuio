//! Native state refinements change matching without an OCaml tree transaction.
use super::*;

fn move_pointer(cx: &mut AsyncApp, handle: WindowHandle<View>, x: f32, y: f32) {
    crate::host::native_test::move_mouse(cx, handle, gpui::point(px(x), px(y)), false);
    draw(cx, handle);
}

pub(super) async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    move_pointer(cx, handle, 390., 200.);
    let mut hover = panel(false);
    hover.push(Style::State(2, vec![Field::Visibility(1)]));
    apply(cx, handle, vec![Op::SetStyle(id(17), hover)]);
    draw(cx, handle);
    pause(cx).await;
    let revision = handle
        .update(cx, |view, _, _| {
            view.session.borrow().tree(view.id).unwrap().revision()
        })
        .unwrap();
    let _ = observations(transport);
    move_pointer(cx, handle, 40., 20.);
    count(cx, handle, transport, 0).await;
    assert_eq!(
        red_pixels(cx, handle),
        0,
        "hover-hidden ancestor removes descendant washes"
    );
    let retained = result(cx, handle);
    move_pointer(cx, handle, 50., 20.);
    for _ in 0..4 {
        draw(cx, handle);
        pause(cx).await;
    }
    assert!(Arc::ptr_eq(&retained, &result(cx, handle)));
    assert!(
        observations(transport).is_empty(),
        "unchanged native visibility is silent"
    );
    move_pointer(cx, handle, 390., 200.);
    count(cx, handle, transport, 1).await;
    assert!(red_pixels(cx, handle) > 20);
    assert_eq!(
        handle
            .update(cx, |view, _, _| view
                .session
                .borrow()
                .tree(view.id)
                .unwrap()
                .revision())
            .unwrap(),
        revision
    );

    let mut pressed = panel(false);
    pressed.push(Style::State(3, vec![Field::Visibility(1)]));
    apply(cx, handle, vec![Op::SetStyle(id(17), pressed)]);
    draw(cx, handle);
    move_pointer(cx, handle, 40., 20.);
    crate::host::native_test::mouse(cx, handle, gpui::point(px(40.), px(20.)), true);
    count(cx, handle, transport, 0).await;
    assert_eq!(red_pixels(cx, handle), 0);
    crate::host::native_test::mouse(cx, handle, gpui::point(px(390.), px(200.)), false);
    move_pointer(cx, handle, 390., 200.);
    // The pinned GPUI Div skips its mouse-up listeners while visibility-hidden.
    // Matching must follow that actual hidden state, not invent a release. The
    // independent pressed-state recovery issue remains in the style audit.
    assert_eq!(red_pixels(cx, handle), 0);
    apply(cx, handle, vec![Op::SetStyle(id(17), panel(false))]);
    count(cx, handle, transport, 1).await;

    // Focused selectable text can override a hidden base style. Returning focus
    // to the root restores the hidden count; no editor value joins the projection.
    let mut focused = row(0., true, false);
    focused.push(Style::Fields(vec![Field::Visibility(1)]));
    focused.push(Style::State(1, vec![Field::Visibility(0)]));
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(id(17), panel(false)),
            Op::SetStyle(id(18), focused),
        ],
    );
    count(cx, handle, transport, 0).await;
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.selections[&id(18)].borrow().focus, cx)
        })
        .unwrap();
    count(cx, handle, transport, 1).await;
    assert!(red_pixels(cx, handle) > 20);
    handle
        .update(cx, |view, window, cx| {
            window.focus(view.root_focus.as_ref().unwrap(), cx)
        })
        .unwrap();
    count(cx, handle, transport, 0).await;
    assert_eq!(red_pixels(cx, handle), 0);
    apply(
        cx,
        handle,
        vec![Op::SetStyle(id(18), row(0., false, false))],
    );
    count(cx, handle, transport, 1).await;
    assert!(
        red_pixels(cx, handle) > 20,
        "restyling discards old resolved visibility"
    );
    let mut focused = row(0., true, false);
    focused.push(Style::State(1, vec![Field::Display(3)]));
    apply(cx, handle, vec![Op::SetStyle(id(18), focused)]);
    draw(cx, handle);
    pause(cx).await;
    let _ = observations(transport);
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.selections[&id(18)].borrow().focus, cx)
        })
        .unwrap();
    count(cx, handle, transport, 0).await;
    assert_eq!(
        red_pixels(cx, handle),
        0,
        "display-none text is excluded even without child paint"
    );
    handle
        .update(cx, |view, window, cx| {
            window.focus(view.root_focus.as_ref().unwrap(), cx)
        })
        .unwrap();
    count(cx, handle, transport, 1).await;
    assert!(red_pixels(cx, handle) > 20);
    eprintln!(
        "GPUIO_NATIVE_HIGHLIGHT_STATE_STYLE_OK: native hover/pressed/focus visibility, hidden-base override, ancestor propagation, silent reuse and restyle"
    );
}
