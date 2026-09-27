//! Retained rows, hidden panels and cross-window program ownership.
use super::*;
use gpuio_protocol::list::{
    Config as ListConfig, IdRun, Order, Row, ScrollPolicy, ScrollRequest, ScrollTarget,
};

pub(super) fn open(cx: &mut gpui::AsyncApp, transport: &Arc<Transport>) -> WindowHandle<View> {
    cx.update(|cx| {
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, wid(), "Retained motion lifecycle", 360., 220.)
            .unwrap();
        session
            .borrow()
            .motion()
            .borrow_mut()
            .set_test_time(Some(Duration::ZERO));
        crate::motion_preference::bind_clocks(&session.borrow().motion(), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(360.), px(220.)),
                        cx,
                    ))),
                    focus: true,
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(wid(), session.clone(), transport.clone())),
            )
            .unwrap();
        cx.activate(true);
        window
    })
}
fn row_config(generation: i64) -> Config {
    let mut c = config(generation);
    c.program.stages.truncate(1);
    c
}
fn scroll(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, serial: i64, row: i64) {
    apply(
        cx,
        window,
        vec![Op::ScrollList(
            node(0),
            ScrollRequest {
                serial,
                target: ScrollTarget::Offset(row, 0.),
            },
        )],
    );
}
pub(super) async fn rows(cx: &mut gpui::AsyncApp, transport: &Arc<Transport>) {
    let window = open(cx, transport);
    let mut ops = vec![
        Op::Create(node(0), Kind::VirtualList, "".into(), None),
        Op::SetListConfig(
            node(0),
            ListConfig {
                estimated_height: 40.,
                overscan: 0.,
                max_active: 64,
                scroll_policy: ScrollPolicy::KeepPosition,
                scrollbar: false,
                managed: false,
            },
        ),
        Op::SetListOrder(
            node(0),
            Order {
                revision: 1,
                runs: vec![IdRun {
                    first: 1,
                    count: 32,
                }],
            },
        ),
        Op::SetStyle(
            node(0),
            vec![
                Style::Width(Length::Px(320.)),
                Style::Height(Length::Px(160.)),
            ],
        ),
    ];
    for i in 1..=32 {
        ops.extend([
            Op::Create(
                node(i),
                Kind::AnimationProgram,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(i, 1).unwrap()),
            ),
            Op::SetAnimationProgram(node(i), row_config(1)),
            Op::SetStyle(
                node(i),
                vec![Style::Fields(vec![
                    Field::Height(Length::Px(40.)),
                    Field::Shrink(0.),
                ])],
            ),
        ]);
    }
    ops.extend([
        Op::SetListRows(
            node(0),
            (1..=32)
                .map(|i| Row {
                    id: i,
                    node: node(i),
                })
                .collect(),
        ),
        Op::Splice(node(0), 0, 0, (1..=32).map(node).collect()),
        Op::SetRoot(Some(node(0))),
    ]);
    apply(cx, window, ops);
    at(cx, window, 500).await;
    assert!((width(cx, window, 1) - 60.).abs() < 0.1);
    scroll(cx, window, 1, 20);
    frame(cx, window).await;
    let paints = window
        .update(cx, |v, _, _| {
            v.animation_programs[&node(1)].borrow().paint_count
        })
        .unwrap();
    at(cx, window, 1500).await;
    assert_eq!(
        window
            .update(cx, |v, _, _| v.animation_programs[&node(1)]
                .borrow()
                .paint_count)
            .unwrap(),
        paints,
        "offscreen row did not paint"
    );
    scroll(cx, window, 2, 1);
    frame(cx, window).await;
    let resumed = width(cx, window, 1);
    assert!(
        (resumed - 60.).abs() < 0.1,
        "offscreen retained row resumes at its painted value: {resumed}"
    );
    at(cx, window, 1750).await;
    assert!((width(cx, window, 1) - 80.).abs() < 0.1);
    let mut delayed = row_config(2);
    delayed.program.delay_ms = 500;
    delayed.program.stages[0].targets[0].value = 180.;
    apply(cx, window, vec![Op::SetAnimationProgram(node(1), delayed)]);
    frame(cx, window).await;
    assert!(
        window
            .update(cx, |v, _, _| v.animation_programs[&node(1)]
                .borrow()
                .has_deadline())
            .unwrap()
    );
    scroll(cx, window, 3, 20);
    frame(cx, window).await;
    assert!(
        !window
            .update(cx, |v, _, _| v.animation_programs[&node(1)]
                .borrow()
                .has_deadline())
            .unwrap(),
        "virtualization cancels delayed work"
    );
    at(cx, window, 3000).await;
    scroll(cx, window, 4, 1);
    frame(cx, window).await;
    assert!((width(cx, window, 1) - 80.).abs() < 0.1);
    assert!(
        window
            .update(cx, |v, _, _| v.animation_programs[&node(1)]
                .borrow()
                .has_deadline())
            .unwrap(),
        "visible row re-arms remaining delay"
    );
    let store = window
        .update(cx, |v, w, _| {
            let store = v.session.borrow().motion();
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
            store
        })
        .unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(80))
        .await;
    assert_eq!(store.borrow().counts(), (0, 0, 0));
    signals(transport);
    eprintln!(
        "GPUIO_ANIMATION_ROWS_OK: offscreen retained rows pause, cancel deadlines, resume remaining delay and release"
    );
}

pub(super) async fn panels(cx: &mut gpui::AsyncApp, transport: &Arc<Transport>) {
    let window = open(cx, transport);
    apply(
        cx,
        window,
        vec![
            Op::Create(node(0), Kind::TabPanel, "Retained panel".into(), None),
            Op::Create(node(1), Kind::AnimationProgram, "".into(), None),
            Op::SetAnimationProgram(node(1), row_config(1)),
            Op::SetStyle(node(1), vec![Style::Height(Length::Px(40.))]),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    at(cx, window, 250).await;
    assert!((width(cx, window, 1) - 40.).abs() < 0.1);
    let retained = window
        .update(cx, |v, _, _| Rc::downgrade(&v.animation_programs[&node(1)]))
        .unwrap();
    for (index, field) in [Field::Display(3), Field::Visibility(1)]
        .into_iter()
        .enumerate()
    {
        let hidden_at = 250 + index as u64 * 1000;
        apply(
            cx,
            window,
            vec![Op::SetStyle(node(0), vec![Style::Fields(vec![field])])],
        );
        frame(cx, window).await;
        at(cx, window, hidden_at + 1000).await;
        let before = window.update(cx, |v, _, _| v.render_count).unwrap();
        cx.background_executor()
            .timer(Duration::from_millis(120))
            .await;
        assert_eq!(
            before,
            window.update(cx, |v, _, _| v.render_count).unwrap(),
            "hidden panel does not request frames"
        );
        apply(cx, window, vec![Op::SetStyle(node(0), vec![])]);
        frame(cx, window).await;
        assert!(
            (width(cx, window, 1) - 40.).abs() < 0.1,
            "panel resumes preserved active time"
        );
        assert!(retained.upgrade().is_some());
    }
    at(cx, window, 2500).await;
    assert!((width(cx, window, 1) - 60.).abs() < 0.1);
    apply(
        cx,
        window,
        vec![
            Op::Create(
                node(2),
                Kind::FocusScope,
                "".into(),
                Some(gpuio_protocol::HandlerId::from_parts(2, 1).unwrap()),
            ),
            Op::SetFocusScope(
                node(2),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::SetOverlay(
                node(2),
                Some(OverlayConfig {
                    kind: OverlayKind::Dialog,
                    label: "Animated overlay".into(),
                    width: 220.,
                    dismiss_on_escape: true,
                    dismiss_on_outside_pointer: true,
                }),
            ),
            Op::Create(node(3), Kind::AnimationProgram, "".into(), None),
            Op::SetAnimationProgram(node(3), row_config(1)),
            Op::SetStyle(node(3), vec![Style::Height(Length::Px(40.))]),
            Op::Splice(node(2), 0, 0, vec![node(3)]),
            Op::Splice(node(0), 1, 0, vec![node(2)]),
        ],
    );
    at(cx, window, 2750).await;
    assert!((width(cx, window, 3) - 40.).abs() < 0.1);
    at(cx, window, 3000).await;
    assert!(
        (width(cx, window, 3) - 60.).abs() < 0.1,
        "deferred overlay paint participates in visibility tracking"
    );
    window
        .update(cx, |v, w, _| {
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
        })
        .unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(80))
        .await;
    assert!(retained.upgrade().is_none());
    signals(transport);
    eprintln!(
        "GPUIO_ANIMATION_PANELS_OK: display/visibility-hidden panels pause; deferred overlays commit motion"
    );
}

fn window_id(slot: i64, generation: i64) -> WindowId {
    WindowId::from_parts(slot, generation).unwrap()
}
fn open_shared(
    cx: &mut gpui::AsyncApp,
    session: &SharedSession,
    transport: &Arc<Transport>,
    id: WindowId,
) -> WindowHandle<View> {
    cx.update(|cx| {
        session
            .borrow_mut()
            .open(1, id, "Shared motion", 320., 180.)
            .unwrap();
        let handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::new(
                        gpui::point(px(40. + id.slot() as f32 * 380.), px(60.)),
                        size(px(320.), px(180.)),
                    ))),
                    focus: true,
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(id, session.clone(), transport.clone())),
            )
            .unwrap();
        cx.activate(true);
        handle
    })
}
fn shared_config(generation: i64, clock: Clock) -> Config {
    let mut c = row_config(generation);
    c.program.repeat = Repeat::Alternate;
    c.program.clock = clock;
    c
}
fn mount_shared(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, clock: Clock) {
    apply(
        cx,
        window,
        vec![
            Op::Create(node(0), Kind::AnimationProgram, "".into(), None),
            Op::SetAnimationProgram(node(0), shared_config(1, clock)),
            Op::SetStyle(node(0), vec![Style::Height(Length::Px(40.))]),
            Op::SetRoot(Some(node(0))),
        ],
    );
}
pub(super) async fn windows(cx: &mut gpui::AsyncApp, transport: &Arc<Transport>) {
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    let store = session.borrow().motion();
    store.borrow_mut().set_test_time(Some(Duration::ZERO));
    cx.update(|cx| crate::motion_preference::bind_clocks(&store, cx));
    let group = || Clock::Group("cross-window".into());
    let first = open_shared(cx, &session, transport, window_id(0, 1));
    mount_shared(cx, first, group());
    at(cx, first, 250).await;
    assert!((width(cx, first, 0) - 40.).abs() < 0.1);
    let second = open_shared(cx, &session, transport, window_id(1, 1));
    mount_shared(cx, second, group());
    frame(cx, second).await;
    assert!((width(cx, second, 0) - 40.).abs() < 0.1);
    at(cx, first, 500).await;
    frame(cx, second).await;
    assert!((width(cx, first, 0) - width(cx, second, 0)).abs() < 0.1);
    let mut paused = shared_config(2, group());
    paused.playback = Playback::Paused;
    apply(cx, first, vec![Op::SetAnimationProgram(node(0), paused)]);
    at(cx, second, 750).await;
    frame(cx, first).await;
    assert!((width(cx, first, 0) - 60.).abs() < 0.1);
    assert!((width(cx, second, 0) - 80.).abs() < 0.1);
    apply(
        cx,
        first,
        vec![Op::SetAnimationProgram(node(0), shared_config(3, group()))],
    );
    frame(cx, first).await;
    assert!(
        (width(cx, first, 0) - 80.).abs() < 0.1,
        "resume rejoins shared phase"
    );
    let old_group = store.borrow().clocks.group_id("cross-window").unwrap();
    let snapshot = store
        .borrow_mut()
        .clocks
        .sample(window_id(0, 1), node(0), Duration::from_millis(750))
        .unwrap();
    let first_owner = first
        .update(cx, |v, w, _| {
            let weak = Rc::downgrade(&v.animation_programs[&node(0)]);
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
            weak
        })
        .unwrap();
    at(cx, second, 1000).await;
    assert!(first_owner.upgrade().is_none());
    assert!(snapshot.is_current());
    assert_eq!(store.borrow().counts().0, 1);
    assert_eq!(store.borrow().counts().1, 1);
    assert!((width(cx, second, 0) - 100.).abs() < 0.1);
    second
        .update(cx, |v, w, _| {
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
        })
        .unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(80))
        .await;
    assert!(!snapshot.is_current());
    assert_eq!(store.borrow().counts(), (0, 0, 0));
    store
        .borrow_mut()
        .set_test_time(Some(Duration::from_millis(1250)));
    let reopened = open_shared(cx, &session, transport, window_id(0, 2));
    mount_shared(cx, reopened, group());
    at(cx, reopened, 1500).await;
    assert!(
        (width(cx, reopened, 0) - 40.).abs() < 0.1,
        "new group lifetime starts fresh"
    );
    assert!(!store.borrow_mut().clocks.set_group_paused(
        &old_group,
        true,
        Duration::from_millis(1500)
    ));
    apply(
        cx,
        reopened,
        vec![Op::SetAnimationProgram(
            node(0),
            shared_config(2, Clock::Application),
        )],
    );
    frame(cx, reopened).await;
    assert!(
        (width(cx, reopened, 0) - 60.).abs() < 0.1,
        "application phase survives the gap with no windows"
    );
    reopened
        .update(cx, |v, w, _| {
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
        })
        .unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(80))
        .await;
    assert_eq!(store.borrow().counts(), (0, 0, 0));
    signals(transport);
    eprintln!(
        "GPUIO_ANIMATION_WINDOWS_OK: actual late-joining windows, individual pause/resume, group lifetime and application phase"
    );
}
