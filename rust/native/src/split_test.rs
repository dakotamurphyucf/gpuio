//! Real layout and native input for retained split geometry.
use super::*;
use gpuio_protocol::split::{Axis, Config, Snapshot};
fn config() -> Config {
    Config {
        label: "Workspace divider".into(),
        axis: Axis::Horizontal,
        initial_first: 160.,
        minimum_first: 80.,
        maximum_first: 280.,
        minimum_second: 80.,
        keyboard_step: 16.,
        reset_generation: 0,
    }
}
fn style(hidden: bool) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Position(1),
        Field::Left(Length::Px(0.)),
        Field::Top(Length::Px(0.)),
        Field::Width(Length::Px(400.)),
        Field::Height(Length::Px(250.)),
        Field::Display(if hidden { 3 } else { 0 }),
    ])]
}
fn draw(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
}
fn sizes(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> Snapshot {
    handle
        .update(cx, |view, _, cx| {
            let s = view.splits[&node(5)].native.read(cx).sizes();
            Snapshot {
                first: s[0].as_f32() as f64,
                second: s[1].as_f32() as f64,
            }
        })
        .unwrap()
}
fn assert_sizes(actual: Snapshot, first: f64, second: f64) {
    assert!(
        (actual.first - first).abs() < 0.1 && (actual.second - second).abs() < 0.1,
        "sizes {actual:?}, expected {first}/{second}"
    );
}
fn resizing(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> bool {
    handle
        .update(cx, |view, _, cx| {
            view.splits[&node(5)].native.read(cx).is_resizing()
        })
        .unwrap()
}
fn events(transport: &Transport) -> Vec<(i64, Snapshot)> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::SplitResized(_, _, _, _, generation, sizes) => Some((generation, sizes)),
            _ => None,
        })
        .collect()
}
fn begin_drag(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    let x = sizes(cx, handle).first as f32;
    let p = gpui::point(px(x), px(100.));
    super::super::native_test::move_mouse(cx, handle, p, false);
    super::super::native_test::mouse(cx, handle, p, true);
    super::super::native_test::move_mouse(cx, handle, gpui::point(px(x + 20.), px(100.)), true);
    draw(cx, handle);
    assert!(resizing(cx, handle), "divider owns native drag");
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    let mut config = config();
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(5),
                Kind::SplitPane,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(5, 1).unwrap()),
            ),
            Op::SetSplit(node(5), config.clone()),
            Op::SetStyle(node(5), style(false)),
            Op::Create(node(6), Kind::Container, "".into(), None),
            Op::Create(node(7), Kind::Text, "Transcript".into(), None),
            Op::Splice(node(5), 0, 0, vec![node(6), node(7)]),
            Op::Splice(node(0), 4, 0, vec![node(5)]),
        ],
    );
    frame(cx, handle).await;
    assert_sizes(sizes(cx, handle), 160., 240.);
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.editors[&node(4)].focus_handle(cx), cx)
        })
        .unwrap();
    key(cx, handle, "tab");
    assert!(
        handle
            .update(cx, |view, window, _| view.splits[&node(5)]
                .focus
                .is_focused(window))
            .unwrap(),
        "split divider is a tab stop"
    );
    #[cfg(target_os = "macos")]
    {
        let _ = accessible(cx, handle, "Workspace divider", false);
        frame(cx, handle).await;
        let info = accessible_request(
            cx,
            handle,
            "Workspace divider",
            Some("AXSplitter"),
            AccessibilityRequest::Increment,
        )
        .expect("native splitter AX role");
        assert_eq!(info.value, 160);
        frame(cx, handle).await;
        assert_eq!(sizes(cx, handle).first, 176.);
        accessible_request(
            cx,
            handle,
            "Workspace divider",
            Some("AXSplitter"),
            AccessibilityRequest::Decrement,
        )
        .unwrap();
        frame(cx, handle).await;
        assert_eq!(sizes(cx, handle).first, 160.);
    }
    events(transport);
    key(cx, handle, "right");
    draw(cx, handle);
    assert_eq!(sizes(cx, handle).first, 176.);
    assert_eq!(events(transport).last().unwrap().1.first, 176.);
    key(cx, handle, "end");
    draw(cx, handle);
    assert_sizes(sizes(cx, handle), 280., 120.);
    key(cx, handle, "home");
    draw(cx, handle);
    assert_sizes(sizes(cx, handle), 80., 320.);
    let entity = handle
        .update(cx, |view, _, _| view.splits[&node(5)].native.entity_id())
        .unwrap();
    config.initial_first = 200.;
    apply(cx, handle, vec![Op::SetSplit(node(5), config.clone())]);
    draw(cx, handle);
    assert_eq!(
        sizes(cx, handle).first,
        80.,
        "rerender does not echo initial size"
    );
    assert_eq!(
        handle
            .update(cx, |view, _, _| view.splits[&node(5)].native.entity_id())
            .unwrap(),
        entity
    );
    let editor_config = handle
        .update(cx, |view, _, _| {
            view.session
                .borrow()
                .tree(view.id)
                .unwrap()
                .get(node(4))
                .unwrap()
                .editor
                .as_ref()
                .unwrap()
                .as_ref()
                .clone()
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                node(8),
                Kind::Input,
                "Retained draft λ".into(),
                Some(gpuio_protocol::HandlerId::from_parts(8, 1).unwrap()),
            ),
            Op::SetEditor(node(8), editor_config),
            Op::Splice(node(6), 0, 0, vec![node(8)]),
        ],
    );
    draw(cx, handle);
    let editor_focus = handle
        .update(cx, |view, _, cx| view.editors[&node(8)].focus_handle(cx))
        .unwrap();
    events(transport);
    begin_drag(cx, handle);
    super::super::native_test::move_mouse(cx, handle, gpui::point(px(210.), px(100.)), true);
    draw(cx, handle);
    assert_eq!(sizes(cx, handle).first, 210.);
    assert!(events(transport).is_empty(), "no per-move bridge traffic");
    super::super::native_test::mouse(cx, handle, gpui::point(px(210.), px(100.)), false);
    assert_eq!(events(transport).len(), 1, "one completed gesture");
    begin_drag(cx, handle);
    key(cx, handle, "escape");
    assert!(!resizing(cx, handle));
    super::super::native_test::mouse(cx, handle, gpui::point(px(250.), px(100.)), false);
    assert!(
        events(transport).is_empty(),
        "cancelled gesture has no completion"
    );
    begin_drag(cx, handle);
    apply(cx, handle, vec![Op::SetStyle(node(5), style(true))]);
    assert!(!resizing(cx, handle));
    super::super::native_test::mouse(cx, handle, gpui::point(px(260.), px(100.)), false);
    assert!(events(transport).is_empty());
    apply(cx, handle, vec![Op::SetStyle(node(5), style(false))]);
    draw(cx, handle);
    begin_drag(cx, handle);
    config.reset_generation = 1;
    apply(cx, handle, vec![Op::SetSplit(node(5), config.clone())]);
    draw(cx, handle);
    assert_ne!(
        handle
            .update(cx, |view, _, _| view.splits[&node(5)].native.entity_id())
            .unwrap(),
        entity
    );
    assert_eq!(sizes(cx, handle).first, 200.);
    handle
        .update(cx, |view, window, cx| {
            assert_eq!(
                view.editors[&node(8)].snapshot(window, cx).text,
                "Retained draft λ"
            );
            assert!(view.editors[&node(8)].focus_handle(cx) == editor_focus);
        })
        .unwrap();
    super::super::native_test::mouse(cx, handle, gpui::point(px(280.), px(100.)), false);
    assert!(events(transport).is_empty());
    config.axis = Axis::Vertical;
    config.initial_first = 100.;
    apply(cx, handle, vec![Op::SetSplit(node(5), config.clone())]);
    draw(cx, handle);
    assert_sizes(sizes(cx, handle), 100., 150.);
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.splits[&node(5)].focus, cx)
        })
        .unwrap();
    key(cx, handle, "down");
    draw(cx, handle);
    assert_eq!(sizes(cx, handle).first, 116.);
    assert_eq!(events(transport).last().unwrap().0, 1);
    key(cx, handle, "end");
    draw(cx, handle);
    assert_sizes(sizes(cx, handle), 170., 80.);
    config.axis = Axis::Horizontal;
    config.reset_generation = 2;
    config.initial_first = 180.;
    apply(cx, handle, vec![Op::SetSplit(node(5), config)]);
    draw(cx, handle);
    events(transport);
    begin_drag(cx, handle);
    let mut disabled = style(false);
    disabled.push(Style::Fields(vec![Field::PointerEvents(false)]));
    apply(cx, handle, vec![Op::SetStyle(node(5), disabled)]);
    assert!(!resizing(cx, handle));
    draw(cx, handle);
    let divider = gpui::point(px(sizes(cx, handle).first as f32), px(100.));
    super::super::native_test::mouse(cx, handle, divider, false);
    super::super::native_test::move_mouse(cx, handle, divider, false);
    super::super::native_test::mouse(cx, handle, divider, true);
    super::super::native_test::move_mouse(cx, handle, divider + gpui::point(px(20.), px(0.)), true);
    assert!(
        !resizing(cx, handle),
        "disabled pointer policy cannot start another drag"
    );
    super::super::native_test::mouse(cx, handle, gpui::point(px(260.), px(100.)), false);
    assert!(events(transport).is_empty());
    apply(cx, handle, vec![Op::SetStyle(node(5), style(false))]);
    draw(cx, handle);
    begin_drag(cx, handle);
    // An externally cancelled drag leaves an old painted listener behind.
    handle
        .update(cx, |_, window, cx| {
            assert!(cx.stop_active_drag(window));
        })
        .unwrap();
    let before = sizes(cx, handle);
    super::super::native_test::move_mouse(cx, handle, gpui::point(px(270.), px(100.)), true);
    super::super::native_test::mouse(cx, handle, gpui::point(px(270.), px(100.)), false);
    assert_eq!(sizes(cx, handle), before);
    assert!(events(transport).is_empty());
    begin_drag(cx, handle);

    apply(
        cx,
        handle,
        vec![
            Op::Splice(node(0), 4, 1, vec![]),
            Op::Splice(node(5), 0, 2, vec![]),
            Op::Splice(node(6), 0, 1, vec![]),
            Op::Remove(node(8)),
            Op::Remove(node(6)),
            Op::Remove(node(7)),
            Op::Remove(node(5)),
        ],
    );
    assert!(
        handle
            .update(cx, |view, _, _| view.splits.is_empty())
            .unwrap()
    );
    assert!(!cx.update(|cx| cx.has_active_drag()));
    println!(
        "GPUIO_SPLIT_NATIVE_OK: native pointer/keyboard geometry, completed events, reset, hide, Escape, axis, disposal"
    );
}
