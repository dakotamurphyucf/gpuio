//! Independent inspection subtrees share neither gates nor action lifetimes.
use super::*;
use crate::host::chart_view::test::native_ax as accessibility;
use objc2::{
    msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool},
};

fn chart(source: ResourceId, shown: bool, container: Container) -> Config {
    let mut config = config(source, 0xff0000ff);
    config.inspection_content = vec![Entry {
        target: Some(Target::Slice(7)),
        container,
    }];
    config.style.inspection.card.visible = shown;
    config.style.inspection.card.width = 120.;
    config
}

async fn preview(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, chart: NodeId) {
    focus_window(cx, handle).await;
    let mut prepared = false;
    for _ in 0..300 {
        draw(cx, handle);
        prepared = handle
            .update(cx, |view, _, _| {
                let state = view.charts[&chart].borrow();
                state.ready.as_ref().is_some_and(|ready| {
                    ready.config == state.config
                        && state.requested_frame == state.ready_frame
                        && state
                            .lease
                            .as_ref()
                            .and_then(|lease| lease.snapshot())
                            .is_some_and(|live| live.revision() == ready.snapshot.revision())
                })
            })
            .unwrap();
        if prepared {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    assert!(prepared, "inspection chart prepared in its own window");
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.charts[&chart].borrow().input.focus, cx)
        })
        .unwrap();
    key(cx, handle, "home");
    key(cx, handle, "enter");
    draw(cx, handle);
}

fn create_source(session: &SharedSession, data: &Data) -> ResourceId {
    let Response::Created(source) = immediate(session, Request::Create) else {
        panic!("isolation source")
    };
    stage_data(session, source, 0, 1, data);
    let crate::session::ChartDispatch::Publish(work) = session
        .borrow_mut()
        .chart_request(Request::Publish(source, 1))
    else {
        panic!("isolation initial publication")
    };
    assert_eq!(
        session.borrow_mut().complete_chart(work.run()),
        Response::Ack
    );
    source
}

fn mount_pair(left: ResourceId, right: ResourceId) -> Vec<Op> {
    let mut ops = vec![
        Op::Create(id(0), Kind::Container, "".into(), None),
        Op::SetStyle(
            id(0),
            vec![Style::Fields(vec![
                Field::Display(1),
                Field::Direction(0),
                Field::Width(Length::Px(440.)),
                Field::Height(Length::Px(240.)),
            ])],
        ),
    ];
    for (chart_id, source, title) in [(1, left, "Left inspection"), (4, right, "Right inspection")]
    {
        ops.extend([
            Op::Create(id(chart_id), Kind::ChartView, "".into(), None),
            Op::SetChart(
                id(chart_id),
                Box::new(chart(
                    source,
                    true,
                    if chart_id == 1 {
                        Container::Card
                    } else {
                        Container::Overlay
                    },
                )),
            ),
            Op::SetStyle(
                id(chart_id),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(200.)),
                    Field::Height(Length::Px(200.)),
                    Field::Shrink(0.),
                ])],
            ),
            Op::Create(id(chart_id + 1), Kind::Container, "".into(), None),
            Op::Create(
                id(chart_id + 2),
                Kind::Button,
                title.into(),
                Some(HandlerId::from_parts(chart_id + 2, 1).unwrap()),
            ),
            Op::SetStyle(
                id(chart_id + 2),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(20.)),
                ])],
            ),
            Op::Splice(id(chart_id + 1), 0, 0, vec![id(chart_id + 2)]),
            Op::Splice(id(chart_id), 0, 0, vec![id(chart_id + 1)]),
        ]);
    }
    ops.extend([
        Op::Splice(id(0), 0, 0, vec![id(1), id(4)]),
        Op::SetRoot(Some(id(0))),
    ]);
    ops
}

fn drain(transport: &Transport) -> Vec<(WindowId, NodeId)> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(1024)
        .into_iter()
        .filter_map(|e| match e {
            Event::Press(window, node, ..) => Some((window, node)),
            _ => None,
        })
        .collect()
}

fn eligibility(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, left: bool, right: bool) {
    handle
        .update(cx, |view, _, _| {
            let gate = view.focus.borrow();
            assert_eq!(gate.allows(id(3)), left, "left chart gate in {:?}", view.id);
            assert_eq!(
                gate.allows(id(6)),
                right,
                "right chart gate in {:?}",
                view.id
            );
        })
        .unwrap();
}

async fn delivered(cx: &mut gpui::AsyncApp, transport: &Transport, window: WindowId, node: NodeId) {
    for _ in 0..100 {
        let events = drain(transport);
        if !events.is_empty() {
            assert_eq!(
                events,
                vec![(window, node)],
                "exact native window/node routing"
            );
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    panic!("isolated AX action not delivered to {window:?}/{node:?}");
}

async fn activate(
    cx: &mut gpui::AsyncApp,
    transport: &Transport,
    target: &Retained<AnyObject>,
    window: WindowId,
    node: NodeId,
) {
    assert!(drain(transport).is_empty());
    accessibility::press(target);
    delivered(cx, transport, window, node).await;
}

async fn retired(cx: &mut gpui::AsyncApp, transport: &Transport, target: &Retained<AnyObject>) {
    assert!(drain(transport).is_empty());
    unsafe {
        let _: Bool = msg_send![&**target, accessibilityPerformPress];
    }
    cx.background_executor()
        .timer(Duration::from_millis(100))
        .await;
    assert!(
        drain(transport).is_empty(),
        "retired AX object reached a live window"
    );
}

async fn focus_window(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    handle
        .update(cx, |_, window, _| window.activate_window())
        .unwrap();
    for _ in 0..100 {
        if handle
            .update(cx, |_, window, _| window.is_window_active())
            .unwrap()
        {
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    panic!("isolation window did not activate");
}

fn close(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    session: &SharedSession,
    window_id: WindowId,
) {
    let _ = handle.update(cx, |view, window, _| {
        for chart in view.charts.values() {
            chart.borrow_mut().close(window);
        }
        view.charts.clear();
        window.remove_window();
    });
    if session.borrow().tree(window_id).is_some() {
        session.borrow_mut().close(window_id).unwrap();
    }
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    original: WindowHandle<View>,
    session: &SharedSession,
    transport: &Transport,
    data: &Data,
    absent: &Data,
) {
    let original_bytes = session.borrow().chart_bytes();
    let left = create_source(session, data);
    let right = create_source(session, data);
    let transport_owner = original
        .update(cx, |view, _, _| view.transport.clone())
        .unwrap();
    let open = |slot| {
        let window_id = WindowId::from_parts(slot, 1).unwrap();
        session
            .borrow_mut()
            .open(100 + slot, window_id, "Inspection isolation", 440., 240.)
            .unwrap();
        let handle = cx.update(|cx| {
            cx.open_window(
                WindowOptions {
                    focus: false,
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        gpui::size(px(440.), px(240.)),
                        cx,
                    ))),
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(window_id, session.clone(), transport_owner.clone())),
            )
            .unwrap()
        });
        (window_id, handle)
    };
    let (first_id, first) = open(1);
    let (second_id, second) = open(2);
    let result = crate::host::native_test::protect(async {
        for window in [first, second] {
            apply(cx, window, mount_pair(left, right));
            preview(cx, window, id(1)).await;
            preview(cx, window, id(4)).await;
        }
        let first_left = accessibility::target_named(cx, first, "Left inspection").await;
        let first_right = accessibility::target_named(cx, first, "Right inspection").await;
        let second_left = accessibility::target_named(cx, second, "Left inspection").await;
        let second_right = accessibility::target_named(cx, second, "Right inspection").await;
        drain(transport);
        for (target, window, node) in [(&first_left, first_id, id(3)), (&first_right, first_id, id(6)), (&second_left, second_id, id(3)), (&second_right, second_id, id(6))] {
            activate(cx, transport, target, window, node).await;
        }
        apply(cx, first, vec![Op::SetChart(id(1), Box::new(chart(left, false, Container::Card)))]);
        eligibility(cx, first, false, true);
        eligibility(cx, second, true, true);
        retired(cx, transport, &first_left).await;
        activate(cx, transport, &first_right, first_id, id(6)).await;
        activate(cx, transport, &second_left, second_id, id(3)).await;
        apply(cx, first, vec![Op::SetChart(id(4), Box::new(chart(right, false, Container::Overlay)))]);
        apply(cx, first, vec![Op::SetChart(id(1), Box::new(chart(left, true, Container::Card)))]);
        // Select the revealed target through native navigation after its
        // replacement configuration has been prepared.
        preview(cx, first, id(1)).await;
        let revealed = accessibility::target_named(cx, first, "Left inspection").await;
        eligibility(cx, first, true, false);
        eligibility(cx, second, true, true);
        activate(cx, transport, &revealed, first_id, id(3)).await;
        retired(cx, transport, &first_right).await;
        activate(cx, transport, &second_right, second_id, id(6)).await;
        cx.update(|cx| { publish(session, left, absent, cx); publish(session, right, data, cx); });
        eligibility(cx, first, false, false);
        eligibility(cx, second, false, true);
        retired(cx, transport, &second_left).await;
        activate(cx, transport, &second_right, second_id, id(6)).await;
        cx.update(|cx| { publish(session, left, data, cx); });
        for window in [first, second] { preview(cx, window, id(1)).await; }
        eligibility(cx, first, true, false);
        eligibility(cx, second, true, true);
        let fresh_first = accessibility::target_named(cx, first, "Left inspection").await;
        let fresh_second = accessibility::target_named(cx, second, "Left inspection").await;
        activate(cx, transport, &fresh_first, first_id, id(3)).await;
        activate(cx, transport, &fresh_second, second_id, id(3)).await;
        apply(cx, first, vec![
            Op::Splice(id(0), 0, 1, vec![]),
            Op::Remove(id(3)), Op::Remove(id(2)), Op::Remove(id(1)),
        ]);
        eligibility(cx, first, false, false);
        eligibility(cx, second, true, true);
        retired(cx, transport, &fresh_first).await;
        activate(cx, transport, &fresh_second, second_id, id(3)).await;
        close(cx, first, session, first_id);
        retired(cx, transport, &fresh_first).await;
        retired(cx, transport, &first_right).await;
        activate(cx, transport, &second_right, second_id, id(6)).await;
        focus_window(cx, second).await;
        draw(cx, second);
        let point = second.update(cx, |view, _, _| view.probes.borrow()[&id(6)].bounds.center()).unwrap();
        move_mouse(cx, second, point, false);
        mouse(cx, second, point, true);
        // A different shared source disappears during this native gesture.
        cx.update(|cx| dispatch(cx, transport, 170, Request::Release(left)));
        eligibility(cx, second, false, true);
        mouse(cx, second, point, false);
        assert_eq!(drain(transport), vec![(second_id, id(6))]);
        second.update(cx, |view, window, cx| window.focus(&view.buttons[&id(6)].focus, cx)).unwrap();
        draw(cx, second);
        key(cx, second, "space");
        second.update(cx, |_, window, cx| {
            window.dispatch_event(gpui::PlatformInput::KeyUp(gpui::KeyUpEvent {
                keystroke: gpui::Keystroke::parse("space").unwrap(),
            }), cx);
        }).unwrap();
        assert_eq!(drain(transport), vec![(second_id, id(6))]);
        cx.update(|cx| dispatch(cx, transport, 171, Request::Release(right)));
        eligibility(cx, second, false, false);
        retired(cx, transport, &second_right).await;
        eprintln!("GPUIO_INSPECTION_ISOLATION_OK: Card/Overlay charts in two windows, identical node IDs with exact AX routing, per-chart hiding, shared-source removal/return, peer pointer/keyboard preservation and independent unmount/close");
    }).await;
    for (id, window) in [(first_id, first), (second_id, second)] {
        close(cx, window, session, id);
    }
    for source in [left, right] {
        if session.borrow().chart(source).is_ok() {
            assert_eq!(immediate(session, Request::Release(source)), Response::Ack);
        }
    }
    assert_eq!(session.borrow().chart_bytes(), original_bytes);
    focus_window(cx, original).await;
    if let Err(error) = result {
        std::panic::resume_unwind(error);
    }
}
