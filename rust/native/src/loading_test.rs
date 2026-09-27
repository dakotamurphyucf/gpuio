use super::*;
use gpuio_protocol::loading::{Config as LoadingConfig, Kind as LoadingKind};
use gpuio_protocol::v1::Field as StyleField;

fn config(kind: LoadingKind, animated: bool) -> LoadingConfig {
    LoadingConfig {
        kind,
        label: "Loading workspace".into(),
        animated,
        period_ms: 1200,
    }
}
fn styles() -> Vec<Style> {
    vec![
        Style::Radius(8.),
        Style::Fields(vec![
            StyleField::Width(Length::Px(64.)),
            StyleField::Height(Length::Px(32.)),
            StyleField::Foreground(Color::Rgba(0x44bb88ff)),
        ]),
    ]
}
fn paint(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> super::super::loading::Paint {
    handle
        .update(cx, |v, _, _| v.loading_probes[&node(4)].get())
        .unwrap()
}
async fn pause(cx: &mut gpui::AsyncApp) {
    cx.background_executor()
        .timer(std::time::Duration::from_millis(120))
        .await;
}
async fn idle(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    frame(cx, handle).await;
    let count = handle.update(cx, |v, _, _| v.render_count).unwrap();
    pause(cx).await;
    assert_eq!(
        handle.update(cx, |v, _, _| v.render_count).unwrap(),
        count,
        "inactive loading leaves the entire window idle"
    );
}

pub(super) async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(3), Kind::Container, "".into(), None),
            Op::Create(node(4), Kind::Loading, "".into(), None),
            Op::SetLoading(node(4), config(LoadingKind::Skeleton, true)),
            Op::SetStyle(node(4), styles()),
            Op::Splice(node(3), 0, 0, vec![node(4)]),
            Op::SetRoot(Some(node(3))),
        ],
    );
    for kind in [
        LoadingKind::Skeleton,
        LoadingKind::Shimmer,
        LoadingKind::Spinner,
    ] {
        apply(
            cx,
            handle,
            vec![Op::SetLoading(node(4), config(kind, true))],
        );
        frame(cx, handle).await;
        let before = paint(cx, handle);
        let revision = handle
            .update(cx, |v, _, _| {
                v.session.borrow().tree(v.id).unwrap().revision()
            })
            .unwrap();
        pause(cx).await;
        let after = paint(cx, handle);
        assert!(
            after.count > before.count,
            "loading {kind:?}: paints {} -> {}, phase {} -> {}, reduced={}",
            before.count,
            after.count,
            before.phase,
            after.phase,
            cx.update(|cx| cx.reduce_motion())
        );
        assert_ne!(after.phase, before.phase);
        assert_eq!(after.bounds.size, size(px(64.), px(32.)));
        assert_eq!(after.corners, px(8.).into());
        assert_eq!(after.color, gpui::Hsla::from(rgba(0x44bb88ff)));
        handle
            .update(cx, |v, _, _| {
                assert_eq!(v.session.borrow().tree(v.id).unwrap().revision(), revision);
                assert!(v.buttons.is_empty() && v.editors.is_empty());
            })
            .unwrap();
        #[cfg(target_os = "macos")]
        {
            let _ = accessible(cx, handle, "Loading workspace", false);
            frame(cx, handle).await;
            assert_eq!(
                accessible(cx, handle, "Loading workspace", false)
                    .unwrap()
                    .role,
                "AXProgressIndicator"
            );
        }
        cx.update(|cx| {
            crate::motion_preference::set(gpuio_protocol::animation::Preference::Reduce, cx)
        });
        idle(cx, handle).await;
        assert_eq!(
            paint(cx, handle).phase,
            if kind == LoadingKind::Shimmer {
                0.5
            } else {
                0.
            }
        );
        cx.update(|cx| {
            crate::motion_preference::set(gpuio_protocol::animation::Preference::Full, cx)
        });
        frame(cx, handle).await;
        let count = paint(cx, handle).count;
        pause(cx).await;
        assert!(paint(cx, handle).count > count);
        apply(
            cx,
            handle,
            vec![Op::SetLoading(node(4), config(kind, false))],
        );
        idle(cx, handle).await;
        apply(
            cx,
            handle,
            vec![
                Op::SetLoading(node(4), config(kind, true)),
                Op::SetStyle(
                    node(3),
                    vec![
                        Style::Radius(8.),
                        Style::Fields(vec![StyleField::Visibility(1)]),
                    ],
                ),
            ],
        );
        idle(cx, handle).await;
        let count = paint(cx, handle).count;
        pause(cx).await;
        assert_eq!(paint(cx, handle).count, count);
        #[cfg(target_os = "macos")]
        assert!(
            accessible(cx, handle, "Loading workspace", false).is_none(),
            "hidden loading must leave the native accessibility tree"
        );
        apply(cx, handle, vec![Op::SetStyle(node(3), vec![])]);
        frame(cx, handle).await;
        assert!(paint(cx, handle).count > count);
    }
    #[cfg(target_os = "macos")]
    {
        handle.update(cx, |_, w, _| w.minimize_window()).unwrap();
        // Allow the platform minimize transition to finish; don't ask a hidden
        // window to paint a test frame.
        for _ in 0..6 {
            pause(cx).await;
        }
        let count = handle.update(cx, |v, _, _| v.render_count).unwrap();
        pause(cx).await;
        assert_eq!(
            handle.update(cx, |v, _, _| v.render_count).unwrap(),
            count,
            "minimized loading window must stop rendering"
        );
        handle.update(cx, |_, w, _| w.activate_window()).unwrap();
        frame(cx, handle).await;
        let count = paint(cx, handle).count;
        pause(cx).await;
        assert!(paint(cx, handle).count > count);
        eprintln!("GPUIO_LOADING_MINIMIZED_OK: actual macOS minimize idle and restore animation");
    }
    let weak = handle
        .update(cx, |v, _, _| Rc::downgrade(&v.loading_probes[&node(4)]))
        .unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetRoot(None), Op::Remove(node(4)), Op::Remove(node(3))],
    );
    idle(cx, handle).await;
    assert!(weak.upgrade().is_none());
    handle
        .update(cx, |v, _, _| {
            assert_eq!(v.session.borrow().retained_bytes(), 0)
        })
        .unwrap();
    eprintln!(
        "GPUIO_LOADING_NATIVE_OK: three native cycles, styles/AX, reduced/static/ancestor-hidden idle, resume and disposal"
    );
}
