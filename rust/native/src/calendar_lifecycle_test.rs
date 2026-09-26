//! Native modal/visibility policy, managed-row pins and independent windows.
use super::*;
use gpuio_protocol::{
    list::{Config, IdRun, Order, Retained, Row, ScrollPolicy},
    v1::{FocusScopeConfig, Length},
};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn command(cx: &mut AsyncApp, handle: WindowHandle<View>, command: c::Command) -> c::Response {
    handle
        .update(cx, |v, w, cx| v.calendars[&node()].command(&command, w, cx))
        .unwrap()
}
fn accepted(response: c::Response) -> c::Snapshot {
    match response {
        c::Response::Applied(snapshot) => snapshot,
        other => panic!("calendar lifecycle command: {other:?}"),
    }
}
fn palette(pointer: bool) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Background(Fill::Solid(Color::Rgba(0xf4f6faff))),
        Field::Foreground(Color::Rgba(0x182332ff)),
        Field::PointerEvents(pointer),
    ])]
}
fn list_style() -> Vec<Style> {
    vec![
        Style::Width(Length::Px(320.)),
        Style::Height(Length::Px(350.)),
    ]
}

pub(super) async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    apply(
        cx,
        handle,
        vec![
            set(c::Config {
                auto_focus: false,
                ..config()
            }),
            Op::SetStyle(node(), palette(false)),
        ],
    );
    frame(cx, handle).await;
    accepted(command(
        cx,
        handle,
        c::Command::FocusDate(date(2024, 2, 29)),
    ));
    frame(cx, handle).await;
    let before = snapshot(cx, handle);
    click(cx, handle, 40., 24.).await; // Previous month, while pointer events are disabled.
    assert_eq!(snapshot(cx, handle), before);
    apply(cx, handle, vec![Op::SetStyle(node(), palette(true))]);
    frame(cx, handle).await;
    click(cx, handle, 40., 24.).await;
    assert_eq!(snapshot(cx, handle).month, c::Month::new(2024, 1).unwrap());
    assert_eq!(snapshot(cx, handle).selection, before.selection);
    let entity = handle
        .update(cx, |v, _, _| v.calendars[&node()].state.entity_id())
        .unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                id(1),
                Kind::VirtualList,
                "".into(),
                Some(HandlerId::from_parts(10, 1).unwrap()),
            ),
            Op::SetListConfig(
                id(1),
                Config {
                    estimated_height: 340.,
                    overscan: 0.,
                    max_active: 4,
                    scroll_policy: ScrollPolicy::KeepPosition,
                    scrollbar: true,
                    managed: true,
                },
            ),
            Op::SetListOrder(
                id(1),
                Order {
                    revision: 1,
                    runs: vec![IdRun { first: 7, count: 1 }],
                },
            ),
            Op::SetStyle(id(1), list_style()),
            Op::SetListRows(
                id(1),
                vec![Row {
                    id: 7,
                    node: node(),
                }],
            ),
            Op::Splice(id(1), 0, 0, vec![node()]),
            Op::SetRoot(Some(id(1))),
        ],
    );
    frame(cx, handle).await;
    accepted(command(cx, handle, c::Command::Focus));
    frame(cx, handle).await;
    handle
        .update(cx, |v, w, cx| {
            let pins = v.list_pins(w, cx);
            assert_eq!(
                pins,
                vec![Retained {
                    node: id(1),
                    rows: vec![7]
                }]
            );
            let base = v.session.borrow().tree(v.id).unwrap().revision();
            let result = v.session.borrow_mut().apply_guarded(
                &Transaction {
                    window: v.id,
                    base,
                    revision: base + 1,
                    operations: vec![
                        Op::Remove(node()),
                        Op::Splice(id(1), 0, 1, vec![]),
                        Op::SetListRows(id(1), vec![]),
                    ],
                },
                &pins,
            );
            assert!(matches!(
                result,
                Err(crate::tree::ApplyFailure::Retained(_))
            ));
            assert_eq!(v.session.borrow().tree(v.id).unwrap().revision(), base);
        })
        .unwrap();
    let before = snapshot(cx, handle);
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id(1),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    frame(cx, handle).await;
    frame(cx, handle).await;
    assert!(!snapshot(cx, handle).focused);
    assert_eq!(
        command(cx, handle, c::Command::Focus),
        c::Response::Failed(c::Error::FocusBlocked)
    );
    handle
        .update(cx, |v, w, cx| assert!(v.list_pins(w, cx).is_empty()))
        .unwrap();
    key(cx, handle, "enter");
    // Simulate a queued old native callback, in addition to dispatching a real key.
    handle
        .update(cx, |v, _, cx| {
            v.calendars[&node()]
                .state
                .update(cx, |s, cx| s.act(Action::Activate(date(2024, 1, 10)), cx))
        })
        .unwrap();
    assert_eq!(snapshot(cx, handle).selection, before.selection);
    accepted(command(
        cx,
        handle,
        c::Command::Replace {
            selection: c::Selection::Single(date(2024, 1, 8)),
            if_revision: None,
        },
    ));
    apply(cx, handle, vec![Op::SetStyle(id(1), list_style())]);
    frame(cx, handle).await;
    accepted(command(cx, handle, c::Command::Focus));
    frame(cx, handle).await;
    apply(
        cx,
        handle,
        vec![
            Op::Create(id(2), Kind::Container, "".into(), None),
            Op::Create(id(3), Kind::FocusScope, "".into(), None),
            Op::SetFocusScope(
                id(3),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::Create(
                id(4),
                Kind::Button,
                "Modal action".into(),
                Some(HandlerId::from_parts(13, 1).unwrap()),
            ),
            Op::Splice(id(3), 0, 0, vec![id(4)]),
            Op::Splice(id(2), 0, 0, vec![id(1), id(3)]),
            Op::SetRoot(Some(id(2))),
        ],
    );
    frame(cx, handle).await;
    frame(cx, handle).await;
    assert_eq!(
        command(cx, handle, c::Command::Focus),
        c::Response::Failed(c::Error::FocusBlocked)
    );
    let blocked = snapshot(cx, handle);
    handle
        .update(cx, |v, _, cx| {
            v.calendars[&node()]
                .state
                .update(cx, |s, cx| s.act(Action::Activate(date(2024, 1, 10)), cx))
        })
        .unwrap();
    click(cx, handle, 40., 24.).await;
    assert_eq!(snapshot(cx, handle).selection, blocked.selection);
    assert_eq!(snapshot(cx, handle).month, blocked.month);
    apply(
        cx,
        handle,
        vec![
            Op::Splice(id(2), 0, 2, vec![]),
            Op::SetRoot(Some(id(1))),
            Op::Remove(id(4)),
            Op::Remove(id(3)),
            Op::Remove(id(2)),
        ],
    );
    frame(cx, handle).await;
    accepted(command(cx, handle, c::Command::Focus));
    assert_eq!(
        handle
            .update(cx, |v, _, _| v.calendars[&node()].state.entity_id())
            .unwrap(),
        entity
    );
    apply(
        cx,
        handle,
        vec![
            Op::SetListRows(id(1), vec![]),
            Op::Splice(id(1), 0, 1, vec![]),
            Op::SetRoot(Some(node())),
            Op::Remove(id(1)),
        ],
    );
    frame(cx, handle).await;
    events(transport);
    independent_window(cx, handle, transport).await;
    eprintln!(
        "GPUIO_CALENDAR_LIFECYCLE_OK: pointer policy, managed-row pins/atomic rejection, hidden/modal gates, stale callbacks and independent-window close"
    );
}

async fn independent_window(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    let original = snapshot(cx, handle);
    let (session, bridge) = handle
        .update(cx, |v, _, _| (v.session.clone(), v.transport.clone()))
        .unwrap();
    let other_id = WindowId::from_parts(1, 1).unwrap();
    session
        .borrow_mut()
        .open(2, other_id, "Independent calendar", 340., 360.)
        .unwrap();
    let other = cx.update(|cx| {
        cx.open_window(
            WindowOptions {
                inactive_frame_interval: None,
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(340.), px(360.)),
                    cx,
                ))),
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(other_id, session.clone(), bridge.clone())),
        )
        .unwrap()
    });
    apply(
        cx,
        other,
        vec![
            Op::Create(node(), Kind::Calendar, "".into(), Some(handler())),
            set(config()),
            Op::SetStyle(node(), palette(true)),
            Op::SetRoot(Some(node())),
        ],
    );
    frame(cx, other).await;
    frame(cx, other).await;
    accepted(command(cx, other, c::Command::FocusDate(date(2030, 7, 15))));
    frame(cx, other).await;
    events(transport);
    key(cx, other, "enter");
    frame(cx, other).await;
    let output = transport.mailbox.lock().unwrap().drain(128);
    assert!(matches!(output.as_slice(),[
        Event::CalendarEvent(w1,_,_,_,c::Event::Changed(_)),
        Event::CalendarEvent(w2,_,_,_,c::Event::Selected(_))
    ] if *w1==other_id && *w2==other_id));
    assert_eq!(
        snapshot(cx, other).selection,
        c::Selection::Single(date(2030, 7, 15))
    );
    assert_eq!(snapshot(cx, handle).selection, original.selection);
    let weak = other
        .update(cx, |v, w, _| {
            let weak = v.calendars[&node()].state.downgrade();
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
            weak
        })
        .unwrap();
    frame(cx, handle).await;
    assert!(weak.upgrade().is_none());
    assert!(session.borrow().tree(other_id).is_none());
    accepted(command(cx, handle, c::Command::FocusDate(date(2040, 1, 2))));
    frame(cx, handle).await;
    events(transport);
    key(cx, handle, "enter");
    frame(cx, handle).await;
    assert_eq!(
        snapshot(cx, handle).selection,
        c::Selection::Single(date(2040, 1, 2))
    );
    assert!(matches!(
        events(transport).as_slice(),
        [c::Event::Changed(_), c::Event::Selected(_)]
    ));
}
