//! Real AppKit popup leases under Card/Overlay inspection content.
use super::*;
use chart_labels::{assert_retired, immediate, publish_data};
use gpuio_protocol::{
    ResourceId,
    chart_data::{Contents, Data, Slice},
    chart_inspection_content::{Container, Entry, Target},
    chart_resource::{Request, Response},
    chart_view::Config,
};
fn chart() -> NodeId {
    NodeId::from_parts(2, 2).unwrap()
}
fn wrapper() -> NodeId {
    NodeId::from_parts(3, 2).unwrap()
}
fn data(present: bool) -> Data {
    Data {
        version: 3,
        bar_baselines: vec![],
        bar_backgrounds: vec![],
        contents: Contents::Pie(
            (if present { vec![7, 9] } else { vec![9] })
                .into_iter()
                .map(|id| Slice {
                    id,
                    label: format!("Slice {id}"),
                    value: 1.,
                })
                .collect(),
        ),
    }
}
fn config(source: ResourceId, container: Container) -> Config {
    Config {
        version: -2,
        source: Some(source),
        label: "Inspection popup owner".into(),
        options: Default::default(),
        sampling: Default::default(),
        style: Default::default(),
        radar_labels: vec![],
        inspection_content: vec![Entry {
            target: Some(Target::Slice(7)),
            container,
        }],
        legend: false,
        disabled: false,
    }
}
async fn select(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, owner: NodeId) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        host::editor_test::frame(cx, handle).await;
        let center = gpui::point(px(200.), px(80.));
        host::native_test::move_mouse(cx, handle, center, false);
        host::native_test::mouse(cx, handle, center, true);
        host::native_test::mouse(cx, handle, center, false);
        // Overlay content can cover the clicked mark and retain child focus.
        // Navigate through the real tab order before issuing chart commands.
        for _ in 0..8 {
            if handle
                .update(cx, |view, window, _| {
                    view.charts[&chart()].borrow().chart_focused(window)
                })
                .unwrap()
            {
                break;
            }
            host::editor_test::key(cx, handle, "tab");
        }
        host::editor_test::key(cx, handle, "home");
        host::editor_test::key(cx, handle, "enter");
        host::editor_test::frame(cx, handle).await;
        if handle
            .update(cx, |view, _, _| view.focus.borrow().allows(owner))
            .unwrap()
        {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "inspection popup owner not eligible"
        );
    }
}
pub(super) async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, owner: NodeId) {
    let session = handle
        .update(cx, |view, _, _| view.session.clone())
        .unwrap();
    let source = match immediate(&session, Request::Create) {
        Response::Created(id) => id,
        _ => panic!("inspection source"),
    };
    cx.update(|cx| publish_data(&session, source, &data(true), cx));
    handle
        .update(cx, |view, window, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(chart(), Kind::ChartView, "".into(), None),
                    Op::SetChart(chart(), Box::new(config(source, Container::Card))),
                    Op::SetStyle(
                        chart(),
                        vec![Style::Fields(vec![
                            Field::Width(Length::Px(400.)),
                            Field::Height(Length::Px(160.)),
                        ])],
                    ),
                    Op::Create(wrapper(), Kind::Container, "".into(), None),
                    Op::SetRoot(None),
                    Op::Splice(wrapper(), 0, 0, vec![owner]),
                    Op::Splice(chart(), 0, 0, vec![wrapper()]),
                    Op::SetRoot(Some(chart())),
                ],
            );
        })
        .unwrap();
    for container in [Container::Card, Container::Overlay] {
        cx.update(|cx| publish_data(&session, source, &data(true), cx));
        handle
            .update(cx, |view, window, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![Op::SetChart(chart(), Box::new(config(source, container)))],
                )
            })
            .unwrap();
        select(cx, handle, owner).await;
        let before = popup::tracking_calls();
        // Queue tracking and retire its target in one UI update, before AppKit
        // can execute the runner. Returning the ID must not revive the lease.
        cx.update(|cx| {
            handle
                .update(cx, |view, window, cx| show(view, window, cx, owner, 2))
                .unwrap();
            publish_data(&session, source, &data(false), cx);
            assert_retired(handle, owner, cx);
            publish_data(&session, source, &data(true), cx);
        });
        retired(cx).await;
        assert_eq!(
            popup::tracking_calls(),
            before,
            "queued inspection popup never tracks"
        );
        select(cx, handle, owner).await;
        // Original-data browsing is another immediate retirement boundary.
        cx.update(|cx| {
            handle
                .update(cx, |view, window, cx| {
                    assert!(view.charts[&chart()].borrow().chart_focused(window));
                    show(view, window, cx, owner, 2);
                })
                .unwrap();
            cx.update_window(handle.into(), |_, window, cx| {
                window.dispatch_event(
                    gpui::PlatformInput::KeyDown(gpui::KeyDownEvent {
                        keystroke: gpui::Keystroke::parse("d").unwrap(),
                        is_held: false,
                        prefer_character_input: false,
                    }),
                    cx,
                );
            })
            .unwrap();
            handle
                .update(cx, |view, _, _| assert!(!view.focus.borrow().allows(owner)))
                .unwrap();
            assert_retired(handle, owner, cx);
        });
        retired(cx).await;
        assert_eq!(popup::tracking_calls(), before);
        host::editor_test::key(cx, handle, "escape");
        select(cx, handle, owner).await;
        // Positive control: this lease enters AppKit's actual tracking loop.
        // Only then does a GPUI task remove the target and require cancellation.
        let live = session.clone();
        let closer = cx.spawn(async move |cx| {
            let deadline = Instant::now() + Duration::from_secs(5);
            while popup::tracking_calls() == before {
                assert!(
                    Instant::now() < deadline,
                    "inspection popup never entered AppKit"
                );
                cx.background_executor()
                    .timer(Duration::from_millis(10))
                    .await;
            }
            cx.update(|cx| {
                publish_data(&live, source, &data(false), cx);
                assert_retired(handle, owner, cx);
            });
        });
        handle
            .update(cx, |view, window, cx| show(view, window, cx, owner, 2))
            .unwrap();
        retired(cx).await;
        closer.await;
        assert_eq!(popup::tracking_calls(), before + 1);
        eprintln!(
            "POPUP_INSPECTION_RETIRE_OK: {container:?}, queued source hide/return, original-data browser and actual live AppKit popup cancellation before paint"
        );
    }
    handle
        .update(cx, |view, window, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Splice(wrapper(), 0, 1, vec![]),
                    Op::SetRoot(Some(owner)),
                    Op::Remove(wrapper()),
                    Op::Remove(chart()),
                ],
            );
        })
        .unwrap();
    assert_eq!(immediate(&session, Request::Release(source)), Response::Ack);
    host::editor_test::frame(cx, handle).await;
}
