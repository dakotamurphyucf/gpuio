//! Repeated frames and shared publications through two production managed lists.
//! GPUI retains row measurements here; visible elements still paint every frame.
use super::*;
use gpuio_protocol::list::{Config as ListConfig, IdRun, Order, Row, ScrollPolicy};

fn dimensions(width: f64, height: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(height)),
        Field::Shrink(0.),
    ])]
}
fn mount_list(source: ResourceId) -> Vec<Op> {
    let mut operations = vec![
        Op::Create(id(0), Kind::VirtualList, String::new(), None),
        Op::SetStyle(id(0), dimensions(240., 240.)),
        Op::SetListConfig(
            id(0),
            ListConfig {
                estimated_height: 120.,
                overscan: 0.,
                max_active: 3,
                scroll_policy: ScrollPolicy::KeepPosition,
                scrollbar: false,
                managed: true,
            },
        ),
        Op::SetListOrder(
            id(0),
            Order {
                revision: 1,
                runs: vec![IdRun { first: 1, count: 3 }],
            },
        ),
    ];
    for slot in 1..=3 {
        operations.extend([
            Op::Create(id(slot), Kind::ChartView, String::new(), None),
            Op::SetChart(id(slot), Box::new(config(source, 0x00ff00ff))),
            Op::SetStyle(id(slot), dimensions(200., 120.)),
        ]);
    }
    operations.extend([
        Op::SetListRows(
            id(0),
            (1..=3)
                .map(|slot| Row {
                    id: slot,
                    node: id(slot),
                })
                .collect(),
        ),
        Op::Splice(id(0), 0, 0, (1..=3).map(id).collect()),
        Op::SetRoot(Some(id(0))),
    ]);
    operations
}
async fn await_rows(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    revision: i64,
    first: i64,
) {
    for _ in 0..300 {
        draw(cx, handle);
        let ready = handle
            .update(cx, |view, _, _| {
                (first..first + 2).all(|slot| {
                    view.charts[&id(slot)]
                        .borrow()
                        .ready
                        .as_ref()
                        .is_some_and(|ready| ready.snapshot.revision() == revision)
                })
            })
            .unwrap();
        if ready {
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    panic!("two visible rows did not prepare publication {revision}");
}
fn accounted_frame(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, first: i64) {
    // Force a real paint even if no state changed, so zeroed counters cannot be
    // mistaken for a valid replay from an earlier frame.
    handle.update(cx, |_, window, _| window.refresh()).unwrap();
    draw(cx, handle);
    handle
        .update(cx, |view, window, _| {
            let (vertices, quads) = (first..first + 2).fold((0, 0), |(v, q), slot| {
                let state = view.charts[&id(slot)].borrow();
                let plan = &state.ready.as_ref().unwrap().plan;
                (v + plan.vertices(), q + plan.quad_count())
            });
            assert!(vertices > 0);
            let budget = view.chart_budget.borrow();
            assert_eq!(
                (budget.used_vertices(), budget.used_quads()),
                (vertices, quads),
                "every visible chart must be charged on every repeated list frame"
            );
            let image = window.render_to_image().unwrap();
            let scale = window.scale_factor();
            for y in [70., 190.] {
                assert_eq!(
                    image.get_pixel((100. * scale) as u32, (y * scale) as u32).0,
                    [0, 255, 0, 255],
                    "both managed rows must actually paint"
                );
            }
        })
        .unwrap();
}
fn scroll(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, index: usize) {
    handle
        .update(cx, |view, window, cx| {
            view.lists[&id(0)]
                .borrow()
                .native
                .handle()
                .scroll_to(gpui::ListOffset {
                    item_ix: index,
                    offset_in_item: px(0.),
                });
            window.refresh();
            cx.notify();
        })
        .unwrap();
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    first: WindowHandle<View>,
    session: SharedSession,
    transport: Arc<Transport>,
) {
    first
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    let source = match immediate(&session, Request::Create) {
        Response::Created(source) => source,
        _ => panic!("create streaming source"),
    };
    publish(&session, source, 0, 1);
    let open = |slot| {
        let window_id = WindowId::from_parts(slot, 1).unwrap();
        session
            .borrow_mut()
            .open(slot + 1, window_id, "Chart shared resource", 240., 240.)
            .unwrap();
        cx.update(|cx| {
            cx.open_window(
                WindowOptions {
                    show: false,
                    focus: false,
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        gpui::size(px(240.), px(240.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(window_id, session.clone(), transport.clone())),
            )
            .unwrap()
        })
    };
    let first = open(1);
    let second = open(2);
    let result = crate::host::native_test::protect(async {
        for window in [first, second] {
            apply(cx, window, mount_list(source));
            await_rows(cx, window, 1, 1).await;
            for _ in 0..3 { accounted_frame(cx, window, 1); }
        }
        // Same source, independent native list positions and worker output.
        scroll(cx, first, 1);
        await_rows(cx, first, 1, 2).await;
        accounted_frame(cx, first, 2);
        accounted_frame(cx, second, 1);
        for revision in 2..=13 {
            stage(&session, source, revision - 1, 1, revision as f64);
            cx.update(|cx| dispatch(cx, &transport, 100 + revision, Request::Publish(source, revision)));
            for (window, first_row) in [(first, 2), (second, 1)] {
                await_rows(cx, window, revision, first_row).await;
                accounted_frame(cx, window, first_row);
            }
            transport.mailbox.lock().unwrap().drain(128);
        }
        // Closing one observer must not retire the shared source or the other.
        second.update(cx, |view, window, _| {
            for state in view.charts.values() { state.borrow_mut().close(window); }
            view.charts.clear();
            window.remove_window();
        }).unwrap();
        stage(&session, source, 13, 1, 14.);
        cx.update(|cx| dispatch(cx, &transport, 114, Request::Publish(source, 14)));
        await_rows(cx, first, 14, 2).await;
        accounted_frame(cx, first, 2);
        cx.update(|cx| dispatch(cx, &transport, 115, Request::Release(source)));
        first.update(cx, |view, _, _| {
            assert!(view.charts.values().all(|state| {
                let state = state.borrow();
                state.ready.is_none() && state.job.is_none() && state.lease.is_none()
            }));
        }).unwrap();
        eprintln!("GPUIO_CHART_LIST_STREAM_OK: two windows, repeated-frame accounting and pixels, independent scroll, 14 shared publications, observer close, idle release");
    }).await;
    for handle in [first, second] {
        let _ = handle.update(cx, |view, window, _| {
            for state in view.charts.values() {
                state.borrow_mut().close(window);
            }
            view.charts.clear();
            window.remove_window();
        });
    }
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}
