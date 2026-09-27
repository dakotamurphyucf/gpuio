//! Mounted GPUI programs: real geometry, event batches, shared phase and cleanup.
use super::*;
#[path = "animation_program_controls_test.rs"]
mod controls;
#[path = "animation_program_lifecycle_test.rs"]
mod lifecycle;
#[path = "animation_program_workload_test.rs"]
mod workload;
use gpuio_protocol::{
    animation::{Easing, Property, Repeat, Spring, Target},
    animation_program::*,
};
use std::{
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    time::Duration,
};
fn node(n: i64) -> NodeId {
    NodeId::from_parts(n, 1).unwrap()
}
fn wid() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn config(generation: i64) -> Config {
    Config {
        generation,
        playback: Playback::Running,
        restart: 0,
        program: Program {
            initial: Some(vec![Target {
                property: Property::Width,
                value: 20.,
            }]),
            stages: vec![
                Stage {
                    targets: vec![Target {
                        property: Property::Width,
                        value: 100.,
                    }],
                    timing: Timing::Tween(1000, Easing::Linear),
                    delay_ms: 0,
                },
                Stage {
                    targets: vec![Target {
                        property: Property::Width,
                        value: 180.,
                    }],
                    timing: Timing::Spring(Spring {
                        stiffness: 100.,
                        damping: 18.,
                        mass: 1.,
                        epsilon: 0.001,
                        max_duration_ms: 1000,
                    }),
                    delay_ms: 0,
                },
            ],
            delay_ms: 0,
            repeat: Repeat::Once,
            clock: Clock::Independent,
        },
    }
}
fn apply(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, operations: Vec<Op>) {
    window
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let result = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: view.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            view.update_editors(&result.dirty, window, cx);
            view.list_actions(&result.lists, window, cx);
            cx.notify();
        })
        .unwrap();
}
async fn frame(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    super::editor_test::frame(cx, window).await;
    super::editor_test::frame(cx, window).await;
}
async fn at(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, millis: u64) {
    window
        .update(cx, |view, window, _| {
            view.session
                .borrow()
                .motion()
                .borrow_mut()
                .set_test_time(Some(Duration::from_millis(millis)));
            window.refresh();
        })
        .unwrap();
    frame(cx, window).await;
}
fn width(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, n: i64) -> f32 {
    window
        .update(cx, |view, _, _| {
            f32::from(view.probes.borrow()[&node(n)].bounds.size.width)
        })
        .unwrap()
}
fn signals(transport: &Transport) -> Vec<Signal> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .flat_map(|event| match event {
            Event::AnimationProgramEvent(_, _, _, _, signals) => signals,
            _ => vec![],
        })
        .collect()
}
async fn assert_idle_programs(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    transport: &Transport,
) {
    let snapshot = |cx: &mut gpui::AsyncApp| {
        window
            .update(cx, |view, _, _| {
                let mut counters: Vec<_> = view
                    .animation_programs
                    .iter()
                    .map(|(id, state)| {
                        let state = state.borrow();
                        assert!(!state.has_deadline(), "idle program owns no timer");
                        (*id, state.wake_count)
                    })
                    .collect();
                counters.sort_unstable_by_key(|(id, _)| *id);
                counters
            })
            .unwrap()
    };
    let before = snapshot(cx);
    // Platform exposure/hover can paint an otherwise idle window. Measure the
    // animation's own wake requests, and deliberately repaint to prove that
    // painting retained final/reduced geometry does not restart native work.
    window.update(cx, |_, window, _| window.refresh()).unwrap();
    frame(cx, window).await;
    cx.background_executor()
        .timer(Duration::from_millis(120))
        .await;
    assert_eq!(
        before,
        snapshot(cx),
        "idle programs request no native wakes"
    );
    assert!(signals(transport).is_empty(), "idle paints emit no events");
}
async fn exercise(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, transport: &Transport) {
    let handler = gpuio_protocol::HandlerId::from_parts(0, 1).unwrap();
    apply(
        cx,
        window,
        vec![
            Op::Create(node(0), Kind::Container, "".into(), None),
            Op::Create(node(1), Kind::AnimationProgram, "".into(), Some(handler)),
            Op::SetAnimationProgram(node(1), config(1)),
            Op::SetStyle(
                node(1),
                vec![
                    Style::Width(Length::Px(999.)),
                    Style::Height(Length::Px(40.)),
                    Style::Background(Color::Rgba(0x397cf6ff)),
                ],
            ),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    at(cx, window, 500).await;
    assert!((width(cx, window, 1) - 60.).abs() < 0.1);
    assert!(
        signals(transport).is_empty(),
        "frames do not generate bridge traffic"
    );
    let mut paused = config(2);
    paused.playback = Playback::Paused;
    apply(cx, window, vec![Op::SetAnimationProgram(node(1), paused)]);
    at(cx, window, 1500).await;
    assert!((width(cx, window, 1) - 60.).abs() < 0.1);
    apply(
        cx,
        window,
        vec![Op::SetAnimationProgram(node(1), config(3))],
    );
    at(cx, window, 2250).await;
    let spring_width = width(cx, window, 1);
    assert!(
        spring_width > 100. && spring_width < 200.,
        "spring width {spring_width}"
    );
    let stages = signals(transport);
    assert_eq!(stages.len(), 1);
    assert_eq!(stages[0].generation, 1);
    assert_eq!(stages[0].index, 1);
    at(cx, window, 3500).await;
    assert!((width(cx, window, 1) - 180.).abs() < 0.1);
    let finished = signals(transport);
    assert_eq!(
        finished.iter().map(|s| s.index).collect::<Vec<_>>(),
        vec![2, 33]
    );
    assert_idle_programs(cx, window, transport).await;
    let mut shared = config(4);
    shared.program.stages.truncate(1);
    shared.program.repeat = Repeat::Alternate;
    shared.program.clock = Clock::Group("activity".into());
    apply(
        cx,
        window,
        vec![Op::SetAnimationProgram(node(1), shared.clone())],
    );
    at(cx, window, 3750).await;
    shared.generation = 1;
    apply(
        cx,
        window,
        vec![
            Op::Create(node(2), Kind::AnimationProgram, "".into(), Some(handler)),
            Op::SetAnimationProgram(node(2), shared),
            Op::SetStyle(node(2), vec![Style::Height(Length::Px(40.))]),
            Op::Splice(node(0), 1, 0, vec![node(2)]),
        ],
    );
    at(cx, window, 4000).await;
    assert!((width(cx, window, 1) - 60.).abs() < 0.1);
    assert!(
        (width(cx, window, 1) - width(cx, window, 2)).abs() < 0.1,
        "late members share phase"
    );
    assert!(
        signals(transport).is_empty(),
        "shared repeats emit no cycle callbacks"
    );
    let before = window
        .update(cx, |v, _, _| {
            v.session.borrow().motion().borrow_mut().set_test_time(None);
            v.animation_programs[&node(1)].borrow().paint_count
        })
        .unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(120))
        .await;
    let after = window
        .update(cx, |v, _, _| {
            v.animation_programs[&node(1)].borrow().paint_count
        })
        .unwrap();
    assert!(
        after > before + 1,
        "shared repetition advances natively without OCaml updates"
    );
    assert!(signals(transport).is_empty());
    cx.update(|cx| {
        crate::motion_preference::set(gpuio_protocol::animation::Preference::Reduce, cx)
    });
    // Clocks receive the policy at application delivery, before any new paint.
    window
        .update(cx, |view, _, _| {
            let store = view.session.borrow().motion();
            let mut store = store.borrow_mut();
            let now = store.now();
            assert!(store.clocks.sample(wid(), node(1), now).unwrap().paused);
        })
        .unwrap();
    frame(cx, window).await;
    assert_eq!(width(cx, window, 1), 20.);
    assert_eq!(width(cx, window, 2), 20.);
    assert_idle_programs(cx, window, transport).await;
    cx.update(|cx| crate::motion_preference::set(gpuio_protocol::animation::Preference::Full, cx));
    let mut delayed = config(5);
    delayed.program.delay_ms = 10_000;
    apply(cx, window, vec![Op::SetAnimationProgram(node(1), delayed)]);
    frame(cx, window).await;
    let owner = window
        .update(cx, |view, _, _| {
            assert!(view.animation_programs[&node(1)].borrow().has_deadline());
            Rc::downgrade(&view.animation_programs[&node(1)])
        })
        .unwrap();
    apply(
        cx,
        window,
        vec![
            Op::Splice(node(0), 0, 2, vec![]),
            Op::Remove(node(1)),
            Op::Remove(node(2)),
        ],
    );
    frame(cx, window).await;
    assert!(
        owner.upgrade().is_none(),
        "paint closures and deadline tasks do not retain removed owners"
    );
    window
        .update(cx, |v, _, _| {
            assert_eq!(v.session.borrow().motion().borrow().counts(), (0, 0, 0))
        })
        .unwrap();
    signals(transport);
    cx.background_executor()
        .timer(Duration::from_millis(80))
        .await;
    assert!(signals(transport).is_empty());
    let renewed = NodeId::from_parts(1, 2).unwrap();
    let mut timer = config(1);
    timer.program.stages.truncate(1);
    timer.program.stages[0].timing = Timing::Tween(0, Easing::Linear);
    timer.program.delay_ms = 150;
    apply(
        cx,
        window,
        vec![
            Op::Create(renewed, Kind::AnimationProgram, "".into(), Some(handler)),
            Op::SetAnimationProgram(renewed, timer.clone()),
            Op::SetStyle(renewed, vec![Style::Height(Length::Px(40.))]),
            Op::Splice(node(0), 0, 0, vec![renewed]),
        ],
    );
    frame(cx, window).await;
    window
        .update(cx, |v, _, _| {
            assert!(v.animation_programs[&renewed].borrow().has_deadline())
        })
        .unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(250))
        .await;
    let completed = signals(transport);
    assert_eq!(
        completed.iter().map(|s| s.index).collect::<Vec<_>>(),
        vec![1, 33],
        "native deadline wakes an otherwise idle window"
    );
    timer.generation = 2;
    timer.restart = 1;
    timer.program.delay_ms = 10_000;
    apply(cx, window, vec![Op::SetAnimationProgram(renewed, timer)]);
    frame(cx, window).await;
    let (owner, store) = window
        .update(cx, |view, window, _| {
            let owner = Rc::downgrade(&view.animation_programs[&renewed]);
            let store = view.session.borrow().motion();
            view.session.borrow_mut().close(view.id).unwrap();
            window.remove_window();
            (owner, store)
        })
        .unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(80))
        .await;
    assert!(owner.upgrade().is_none());
    assert_eq!(store.borrow().counts(), (0, 0, 0));
    assert!(signals(transport).is_empty());
    eprintln!(
        "GPUIO_ANIMATION_PROGRAM_TIMER_CLOSE_OK: real idle deadline and window-close disposal"
    );
    eprintln!(
        "GPUIO_ANIMATION_PROGRAM_OK: spring/sequence geometry, pause, run events, late shared phase, reduced idle and delayed disposal"
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
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        gpui_base::init(cx);
        let motion_watch = crate::motion_preference::init(cx);
        crate::motion_preference::set(gpuio_protocol::animation::Preference::Full, cx);
        let session = Rc::new(RefCell::new(Session::default()));
        crate::motion_preference::bind_clocks(&session.borrow().motion(), cx);
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, wid(), "Animation programs", 480., 240.)
            .unwrap();
        session
            .borrow()
            .motion()
            .borrow_mut()
            .set_test_time(Some(Duration::ZERO));
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
        cx.activate(true);
        cx.spawn(async move |cx| {
            let result = super::native_test::protect(async {
                exercise(cx, window, &transport).await;
                lifecycle::rows(cx, &transport).await;
                lifecycle::panels(cx, &transport).await;
                lifecycle::windows(cx, &transport).await;
                controls::exercise(cx, &transport).await;
                workload::exercise(cx, &transport).await;
            })
            .await;
            *task_failure.borrow_mut() = result.err();
            drop(motion_watch);
            cx.update(super::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
