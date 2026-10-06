//! Shared command routing must honor the same retained label input lifetime.
use super::*;

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
                id(6),
                Kind::CommandScope,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(6, 1).unwrap()),
            ),
            Op::SetCommands(
                id(6),
                vec![gpuio_protocol::v1::CommandConfig {
                    id: "run".into(),
                    generation: 1,
                    label: "Run radar action".into(),
                    enabled: true,
                    checked: None,
                    shortcuts: vec![],
                    target: gpuio_protocol::v1::CommandTarget::Callback,
                }],
            ),
            Op::SetStyle(
                id(6),
                vec![Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(0.)),
                    Field::Top(Length::Px(0.)),
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(20.)),
                ])],
            ),
            Op::Create(id(7), Kind::CommandButton, "".into(), None),
            Op::SetCommandRef(id(7), "run".into()),
            Op::SetStyle(
                id(7),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(20.)),
                ])],
            ),
            Op::Splice(id(6), 0, 0, vec![id(7)]),
            Op::Splice(id(3), 1, 0, vec![id(6)]),
        ],
    );
    draw(cx, handle);
    let point = handle
        .update(cx, |view, _, _| {
            assert!(view.focus.borrow().allows(id(7)));
            view.probes.borrow()[&id(7)].bounds.center()
        })
        .unwrap();
    presses(transport);
    move_mouse(cx, handle, point, false);
    mouse(cx, handle, point, true);
    mouse(cx, handle, point, false);
    assert_eq!(presses(transport), 1, "fresh native command button");
    mouse(cx, handle, point, true);
    cx.update(|cx| {
        publish(session, source, data, cx);
    });
    mouse(cx, handle, point, false);
    assert_eq!(
        presses(transport),
        1,
        "unchanged label preserves command gesture"
    );
    for paint in [false, true] {
        draw(cx, handle);
        mouse(cx, handle, point, true);
        let revision = cx.update(|cx| {
            publish(session, source, absent, cx);
            publish(session, source, data, cx)
        });
        if paint {
            ready(cx, handle, revision, 0xff0000ff).await;
            draw(cx, handle);
        }
        mouse(cx, handle, point, false);
        assert_eq!(
            presses(transport),
            0,
            "retired command gesture, replacement paint={paint}"
        );
        ready(cx, handle, revision, 0xff0000ff).await;
        draw(cx, handle);
        move_mouse(cx, handle, point, false);
        mouse(cx, handle, point, true);
        mouse(cx, handle, point, false);
        assert_eq!(presses(transport), 1, "fresh command gesture recovers");
    }
    #[cfg(target_os = "macos")]
    accessibility::exercise(cx, handle, source, session, transport, data, absent).await;
    apply(
        cx,
        handle,
        vec![
            Op::Splice(id(3), 1, 1, vec![]),
            Op::Remove(id(7)),
            Op::Remove(id(6)),
        ],
    );
    eprintln!(
        "GPUIO_RADAR_COMMAND_LIFETIME_OK: native callback command, same-axis preservation, stale gestures rejected before/after paint, recovery"
    );
}
