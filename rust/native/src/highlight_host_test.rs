//! Real GPUI background-executor wakeups and shutdown, no keyboard/focus input.
use super::*;
use crate::highlight_projection::{Group, Kind, Run, RunKey, Source};
use gpui::{
    Bounds, Context, IntoElement, Render, WindowBounds, WindowHandle, WindowOptions, div,
    prelude::*, px, size,
};
use gpuio_protocol::{
    NodeId,
    highlight::{Appearance, Query, Spec},
};
use std::time::Duration;

struct Scene {
    handle: Option<Handle>,
    pending_paints: usize,
    ready_epoch: i64,
}
impl Render for Scene {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        if let Some(handle) = &self.handle {
            match handle.status() {
                jobs::Status::Pending => self.pending_paints += 1,
                jobs::Status::Ready(_) => self.ready_epoch = handle.epoch(),
                jobs::Status::Failed(_) => (),
            }
        }
        div().size_full().child("Highlight worker lifecycle check")
    }
}
fn source(text: &str) -> Arc<Projection> {
    Arc::new(
        Projection::new(vec![Group {
            kind: Kind::Ordinary,
            runs: vec![Run {
                key: RunKey {
                    node: NodeId::from_parts(0, 1).unwrap(),
                    fragment: 0,
                },
                source: Source::Text(text.into()),
            }],
        }])
        .unwrap(),
    )
}
fn config() -> Arc<Config> {
    Arc::new(Config(vec![Spec {
        query: Some(Query {
            text: "a".into(),
            case_sensitive: true,
            whole_word: false,
        }),
        ranges: vec![],
        appearance: Appearance {
            color: 1,
            active_color: 2,
            radius: 2.,
        },
        active_index: None,
        match_index_offset: 0,
    }]))
}
async fn pause(cx: &mut AsyncApp) {
    cx.background_executor()
        .timer(Duration::from_millis(16))
        .await;
}
async fn ready(cx: &mut AsyncApp, window: WindowHandle<Scene>, epoch: i64) -> Arc<jobs::Ready> {
    for _ in 0..300 {
        let ready = window
            .update(cx, |scene, _, _| {
                if scene.ready_epoch == epoch
                    && let jobs::Status::Ready(ready) = scene.handle.as_ref().unwrap().status()
                {
                    Some(ready)
                } else {
                    None
                }
            })
            .unwrap();
        if let Some(ready) = ready {
            return ready;
        }
        pause(cx).await;
    }
    panic!("highlight completion did not refresh its owning window at epoch {epoch}");
}
fn draw(cx: &mut AsyncApp, window: WindowHandle<Scene>) {
    cx.update_window(window.into(), |_, window, cx| window.draw(cx).clear(cx))
        .unwrap();
}
pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        init(cx);
        let window = cx.open_window(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(320.),px(100.)),cx))),
            focus: false,
            ..Default::default()
        }, |_,cx|cx.new(|_|Scene{handle:None,pending_paints:0,ready_epoch:0})).unwrap();
        cx.spawn(async move |cx| {
            let result = crate::host::native_test::protect(async {
                let original_source = source("a a a");
                let original_config = config();
                window.update(cx, |scene, window, cx| {
                    scene.handle = Some(request(original_source.clone(),original_config.clone(),window,cx).unwrap());
                    cx.notify();
                }).unwrap();
                // Establish a pending paint before the deferred worker pump.
                draw(cx,window);
                assert!(window.update(cx,|s,_,_|s.pending_paints>0).unwrap());
                // No more manual drawing. The completion must wake this window.
                let initial = ready(cx,window,1).await;
                assert_eq!(initial.matches.counts[0].total,3);
                window.update(cx,|scene,_,cx| {
                    let handle=scene.handle.as_ref().unwrap();
                    let mut c=(*original_config).clone();c.0[0].appearance.radius=6.;
                    assert_eq!(handle.update(original_source.clone(),Arc::new(c),cx).unwrap(),jobs::Update::Presentation);
                    assert_eq!(handle.epoch(),1);
                    let jobs::Status::Ready(current)=handle.status() else {panic!("cosmetic update rematched")};
                    assert!(Arc::ptr_eq(&initial,&current));
                    for text in ["a", "a a", "a a a a"] {
                        assert_eq!(handle.update(source(text),original_config.clone(),cx).unwrap(),jobs::Update::Rematch);
                        assert!(matches!(handle.status(),jobs::Status::Pending));
                    }
                    cx.notify();
                }).unwrap();
                let latest=ready(cx,window,4).await;
                assert_eq!(latest.matches.counts[0].total,4);
                drop((initial,latest));
                // Detach the handle from the entity: closing the OS window must
                // cancel it even though this task still owns a live native handle.
                let survivor=window.update(cx,|scene,window,cx| {
                    let handle=scene.handle.take().unwrap();
                    handle.update(source(&"a".repeat(1024*1024)),original_config.clone(),cx).unwrap();
                    let service=cx.global::<Global>().0.clone();
                    pump(&service,cx);
                    assert_eq!(service.borrow().pool.running_count(),1);
                    window.remove_window(); handle
                }).unwrap();
                pause(cx).await;
                assert!(matches!(survivor.status(),jobs::Status::Failed(jobs::Error::Closed)));
                cx.update(|cx| {
                    let state=cx.global::<Global>().0.borrow();
                    assert!(state.routes.is_empty());assert!(state.windows.is_empty());
                });
                drop(survivor);
                // Quit while two actual executor jobs are in flight. Their
                // outputs must not wait for UI delivery after the receiver closes.
                let (quit_window, survivors)=cx.update(|cx| {
                    let window=cx.open_window(WindowOptions { focus:false, ..Default::default() },|_,cx|cx.new(|_|Scene{handle:None,pending_paints:0,ready_epoch:0})).unwrap();
                    let handles=window.update(cx,|_,window,cx| {
                        (0..2).map(|_|request(source(&"a".repeat(1024*1024)),config(),window,cx).unwrap()).collect::<Vec<_>>()
                    }).unwrap();
                    let service=cx.global::<Global>().0.clone();
                    pump(&service,cx);
                    assert_eq!(service.borrow().pool.running_count(),2);
                    (window,handles)
                });
                cx.update(|cx| {
                    let (service, fences)=begin_close(cx).unwrap();
                    assert_eq!(fences.len(),2);
                    assert_eq!(service.borrow().workers.len(),2,"reentrant quit retains worker fences");
                });
                shutdown(cx).await;
                assert!(survivors.iter().all(|h|matches!(h.status(),jobs::Status::Failed(jobs::Error::Closed))));
                drop(survivors);
                quit_window.update(cx,|_,window,_|window.remove_window()).unwrap();
                cx.update(|cx| {
                    let service=cx.global::<Global>().0.borrow();
                    assert!(service.closed); assert!(service.workers.is_empty());
                    assert!(service.input.is_empty()); assert!(service.delivering.is_none());
                    assert_eq!(service.pool.reserved_bytes(),0);
                    assert_eq!(service.pool.running_count(),0);
                });
                eprintln!("GPUIO_NATIVE_HIGHLIGHT_HOST_OK: real worker completion wakes background window; cosmetics reuse, latest request, closed-window survivor cancellation and shutdown reclaim ownership");
            }).await;
            *task_failure.borrow_mut()=result.err();
            // Cleanup also runs after an assertion failure; no orphan test window.
            shutdown(cx).await;
            cx.update(|cx| { for window in cx.windows() { let _=window.update(cx,|_,window,_|window.remove_window()); } });
            cx.update(crate::host::stop_application);
        }).detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
