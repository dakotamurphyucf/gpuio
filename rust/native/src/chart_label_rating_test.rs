//! Specialized rating actions must obey the same label-exposure lifetime.
use super::*;
use gpuio_protocol::rating::{Config as Rating, Request as Intent};
use objc2::{
    msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool},
};

fn requests(transport: &Transport) -> Vec<Intent> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(1024)
        .into_iter()
        .filter_map(|e| match e {
            Event::RatingRequested(_, node, _, _, intent) if node == id(8) => Some(intent),
            Event::Press(_, node, ..) if node == id(8) => panic!("rating emitted generic Press"),
            _ => None,
        })
        .collect()
}

fn increment(target: &Retained<AnyObject>) {
    unsafe {
        let accepted: Bool = msg_send![&**target, accessibilityPerformIncrement];
        assert!(accepted.as_bool());
    }
}

async fn delivered(cx: &mut gpui::AsyncApp, transport: &Transport) {
    for _ in 0..100 {
        let events = requests(transport);
        if !events.is_empty() {
            assert_eq!(events, vec![Intent::Increase]);
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    panic!("native rating increment missing");
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    source: ResourceId,
    session: &SharedSession,
    transport: &Transport,
    data: &Data,
    absent: &Data,
) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                id(8),
                Kind::Rating,
                "".into(),
                Some(HandlerId::from_parts(8, 1).unwrap()),
            ),
            Op::SetRating(
                id(8),
                Rating {
                    label: "Radar rating".into(),
                    value: 2,
                    maximum: 5,
                    star_size: 16.,
                    disabled: false,
                    read_only: false,
                },
            ),
            Op::SetStyle(
                id(8),
                vec![Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(0.)),
                    Field::Top(Length::Px(0.)),
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(16.)),
                ])],
            ),
            Op::Splice(id(3), 1, 0, vec![id(8)]),
        ],
    );
    let target = accessibility::target_role(cx, handle, "Radar rating", "AXSlider").await;
    requests(transport);
    increment(&target);
    delivered(cx, transport).await;
    let revision = cx.update(|cx| {
        increment(&target);
        assert!(
            requests(transport).is_empty(),
            "increment must still be queued"
        );
        publish(session, source, absent, cx);
        publish(session, source, data, cx)
    });
    cx.background_executor()
        .timer(Duration::from_millis(100))
        .await;
    assert!(
        requests(transport).is_empty(),
        "rating action must not survive prepaint hide/return"
    );
    ready(cx, handle, revision, 0xff0000ff).await;
    let fresh = accessibility::target_role(cx, handle, "Radar rating", "AXSlider").await;
    increment(&fresh);
    delivered(cx, transport).await;
    draw(cx, handle);
    let point = handle
        .update(cx, |view, _, _| {
            assert!(view.focus.borrow().allows(id(8)));
            view.probes.borrow()[&id(8)].bounds.origin + gpui::point(px(8.), px(8.))
        })
        .unwrap();
    move_mouse(cx, handle, point, false);
    mouse(cx, handle, point, true);
    mouse(cx, handle, point, false);
    assert_eq!(
        requests(transport),
        vec![Intent::Toggle(1)],
        "fresh rating pointer gesture"
    );
    mouse(cx, handle, point, true);
    cx.update(|cx| {
        publish(session, source, data, cx);
    });
    mouse(cx, handle, point, false);
    assert_eq!(
        requests(transport),
        vec![Intent::Toggle(1)],
        "ordinary publication keeps rating gesture"
    );
    for repaint in [false, true] {
        draw(cx, handle);
        mouse(cx, handle, point, true);
        let revision = cx.update(|cx| {
            publish(session, source, absent, cx);
            publish(session, source, data, cx)
        });
        if repaint {
            ready(cx, handle, revision, 0xff0000ff).await;
            draw(cx, handle);
        }
        mouse(cx, handle, point, false);
        assert!(
            requests(transport).is_empty(),
            "rating gesture survived hide/return, repaint={repaint}"
        );
        ready(cx, handle, revision, 0xff0000ff).await;
        draw(cx, handle);
        move_mouse(cx, handle, point, false);
        mouse(cx, handle, point, true);
        mouse(cx, handle, point, false);
        assert_eq!(
            requests(transport),
            vec![Intent::Toggle(1)],
            "fresh rating gesture recovers"
        );
    }
    unsafe {
        let _: Bool = msg_send![&*target, accessibilityPerformIncrement];
    }
    cx.background_executor()
        .timer(Duration::from_millis(100))
        .await;
    assert!(
        requests(transport).is_empty(),
        "retired rating AX object cannot invoke fresh exposure"
    );
    // The pointer cases above deliberately retired several later exposures too.
    let newest = accessibility::target_role(cx, handle, "Radar rating", "AXSlider").await;
    increment(&newest);
    delivered(cx, transport).await;
    apply(
        cx,
        handle,
        vec![Op::Splice(id(3), 1, 1, vec![]), Op::Remove(id(8))],
    );
    eprintln!(
        "GPUIO_RADAR_RATING_OK: actual AppKit action lifetimes, native pointer preservation on ordinary updates and stale gesture rejection before/after repaint"
    );
}
