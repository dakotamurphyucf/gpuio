//! Background GPU comparison against ordinary style opacity; no OS input claims.
use super::styled_text_test::{apply, draw};
use super::*;
use gpuio_protocol::{
    animation::{Easing, Property, Repeat, Target},
    animation_program::*,
};
use std::{
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    time::Duration,
};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn target(value: f64) -> Vec<Target> {
    vec![Target {
        property: Property::OpacityFactor,
        value,
    }]
}
fn config(generation: i64, factor: f64, pulse: bool) -> Config {
    Config {
        generation,
        restart: 0,
        playback: Playback::Running,
        program: Program {
            initial: Some(target(if pulse { 1. } else { factor })),
            stages: if pulse {
                [0.6, 1.]
                    .map(|value| Stage {
                        targets: target(value),
                        timing: Timing::Tween(1000, Easing::Linear),
                        delay_ms: 0,
                    })
                    .to_vec()
            } else {
                vec![Stage {
                    targets: target(factor),
                    timing: Timing::Tween(0, Easing::Linear),
                    delay_ms: 0,
                }]
            },
            delay_ms: 0,
            repeat: if pulse { Repeat::Loop } else { Repeat::Once },
            clock: Clock::Independent,
        },
    }
}
fn style(left: f64, opacity: Option<f64>, states: bool) -> Vec<Style> {
    let mut fields = vec![
        Field::Position(1),
        Field::Left(Length::Px(left)),
        Field::Top(Length::Px(20.)),
        Field::Width(Length::Px(120.)),
        Field::Height(Length::Px(80.)),
        Field::Background(Fill::Solid(Color::Rgba(0xff0000ff))),
    ];
    if let Some(opacity) = opacity {
        fields.push(Field::Opacity(opacity));
    }
    let mut result = vec![Style::Fields(fields)];
    if states {
        result.push(Style::State(2, vec![Field::Opacity(0.6)]));
        result.push(Style::State(3, vec![Field::Opacity(0.4)]));
    }
    result
}
fn set_time(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, millis: u64) {
    handle
        .update(cx, |view, window, _| {
            view.session
                .borrow()
                .motion()
                .borrow_mut()
                .set_test_time(Some(Duration::from_millis(millis)));
            window.refresh();
        })
        .unwrap();
}
fn check(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, expected_opacity: f64) {
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id(3),
            style(180., Some(expected_opacity), false),
        )],
    );
    draw(cx, handle);
    handle
        .update(cx, |view, window, _| {
            let image = window
                .render_to_image()
                .expect("opacity factor GPU readback");
            let scale = window.scale_factor();
            // Compare actual backgrounds and translucent descendants against an
            // independently styled reference, avoiding assumptions about color space.
            for (x, y) in [(5., 5.), (30., 30.), (100., 60.)] {
                let pixel = |left: f32| {
                    image
                        .get_pixel(
                            ((left + x) * scale).round() as u32,
                            ((20. + y) * scale).round() as u32,
                        )
                        .0
                };
                let a = pixel(20.);
                let b = pixel(180.);
                assert!(
                    a.iter().zip(b).all(|(a, b)| a.abs_diff(b) <= 2),
                    "opacity {expected_opacity}: animated {a:?}, reference {b:?} at {x},{y}"
                );
            }
            let actual = view.probes.borrow()[&id(1)].bounds;
            assert_eq!(
                actual.size,
                size(px(120.), px(80.)),
                "factor must not affect layout"
            );
        })
        .unwrap();
}
async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    cx.update(|cx| crate::motion_preference::set(gpuio_protocol::animation::Preference::Full, cx));
    set_time(cx, handle, 0);
    apply(
        cx,
        handle,
        vec![
            Op::Create(id(0), Kind::Container, String::new(), None),
            Op::SetStyle(
                id(0),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(360.)),
                    Field::Height(Length::Px(200.)),
                    Field::Background(Fill::Solid(Color::Rgba(0x000000ff))),
                    Field::Opacity(0.5),
                ])],
            ),
            Op::Create(id(1), Kind::AnimationProgram, String::new(), None),
            Op::SetAnimationProgram(id(1), config(1, 0.5, false)),
            Op::Create(id(2), Kind::Container, String::new(), None),
            Op::Create(id(3), Kind::Container, String::new(), None),
            Op::Create(id(4), Kind::Container, String::new(), None),
            Op::Splice(id(1), 0, 0, vec![id(2)]),
            Op::Splice(id(3), 0, 0, vec![id(4)]),
            Op::Splice(id(0), 0, 0, vec![id(1), id(3)]),
            Op::SetRoot(Some(id(0))),
        ],
    );
    for child in [id(2), id(4)] {
        apply(
            cx,
            handle,
            vec![Op::SetStyle(
                child,
                vec![Style::Fields(vec![
                    Field::Position(1),
                    Field::Left(Length::Px(20.)),
                    Field::Top(Length::Px(20.)),
                    Field::Width(Length::Px(30.)),
                    Field::Height(Length::Px(30.)),
                    Field::Background(Fill::Solid(Color::Rgba(0x00ff00ff))),
                    Field::Opacity(0.5),
                ])],
            )],
        );
    }
    let mut generation = 1;
    for base in [None, Some(0.), Some(0.4), Some(0.8), Some(1.)] {
        for factor in [0., 0.5, 1.] {
            generation += 1;
            apply(
                cx,
                handle,
                vec![
                    Op::SetStyle(id(1), style(20., base, false)),
                    Op::SetAnimationProgram(id(1), config(generation, factor, false)),
                ],
            );
            check(cx, handle, base.unwrap_or(1.) * factor);
        }
    }
    generation += 1;
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(id(1), style(20., Some(0.8), true)),
            Op::SetAnimationProgram(id(1), config(generation, 0.5, false)),
        ],
    );
    let inside = gpui::point(px(100.), px(60.));
    let outside = gpui::point(px(340.), px(180.));
    native_test::move_mouse(cx, handle, outside, false);
    check(cx, handle, 0.4);
    native_test::move_mouse(cx, handle, inside, false);
    check(cx, handle, 0.3);
    native_test::mouse(cx, handle, inside, true);
    check(cx, handle, 0.2);
    native_test::mouse(cx, handle, inside, false);
    check(cx, handle, 0.3);
    native_test::move_mouse(cx, handle, outside, false);
    check(cx, handle, 0.4);
    // Restoring factor one restores both base and live interaction styles.
    generation += 1;
    apply(
        cx,
        handle,
        vec![Op::SetAnimationProgram(
            id(1),
            config(generation, 1., false),
        )],
    );
    check(cx, handle, 0.8);
    native_test::move_mouse(cx, handle, inside, false);
    check(cx, handle, 0.6);
    native_test::move_mouse(cx, handle, outside, false);
    generation += 1;
    apply(
        cx,
        handle,
        vec![Op::SetAnimationProgram(id(1), config(generation, 1., true))],
    );
    check(cx, handle, 0.8);
    set_time(cx, handle, 1000);
    check(cx, handle, 0.8 * 0.6);
    set_time(cx, handle, 2000);
    check(cx, handle, 0.8);
    cx.update(|cx| {
        crate::motion_preference::set(gpuio_protocol::animation::Preference::Reduce, cx)
    });
    set_time(cx, handle, 3000);
    check(cx, handle, 0.8);
    let wakes = handle
        .update(cx, |view, _, _| {
            view.animation_programs[&id(1)].borrow().wake_count
        })
        .unwrap();
    set_time(cx, handle, 4000);
    check(cx, handle, 0.8);
    handle
        .update(cx, |view, _, _| {
            let state = view.animation_programs[&id(1)].borrow();
            assert_eq!(state.wake_count, wakes, "reduced pulse stays idle");
            assert!(!state.has_deadline());
        })
        .unwrap();
    let owner = handle
        .update(cx, |view, _, _| {
            Rc::downgrade(&view.animation_programs[&id(1)])
        })
        .unwrap();
    eprintln!("OPACITY_FACTOR_PAINT_CHECKS_PASSED");
    apply(
        cx,
        handle,
        vec![
            Op::SetRoot(None),
            Op::Remove(id(4)),
            Op::Remove(id(3)),
            Op::Remove(id(2)),
            Op::Remove(id(1)),
            Op::Remove(id(0)),
        ],
    );
    draw(cx, handle);
    assert!(owner.upgrade().is_none(), "unmount retires the pulse owner");
    handle
        .update(cx, |view, _, _| {
            assert_eq!(
                view.session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .retained_bytes(),
                0
            )
        })
        .unwrap();
    eprintln!(
        "GPUIO_OPACITY_FACTOR_OK: 15 base/factor GPU pairs, ancestor/descendant alpha, layout, hover/press/restoration, native pulse/reduced idle, owner teardown"
    );
}
pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        gpui_base::init(cx);
        let motion_watch = crate::motion_preference::init(cx);
        let session = Rc::new(RefCell::new(Session::default()));
        crate::motion_preference::bind_clocks(&session.borrow().motion(), cx);
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        let window_id = WindowId::from_parts(0, 1).unwrap();
        session
            .borrow_mut()
            .open(1, window_id, "Opacity factor check", 360., 320.)
            .unwrap();
        let handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(360.), px(320.)),
                        cx,
                    ))),
                    focus: false,
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(window_id, session, transport)),
            )
            .unwrap();
        cx.spawn(async move |cx| {
            let checked = native_test::protect(exercise(cx, handle)).await;
            *task_failure.borrow_mut() = checked.err();
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            drop(motion_watch);
            cx.update(stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
