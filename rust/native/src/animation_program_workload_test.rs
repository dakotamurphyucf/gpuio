//! Admitted maximum owner count with actual painting, input and disposal.
use super::*;
use std::time::Instant;

fn program(generation: i64, target: f64) -> Config {
    Config {
        generation,
        restart: 0,
        playback: Playback::Running,
        program: Program {
            initial: Some(vec![
                Target {
                    property: Property::Width,
                    value: 10.,
                },
                Target {
                    property: Property::Opacity,
                    value: 0.2,
                },
            ]),
            stages: vec![Stage {
                targets: vec![
                    Target {
                        property: Property::Width,
                        value: target,
                    },
                    Target {
                        property: Property::Opacity,
                        value: 1.,
                    },
                ],
                timing: Timing::Spring(Spring {
                    stiffness: 100.,
                    damping: 12.,
                    mass: 1.,
                    epsilon: 0.001,
                    max_duration_ms: 1000,
                }),
                delay_ms: 0,
            }],
            delay_ms: 0,
            repeat: Repeat::Once,
            clock: Clock::Independent,
        },
    }
}
pub(super) async fn exercise(cx: &mut gpui::AsyncApp, transport: &Arc<Transport>) {
    let window = lifecycle::open(cx, transport);
    window
        .update(cx, |v, w, _| {
            w.resize(size(px(540.), px(360.)));
            v.session.borrow().motion().borrow_mut().set_test_time(None);
        })
        .unwrap();
    let mut ops = vec![Op::Create(node(0), Kind::Container, "".into(), None)];
    for row in 1..=32 {
        ops.extend([
            Op::Create(node(row), Kind::Container, "".into(), None),
            Op::SetStyle(
                node(row),
                vec![Style::Fields(vec![
                    Field::Direction(0),
                    Field::Height(Length::Px(8.)),
                    Field::Shrink(0.),
                ])],
            ),
        ]);
    }
    for item in 33..=1056 {
        ops.extend([
            Op::Create(node(item), Kind::AnimationProgram, "".into(), None),
            Op::SetAnimationProgram(node(item), program(1, 12.)),
            Op::SetStyle(
                node(item),
                vec![
                    Style::Fields(vec![Field::Height(Length::Px(6.)), Field::Shrink(0.)]),
                    Style::Background(Color::Rgba(0x528bffff)),
                ],
            ),
        ]);
    }
    for row in 1..=32 {
        let start = 33 + (row - 1) * 32;
        ops.push(Op::Splice(
            node(row),
            0,
            0,
            (start..start + 32).map(node).collect(),
        ));
    }
    let button = node(1057);
    let handler = gpuio_protocol::HandlerId::from_parts(0, 1).unwrap();
    ops.extend([
        Op::Create(
            button,
            Kind::Button,
            "Input remains active".into(),
            Some(handler),
        ),
        Op::SetControl(button, Control::Button(false)),
        Op::SetStyle(
            button,
            vec![
                Style::Width(Length::Px(180.)),
                Style::Height(Length::Px(32.)),
            ],
        ),
        Op::Splice(node(0), 0, 0, (1..=32).map(node).chain([button]).collect()),
        Op::SetRoot(Some(node(0))),
    ]);
    let started = Instant::now();
    apply(cx, window, ops);
    let mount = started.elapsed();
    frame(cx, window).await;
    let (owners, groups, reserved) = window
        .update(cx, |v, _, _| v.session.borrow().motion().borrow().counts())
        .unwrap();
    assert_eq!(owners, 1024);
    assert_eq!(groups, 0);
    assert!(reserved <= crate::motion_host::MAX_BYTES);
    let painted = window
        .update(cx, |v, _, _| {
            v.animation_programs
                .values()
                .filter(|s| s.borrow().paint_count > 0)
                .count()
        })
        .unwrap();
    assert_eq!(
        painted, owners,
        "all admitted owners painted in the visible grid"
    );
    assert!(signals(transport).is_empty());
    let position = window
        .update(cx, |v, _, _| v.probes.borrow()[&button].bounds.center())
        .unwrap();
    let started = Instant::now();
    super::super::native_test::move_mouse(cx, window, position, false);
    super::super::native_test::mouse(cx, window, position, true);
    super::super::native_test::mouse(cx, window, position, false);
    let input = started.elapsed();
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, Event::Press(_, n, h, _) if *n == button && *h == handler))
            .count(),
        1
    );
    let started = Instant::now();
    apply(
        cx,
        window,
        (33..=1056)
            .map(|i| Op::SetAnimationProgram(node(i), program(2, 15.)))
            .collect(),
    );
    let retarget = started.elapsed();
    let mut frames = Vec::new();
    for _ in 0..6 {
        let started = Instant::now();
        frame(cx, window).await;
        frames.push(started.elapsed());
    }
    assert!(
        signals(transport).is_empty(),
        "unobserved native motion produces no bridge events"
    );
    let weak = window
        .update(cx, |v, _, _| {
            Rc::downgrade(&v.animation_programs[&node(33)])
        })
        .unwrap();
    let started = Instant::now();
    let mut remove = vec![Op::SetRoot(None)];
    remove.extend((0..=1057).rev().map(|i| Op::Remove(node(i))));
    apply(cx, window, remove);
    let disposal = started.elapsed();
    assert!(weak.upgrade().is_none());
    window
        .update(cx, |v, _, _| {
            assert_eq!(v.session.borrow().motion().borrow().counts(), (0, 0, 0))
        })
        .unwrap();
    frame(cx, window).await;
    cx.background_executor()
        .timer(Duration::from_millis(80))
        .await;
    let before = window.update(cx, |v, _, _| v.render_count).unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(120))
        .await;
    assert_eq!(before, window.update(cx, |v, _, _| v.render_count).unwrap());
    window
        .update(cx, |v, w, _| {
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
        })
        .unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(80))
        .await;
    frames.sort();
    eprintln!(
        "GPUIO_ANIMATION_WORKLOAD_OK: owners={owners}, painted={painted}, reserved_bytes={reserved}, mount={mount:?}, retarget={retarget:?}, native_input_dispatch={input:?}, disposal={disposal:?}, frame_barrier_median={:?}, maximum={:?}; debug wall times, not end-to-end OS input or frame CPU times",
        frames[frames.len() / 2],
        frames.last().unwrap()
    );
}
