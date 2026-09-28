//! Native GPUI dispatch through the production retained chart view.
use super::*;
use crate::host::native_test::{mouse, move_mouse};
use gpuio_protocol::chart_selection::Selection;
fn position(x: f32, y: f32) -> gpui::Point<gpui::Pixels> {
    gpui::point(px(x), px(y))
}
fn observations(transport: &Transport) -> Vec<Option<Selection>> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| match event {
            Event::ChartEvent(_, _, _, _, _, _, _, Observation::SelectionChanged(selection)) => {
                Some(selection)
            }
            _ => None,
        })
        .collect()
}
fn key(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, value: &str) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::KeyDown(gpui::KeyDownEvent {
                keystroke: gpui::Keystroke::parse(value).unwrap(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
    })
    .unwrap();
}
pub(super) fn publish_input(
    session: &SharedSession,
    source: ResourceId,
    base: i64,
    generation: i64,
    reverse: bool,
) {
    let mut values = vec![
        Slice {
            id: 7,
            label: "Reasoning".into(),
            value: 1.,
        },
        Slice {
            id: 9,
            label: "Code".into(),
            value: 1.,
        },
    ];
    if reverse {
        values.reverse();
    }
    stage_data(
        session,
        source,
        base,
        generation,
        &Data {
            version: 1,
            contents: Contents::Pie(values),
        },
    );
    let crate::session::ChartDispatch::Publish(work) = session
        .borrow_mut()
        .chart_request(Request::Publish(source, base + 1))
    else {
        panic!("publication")
    };
    assert_eq!(
        session.borrow_mut().complete_chart(work.run()),
        Response::Ack
    );
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: SharedSession,
    transport: Arc<Transport>,
) {
    apply(cx, handle, mount(source));
    ready(cx, handle, 1, 0xff0000ff).await;
    for _ in 0..100 {
        if handle
            .update(cx, |_, window, _| window.is_window_active())
            .unwrap()
        {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    assert!(
        handle
            .update(cx, |_, window, _| window.is_window_active())
            .unwrap()
    );
    observations(&transport);
    move_mouse(cx, handle, position(150., 100.), false);
    draw(cx, handle);
    assert!(
        handle
            .update(cx, |view, _, _| view.charts[&id(1)]
                .borrow()
                .input_overlay()
                .is_some())
            .unwrap()
    );
    assert!(observations(&transport).is_empty());
    mouse(cx, handle, position(150., 100.), true);
    assert!(
        handle
            .update(cx, |view, window, _| view.charts[&id(1)]
                .borrow()
                .chart_focused(window)
                && window.captured_hitbox().is_some())
            .unwrap()
    );
    move_mouse(cx, handle, position(50., 100.), true);
    draw(cx, handle);
    assert!(
        observations(&transport).is_empty(),
        "hover and drag are native-only previews"
    );
    mouse(cx, handle, position(50., 100.), false);
    draw(cx, handle);
    assert_eq!(observations(&transport), vec![Some(Selection::Slice(9))]);
    handle
        .update(cx, |view, window, _| {
            assert_eq!(
                view.charts[&id(1)].borrow().input.selected,
                Some(Selection::Slice(9))
            );
            assert!(window.captured_hitbox().is_none());
            let image = window.render_to_image().unwrap();
            let scale = window.scale_factor();
            assert_ne!(
                image
                    .get_pixel((50. * scale) as u32, (100. * scale) as u32)
                    .0,
                [255, 0, 0, 255],
                "selection marker is actually painted"
            );
        })
        .unwrap();
    // Outside release and Escape cancel preview without changing the selection.
    move_mouse(cx, handle, position(150., 100.), false);
    mouse(cx, handle, position(150., 100.), true);
    move_mouse(cx, handle, position(-10., 100.), true);
    mouse(cx, handle, position(-10., 100.), false);
    assert!(observations(&transport).is_empty());
    move_mouse(cx, handle, position(150., 100.), false);
    mouse(cx, handle, position(150., 100.), true);
    key(cx, handle, "escape");
    mouse(cx, handle, position(150., 100.), false);
    assert!(observations(&transport).is_empty());
    key(cx, handle, "escape");
    assert_eq!(observations(&transport), vec![None]);
    key(cx, handle, "home");
    assert!(observations(&transport).is_empty());
    key(cx, handle, "enter");
    assert_eq!(observations(&transport), vec![Some(Selection::Slice(7))]);
    key(cx, handle, "end");
    key(cx, handle, "space");
    assert_eq!(observations(&transport), vec![Some(Selection::Slice(9))]);
    // Publication releases capture immediately; identity survives a reorder.
    move_mouse(cx, handle, position(150., 100.), false);
    mouse(cx, handle, position(150., 100.), true);
    publish_input(&session, source, 1, 1, true);
    cx.update(|cx| crate::host::chart_source_changed(Some(source), cx));
    assert!(
        handle
            .update(cx, |_, window, _| window.captured_hitbox().is_none())
            .unwrap()
    );
    mouse(cx, handle, position(150., 100.), false);
    assert!(observations(&transport).is_empty());
    ready(cx, handle, 2, 0xff0000ff).await;
    handle
        .update(cx, |view, _, _| {
            let state = view.charts[&id(1)].borrow();
            assert_eq!(state.input.selected, Some(Selection::Slice(9)));
            assert_eq!(state.input.selected_index, Some(0));
        })
        .unwrap();
    assert!(
        observations(&transport).is_empty(),
        "publication reconciliation is silent"
    );
    // Disable/re-enable without repaint cannot revive retained callback tokens.
    move_mouse(cx, handle, position(150., 100.), false);
    mouse(cx, handle, position(150., 100.), true);
    apply(
        cx,
        handle,
        vec![Op::SetChart(
            id(1),
            Config {
                disabled: true,
                ..config(source, 0xff0000ff)
            },
        )],
    );
    apply(
        cx,
        handle,
        vec![Op::SetChart(id(1), config(source, 0xff0000ff))],
    );
    mouse(cx, handle, position(150., 100.), false);
    assert!(observations(&transport).is_empty());
    draw(cx, handle);
    move_mouse(cx, handle, position(150., 100.), false);
    mouse(cx, handle, position(150., 100.), true);
    handle
        .update(cx, |view, window, cx| {
            window.focus(view.root_focus.as_ref().unwrap(), cx)
        })
        .unwrap();
    draw(cx, handle);
    assert!(
        handle
            .update(cx, |_, window, _| window.captured_hitbox().is_none())
            .unwrap()
    );
    mouse(cx, handle, position(150., 100.), false);
    assert!(observations(&transport).is_empty());
    // A reset clears local selection; a release cancels an active gesture and
    // drops readers without waiting for another frame or pointer event.
    publish_input(&session, source, 2, 2, false);
    cx.update(|cx| crate::host::chart_source_changed(Some(source), cx));
    assert!(
        handle
            .update(cx, |view, _, _| view.charts[&id(1)]
                .borrow()
                .input
                .selected
                .is_none())
            .unwrap()
    );
    ready(cx, handle, 3, 0xff0000ff).await;
    move_mouse(cx, handle, position(150., 100.), false);
    mouse(cx, handle, position(150., 100.), true);
    cx.update(|cx| dispatch(cx, &transport, 40, Request::Release(source)));
    assert!(
        handle
            .update(cx, |_, window, _| window.captured_hitbox().is_none())
            .unwrap()
    );
    mouse(cx, handle, position(150., 100.), false);
    assert!(observations(&transport).is_empty());
    apply(
        cx,
        handle,
        vec![Op::Splice(id(0), 0, 1, vec![]), Op::Remove(id(1))],
    );
    eprintln!(
        "GPUIO_NATIVE_CHART_INPUT_OK: native hover/drag preview, commit and marker pixels, escape/outside release, keyboard, stable-ID reorder, disable token, blur/reset/release fences"
    );
}
