//! Actual paint/transport behavior for retargeting and terminal controls.
use super::*;

pub(super) async fn exercise(cx: &mut gpui::AsyncApp, transport: &Arc<Transport>) {
    let window = lifecycle::open(cx, transport);
    // Layout rounds to device pixels; logical physics remains unrounded.
    let tolerance = window
        .update(cx, |_, w, _| 0.5 / f64::from(w.scale_factor()) + 1e-4)
        .unwrap();
    let handler = gpuio_protocol::HandlerId::from_parts(0, 1).unwrap();
    let parameters = Spring {
        stiffness: 100.,
        damping: 10.,
        mass: 1.,
        epsilon: 0.001,
        max_duration_ms: 1000,
    };
    let mut current = config(1);
    current.program.stages = vec![Stage {
        targets: vec![Target {
            property: Property::Width,
            value: 180.,
        }],
        timing: Timing::Spring(parameters),
        delay_ms: 0,
    }];
    apply(
        cx,
        window,
        vec![
            Op::Create(node(0), Kind::AnimationProgram, "".into(), Some(handler)),
            Op::SetAnimationProgram(node(0), current.clone()),
            Op::SetStyle(node(0), vec![Style::Height(Length::Px(40.))]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    at(cx, window, 100).await;
    let trajectory =
        crate::motion::spring::Trajectory::new(parameters, Property::Width, 20., 0., 180.).unwrap();
    let painted = trajectory.sample(Duration::from_millis(100));
    assert!((f64::from(width(cx, window, 0)) - painted.position).abs() < tolerance);
    current.generation = 2;
    current.program.stages[0].targets[0].value = 40.;
    apply(
        cx,
        window,
        vec![Op::SetAnimationProgram(node(0), current.clone())],
    );
    frame(cx, window).await;
    assert!(
        (f64::from(width(cx, window, 0)) - painted.position).abs() < tolerance,
        "retarget starts at actual last paint"
    );
    let expected = crate::motion::spring::Trajectory::new(
        parameters,
        Property::Width,
        painted.position,
        painted.velocity,
        40.,
    )
    .unwrap()
    .sample(Duration::from_millis(100));
    at(cx, window, 200).await;
    let actual = f64::from(width(cx, window, 0));
    assert!(
        (actual - expected.position).abs() < tolerance,
        "painted velocity reaches the retargeted trajectory: actual={actual}, expected={expected:?}, from={painted:?}"
    );
    let replaced = signals(transport);
    assert_eq!(replaced.len(), 1);
    assert_eq!(replaced[0].generation, 1);
    assert_eq!(
        replaced[0].observation,
        Observation::Cancelled(CancelReason::Replaced)
    );
    current.generation = 3;
    current.playback = Playback::Cancelled;
    apply(
        cx,
        window,
        vec![Op::SetAnimationProgram(node(0), current.clone())],
    );
    at(cx, window, 1000).await;
    assert!(
        (f64::from(width(cx, window, 0)) - expected.position).abs() < tolerance,
        "cancel holds last paint"
    );
    let cancelled = signals(transport);
    assert_eq!(cancelled.len(), 1);
    assert_eq!(cancelled[0].generation, 2);
    assert_eq!(
        cancelled[0].observation,
        Observation::Cancelled(CancelReason::Requested)
    );
    current.generation = 4;
    current.playback = Playback::Running;
    apply(
        cx,
        window,
        vec![Op::SetAnimationProgram(node(0), current.clone())],
    );
    frame(cx, window).await;
    assert!(
        (f64::from(width(cx, window, 0)) - expected.position).abs() < tolerance,
        "Running cannot revive a cancelled run"
    );
    assert!(signals(transport).is_empty());
    let before = window.update(cx, |v, _, _| v.render_count).unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(120))
        .await;
    assert_eq!(before, window.update(cx, |v, _, _| v.render_count).unwrap());
    current.generation = 5;
    current.restart = 1;
    apply(
        cx,
        window,
        vec![Op::SetAnimationProgram(node(0), current.clone())],
    );
    frame(cx, window).await;
    assert!(
        (width(cx, window, 0) - 20.).abs() < 0.1,
        "explicit restart resets initial values"
    );
    at(cx, window, 1100).await;
    let before_reverse = width(cx, window, 0);
    current.generation = 6;
    current.program.initial.as_mut().unwrap()[0].value = 40.;
    current.program.stages[0].targets[0].value = 20.;
    apply(
        cx,
        window,
        vec![Op::SetAnimationProgram(node(0), current.clone())],
    );
    frame(cx, window).await;
    assert!((width(cx, window, 0) - before_reverse).abs() < 0.1);
    at(cx, window, 2200).await;
    assert!((width(cx, window, 0) - 20.).abs() < 0.1);
    let reversed = signals(transport);
    assert_eq!(
        reversed
            .iter()
            .map(|s| (s.generation, s.index))
            .collect::<Vec<_>>(),
        vec![(5, 33), (6, 1), (6, 33)]
    );
    current.generation = 7;
    current.restart = 2;
    current.program.initial.as_mut().unwrap()[0].value = 20.;
    current.program.stages[0].targets[0].value = 180.;
    apply(
        cx,
        window,
        vec![
            Op::Bind(node(0), None),
            Op::SetAnimationProgram(node(0), current.clone()),
        ],
    );
    at(cx, window, 3300).await;
    assert!((width(cx, window, 0) - 180.).abs() < 0.1);
    assert!(signals(transport).is_empty());
    let renewed = gpuio_protocol::HandlerId::from_parts(0, 2).unwrap();
    apply(cx, window, vec![Op::Bind(node(0), Some(renewed))]);
    frame(cx, window).await;
    assert!(
        signals(transport).is_empty(),
        "late observer does not replay a completed run"
    );
    current.generation = 8;
    current.restart = 3;
    apply(cx, window, vec![Op::SetAnimationProgram(node(0), current)]);
    at(cx, window, 4400).await;
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert!(events.iter().any(|event| matches!(event, Event::AnimationProgramEvent(_, _, found, _, batch) if *found == renewed && batch.last().is_some_and(|s| s.generation == 8 && s.observation == Observation::Finished))));
    window
        .update(cx, |v, w, _| {
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
        })
        .unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(80))
        .await;
    assert!(signals(transport).is_empty());
    eprintln!(
        "GPUIO_ANIMATION_CONTROLS_OK: painted spring velocity, cancellation, restart, reverse and observer lifetime"
    );
}
