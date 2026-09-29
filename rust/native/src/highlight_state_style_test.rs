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

    for hidden in [Field::Visibility(1), Field::Display(3)] {
        let mut pressed = panel(false);
        pressed.push(Style::State(3, vec![hidden]));
        apply(cx, handle, vec![Op::SetStyle(id(17), pressed)]);
        draw(cx, handle);
        for release_x in [40., 390.] {
            move_pointer(cx, handle, 40., 20.);
            crate::host::native_test::mouse(cx, handle, gpui::point(px(40.), px(20.)), true);
            count(cx, handle, transport, 0).await;
            assert_eq!(red_pixels(cx, handle), 0);
            crate::host::native_test::mouse(cx, handle, gpui::point(px(release_x), px(20.)), false);
            count(cx, handle, transport, 1).await;
            assert!(
                red_pixels(cx, handle) > 20,
                "release restores the native style without restyling"
            );
        }
        // A lost release outside the window is recovered by a later no-button
        // motion, using the platform's actual pressed-button observation.
        move_pointer(cx, handle, 40., 20.);
        crate::host::native_test::mouse(cx, handle, gpui::point(px(40.), px(20.)), true);
        count(cx, handle, transport, 0).await;
        move_pointer(cx, handle, 390., 200.);
        count(cx, handle, transport, 1).await;
    }

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
    button_activation(cx, handle, transport).await;
    eprintln!(
        "GPUIO_NATIVE_HIGHLIGHT_STATE_STYLE_OK: native hover/pressed/focus visibility, hidden-base override, ancestor propagation, silent reuse and restyle"
    );
}

fn button_style(hide: bool) -> Vec<Style> {
    let mut styles = vec![Style::Fields(vec![
        Field::Position(1),
        Field::Left(Length::Px(160.)),
        Field::Top(Length::Px(0.)),
        Field::Width(Length::Px(120.)),
        Field::Height(Length::Px(40.)),
        Field::Background(Fill::Solid(Color::Rgba(0x00aa00ff))),
    ])];
    if hide {
        styles.push(Style::State(3, vec![Field::Visibility(1)]));
    }
    styles
}
fn presses(transport: &Transport) -> Vec<NodeId> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| {
            if let Event::Press(_, node, _, _) = event {
                Some(node)
            } else {
                None
            }
        })
        .collect()
}
async fn button_activation(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(id(21), Kind::Button, "Push".into(), Some(handler(2))),
            Op::SetControl(id(21), Control::Button(false)),
            Op::SetStyle(id(21), button_style(true)),
            Op::Splice(id(17), 1, 0, vec![id(21)]),
        ],
    );
    draw(cx, handle);
    pause(cx).await;
    let _ = presses(transport);
    let position = gpui::point(px(180.), px(20.));
    move_pointer(cx, handle, 180., 20.);
    crate::host::native_test::mouse(cx, handle, position, true);
    draw(cx, handle);
    pause(cx).await;
    assert_eq!(pixel(cx, handle, 170., 5.), [255, 255, 255, 255]);
    crate::host::native_test::mouse(cx, handle, position, false);
    draw(cx, handle);
    pause(cx).await;
    assert_eq!(pixel(cx, handle, 170., 5.), [0, 170, 0, 255]);
    assert!(
        presses(transport).is_empty(),
        "hidden press cannot activate"
    );
    crate::host::native_test::mouse(cx, handle, position, false);
    assert!(
        presses(transport).is_empty(),
        "revealed button cannot replay a stale mouse down"
    );
    apply(cx, handle, vec![Op::SetStyle(id(21), button_style(false))]);
    draw(cx, handle);
    crate::host::native_test::mouse(cx, handle, position, true);
    crate::host::native_test::mouse(cx, handle, position, false);
    assert_eq!(
        presses(transport),
        vec![id(21)],
        "fresh visible click still activates exactly once"
    );
    move_pointer(cx, handle, 390., 200.);
    eprintln!(
        "GPUIO_NATIVE_HIDDEN_PRESS_OK: visibility/display hide, repeated inside/outside release, lost-release motion, stale activation cancellation and fresh click"
    );
}
