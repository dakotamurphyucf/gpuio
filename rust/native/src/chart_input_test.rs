//! Native GPUI dispatch through the production retained chart view.
use super::*;
use crate::host::native_test::{mouse, move_mouse};
use gpuio_protocol::chart_selection::Selection;
#[path = "chart_guide_span_test.rs"]
mod guide_spans;
#[path = "chart_inspection_content_test.rs"]
mod inspection_content;
#[path = "chart_label_input_test.rs"]
mod labels;
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
    guide_spans::exercise(cx, handle, source, &transport).await;
    observations(&transport);
    // Moving inside the same pie wedge must repaint a cursor card without
    // publishing selection or rerunning source preparation.
    let mut cursor_config = config(source, 0xff0000ff);
    cursor_config.style.inspection.card.placement =
        gpuio_protocol::chart_inspection::Placement::Cursor;
    cursor_config.style.inspection.card.width = 96.;
    cursor_config.style.inspection.card.background = Some(0xff00ffff);
    apply(
        cx,
        handle,
        vec![Op::SetChart(id(1), Box::new(cursor_config))],
    );
    ready(cx, handle, 1, 0xff0000ff).await;
    observations(&transport);
    let mut edges = vec![];
    for x in [130., 155.] {
        move_mouse(cx, handle, position(x, 100.), false);
        draw(cx, handle);
        edges.push(
            handle
                .update(cx, |view, window, _| {
                    let state = view.charts[&id(1)].borrow();
                    assert!(state.input.pointer.is_some());
                    assert!(state.input.selected.is_none());
                    let image = window.render_to_image().unwrap();
                    let left = image
                        .enumerate_pixels()
                        .filter(|(_, _, p)| p.0 == [255, 0, 255, 255])
                        .map(|(x, _, _)| x)
                        .min()
                        .expect("cursor card pixels");
                    f64::from(left) / f64::from(window.scale_factor())
                })
                .unwrap(),
        );
        assert!(
            observations(&transport).is_empty(),
            "pointer motion stays native"
        );
    }
    assert!(
        (edges[1] - edges[0] - 25.).abs() <= 1.,
        "same-wedge cursor movement: {edges:?}"
    );
    move_mouse(cx, handle, position(-10., 100.), false);
    draw(cx, handle);
    handle
        .update(cx, |view, _, _| {
            assert!(view.charts[&id(1)].borrow().input.pointer.is_none());
        })
        .unwrap();
    move_mouse(cx, handle, position(155., 100.), false);
    draw(cx, handle);
    handle
        .update(cx, |view, window, cx| {
            let focus = view.charts[&id(1)].borrow().input.focus.clone();
            window.focus(&focus, cx)
        })
        .unwrap();
    key(cx, handle, "home");
    draw(cx, handle);
    handle
        .update(cx, |view, _, _| {
            assert!(view.charts[&id(1)].borrow().input.pointer.is_none())
        })
        .unwrap();
    assert!(observations(&transport).is_empty());
    apply(
        cx,
        handle,
        vec![Op::SetChart(id(1), Box::new(config(source, 0xff0000ff)))],
    );
    ready(cx, handle, 1, 0xff0000ff).await;
    eprintln!(
        "GPUIO_CHART_CURSOR_CARD_OK: same-wedge pointer pixel movement, no selection events, keyboard anchor fallback"
    );
    move_mouse(cx, handle, position(150., 100.), false);
    draw(cx, handle);
    assert!(
        handle
            .update(cx, |view, _, _| view.charts[&id(1)]
                .borrow()
                .input_overlay(None)
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
                    .get_pixel((58. * scale) as u32, (116. * scale) as u32)
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
    key(cx, handle, "d");
    draw(cx, handle);
    key(cx, handle, "end");
    draw(cx, handle);
    handle
        .update(cx, |view, _, _| {
            let state = view.charts[&id(1)].borrow();
            assert_eq!(state.input.data_cursor, Some(1));
            assert_eq!(state.input.selected, Some(Selection::Slice(9)));
        })
        .unwrap();
    assert!(
        observations(&transport).is_empty(),
        "data browsing does not commit plot selection"
    );
    let stale_data_route = handle
        .update(cx, |view, _, _| {
            input::capture_browse(view.charts[&id(1)].clone(), 0)
        })
        .unwrap();
    key(cx, handle, "escape");
    draw(cx, handle);
    assert_eq!(
        handle
            .update(cx, |view, _, _| view.charts[&id(1)]
                .borrow()
                .input
                .data_cursor)
            .unwrap(),
        None
    );
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
    key(cx, handle, "d");
    key(cx, handle, "end");
    handle
        .update(cx, |view, window, cx| {
            stale_data_route(window, cx);
            assert_eq!(view.charts[&id(1)].borrow().input.data_cursor, Some(1));
        })
        .unwrap();
    key(cx, handle, "escape");
    draw(cx, handle);
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
            Box::new(Config {
                disabled: true,
                ..config(source, 0xff0000ff)
            }),
        )],
    );
    apply(
        cx,
        handle,
        vec![Op::SetChart(id(1), Box::new(config(source, 0xff0000ff)))],
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
    let budget_hold = cx.update(|cx| renderer::hold_remaining_budget_for_test(cx));
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
    // Exercise the real worker admission failure, without installing a fake
    // Ready/Failed state. Original data remains independently browsable.
    for _ in 0..300 {
        draw(cx, handle);
        if handle
            .update(cx, |view, _, _| {
                view.charts[&id(1)].borrow().failure == Some(Error::RenderLimit)
            })
            .unwrap()
        {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    handle
        .update(cx, |view, window, cx| {
            let state = view.charts[&id(1)].borrow();
            assert_eq!(state.failure, Some(Error::RenderLimit));
            assert!(state.ready.is_none());
            window.focus(&state.input.focus, cx);
        })
        .unwrap();
    key(cx, handle, "d");
    key(cx, handle, "end");
    draw(cx, handle);
    assert_eq!(
        handle
            .update(cx, |view, _, _| view.charts[&id(1)]
                .borrow()
                .input
                .data_cursor)
            .unwrap(),
        Some(1)
    );
    assert!(observations(&transport).is_empty());
    key(cx, handle, "escape");
    drop(budget_hold);
    let mut retry = config(source, 0xff0000ff);
    retry.style.stroke_width = 3.;
    apply(cx, handle, vec![Op::SetChart(id(1), Box::new(retry))]);
    ready(cx, handle, 3, 0xff0000ff).await;
    key(cx, handle, "d");
    key(cx, handle, "end");
    stage_data(&session, source, 3, 2, &data(1.));
    let crate::session::ChartDispatch::Publish(work) = session
        .borrow_mut()
        .chart_request(Request::Publish(source, 4))
    else {
        panic!("publication");
    };
    assert_eq!(
        session.borrow_mut().complete_chart(work.run()),
        Response::Ack
    );
    cx.update(|cx| crate::host::chart_source_changed(Some(source), cx));
    assert_eq!(
        handle
            .update(cx, |view, _, _| view.charts[&id(1)]
                .borrow()
                .input
                .data_cursor)
            .unwrap(),
        Some(0)
    );
    ready(cx, handle, 4, 0xff0000ff).await;
    publish_input(&session, source, 4, 2, false);
    cx.update(|cx| crate::host::chart_source_changed(Some(source), cx));
    ready(cx, handle, 5, 0xff0000ff).await;
    assert_eq!(
        handle
            .update(cx, |view, _, _| view.charts[&id(1)]
                .borrow()
                .input
                .data_cursor)
            .unwrap(),
        Some(0),
        "a clamped cursor must not jump back when data grows"
    );
    key(cx, handle, "escape");
    draw(cx, handle);
    labels::exercise(cx, handle, source, &session, &transport).await;
    inspection_content::exercise(cx, handle, source, &session, &transport).await;
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
        "GPUIO_NATIVE_CHART_INPUT_OK: native hover/drag preview, commit and marker pixels, escape/outside release, keyboard, stable-ID reorder, disable token, blur/reset/release fences, bounded original-data browsing, stale data route, real preparation quota failure recovery"
    );
}
