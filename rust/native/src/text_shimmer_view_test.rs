//! Actual retained Host/Session integration. Background windows, controlled native
//! time, GPU readback and dispatched GPUI keys; no OS keyboard/IME claim.
use super::styled_text_test::{apply, draw};
use super::*;
use crate::text_shimmer_clock::{Clock, probe::Probe};
use gpuio_protocol::text_shimmer::{Appearance, Config, Direction, Repeat, Spread};
use std::{
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    time::Duration,
};

#[path = "text_shimmer_lifecycle_test.rs"]
mod lifecycle;

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn config() -> Config {
    Config {
        duration_ms: 1000,
        spread: Spread::Relative(0.3),
        direction: Direction::LeftToRight,
        repeat: Repeat::Loop,
        animated: true,
        highlight: Some(0xff0000ff),
        appearance: None,
    }
}
fn root(extra: Vec<Field>) -> Vec<Style> {
    let mut fields = vec![
        Field::Width(Length::Px(360.)),
        Field::Height(Length::Px(260.)),
        Field::Background(Fill::Solid(Color::Rgba(0xffffffff))),
        Field::OverflowX(1),
        Field::OverflowY(1),
    ];
    fields.extend(extra);
    vec![Style::Fields(fields)]
}
fn row(y: f64, selectable: bool) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Position(1),
        Field::Left(Length::Px(20.)),
        Field::Top(Length::Px(y)),
        Field::Width(Length::Px(310.)),
        Field::Height(Length::Px(100.)),
        Field::FontSize(24.),
        Field::LineHeight(Length::Px(30.)),
        Field::Foreground(Color::Rgba(0x000000ff)),
        Field::UserSelect(selectable),
    ])]
}
fn delivery(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.simulate_next_frame(cx);
    })
    .unwrap();
}
fn image(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> image::RgbaImage {
    handle
        .update(cx, |_, window, _| window.render_to_image().unwrap())
        .unwrap()
}
fn colors(image: &image::RgbaImage) -> [usize; 3] {
    let mut count = [0; 3];
    for pixel in image.pixels() {
        let [r, g, b, _] = pixel.0;
        count[0] += usize::from(r > 70 && r > g.saturating_add(30) && r > b.saturating_add(30));
        count[1] += usize::from(b > 70 && b > r.saturating_add(30) && b > g.saturating_add(30));
        count[2] += usize::from(g > 180 && r > 180 && b < 80);
    }
    count
}
fn probe(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, node: NodeId) -> Probe {
    handle
        .update(cx, |view, _, _| view.text_shimmers[&node].probe())
        .unwrap()
}
fn both(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, config: Option<Config>) {
    apply(
        cx,
        handle,
        vec![
            Op::SetTextShimmer(id(1), config),
            Op::SetTextShimmer(id(2), config),
        ],
    );
}
fn idle(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, owners: &[&Probe]) {
    let before: Vec<_> = owners
        .iter()
        .map(|p| {
            let s = p.snapshot().unwrap();
            assert!(!s.running);
            s.notifications
        })
        .collect();
    delivery(cx, handle);
    draw(cx, handle);
    delivery(cx, handle);
    for (owner, before) in owners.iter().zip(before) {
        let state = owner.snapshot().unwrap();
        assert!(!state.running && !state.pending);
        assert_eq!(
            state.notifications, before,
            "ineligible mounted text must not wake the Host"
        );
    }
}

async fn independent_windows(
    cx: &mut gpui::AsyncApp,
    first: WindowHandle<View>,
    first_clock: &Clock,
    now: u64,
    first_node: NodeId,
) {
    let (session, transport) = first
        .update(cx, |view, _, _| {
            (view.session.clone(), view.transport.clone())
        })
        .unwrap();
    let window_id = WindowId::from_parts(1, 1).unwrap();
    session
        .borrow_mut()
        .open(2, window_id, "Independent shimmer check", 360., 260.)
        .unwrap();
    let clock = Rc::new(Clock::default());
    clock.set_time(0);
    let owner_clock = clock.clone();
    let other = cx.update(|cx| {
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(360.), px(260.)),
                    cx,
                ))),
                focus: false,
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| {
                    let mut view = View::new(window_id, session, transport);
                    view.text_shimmer_clock = owner_clock;
                    view
                })
            },
        )
        .unwrap()
    });
    apply(
        cx,
        other,
        vec![
            Op::Create(id(0), Kind::Text, "Independent window".into(), None),
            Op::SetStyle(id(0), row(10., false)),
            Op::SetTextShimmer(id(0), Some(config())),
            Op::SetRoot(Some(id(0))),
        ],
    );
    draw(cx, other);
    let first_owner = probe(cx, first, first_node);
    let other_owner = probe(cx, other, id(0));
    clock.set_time(250);
    draw(cx, other);
    assert_eq!(other_owner.snapshot().unwrap().phase, 0.25);
    assert_eq!(first_owner.snapshot().unwrap().phase, 0.);
    first_clock.set_time(now + 500);
    draw(cx, first);
    assert_eq!(first_owner.snapshot().unwrap().phase, 0.5);
    assert_eq!(other_owner.snapshot().unwrap().phase, 0.25);
    first
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    for _ in 0..100 {
        if first_owner.snapshot().is_none() {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    assert!(
        first_owner.snapshot().is_none(),
        "closing one window releases its owner while the other stays open"
    );
    draw(cx, other);
    assert!(other_owner.snapshot().unwrap().running);
    assert_eq!(other_owner.snapshot().unwrap().phase, 0.25);
    other
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    for _ in 0..100 {
        if other_owner.snapshot().is_none() {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    assert!(
        other_owner.snapshot().is_none(),
        "window disposal does not require application shutdown"
    );
}

async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, clock: &Rc<Clock>) {
    let source = "Working Aé世界 e\u{301} 👩‍💻 repeated words wrap onto another line";
    apply(
        cx,
        handle,
        vec![
            Op::Create(id(0), Kind::Container, String::new(), None),
            Op::SetStyle(id(0), root(vec![])),
            Op::Create(id(1), Kind::Text, source.into(), None),
            Op::SetStyle(id(1), row(10., false)),
            Op::Create(id(2), Kind::Text, source.into(), None),
            Op::SetStyle(id(2), row(140., true)),
            Op::Create(id(3), Kind::HighlightScope, String::new(), None),
            Op::SetHighlightScope(id(3), gpuio_protocol::highlight::Config(vec![])),
            Op::SetStyle(
                id(3),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(360.)),
                    Field::Height(Length::Px(260.)),
                ])],
            ),
            Op::Splice(id(3), 0, 0, vec![id(1), id(2)]),
            Op::Splice(id(0), 0, 0, vec![id(3)]),
            Op::SetRoot(Some(id(0))),
        ],
    );
    draw(cx, handle);
    let base = image(cx, handle);
    let geometry = styled_text_test::geometry(cx, handle);
    assert!(
        geometry.iter().any(|p| p.y > geometry[0].y + px(30.)),
        "fixture really wraps"
    );
    let selection = handle
        .update(cx, |view, window, cx| {
            let state = view.selections[&id(2)].clone();
            window.focus(&state.borrow().focus, cx);
            state
        })
        .unwrap();
    both(cx, handle, Some(config()));
    draw(cx, handle);
    assert_eq!(image(cx, handle), base, "first phase preserves base glyphs");
    let first = probe(cx, handle, id(1));
    let second = probe(cx, handle, id(2));
    for _ in 0..20 {
        draw(cx, handle);
    }
    delivery(cx, handle);
    assert_eq!(first.snapshot().unwrap().notifications, 1);
    assert_eq!(second.snapshot().unwrap().notifications, 1);
    clock.set_time(500);
    draw(cx, handle);
    assert_eq!(first.snapshot().unwrap().phase, 0.5);
    assert!(
        colors(&image(cx, handle))[0] > 30,
        "native time reaches actual red glyph paint"
    );
    styled_text_test::assert_geometry(&styled_text_test::geometry(cx, handle), &geometry);
    editor_test::key(cx, handle, "secondary-a");
    draw(cx, handle);
    assert_eq!(styled_text_test::copied(cx, handle), source);
    let range = selection.borrow().selection.clone();
    let content = gpuio_protocol::text_content::Content {
        text: source.into(),
        spans: vec![gpuio_protocol::text_content::Span {
            start_byte: 0,
            end_byte: 7,
            foreground: 0x008000ff,
        }],
    };
    apply(
        cx,
        handle,
        vec![
            Op::SetStyledText(id(1), content.clone()),
            Op::SetStyledText(id(2), content),
        ],
    );
    draw(cx, handle);
    handle
        .update(cx, |view, _, _| {
            let session = view.session.borrow();
            let tree = session.tree(view.id).unwrap();
            for node in [id(1), id(2)] {
                assert!(
                    view.text_shimmers[&node]
                        .probe()
                        .shares_source(&tree.get(node).unwrap().text)
                );
            }
        })
        .unwrap();
    assert_eq!(second.snapshot().unwrap().phase, 0.5);
    assert_eq!(selection.borrow().selection, range);
    assert_eq!(styled_text_test::copied(cx, handle), source);
    styled_text_test::assert_geometry(&styled_text_test::geometry(cx, handle), &geometry);
    #[cfg(target_os = "macos")]
    {
        let mut labels = Vec::new();
        for _ in 0..100 {
            // AppKit requests the lazily built AccessKit tree on first query.
            labels = styled_text_test::accessible_labels(cx, handle);
            if labels
                .iter()
                .filter(|label| label.as_str() == source)
                .count()
                == 2
            {
                break;
            }
            cx.background_executor()
                .timer(Duration::from_millis(10))
                .await;
            draw(cx, handle);
        }
        assert_eq!(
            labels
                .iter()
                .filter(|label| label.as_str() == source)
                .count(),
            2,
            "full source AX labels: {labels:?}"
        );
    }
    // Application appearance is independent of the actual OS palette. Both use
    // the same source and time, so a theme change cannot restart the sweep.
    editor_test::key(cx, handle, "home");
    draw(cx, handle);
    for dark in [true, false] {
        both(
            cx,
            handle,
            Some(Config {
                highlight: None,
                appearance: Some(Appearance {
                    foreground: 0xff0000ff,
                    background: 0x0000ffff,
                    dark,
                }),
                ..config()
            }),
        );
        draw(cx, handle);
        assert_eq!(first.snapshot().unwrap().phase, 0.5);
        let found = colors(&image(cx, handle));
        assert!(
            found[if dark { 0 } else { 1 }] > 30,
            "application appearance paints the selected target: {found:?}"
        );
    }
    both(cx, handle, Some(config()));
    draw(cx, handle);
    let mut now = 500;
    for fields in [
        vec![Field::Opacity(0.)],
        vec![Field::Visibility(1)],
        vec![Field::Display(3)],
    ] {
        apply(cx, handle, vec![Op::SetStyle(id(0), root(fields))]);
        draw(cx, handle);
        let phase = first.snapshot().unwrap().phase;
        idle(cx, handle, &[&first, &second]);
        now += 2000;
        clock.set_time(now);
        draw(cx, handle);
        assert_eq!(first.snapshot().unwrap().phase, phase);
        apply(cx, handle, vec![Op::SetStyle(id(0), root(vec![]))]);
        draw(cx, handle);
        assert!(first.snapshot().unwrap().running && second.snapshot().unwrap().running);
        assert_eq!(first.snapshot().unwrap().phase, phase);
    }
    // Native interaction refinements change opacity without bridge traffic.
    for state in [2, 3] {
        native_test::move_mouse(cx, handle, gpui::point(px(380.), px(280.)), false);
        let mut styles = root(vec![]);
        styles.push(Style::State(state, vec![Field::Opacity(0.)]));
        apply(cx, handle, vec![Op::SetStyle(id(0), styles)]);
        draw(cx, handle);
        let revision = handle
            .update(cx, |view, _, _| {
                view.session.borrow().tree(view.id).unwrap().revision()
            })
            .unwrap();
        if state == 2 {
            native_test::move_mouse(cx, handle, gpui::point(px(345.), px(245.)), false);
        } else {
            native_test::mouse(cx, handle, gpui::point(px(345.), px(245.)), true);
        }
        draw(cx, handle);
        idle(cx, handle, &[&first, &second]);
        if state == 3 {
            native_test::mouse(cx, handle, gpui::point(px(380.), px(280.)), false);
        }
        native_test::move_mouse(cx, handle, gpui::point(px(380.), px(280.)), false);
        draw(cx, handle);
        assert!(first.snapshot().unwrap().running && second.snapshot().unwrap().running);
        assert_eq!(
            handle
                .update(cx, |view, _, _| view
                    .session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .revision())
                .unwrap(),
            revision
        );
    }
    apply(cx, handle, vec![Op::SetStyle(id(0), root(vec![]))]);
    let mut styles = row(140., true);
    styles.push(Style::State(1, vec![Field::Opacity(0.)]));
    apply(cx, handle, vec![Op::SetStyle(id(2), styles)]);
    handle
        .update(cx, |_, window, cx| {
            window.focus(&selection.borrow().focus, cx)
        })
        .unwrap();
    draw(cx, handle);
    idle(cx, handle, &[&second]);
    assert!(first.snapshot().unwrap().running);
    handle
        .update(cx, |view, window, cx| {
            window.focus(view.root_focus.as_ref().unwrap(), cx)
        })
        .unwrap();
    draw(cx, handle);
    assert!(second.snapshot().unwrap().running);
    apply(cx, handle, vec![Op::SetStyle(id(2), row(140., true))]);
    handle
        .update(cx, |_, window, cx| {
            window.focus(&selection.borrow().focus, cx)
        })
        .unwrap();
    cx.update(|cx| cx.set_reduce_motion(true));
    draw(cx, handle);
    idle(cx, handle, &[&first, &second]);
    cx.update(|cx| cx.set_reduce_motion(false));
    draw(cx, handle);
    both(
        cx,
        handle,
        Some(Config {
            animated: false,
            ..config()
        }),
    );
    draw(cx, handle);
    idle(cx, handle, &[&first, &second]);
    both(cx, handle, Some(config()));
    draw(cx, handle);
    // Ancestor clipping skips paint even though the nodes are retained/rendered.
    apply(cx, handle, vec![Op::SetStyle(id(1), row(900., false))]);
    draw(cx, handle);
    idle(cx, handle, &[&first]);
    assert!(second.snapshot().unwrap().running);
    apply(cx, handle, vec![Op::SetStyle(id(1), row(10., false))]);
    draw(cx, handle);
    // Existing search underlay and text selection share the unchanged layout.
    apply(
        cx,
        handle,
        vec![Op::SetHighlightScope(
            id(3),
            gpuio_protocol::highlight::Config(vec![gpuio_protocol::highlight::Spec {
                query: Some(gpuio_protocol::highlight::Query {
                    text: "Working".into(),
                    case_sensitive: true,
                    whole_word: false,
                }),
                ranges: vec![],
                appearance: gpuio_protocol::highlight::Appearance {
                    color: 0xffff00ff,
                    active_color: 0xffff00ff,
                    radius: 0.,
                },
                active_index: None,
                match_index_offset: 0,
            }]),
        )],
    );
    let mut ready = false;
    for _ in 0..120 {
        draw(cx, handle);
        ready = handle
            .update(cx, |view, _, _| {
                matches!(
                    view.highlights[&id(3)].borrow().observation().state,
                    gpuio_protocol::highlight::State::Ready(_)
                )
            })
            .unwrap();
        if ready {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    assert!(ready, "highlight worker completed");
    draw(cx, handle);
    assert!(
        colors(&image(cx, handle))[2] > 30,
        "search background remains visible with glyph effect"
    );
    assert!(
        colors(&image(cx, handle))[0] > 30,
        "glyph effect remains visible over search background"
    );
    editor_test::key(cx, handle, "secondary-a");
    draw(cx, handle);
    assert_eq!(styled_text_test::copied(cx, handle), source);
    both(cx, handle, None);
    draw(cx, handle);
    assert!(
        first.snapshot().is_none() && second.snapshot().is_none(),
        "clear retires strong owners"
    );
    handle
        .update(cx, |view, _, _| {
            assert!(Rc::ptr_eq(&selection, &view.selections[&id(2)]))
        })
        .unwrap();
    assert_eq!(styled_text_test::copied(cx, handle), source);
    both(
        cx,
        handle,
        Some(Config {
            repeat: Repeat::Once,
            ..config()
        }),
    );
    draw(cx, handle);
    let once = probe(cx, handle, id(1));
    assert_eq!(once.snapshot().unwrap().phase, 0.);
    clock.set_time(now + 1000);
    draw(cx, handle);
    assert_eq!(once.snapshot().unwrap().phase, 1.);
    idle(cx, handle, &[&once]);
    // Generation replacement releases the old owner before drawing the new node.
    let next = NodeId::from_parts(1, 2).unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::Splice(id(3), 0, 1, vec![]),
            Op::Remove(id(1)),
            Op::Create(next, Kind::Text, "Fresh".into(), None),
            Op::SetStyle(next, row(10., false)),
            Op::SetTextShimmer(next, Some(config())),
            Op::Splice(id(3), 0, 0, vec![next]),
        ],
    );
    assert!(once.snapshot().is_none());
    draw(cx, handle);
    assert_eq!(probe(cx, handle, next).snapshot().unwrap().phase, 0.);
    now += 1000;
    lifecycle::exercise(cx, handle, clock, &mut now).await;
    independent_windows(cx, handle, clock, now, next).await;
    eprintln!(
        "GPUIO_NATIVE_TEXT_SHIMMER_VIEW_OK: mounted glyph pixels, stable wrap/selection/source AX, spans/highlight, application themes, coalesced wakes, opacity/hover/press/focus/visibility/display/clip/Reduce/static pause, clear, generation replacement and independent windows/close"
    );
}

pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let retired = Rc::new(RefCell::new(Vec::new()));
    let closed_owners = retired.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let saved = cx.read_from_clipboard();
        let reduced = cx.reduce_motion();
        cx.set_reduce_motion(false);
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        let window_id = WindowId::from_parts(0, 1).unwrap();
        session
            .borrow_mut()
            .open(1, window_id, "Mounted text shimmer check", 360., 260.)
            .unwrap();
        let clock = Rc::new(Clock::default());
        clock.set_time(0);
        let owner_clock = clock.clone();
        let handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(360.), px(260.)),
                        cx,
                    ))),
                    focus: false,
                    ..Default::default()
                },
                |_, cx| {
                    cx.new(|_| {
                        let mut view = View::new(window_id, session, transport);
                        view.text_shimmer_clock = owner_clock;
                        view
                    })
                },
            )
            .unwrap();
        cx.spawn(async move |cx| {
            let checked = native_test::protect(exercise(cx, handle, &clock)).await;
            *task_failure.borrow_mut() = checked.err();
            let owners = handle
                .update(cx, |view, _, _| {
                    view.text_shimmers
                        .values()
                        .map(|o| o.probe())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            cx.update(|cx| {
                cx.write_to_clipboard(
                    saved.unwrap_or_else(|| gpui::ClipboardItem::new_string(String::new())),
                );
                cx.set_reduce_motion(reduced);
                stop_application(cx);
            });
            *closed_owners.borrow_mut() = owners;
        })
        .detach();
    });
    assert!(
        retired.borrow().iter().all(|o| o.snapshot().is_none()),
        "window close retires every owner"
    );
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
