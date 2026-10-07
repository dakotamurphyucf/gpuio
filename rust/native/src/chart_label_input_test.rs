//! Foreground native InputRegion routing and hide/return before another frame.
use super::*;
use gpuio_protocol::{chart_data::RadarAxis, input as observed};
#[path = "chart_label_action_test.rs"]
mod actions;

fn publish(session: &SharedSession, source: ResourceId, data: &Data, cx: &mut App) -> i64 {
    let snapshot = session.borrow().chart(source).unwrap().snapshot().unwrap();
    let revision = snapshot.revision() + 1;
    stage_data(
        session,
        source,
        snapshot.revision(),
        snapshot.generation(),
        data,
    );
    let crate::session::ChartDispatch::Publish(work) = session
        .borrow_mut()
        .chart_request(Request::Publish(source, revision))
    else {
        panic!("input label publication")
    };
    assert_eq!(
        session.borrow_mut().complete_chart(work.run()),
        Response::Ack
    );
    crate::host::chart_source_changed(Some(source), cx);
    revision
}

fn events(transport: &Transport) -> Vec<observed::Kind> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(1024)
        .into_iter()
        .filter_map(|event| match event {
            Event::InputObserved(_, node, _, _, event) if node == id(3) => Some(event.kind()),
            Event::Press(_, node, ..) if node == id(3) => panic!("InputRegion emitted Press"),
            _ => None,
        })
        .collect()
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: &SharedSession,
    transport: &Transport,
) {
    use observed::Kind::{Click, MouseDown, MouseUp};
    let original = session.borrow().chart(source).unwrap().snapshot().unwrap();
    let data = Data {
        version: 3,
        bar_baselines: vec![],
        bar_backgrounds: vec![],
        contents: Contents::Radar(
            [7, 9, 11]
                .into_iter()
                .map(|id| RadarAxis {
                    id,
                    label: "Axis".into(),
                    maximum: 100.,
                })
                .collect(),
            vec![],
        ),
    };
    let revision = cx.update(|cx| publish(session, source, &data, cx));
    let mut chart = config(source, 0xff0000ff);
    chart.radar_labels = vec![7];
    apply(
        cx,
        handle,
        vec![
            Op::SetChart(id(1), Box::new(chart)),
            Op::Create(id(2), Kind::Container, "".into(), None),
            Op::Create(
                id(3),
                Kind::InputRegion,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(3, 1).unwrap()),
            ),
            Op::SetInputRegion(
                id(3),
                observed::Config {
                    label: "Radar input".into(),
                    disabled: false,
                    focus: observed::Focus::None,
                    subscriptions: [Click, MouseDown, MouseUp]
                        .into_iter()
                        .map(|kind| observed::Subscription {
                            kind,
                            phase: observed::Phase::Bubble,
                            policy: if kind == Click {
                                observed::Policy::Observe
                            } else {
                                observed::Policy::PreventAndStop
                            },
                        })
                        .collect(),
                },
            ),
            Op::SetStyle(
                id(3),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(20.)),
                ])],
            ),
            Op::Create(id(4), Kind::Text, "Input label".into(), None),
            Op::Splice(id(3), 0, 0, vec![id(4)]),
            Op::Splice(id(2), 0, 0, vec![id(3)]),
            Op::Splice(id(1), 0, 0, vec![id(2)]),
        ],
    );
    ready(cx, handle, revision, 0xff0000ff).await;
    draw(cx, handle);
    let point = handle
        .update(cx, |view, window, _| {
            assert!(window.is_window_active());
            assert!(view.focus.borrow().allows(id(3)));
            view.probes.borrow()[&id(3)].bounds.center()
        })
        .unwrap();
    events(transport);
    move_mouse(cx, handle, point, false);
    mouse(cx, handle, point, true);
    mouse(cx, handle, point, false);
    assert_eq!(
        events(transport),
        vec![MouseDown, MouseUp, Click],
        "native label region receives bubble down/up/click"
    );
    // A normal publication must not erase the region's pending click.
    mouse(cx, handle, point, true);
    cx.update(|cx| {
        publish(session, source, &data, cx);
    });
    mouse(cx, handle, point, false);
    assert_eq!(events(transport), vec![MouseDown, MouseUp, Click]);
    mouse(cx, handle, point, true);
    assert_eq!(events(transport), vec![MouseDown]);
    let mut absent = data.clone();
    let Contents::Radar(axes, _) = &mut absent.contents else {
        unreachable!()
    };
    axes[0].id = 13;
    let revision = cx.update(|cx| {
        publish(session, source, &absent, cx);
        handle
            .update(cx, |view, _, _| assert!(!view.focus.borrow().allows(id(3))))
            .unwrap();
        let revision = publish(session, source, &data, cx);
        handle
            .update(cx, |view, _, _| assert!(view.focus.borrow().allows(id(3))))
            .unwrap();
        revision
    });
    // Eligibility returns before repaint; the old pending down must not return.
    mouse(cx, handle, point, false);
    assert!(
        !events(transport).contains(&Click),
        "axis hide/return must retire pending click without paint"
    );
    ready(cx, handle, revision, 0xff0000ff).await;
    draw(cx, handle);
    events(transport);
    move_mouse(cx, handle, point, false);
    mouse(cx, handle, point, true);
    mouse(cx, handle, point, false);
    assert_eq!(events(transport), vec![MouseDown, MouseUp, Click]);
    // Browser open/close also changes eligibility without changing the child's
    // handler or requiring the hidden state to paint.
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.charts[&id(1)].borrow().input.focus, cx);
        })
        .unwrap();
    mouse(cx, handle, point, true);
    assert_eq!(events(transport), vec![MouseDown]);
    key(cx, handle, "d");
    handle
        .update(cx, |view, _, _| assert!(!view.focus.borrow().allows(id(3))))
        .unwrap();
    key(cx, handle, "escape");
    handle
        .update(cx, |view, _, _| assert!(view.focus.borrow().allows(id(3))))
        .unwrap();
    mouse(cx, handle, point, false);
    assert!(
        !events(transport).contains(&Click),
        "browser hide/return cannot complete an old click"
    );
    draw(cx, handle);
    move_mouse(cx, handle, point, false);
    mouse(cx, handle, point, true);
    mouse(cx, handle, point, false);
    assert_eq!(events(transport), vec![MouseDown, MouseUp, Click]);
    actions::exercise(cx, handle, source, session, transport, &data, &absent).await;
    apply(
        cx,
        handle,
        vec![
            Op::SetChart(id(1), Box::new(config(source, 0xff0000ff))),
            Op::Splice(id(1), 0, 1, vec![]),
            Op::Remove(id(4)),
            Op::Remove(id(3)),
            Op::Remove(id(2)),
        ],
    );
    let revision = cx.update(|cx| publish(session, source, original.data(), cx));
    ready(cx, handle, revision, 0xff0000ff).await;
    eprintln!(
        "GPUIO_RADAR_INPUT_REGION_OK: bubble down/up/click; same-axis publication preservation; prepaint axis/browser hide/return reject stale click; fresh input recovery"
    );
}
