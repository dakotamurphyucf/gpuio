//! Production View paint/clock policy on TestPlatform, without an OS window.
use super::super::*;
use gpui::TestAppContext;
use gpuio_protocol::loading::{Config, Kind as LoadingKind};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}

fn apply(view: &mut View, window: &mut Window, cx: &mut Context<View>, operations: Vec<Op>) {
    let base = view.session.borrow().tree(view.id).unwrap().revision();
    let applied = view
        .session
        .borrow_mut()
        .apply(&Transaction {
            window: view.id,
            base,
            revision: base + 1,
            operations,
        })
        .unwrap();
    view.update_editors(&applied.dirty, window, cx);
    cx.notify();
}

#[test]
fn inert_loading_keeps_artwork_without_recurring_frames_and_hidden_still_removes_paint() {
    for kind in [
        LoadingKind::Skeleton,
        LoadingKind::Shimmer,
        LoadingKind::Spinner,
    ] {
        for animated in [false, true] {
            let mut app = TestAppContext::single();
            let (_reader, writer) = UnixStream::pair().unwrap();
            let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
            let session = Rc::new(RefCell::new(Session::default()));
            let window_id = WindowId::from_parts(0, 1).unwrap();
            session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
            session
                .borrow_mut()
                .open(1, window_id, "Loading", 100., 100.)
                .unwrap();
            let (owner, cx) = app
                .add_window_view(|_, _| View::new(window_id, session.clone(), transport.clone()));
            cx.update(|window, cx| {
                owner.update(cx, |view, cx| {
                    apply(
                        view,
                        window,
                        cx,
                        vec![
                            Op::Create(id(0), Kind::Container, String::new(), None),
                            Op::Create(id(1), Kind::Loading, String::new(), None),
                            Op::SetLoading(
                                id(1),
                                Config {
                                    kind,
                                    label: "Loading records".into(),
                                    animated,
                                    period_ms: 1200,
                                },
                            ),
                            Op::SetStyle(
                                id(1),
                                vec![Style::Fields(vec![
                                    Field::Width(Length::Px(64.)),
                                    Field::Height(Length::Px(32.)),
                                ])],
                            ),
                            Op::Splice(id(0), 0, 0, vec![id(1)]),
                            Op::SetRoot(Some(id(0))),
                        ],
                    );
                });
                window.draw(cx).clear(cx);
            });
            cx.run_until_parked();
            let weak = owner.read_with(cx, |view, _| Rc::downgrade(&view.loading_probes[&id(1)]));
            let first = weak.upgrade().unwrap().get().count;
            assert!(first > 0, "ordinary loading paints");

            for inert_owner in [id(0), id(1)] {
                let before = weak.upgrade().unwrap().get().count;
                cx.update(|window, cx| {
                    owner.update(cx, |view, cx| {
                        apply(
                            view,
                            window,
                            cx,
                            vec![Op::SetStyle(
                                inert_owner,
                                vec![Style::Fields(vec![Field::Inert(true)])],
                            )],
                        );
                    });
                    window.draw(cx).clear(cx);
                });
                let paint = weak.upgrade().unwrap().get();
                assert!(
                    paint.count > before,
                    "{kind:?}: own/ancestor inert policy must retain loading artwork"
                );
                assert_eq!(
                    paint.phase,
                    if kind == LoadingKind::Shimmer {
                        0.5
                    } else {
                        0.
                    }
                );
                owner.read_with(cx, |view, _| {
                    assert!(
                        !view.focus.borrow().visible(id(1)),
                        "input/timer eligibility stays fenced"
                    );
                });
                // Drain a callback queued by the old animated frame and any initial
                // focus work. Static inert artwork must not replenish the queue.
                for _ in 0..2 {
                    cx.update(|window, cx| {
                        window.simulate_next_frame(cx);
                        window.draw(cx).clear(cx);
                    });
                    cx.run_until_parked();
                }
                cx.update(|window, cx| assert_eq!(window.simulate_next_frame(cx), 0));

                let before_hidden = weak.upgrade().unwrap().get().count;
                cx.update(|window, cx| {
                    owner.update(cx, |view, cx| {
                        apply(
                            view,
                            window,
                            cx,
                            vec![Op::SetStyle(
                                inert_owner,
                                vec![Style::Fields(vec![
                                    Field::Inert(true),
                                    Field::Visibility(1),
                                ])],
                            )],
                        );
                    });
                    window.draw(cx).clear(cx);
                    assert_eq!(window.simulate_next_frame(cx), 0);
                });
                assert_eq!(
                    weak.upgrade().unwrap().get().count,
                    before_hidden,
                    "hidden inert artwork must not paint"
                );

                cx.update(|window, cx| {
                    owner.update(cx, |view, cx| {
                        apply(view, window, cx, vec![Op::SetStyle(inert_owner, vec![])]);
                    });
                    window.draw(cx).clear(cx);
                    assert_eq!(
                        window.simulate_next_frame(cx) > 0,
                        animated,
                        "revealing loading resumes native frame demand only when configured"
                    );
                });
                assert!(weak.upgrade().unwrap().get().count > before_hidden);
                owner.read_with(cx, |view, _| {
                    assert!(Rc::ptr_eq(
                        &weak.upgrade().unwrap(),
                        &view.loading_probes[&id(1)]
                    ))
                });
            }
            cx.update(|window, cx| {
                owner.update(cx, |view, cx| {
                    apply(
                        view,
                        window,
                        cx,
                        vec![Op::SetRoot(None), Op::Remove(id(1)), Op::Remove(id(0))],
                    );
                });
                window.draw(cx).clear(cx);
                window.simulate_next_frame(cx);
                window.draw(cx).clear(cx);
            });
            cx.run_until_parked();
            assert!(
                weak.upgrade().is_none(),
                "unmount retires the loading paint owner"
            );
            assert_eq!(session.borrow().retained_bytes(), 0);
        }
    }
}
