//! Pending ordinary button clicks across source-driven label hiding.
use super::*;
#[cfg(target_os = "macos")]
#[path = "chart_label_action_ax_test.rs"]
mod accessibility;
#[cfg(target_os = "macos")]
#[path = "chart_label_clip_test.rs"]
mod clipping;
#[path = "chart_label_command_test.rs"]
mod commands;
#[cfg(target_os = "macos")]
#[path = "chart_label_isolation_test.rs"]
mod isolation;

fn presses(transport: &Transport) -> usize {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(1024)
        .into_iter()
        .filter(|event| match event {
            Event::Press(_, node, ..) => *node == id(5),
            Event::CommandInvoked(_, scope, _, _, command, generation, _) => {
                *scope == id(6) && command == "run" && *generation == 1
            }
            _ => false,
        })
        .count()
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
                id(5),
                Kind::Button,
                "Run radar action".into(),
                Some(gpuio_protocol::HandlerId::from_parts(5, 1).unwrap()),
            ),
            Op::SetStyle(
                id(5),
                vec![Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(0.)),
                    Field::Top(Length::Px(0.)),
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(20.)),
                ])],
            ),
            Op::Splice(id(3), 1, 0, vec![id(5)]),
        ],
    );
    draw(cx, handle);
    let point = handle
        .update(cx, |view, _, _| {
            assert!(view.focus.borrow().allows(id(5)));
            view.probes.borrow()[&id(5)].bounds.center()
        })
        .unwrap();
    presses(transport);
    move_mouse(cx, handle, point, false);
    mouse(cx, handle, point, true);
    mouse(cx, handle, point, false);
    assert_eq!(presses(transport), 1, "fresh label button click");
    mouse(cx, handle, point, true);
    cx.update(|cx| {
        publish(session, source, data, cx);
    });
    mouse(cx, handle, point, false);
    assert_eq!(
        presses(transport),
        1,
        "same-axis publication preserves button down"
    );
    mouse(cx, handle, point, true);
    let revision = cx.update(|cx| {
        publish(session, source, absent, cx);
        handle
            .update(cx, |view, _, _| assert!(!view.focus.borrow().allows(id(5))))
            .unwrap();
        let revision = publish(session, source, data, cx);
        handle
            .update(cx, |view, _, _| assert!(view.focus.borrow().allows(id(5))))
            .unwrap();
        revision
    });
    mouse(cx, handle, point, false);
    assert_eq!(
        presses(transport),
        0,
        "hidden label button must reject an old pending click after return"
    );
    ready(cx, handle, revision, 0xff0000ff).await;
    draw(cx, handle);
    presses(transport);
    move_mouse(cx, handle, point, false);
    mouse(cx, handle, point, true);
    mouse(cx, handle, point, false);
    assert_eq!(presses(transport), 1, "fresh label button recovers");
    // A replacement frame must not inherit GPUI's old pending-down state either.
    mouse(cx, handle, point, true);
    let revision = cx.update(|cx| {
        publish(session, source, absent, cx);
        publish(session, source, data, cx)
    });
    ready(cx, handle, revision, 0xff0000ff).await;
    draw(cx, handle);
    mouse(cx, handle, point, false);
    assert_eq!(
        presses(transport),
        0,
        "new frame cannot inherit the retired button down"
    );
    #[cfg(target_os = "macos")]
    clipping::exercise(cx, handle, transport).await;
    #[cfg(target_os = "macos")]
    accessibility::exercise(cx, handle, source, session, transport, data, absent).await;
    apply(
        cx,
        handle,
        vec![Op::Splice(id(3), 1, 1, vec![]), Op::Remove(id(5))],
    );
    commands::exercise(cx, handle, source, session, transport, data, absent).await;
    #[cfg(target_os = "macos")]
    isolation::exercise(cx, handle, session, transport, data, absent).await;
    eprintln!(
        "GPUIO_RADAR_BUTTON_LIFETIME_OK: native click, same-axis preservation, prepaint hide/return rejects old click and fresh input recovers"
    );
}
