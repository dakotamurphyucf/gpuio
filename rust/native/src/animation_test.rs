//! Actual GPUI geometry and scheduling with deterministic samples plus a realtime run.
use super::*;
use gpuio_protocol::animation::{CancelReason, Config, Easing, Outcome, Property, Repeat, Target};
use std::{
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    time::Duration,
};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn wid() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn handler() -> gpuio_protocol::HandlerId {
    gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()
}
fn config(generation: i64, width: f64) -> Config {
    Config {
        generation,
        targets: vec![Target {
            property: Property::Width,
            value: width,
        }],
        initial: Some(vec![Target {
            property: Property::Width,
            value: 0.,
        }]),
        duration_ms: 1000,
        delay_ms: 0,
        easing: Easing::Linear,
        repeat: Repeat::Once,
    }
}
fn apply(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, operations: Vec<Op>) {
    window
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let applied = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: wid(),
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
        .unwrap();
}
async fn frame(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    super::editor_test::frame(cx, window).await;
    super::editor_test::frame(cx, window).await;
}
async fn at(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, time: u64) {
    window
        .update(cx, |view, window, _| {
            assert_eq!(view.animations.len(), 1);
            view.animations
                .values()
                .next()
                .unwrap()
                .borrow_mut()
                .set_test_time(Some(Duration::from_millis(time)));
            window.refresh();
        })
        .unwrap();
    frame(cx, window).await;
}
fn width(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, slot: i64) -> f32 {
    window
        .update(cx, |view, _, _| {
            f32::from(view.probes.borrow()[&node(slot)].bounds.size.width)
        })
        .unwrap()
}
fn events(transport: &Transport) -> Vec<gpuio_protocol::animation::Endpoint> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| {
            if let Event::AnimationEndpoint(_, _, _, _, endpoint) = event {
                Some(endpoint)
            } else {
                None
            }
        })
        .collect()
}
async fn exercise(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, transport: &Transport) {
    apply(
        cx,
        window,
        vec![
            Op::Create(node(0), Kind::Container, "".into(), None),
            Op::Create(node(1), Kind::Animated, "".into(), Some(handler())),
            Op::SetAnimation(node(1), config(1, 240.)),
            Op::SetStyle(
                node(1),
                vec![Style::Fields(vec![
                    Field::Height(Length::Px(60.)),
                    Field::Width(Length::Px(999.)),
                    Field::OverflowX(2),
                    Field::Shrink(0.),
                ])],
            ),
            Op::Create(
                node(2),
                Kind::Text,
                "Fixed-width sidebar content wraps at its own width".into(),
                None,
            ),
            Op::SetStyle(
                node(2),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(240.)),
                    Field::Shrink(0.),
                ])],
            ),
            Op::Splice(node(1), 0, 0, vec![node(2)]),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    at(cx, window, 0).await;
    assert_eq!(width(cx, window, 1), 0.);
    at(cx, window, 500).await;
    assert_eq!(
        width(cx, window, 1),
        120.,
        "animation owns width over static style"
    );
    assert_eq!(
        width(cx, window, 2),
        240.,
        "inner content does not reflow with reveal"
    );
    assert!(events(transport).is_empty());
    apply(cx, window, vec![Op::SetAnimation(node(1), config(2, 40.))]);
    at(cx, window, 500).await;
    assert_eq!(
        width(cx, window, 1),
        120.,
        "interruption retains last painted width"
    );
    assert_eq!(
        events(transport)[0].outcome,
        Outcome::Cancelled(CancelReason::Replaced)
    );
    at(cx, window, 1000).await;
    assert_eq!(width(cx, window, 1), 80.);
    at(cx, window, 1500).await;
    assert_eq!(width(cx, window, 1), 40.);
    let endpoints = events(transport);
    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].generation, 2);
    assert_eq!(endpoints[0].outcome, Outcome::Finished);
    let revision = window
        .update(cx, |view, _, _| {
            view.session.borrow().tree(wid()).unwrap().revision()
        })
        .unwrap();
    at(cx, window, 2000).await;
    assert!(events(transport).is_empty());
    assert_eq!(
        window
            .update(cx, |view, _, _| view
                .session
                .borrow()
                .tree(wid())
                .unwrap()
                .revision())
            .unwrap(),
        revision
    );
    let stopped = window.update(cx, |view, _, _| view.render_count).unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(180))
        .await;
    assert_eq!(
        window.update(cx, |view, _, _| view.render_count).unwrap(),
        stopped,
        "finished animation does not keep redrawing"
    );
    // A real-clock repeat must request frames itself, then stop after removal.
    window
        .update(cx, |view, _, _| {
            view.animations[&node(1)].borrow_mut().set_test_time(None)
        })
        .unwrap();
    let mut repeated = config(3, 240.);
    repeated.duration_ms = 80;
    repeated.repeat = Repeat::Alternate;
    apply(cx, window, vec![Op::SetAnimation(node(1), repeated)]);
    frame(cx, window).await;
    let before = window
        .update(cx, |view, _, _| {
            view.animations[&node(1)].borrow().paint_count
        })
        .unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(150))
        .await;
    let after = window
        .update(cx, |view, _, _| {
            view.animations[&node(1)].borrow().paint_count
        })
        .unwrap();
    assert!(
        after > before + 1,
        "native repeat autonomously schedules frames"
    );
    assert!(events(transport).is_empty());
    apply(
        cx,
        window,
        vec![Op::SetStyle(
            node(0),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    frame(cx, window).await;
    let hidden = window.update(cx, |view, _, _| view.render_count).unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(180))
        .await;
    assert_eq!(
        window.update(cx, |view, _, _| view.render_count).unwrap(),
        hidden,
        "hidden ancestor stops repeat redraws"
    );
    apply(cx, window, vec![Op::SetStyle(node(0), vec![])]);
    frame(cx, window).await;
    assert!(window.update(cx, |view, _, _| view.render_count).unwrap() > hidden);
    cx.update(|cx| {
        crate::motion_preference::set(gpuio_protocol::animation::Preference::Reduce, cx)
    });
    window.update(cx, |_, window, _| window.refresh()).unwrap();
    frame(cx, window).await;
    assert_eq!(
        width(cx, window, 1),
        0.,
        "reduced repeat uses initial placement"
    );
    let reduced = window.update(cx, |view, _, _| view.render_count).unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(180))
        .await;
    assert_eq!(
        window.update(cx, |view, _, _| view.render_count).unwrap(),
        reduced,
        "reduced repeat does not redraw"
    );
    let mut finite = config(4, 80.);
    finite.delay_ms = 500;
    apply(cx, window, vec![Op::SetAnimation(node(1), finite)]);
    frame(cx, window).await;
    assert_eq!(
        width(cx, window, 1),
        80.,
        "reduced one-shot skips delay and settles"
    );
    let outcomes = events(transport);
    assert_eq!(outcomes.len(), 2);
    assert_eq!(outcomes[0].generation, 3);
    assert_eq!(
        outcomes[0].outcome,
        Outcome::Cancelled(CancelReason::Replaced)
    );
    assert_eq!(outcomes[1].generation, 4);
    assert_eq!(outcomes[1].outcome, Outcome::Finished);
    cx.update(|cx| crate::motion_preference::set(gpuio_protocol::animation::Preference::Full, cx));
    let mut delayed = config(5, 140.);
    delayed.delay_ms = 500;
    apply(cx, window, vec![Op::SetAnimation(node(1), delayed)]);
    frame(cx, window).await;
    assert!(
        window
            .update(cx, |view, _, _| view.animations[&node(1)]
                .borrow()
                .has_deadline())
            .unwrap()
    );
    let owner = window
        .update(cx, |view, _, _| Rc::downgrade(&view.animations[&node(1)]))
        .unwrap();
    apply(
        cx,
        window,
        vec![
            Op::SetRoot(None),
            Op::Remove(node(2)),
            Op::Remove(node(1)),
            Op::Remove(node(0)),
        ],
    );
    assert!(
        owner.upgrade().is_none(),
        "old frames and timers do not retain the animation"
    );
    frame(cx, window).await;
    assert!(events(transport).is_empty(), "no callbacks after disposal");
    eprintln!(
        "GPUIO_NATIVE_ANIMATION_OK: reveal geometry, fixed inner width, interruption, once-only endpoints, native repeat frames, hidden/reduced idle, delayed-start disposal"
    );
}
async fn geometry_and_close(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    transport: &Transport,
) {
    fn node(slot: i64) -> NodeId {
        NodeId::from_parts(slot, 2).unwrap()
    }
    let mut geometry = config(1, 120.);
    geometry.targets.extend([
        Target {
            property: Property::Height,
            value: 80.,
        },
        Target {
            property: Property::Top,
            value: 20.,
        },
        Target {
            property: Property::Left,
            value: 30.,
        },
        Target {
            property: Property::Opacity,
            value: 0.5,
        },
        Target {
            property: Property::TopLeftRadius,
            value: 4.,
        },
        Target {
            property: Property::TopRightRadius,
            value: 8.,
        },
        Target {
            property: Property::BottomLeftRadius,
            value: 12.,
        },
        Target {
            property: Property::BottomRightRadius,
            value: 16.,
        },
    ]);
    geometry.initial = Some(
        geometry
            .targets
            .iter()
            .map(|target| Target {
                property: target.property,
                value: 0.,
            })
            .collect(),
    );
    apply(
        cx,
        window,
        vec![
            Op::Create(node(0), Kind::Container, "".into(), None),
            Op::SetStyle(
                node(0),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(400.)),
                    Field::Height(Length::Px(200.)),
                ])],
            ),
            Op::Create(node(1), Kind::Animated, "".into(), Some(handler())),
            Op::SetAnimation(node(1), geometry.clone()),
            Op::SetStyle(node(1), vec![Style::Fields(vec![Field::Position(1)])]),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    at(cx, window, 0).await;
    assert!(
        window
            .update(cx, |view, _, _| view.animations[&node(1)]
                .borrow()
                .paint_count)
            .unwrap()
            > 0,
        "zero-area and zero-opacity placement can schedule its next native frame"
    );
    at(cx, window, 500).await;
    window
        .update(cx, |view, _, _| {
            let probes = view.probes.borrow();
            let root = probes[&node(0)].bounds;
            let animated = probes[&node(1)].bounds;
            assert_eq!(animated.size, size(px(60.), px(40.)));
            assert_eq!(animated.origin - root.origin, gpui::point(px(15.), px(10.)));
        })
        .unwrap();
    at(cx, window, 1000).await;
    assert_eq!(events(transport).len(), 1);
    // Exercise bottom/right without over-constraining the absolute rectangle.
    geometry.generation = 2;
    geometry
        .targets
        .retain(|target| !matches!(target.property, Property::Top | Property::Left));
    geometry.targets.extend([
        Target {
            property: Property::Right,
            value: 40.,
        },
        Target {
            property: Property::Bottom,
            value: 30.,
        },
    ]);
    geometry.targets.sort_by_key(|target| target.property);
    geometry.initial = None;
    geometry.duration_ms = 0;
    apply(
        cx,
        window,
        vec![Op::SetAnimation(node(1), geometry.clone())],
    );
    at(cx, window, 1000).await;
    window
        .update(cx, |view, _, _| {
            let probes = view.probes.borrow();
            let root = probes[&node(0)].bounds;
            let animated = probes[&node(1)].bounds;
            assert_eq!(animated.size, size(px(120.), px(80.)));
            assert_eq!(
                animated.origin - root.origin,
                gpui::point(px(240.), px(90.))
            );
        })
        .unwrap();
    assert_eq!(events(transport).len(), 1);
    geometry.generation = 3;
    geometry.targets[0].value = 100.;
    geometry.duration_ms = 1000;
    geometry.delay_ms = 60_000;
    apply(cx, window, vec![Op::SetAnimation(node(1), geometry)]);
    at(cx, window, 1000).await;
    let owner = window
        .update(cx, |view, _, _| {
            assert!(view.animations[&node(1)].borrow().has_deadline());
            Rc::downgrade(&view.animations[&node(1)])
        })
        .unwrap();
    window
        .update(cx, |view, window, _| {
            view.session.borrow_mut().close(view.id).unwrap();
            window.remove_window();
        })
        .unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(80))
        .await;
    assert!(
        owner.upgrade().is_none(),
        "closing a window releases its delayed animation and timer"
    );
    assert!(
        events(transport).is_empty(),
        "closed windows discard animation endpoints"
    );
    eprintln!(
        "GPUIO_ANIMATION_GEOMETRY_CLOSE_OK: zero-area scheduling, two-axis geometry, anchored offsets and delayed close cleanup"
    );
}
pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _read = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let write = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(write.as_raw_fd()).unwrap());
    eprintln!("GPUIO_ANIMATION_START: entering AppKit/GPUI event loop");
    gpui_platform::application().run(move |cx| {
        eprintln!("GPUIO_ANIMATION_LAUNCHED: native application callback");
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        gpui_base::init(cx);
        let motion_watch = crate::motion_preference::init(cx);
        crate::motion_preference::set(gpuio_protocol::animation::Preference::Full, cx);
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, wid(), "Native animations", 480., 240.)
            .unwrap();
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(480.), px(240.)),
                        cx,
                    ))),
                    focus: true,
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(wid(), session.clone(), transport.clone())),
            )
            .unwrap();
        // A fully occluded background window may never receive its first frame.
        // Frame scheduling/idle assertions require a visible, active window.
        cx.activate(true);
        cx.spawn(async move |cx| {
            let result = super::native_test::protect(async {
                exercise(cx, window, &transport).await;
                crate::motion_preference::test(cx, &motion_watch).await;
                geometry_and_close(cx, window, &transport).await;
            })
            .await;
            *task_failure.borrow_mut() = result.err();
            cx.update(super::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
