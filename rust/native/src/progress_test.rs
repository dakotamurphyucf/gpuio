//! Actual progress paint/accessibility and native animation lifetime checks.
use super::*;
fn configuration(fraction: Option<f64>) -> ProgressConfig {
    ProgressConfig {
        label: "Download progress".into(),
        fraction,
    }
}
fn styles() -> Vec<Style> {
    vec![
        Style::Fields(vec![
            Field::Width(Length::Px(240.)),
            Field::Height(Length::Px(12.)),
            Field::Foreground(Color::Rgba(0x22aa77ff)),
        ]),
        Style::State(5, vec![Field::Foreground(Color::Rgba(0xee7733ff))]),
    ]
}
fn paint(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> super::super::progress::Paint {
    handle
        .update(cx, |view, _, _| view.progress_probes[&node(60)].get())
        .unwrap()
}
async fn pause(cx: &mut gpui::AsyncApp) {
    cx.background_executor()
        .timer(std::time::Duration::from_millis(180))
        .await;
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    handle
        .update(cx, |view, window, cx| {
            window.focus(&view.buttons[&node(2)].focus, cx);
        })
        .unwrap();
    frame(cx, handle).await;
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(60), Kind::Progress, "".into(), None),
            Op::SetProgress(node(60), configuration(Some(0.25))),
            Op::SetStyle(node(60), styles()),
            Op::Splice(node(0), 4, 0, vec![node(60)]),
        ],
    );
    frame(cx, handle).await;
    let initial = paint(cx, handle);
    assert_eq!(initial.bounds.size.width, px(60.));
    assert_eq!(initial.bounds.size.height, px(12.));
    assert_eq!(initial.color, gpui::Hsla::from(gpui::rgba(0x22aa77ff)));
    let target = handle
        .update(cx, |view, _, _| {
            view.probes.borrow()[&node(60)].bounds.center()
        })
        .unwrap();
    super::super::native_test::move_mouse(cx, handle, target, false);
    super::super::native_test::mouse(cx, handle, target, true);
    super::super::native_test::mouse(cx, handle, target, false);
    frame(cx, handle).await;
    assert!(
        focused(cx, handle, node(2)),
        "a progress indicator does not take keyboard focus"
    );
    #[cfg(target_os = "macos")]
    {
        let _ = accessible(cx, handle, "Download progress", false);
        frame(cx, handle).await;
        let ax = accessible(cx, handle, "Download progress", false).unwrap();
        assert_eq!(ax.role, "AXProgressIndicator");
        assert_eq!(ax.value, 25);
    }
    for (fraction, width) in [(1., 240.), (0.5, 120.)] {
        apply(
            cx,
            handle,
            vec![Op::SetProgress(node(60), configuration(Some(fraction)))],
        );
        frame(cx, handle).await;
        assert_eq!(paint(cx, handle).bounds.size.width, px(width));
    }
    apply(
        cx,
        handle,
        vec![Op::SetProgress(node(60), configuration(Some(0.)))],
    );
    frame(cx, handle).await;
    #[cfg(target_os = "macos")]
    assert_eq!(
        accessible(cx, handle, "Download progress", false)
            .unwrap()
            .value,
        0
    );
    apply(
        cx,
        handle,
        vec![Op::SetProgress(node(60), configuration(None))],
    );
    frame(cx, handle).await;
    let started = paint(cx, handle);
    let revision = handle
        .update(cx, |view, _, _| {
            view.session.borrow().tree(view.id).unwrap().revision()
        })
        .unwrap();
    pause(cx).await; // Do not request another frame: GPUI must drive this cycle itself.
    let advanced = paint(cx, handle);
    assert!(
        advanced.count > started.count,
        "native animation paints without an OCaml update"
    );
    assert_ne!(advanced.bounds.origin.x, started.bounds.origin.x);
    assert_eq!(advanced.bounds.size.width, px(60.));
    assert_eq!(advanced.color, gpui::Hsla::from(gpui::rgba(0xee7733ff)));
    handle
        .update(cx, |view, _, _| {
            assert_eq!(
                view.session.borrow().tree(view.id).unwrap().revision(),
                revision
            )
        })
        .unwrap();
    #[cfg(target_os = "macos")]
    assert_eq!(
        accessible(cx, handle, "Download progress", false)
            .unwrap()
            .value,
        -1,
        "indeterminate progress has no fabricated numeric value"
    );
    cx.update(|cx| cx.set_reduce_motion(true));
    frame(cx, handle).await;
    let reduced = paint(cx, handle);
    let viewport = handle
        .update(cx, |view, _, _| view.probes.borrow()[&node(60)].bounds)
        .unwrap();
    assert_eq!(reduced.bounds.size.width, px(60.));
    assert_eq!(
        reduced.bounds.center(),
        viewport.center(),
        "reduced indeterminate bar remains visible and centered"
    );
    let renders = handle.update(cx, |view, _, _| view.render_count).unwrap();
    pause(cx).await;
    assert_eq!(
        handle.update(cx, |view, _, _| view.render_count).unwrap(),
        renders,
        "reduced progress leaves window idle"
    );
    cx.update(|cx| cx.set_reduce_motion(false));
    frame(cx, handle).await;
    let resumed = paint(cx, handle).count;
    pause(cx).await;
    assert!(
        paint(cx, handle).count > resumed,
        "full motion resumes progress frames"
    );
    let mut hidden = styles();
    hidden.push(Style::Fields(vec![Field::Visibility(1)]));
    apply(cx, handle, vec![Op::SetStyle(node(60), hidden)]);
    frame(cx, handle).await;
    let count = paint(cx, handle).count;
    pause(cx).await;
    assert_eq!(
        paint(cx, handle).count,
        count,
        "hidden indicators do not paint their cycle"
    );
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(node(60), styles()),
            Op::SetProgress(node(60), configuration(Some(1.))),
        ],
    );
    frame(cx, handle).await;
    let stopped = paint(cx, handle);
    pause(cx).await;
    assert_eq!(
        paint(cx, handle).count,
        stopped.count,
        "determinate progress stops requesting animation frames"
    );
    let weak = handle
        .update(cx, |view, _, _| {
            Rc::downgrade(&view.progress_probes[&node(60)])
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::Splice(node(0), 4, 1, vec![]), Op::Remove(node(60))],
    );
    frame(cx, handle).await;
    assert!(
        weak.upgrade().is_none(),
        "removal releases retained paint/animation ownership"
    );
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(event, Event::Press(_, id, ..) if *id == node(60)))
    );
    println!(
        "GPUIO_PROGRESS_NATIVE_OK: actual fraction bounds/colors, noninteractive focus, native animation without commits and hidden/determinate/unmount cleanup"
    );
    #[cfg(target_os = "macos")]
    println!(
        "GPUIO_PROGRESS_MACOS_AX_OK: progress role, label and determinate/indeterminate numeric semantics"
    );
}
