//! Actual GPUIO track admission, measurement, translated hitboxes and retained owners.
//! GPUI TestPlatform evidence; no physical desktop window is opened.
use super::*;
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpuio_protocol::{
    HandlerId,
    carousel::{self, Axis, Direction},
    carousel_track::{Config, Request},
};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
fn n(i: i64) -> NodeId {
    NodeId::from_parts(i, 1).unwrap()
}
fn h(i: i64) -> HandlerId {
    HandlerId::from_parts(i, 1).unwrap()
}
fn w() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn config(axis: Axis, selected: i64, revision: i64) -> Config {
    Config {
        lineage: 0,
        carousel: carousel::Config {
            revision,
            ids: vec!["a".into(), "b".into(), "c".into()],
            selected: Some(selected),
            looping: false,
            disabled: false,
            axis,
            auto_advance_ms: None,
            direction: Direction::Direct,
        },
    }
}
fn initial(axis: Axis) -> Vec<Op> {
    let vertical = axis == Axis::Vertical;
    let mut ops = vec![
        Op::Create(n(0), Kind::CarouselTrack, "Cards".into(), Some(h(0))),
        Op::Create(n(1), Kind::Container, "".into(), None),
        Op::SetCarouselTrack(n(0), config(axis, 0, 0)),
        Op::SetStyle(
            n(0),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(if vertical { 80. } else { 100. })),
                Field::Height(Length::Px(if vertical { 100. } else { 80. })),
            ])],
        ),
        Op::SetStyle(
            n(1),
            vec![Style::Fields(vec![
                Field::RowGap(Length::Px(8.)),
                Field::ColumnGap(Length::Px(8.)),
            ])],
        ),
    ];
    for (index, extent) in [40., 120., 64.].into_iter().enumerate() {
        let panel = n(2 + index as i64);
        let button = n(5 + index as i64);
        ops.extend([
            Op::Create(panel, Kind::Panel, format!("Card {index}"), None),
            Op::Create(
                button,
                Kind::Button,
                format!("Action {index}"),
                Some(h(5 + index as i64)),
            ),
            Op::SetControl(button, Control::Button(false)),
            Op::SetStyle(
                panel,
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(if vertical { 80. } else { extent })),
                    Field::Height(Length::Px(if vertical { extent } else { 80. })),
                    Field::Shrink(0.),
                    Field::Background(Fill::Solid(Color::Rgba(
                        0x102030ff + ((index as i64) << 24),
                    ))),
                ])],
            ),
            Op::SetStyle(
                button,
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(30.)),
                    Field::Height(Length::Px(24.)),
                ])],
            ),
            Op::Splice(panel, 0, 0, vec![button]),
        ]);
    }
    ops.extend([
        Op::Splice(n(1), 0, 0, vec![n(2), n(3), n(4)]),
        Op::Splice(n(0), 0, 0, vec![n(1)]),
        Op::SetRoot(Some(n(0))),
    ]);
    ops.sort_by_key(|op| match op {
        Op::Create(node, ..) => node.slot(),
        _ => usize::MAX,
    });
    ops
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, ops: Vec<Op>) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let base = view.session.borrow().tree(w()).unwrap().revision();
            let applied = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: w(),
                    base,
                    revision: base + 1,
                    operations: ops,
                })
                .unwrap();
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
}
#[test]
fn measured_track_repositions_retained_neighbors_in_the_same_frame_on_both_axes() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, w(), "Track", 400., 300.)
            .unwrap();
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
        cx.simulate_a11y_active(true);
        apply(&owner, cx, initial(axis));
        let events = transport.mailbox.lock().unwrap().drain(100);
        assert!(events.iter().any(|e| matches!(e,Event::CarouselTrackRequested(_,_,_,_,Request::Layout(l)) if l.stops.as_ref().is_some_and(|s| s.canonical==vec![0,1,2]))),"{events:?}");
        let focus = owner.read_with(cx, |view, _| view.buttons[&n(6)].focus.clone());
        let scale = cx.update(|window, _| f64::from(window.scale_factor()));
        let ax = cx.a11y_tree().unwrap();
        let bounds = |name: &str| {
            ax.nodes
                .iter()
                .find(|(_, node)| node.label() == Some(name))
                .unwrap()
                .1
                .bounds()
                .unwrap()
        };
        let first = bounds("Action 0");
        let second = bounds("Action 1");
        let delta = if axis == Axis::Vertical {
            second.y0 - first.y0
        } else {
            second.x0 - first.x0
        };
        assert!((delta - 48. * scale).abs() < 0.1, "delta={delta}");
        // A partially visible nonselected card has an ordinary live native button.
        let point = gpui::point(
            px((second.x0 / scale) as f32 + 3.),
            px((second.y0 / scale) as f32 + 3.),
        );
        cx.simulate_mouse_move(point, None, Default::default());
        cx.simulate_click(point, Default::default());
        let events = transport.mailbox.lock().unwrap().drain(100);
        assert!(
            events
                .iter()
                .any(|e| matches!(e,Event::Press(_,node,_,_) if *node==n(6))),
            "{events:?}"
        );
        apply(
            &owner,
            cx,
            vec![Op::SetCarouselTrack(n(0), config(axis, 1, 1))],
        );
        owner.read_with(cx, |view, _| assert_eq!(view.buttons[&n(6)].focus, focus));
        let ax = cx.a11y_tree().unwrap();
        let second = ax
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Action 1"))
            .unwrap()
            .1
            .bounds()
            .unwrap();
        let coordinate = if axis == Axis::Vertical {
            second.y0
        } else {
            second.x0
        };
        let start = if axis == Axis::Vertical {
            first.y0
        } else {
            first.x0
        };
        assert!(
            (coordinate - start).abs() < 0.1,
            "selected position={coordinate} start={start}"
        );
        cx.update(|window, _| {
            let background: gpui::Background = rgba(0x112030ff).into();
            let quads = window.painted_quads();
            let painted: Vec<_> = quads
                .iter()
                .filter(|quad| quad.background == background)
                .collect();
            assert_eq!(painted.len(), 1, "one painted owner for selected card");
            let origin = painted[0].bounds.origin;
            let painted_coordinate = if axis == Axis::Vertical {
                origin.y.0
            } else {
                origin.x.0
            };
            assert!(
                (f64::from(painted_coordinate) - coordinate).abs() < 0.1,
                "painted card and accessible/input bounds must move together"
            );
        });
        let events = transport.mailbox.lock().unwrap().drain(100);
        assert!(!events.iter().any(|e| matches!(
            e,
            Event::CarouselTrackRequested(_, _, _, _, Request::Layout(_))
        )));
        apply(
            &owner,
            cx,
            vec![Op::SetCarouselTrack(n(0), config(axis, 2, 2))],
        );
        let position = |cx: &mut VisualTestContext| {
            let ax = cx.a11y_tree().unwrap();
            let bounds = ax
                .nodes
                .iter()
                .find(|(_, node)| node.label() == Some("Action 2"))
                .unwrap()
                .1
                .bounds()
                .unwrap();
            (if axis == Axis::Vertical {
                bounds.y0
            } else {
                bounds.x0
            }) / scale
        };
        assert!((position(cx) - 36.).abs() < 0.1);
        let resize = |extent| {
            Op::SetStyle(
                n(0),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(if axis == Axis::Horizontal {
                        extent
                    } else {
                        80.
                    })),
                    Field::Height(Length::Px(if axis == Axis::Vertical {
                        extent
                    } else {
                        80.
                    })),
                ])],
            )
        };
        transport.mailbox.lock().unwrap().drain(100);
        apply(&owner, cx, vec![resize(150.)]);
        assert!((position(cx) - 86.).abs() < 0.1);
        let events = transport.mailbox.lock().unwrap().drain(100);
        assert!(events.iter().any(|e|matches!(e,Event::CarouselTrackRequested(_,_,_,_,Request::Layout(l)) if l.epoch==1 && l.stops.as_ref().is_some_and(|s|s.canonical==vec![0,1,2]))));
        apply(
            &owner,
            cx,
            vec![Op::SetStyle(
                n(1),
                vec![Style::Fields(vec![
                    Field::RowGap(Length::Px(8.)),
                    Field::ColumnGap(Length::Px(8.)),
                    Field::PaddingLeft(Length::Px(16.)),
                    Field::PaddingRight(Length::Px(16.)),
                    Field::PaddingTop(Length::Px(16.)),
                    Field::PaddingBottom(Length::Px(16.)),
                ])],
            )],
        );
        assert!(
            (position(cx) - 70.).abs() < 0.1,
            "padded position={}",
            position(cx)
        );
        let mut looping = config(axis, 2, 3);
        looping.lineage = 1;
        looping.carousel.looping = true;
        apply(
            &owner,
            cx,
            vec![Op::SetCarouselTrack(n(0), looping.clone())],
        );
        let events = transport.mailbox.lock().unwrap().drain(100);
        assert!(events.iter().any(|e|matches!(e,Event::CarouselTrackRequested(_,_,_,_,Request::Layout(l)) if l.lineage==1 && l.stops.as_ref().is_some_and(|s|s.looping==gpuio_protocol::carousel_track::Loop::Jump))));
        apply(&owner, cx, vec![resize(100.)]);
        assert!((position(cx) - 16.).abs() < 0.1);
        let events = transport.mailbox.lock().unwrap().drain(100);
        assert!(events.iter().any(|e|matches!(e,Event::CarouselTrackRequested(_,_,_,_,Request::Layout(l)) if l.stops.as_ref().is_some_and(|s|s.looping==gpuio_protocol::carousel_track::Loop::Continuous))));
        owner.read_with(cx, |view, _| assert_eq!(view.buttons[&n(6)].focus, focus));
        let ax = cx.a11y_tree().unwrap();
        assert_eq!(
            ax.nodes
                .iter()
                .filter(|(_, node)| node
                    .label()
                    .is_some_and(|label| label.starts_with("Action ")))
                .count(),
            3
        );
        apply(&owner, cx, vec![resize(0.)]);
        let events = transport.mailbox.lock().unwrap().drain(100);
        assert!(events.iter().any(|e|matches!(e,Event::CarouselTrackRequested(_,_,_,_,Request::Layout(l)) if l.stops.is_none())));
        apply(&owner, cx, vec![resize(100.)]);
        let events = transport.mailbox.lock().unwrap().drain(100);
        assert!(events.iter().any(|e|matches!(e,Event::CarouselTrackRequested(_,_,_,_,Request::Layout(l)) if l.stops.is_some())));
        looping.carousel.revision = 4;
        looping.carousel.selected = Some(0);
        apply(
            &owner,
            cx,
            vec![Op::SetCarouselTrack(n(0), looping.clone())],
        );
        looping.lineage = 2;
        looping.carousel.revision = 5;
        looping.carousel.ids.clear();
        looping.carousel.selected = None;
        apply(
            &owner,
            cx,
            vec![
                Op::SetCarouselTrack(n(0), looping),
                Op::Splice(n(1), 0, 3, vec![]),
                Op::Remove(n(5)),
                Op::Remove(n(6)),
                Op::Remove(n(7)),
                Op::Remove(n(2)),
                Op::Remove(n(3)),
                Op::Remove(n(4)),
            ],
        );
        let events = transport.mailbox.lock().unwrap().drain(100);
        assert!(events.iter().any(|e|matches!(e,Event::CarouselTrackRequested(_,_,_,_,Request::Layout(l)) if l.stops.as_ref().is_some_and(|s|s.canonical.is_empty()))));
        apply(
            &owner,
            cx,
            vec![Op::SetRoot(None), Op::Remove(n(1)), Op::Remove(n(0))],
        );
        owner.read_with(cx, |view, _| assert!(view.carousel_tracks.is_empty()));
        assert_eq!(session.borrow().retained_bytes(), 0);
    }
}

fn tick(cx: &mut VisualTestContext, ms: u64) {
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(ms));
    cx.update(|window, cx| {
        window.simulate_next_frame(cx);
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn position(cx: &mut VisualTestContext, axis: Axis) -> f64 {
    let scale = cx.update(|window, _| f64::from(window.scale_factor()));
    let ax = cx.a11y_tree().unwrap();
    let b = ax
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Action 1"))
        .unwrap()
        .1
        .bounds()
        .unwrap();
    if axis == Axis::Horizontal {
        b.x0 / scale
    } else {
        b.y0 / scale
    }
}
#[test]
fn track_motion_paints_intermediate_retained_positions_without_geometry_events() {
    use gpuio_protocol::{animation::Easing, carousel_track::Motion};
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, w(), "Motion", 400., 300.)
            .unwrap();
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
        cx.update(|window, cx| {
            window.activate_window();
            cx.set_reduce_motion(false);
        });
        cx.run_until_parked();
        cx.simulate_a11y_active(true);
        let mut ops = initial(axis);
        ops.push(Op::SetCarouselTrackMotion(
            n(0),
            Some(Motion {
                duration_ms: 200,
                easing: Easing::Linear,
            }),
        ));
        apply(&owner, cx, ops);
        let start = position(cx, axis);
        let focus = owner.read_with(cx, |v, _| v.buttons[&n(6)].focus.clone());
        transport.mailbox.lock().unwrap().drain(100);
        apply(
            &owner,
            cx,
            vec![Op::SetCarouselTrack(n(0), config(axis, 1, 1))],
        );
        assert_eq!(
            position(cx, axis),
            start,
            "first frame keeps painted position"
        );
        tick(cx, 100);
        let half = position(cx, axis);
        assert!(
            (half - (start - 24.)).abs() < 0.01,
            "half={half} start={start}"
        );
        // The visible quad and accessible/input geometry move together.
        cx.update(|window, _| {
            let color: gpui::Background = rgba(0x112030ff).into();
            let quads = window.painted_quads();
            let q = quads
                .iter()
                .filter(|q| q.background == color)
                .collect::<Vec<_>>();
            assert_eq!(q.len(), 1);
            let coordinate = if axis == Axis::Horizontal {
                q[0].bounds.origin.x
            } else {
                q[0].bounds.origin.y
            };
            assert!((f64::from(coordinate) / f64::from(window.scale_factor()) - half).abs() < 0.01);
        });
        let point = if axis == Axis::Horizontal {
            gpui::point(px(half as f32 + 3.), px(3.))
        } else {
            gpui::point(px(3.), px(half as f32 + 3.))
        };
        cx.simulate_mouse_move(point, None, Default::default());
        cx.simulate_click(point, Default::default());
        assert!(
            transport
                .mailbox
                .lock()
                .unwrap()
                .drain(100)
                .iter()
                .any(|e| matches!(e,Event::Press(_,node,_,_) if *node==n(6)))
        );
        assert_eq!(
            owner.read_with(cx, |v, _| v.buttons[&n(6)].focus.clone()),
            focus
        );
        apply(
            &owner,
            cx,
            vec![Op::SetCarouselTrack(n(0), config(axis, 0, 2))],
        );
        assert_eq!(
            position(cx, axis),
            half,
            "retarget keeps last painted offset"
        );
        tick(cx, 100);
        assert!((position(cx, axis) - (start - 12.)).abs() < 0.01);
        tick(cx, 100);
        assert_eq!(position(cx, axis), start);
        assert!(
            !transport
                .mailbox
                .lock()
                .unwrap()
                .drain(100)
                .iter()
                .any(|e| matches!(e, Event::CarouselTrackRequested(..)))
        );
        apply(
            &owner,
            cx,
            vec![Op::SetCarouselTrack(n(0), config(axis, 1, 3))],
        );
        tick(cx, 50);
        cx.update(|_, cx| cx.set_reduce_motion(true));
        tick(cx, 0);
        assert_eq!(position(cx, axis), start - 48.);
        cx.update(|_, cx| cx.set_reduce_motion(false));
        tick(cx, 100);
        assert_eq!(
            position(cx, axis),
            start - 48.,
            "reduced motion does not replay"
        );
        apply(
            &owner,
            cx,
            vec![Op::SetCarouselTrack(n(0), config(axis, 0, 4))],
        );
        tick(cx, 50);
        cx.deactivate_window();
        tick(cx, 0);
        assert_eq!(position(cx, axis), start, "inactive window settles");
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();
        tick(cx, 0);
        assert_eq!(position(cx, axis), start, "activation does not replay");
        apply(
            &owner,
            cx,
            vec![
                Op::SetStyle(n(0), vec![Style::Fields(vec![Field::Display(3)])]),
                Op::SetCarouselTrack(n(0), config(axis, 1, 5)),
            ],
        );
        cx.run_until_parked();
        tick(cx, 50);
        let visible_style = initial(axis)
            .into_iter()
            .find_map(|op| match op {
                Op::SetStyle(id, style) if id == n(0) => Some(style),
                _ => None,
            })
            .unwrap();
        apply(&owner, cx, vec![Op::SetStyle(n(0), visible_style)]);
        assert_eq!(
            position(cx, axis),
            start - 48.,
            "showing a hidden track does not replay travel"
        );

        apply(
            &owner,
            cx,
            vec![
                Op::SetRoot(None),
                Op::Remove(n(5)),
                Op::Remove(n(6)),
                Op::Remove(n(7)),
                Op::Remove(n(2)),
                Op::Remove(n(3)),
                Op::Remove(n(4)),
                Op::Remove(n(1)),
                Op::Remove(n(0)),
            ],
        );
        assert_eq!(session.borrow().retained_bytes(), 0);
        assert!(owner.read_with(cx, |v, _| v.carousel_tracks.is_empty()
            && v.carousel_track_activation.is_none()));
    }
}

fn requests(transport: &Transport) -> Vec<Request> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(100)
        .into_iter()
        .filter_map(|event| match event {
            Event::CarouselTrackRequested(_, _, _, _, Request::Layout(_)) => None,
            Event::CarouselTrackRequested(_, _, _, _, request) => Some(request),
            _ => None,
        })
        .collect()
}
fn automatic(axis: Axis, selected: i64, revision: i64) -> Config {
    let mut c = config(axis, selected, revision);
    c.carousel.auto_advance_ms = Some(1000);
    c
}
#[test]
fn track_viewport_keys_preserve_descendant_input_and_queue_ordered_requests() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, w(), "Keys", 400., 300.)
            .unwrap();
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
        apply(&owner, cx, initial(axis));
        let focus = owner.read_with(cx, |v, _| v.carousel_tracks[&n(0)].borrow().focus_handle());
        cx.update(|window, cx| window.focus(&focus, cx));
        cx.run_until_parked();
        tick(cx, 0);
        requests(&transport);
        let (next, previous, other) = if axis == Axis::Horizontal {
            ("right", "left", "down")
        } else {
            ("down", "up", "right")
        };
        for key in [next, next, previous, "home", "end", other, "shift-right"] {
            cx.simulate_keystrokes(key);
        }
        assert_eq!(
            requests(&transport),
            vec![
                Request::Next,
                Request::Next,
                Request::Previous,
                Request::First,
                Request::Last
            ]
        );
        assert_eq!(
            session
                .borrow()
                .tree(w())
                .unwrap()
                .get(n(0))
                .unwrap()
                .carousel_track
                .as_ref()
                .unwrap()
                .carousel
                .selected,
            Some(0),
            "requests never mutate controlled selection"
        );
        let child = owner.read_with(cx, |v, _| v.buttons[&n(5)].focus.clone());
        cx.update(|window, cx| window.focus(&child, cx));
        cx.run_until_parked();
        for key in [next, previous, "home", "end"] {
            cx.simulate_keystrokes(key);
        }
        assert!(requests(&transport).is_empty(), "nested controls keep keys");
        let mut disabled = config(axis, 0, 1);
        disabled.carousel.disabled = true;
        apply(&owner, cx, vec![Op::SetCarouselTrack(n(0), disabled)]);
        cx.update(|window, cx| window.focus(&focus, cx));
        cx.simulate_keystrokes(next);
        assert!(requests(&transport).is_empty());
    }
}
#[test]
fn track_auto_clock_emits_once_rechecks_pause_and_retires_on_unmount() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Clock", 400., 300.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.update(|window, cx| {
        window.activate_window();
        cx.set_reduce_motion(false);
    });
    cx.run_until_parked();
    cx.simulate_mouse_move(gpui::point(px(300.), px(200.)), None, Default::default());
    let mut ops = initial(Axis::Horizontal);
    ops.push(Op::SetCarouselTrack(
        n(0),
        automatic(Axis::Horizontal, 0, 1),
    ));
    apply(&owner, cx, ops);
    cx.run_until_parked();
    requests(&transport);
    let snapshot = |cx: &mut VisualTestContext| {
        owner.read_with(cx, |v, _| {
            v.carousel_tracks[&n(0)].borrow().clock_snapshot()
        })
    };
    assert_eq!(snapshot(cx), (true, false));
    tick(cx, 500);
    assert!(requests(&transport).is_empty());
    tick(cx, 499);
    assert!(requests(&transport).is_empty());
    tick(cx, 1);
    let first = requests(&transport);
    assert_eq!(first.len(), 1);
    let Request::AutoNext(first) = &first[0] else {
        panic!("automatic proposal");
    };
    assert_eq!(
        (&*first.from, &*first.target, first.revision),
        ("a", "b", 1)
    );
    assert_eq!(snapshot(cx), (false, true));
    tick(cx, 10_000);
    assert!(requests(&transport).is_empty());
    apply(
        &owner,
        cx,
        vec![Op::SetCarouselTrack(
            n(0),
            automatic(Axis::Horizontal, 1, 2),
        )],
    );
    cx.run_until_parked();
    assert_eq!(snapshot(cx), (true, false));
    tick(cx, 500);
    cx.simulate_mouse_move(gpui::point(px(20.), px(20.)), None, Default::default());
    assert_eq!(snapshot(cx), (false, false));
    tick(cx, 1000);
    assert!(requests(&transport).is_empty());
    cx.simulate_mouse_move(gpui::point(px(300.), px(200.)), None, Default::default());
    tick(cx, 500);
    assert!(requests(&transport).is_empty());
    let child = owner.read_with(cx, |v, _| v.buttons[&n(6)].focus.clone());
    cx.update(|window, cx| window.focus(&child, cx));
    cx.run_until_parked();
    assert_eq!(snapshot(cx), (false, false));
    tick(cx, 1000);
    assert!(requests(&transport).is_empty());
    cx.update(|window, cx| window.blur(cx));
    cx.run_until_parked();
    tick(cx, 999);
    assert!(requests(&transport).is_empty());
    tick(cx, 1);
    let events = requests(&transport);
    assert!(
        matches!(&events[..],[Request::AutoNext(p)] if p.from=="b"&&p.target=="c"&&p.revision==2)
    );
    apply(
        &owner,
        cx,
        vec![Op::SetCarouselTrack(
            n(0),
            automatic(Axis::Horizontal, 0, 3),
        )],
    );
    cx.run_until_parked();
    assert_eq!(snapshot(cx), (true, false));
    let weak = owner.read_with(cx, |v, _| Rc::downgrade(&v.carousel_tracks[&n(0)]));
    apply(
        &owner,
        cx,
        vec![
            Op::SetRoot(None),
            Op::Remove(n(5)),
            Op::Remove(n(6)),
            Op::Remove(n(7)),
            Op::Remove(n(2)),
            Op::Remove(n(3)),
            Op::Remove(n(4)),
            Op::Remove(n(1)),
            Op::Remove(n(0)),
        ],
    );
    cx.run_until_parked();
    tick(cx, 10_000);
    assert!(requests(&transport).is_empty());
    assert!(weak.upgrade().is_none());
    assert_eq!(session.borrow().retained_bytes(), 0);
    assert!(owner.read_with(cx, |v, _| v.carousel_track_activation.is_none()));
}

#[test]
fn track_deadline_waits_for_settlement_and_revalidates_geometry_policy_and_source() {
    use gpuio_protocol::{animation::Easing, carousel_track::Motion};
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Lifecycle", 400., 300.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.update(|window, cx| {
        window.activate_window();
        cx.set_reduce_motion(false);
    });
    cx.run_until_parked();
    cx.simulate_mouse_move(gpui::point(px(300.), px(200.)), None, Default::default());
    let mut ops = initial(Axis::Horizontal);
    ops.push(Op::SetCarouselTrackMotion(
        n(0),
        Some(Motion {
            duration_ms: 200,
            easing: Easing::Linear,
        }),
    ));
    apply(&owner, cx, ops);
    cx.run_until_parked();
    requests(&transport);
    let snapshot = |cx: &mut VisualTestContext| {
        owner.read_with(cx, |v, _| {
            v.carousel_tracks[&n(0)].borrow().clock_snapshot()
        })
    };
    apply(
        &owner,
        cx,
        vec![Op::SetCarouselTrack(
            n(0),
            automatic(Axis::Horizontal, 1, 1),
        )],
    );
    cx.run_until_parked();
    assert_eq!(snapshot(cx), (false, false));
    tick(cx, 100);
    assert_eq!(snapshot(cx), (false, false));
    tick(cx, 100);
    assert_eq!(snapshot(cx), (true, false));
    tick(cx, 999);
    assert!(requests(&transport).is_empty());
    tick(cx, 1);
    let first = requests(&transport);
    let [Request::AutoNext(first)] = &first[..] else {
        panic!("{first:?}");
    };
    let old_epoch = first.geometry_epoch;
    // Same stop map with a new width retires the already emitted proposal.
    apply(
        &owner,
        cx,
        vec![Op::SetStyle(
            n(0),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(90.)),
                Field::Height(Length::Px(80.)),
            ])],
        )],
    );
    cx.run_until_parked();
    let observed = transport.mailbox.lock().unwrap().drain(100);
    assert!(observed.iter().any(|e|matches!(e,Event::CarouselTrackRequested(_,_,_,_,Request::Layout(l)) if l.epoch>old_epoch && l.stops.as_ref().is_some_and(|s|s.canonical==vec![0,1,2]))));
    assert_eq!(snapshot(cx), (true, false));
    tick(cx, 1000);
    let second = requests(&transport);
    assert!(
        matches!(&second[..],[Request::AutoNext(p)] if p.geometry_epoch>old_epoch&&p.revision==1)
    );
    // Restart selection policy, then pause before its deadline.
    apply(
        &owner,
        cx,
        vec![Op::SetCarouselTrack(
            n(0),
            automatic(Axis::Horizontal, 1, 2),
        )],
    );
    cx.run_until_parked();
    tick(cx, 500);
    cx.update(|_, cx| cx.set_reduce_motion(true));
    tick(cx, 0);
    assert_eq!(snapshot(cx), (false, false));
    tick(cx, 2000);
    assert!(requests(&transport).is_empty());
    cx.update(|_, cx| cx.set_reduce_motion(false));
    tick(cx, 0);
    assert_eq!(snapshot(cx), (true, false));
    tick(cx, 500);
    cx.deactivate_window();
    assert_eq!(snapshot(cx), (false, false));
    tick(cx, 2000);
    assert!(requests(&transport).is_empty());
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    cx.simulate_mouse_move(gpui::point(px(300.), px(200.)), None, Default::default());
    tick(cx, 0);
    assert_eq!(snapshot(cx), (true, false));
    apply(
        &owner,
        cx,
        vec![Op::SetStyle(
            n(0),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    cx.run_until_parked();
    assert_eq!(snapshot(cx), (false, false));
    tick(cx, 2000);
    assert!(requests(&transport).is_empty());
    let visible = initial(Axis::Horizontal)
        .into_iter()
        .find_map(|op| match op {
            Op::SetStyle(id, style) if id == n(0) => Some(style),
            _ => None,
        })
        .unwrap();
    apply(&owner, cx, vec![Op::SetStyle(n(0), visible)]);
    cx.run_until_parked();
    requests(&transport);
    assert_eq!(snapshot(cx), (true, false));
    tick(cx, 500);
    let old = owner.read_with(cx, |v, _| Rc::downgrade(&v.carousel_tracks[&n(0)]));
    apply(&owner, cx, vec![Op::Bind(n(0), Some(h(9)))]);
    cx.run_until_parked();
    requests(&transport);
    assert!(old.upgrade().is_none());
    tick(cx, 500);
    assert!(
        requests(&transport).is_empty(),
        "old source deadline was cancelled"
    );
    tick(cx, 500);
    let events = transport.mailbox.lock().unwrap().drain(100);
    assert!(
        matches!(events.iter().find(|e|matches!(e,Event::CarouselTrackRequested(_,_,_,_,Request::AutoNext(_)))),Some(Event::CarouselTrackRequested(_,_,handler,_,Request::AutoNext(p))) if *handler==h(9)&&p.revision==2)
    );
    assert_eq!(snapshot(cx), (false, true));
    // Re-arm then close the entire window with work still outstanding.
    apply(
        &owner,
        cx,
        vec![Op::SetCarouselTrack(
            n(0),
            automatic(Axis::Horizontal, 1, 3),
        )],
    );
    cx.run_until_parked();
    assert_eq!(snapshot(cx), (true, false));
    let weak_owner = owner.downgrade();
    session.borrow_mut().close(w()).unwrap();
    cx.update(|window, _| window.remove_window());
    drop(owner);
    cx.cx.update(|_| ());
    cx.run_until_parked();
    assert!(weak_owner.upgrade().is_none());
    requests(&transport);
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(5));
    cx.run_until_parked();
    assert!(requests(&transport).is_empty());
    assert_eq!(session.borrow().retained_bytes(), 0);
}

#[test]
fn measured_track_background_drag_updates_pixels_and_requests_nearest_card() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, w(), "Drag", 400., 300.)
            .unwrap();
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
        cx.update(|window, cx| {
            window.activate_window();
            cx.set_reduce_motion(false);
        });
        cx.run_until_parked();
        cx.simulate_a11y_active(true);
        apply(&owner, cx, initial(axis));
        cx.run_until_parked();
        requests(&transport);
        let point = |main: f32| {
            if axis == Axis::Horizontal {
                gpui::point(px(main), px(60.))
            } else {
                gpui::point(px(60.), px(main))
            }
        };
        cx.simulate_mouse_move(point(80.), None, Default::default());
        cx.simulate_mouse_down(point(80.), gpui::MouseButton::Left, Default::default());
        cx.simulate_mouse_move(
            point(76.),
            Some(gpui::MouseButton::Left),
            Default::default(),
        );
        assert!(cx.update(|window, _| window.captured_hitbox().is_none()));
        cx.simulate_mouse_move(
            point(50.),
            Some(gpui::MouseButton::Left),
            Default::default(),
        );
        assert!(cx.update(|window, _| window.captured_hitbox().is_some()));
        tick(cx, 0);
        assert!(
            (position(cx, axis) - 18.).abs() < 0.01,
            "30-pixel movement should move b from48 to18"
        );
        assert!(
            !transport
                .mailbox
                .lock()
                .unwrap()
                .drain(100)
                .iter()
                .any(|event| matches!(event, Event::CarouselTrackRequested(..))),
            "no per-frame selection or geometry event"
        );
        cx.simulate_mouse_move(
            point(-50.),
            Some(gpui::MouseButton::Left),
            Default::default(),
        );
        tick(cx, 0);
        assert!((position(cx, axis) + 82.).abs() < 0.01);
        cx.simulate_mouse_up(point(-50.), gpui::MouseButton::Left, Default::default());
        assert!(cx.update(|window, _| window.captured_hitbox().is_none()));
        assert_eq!(requests(&transport), vec![Request::Select("c".into())]);
        assert_eq!(
            session
                .borrow()
                .tree(w())
                .unwrap()
                .get(n(0))
                .unwrap()
                .carousel_track
                .as_ref()
                .unwrap()
                .carousel
                .selected,
            Some(0)
        );
        apply(
            &owner,
            cx,
            vec![Op::SetCarouselTrack(n(0), config(axis, 2, 1))],
        );
        assert_eq!(position(cx, axis), -92.);
    }
}

#[test]
fn track_drag_respects_children_and_cancels_escape_capture_geometry_and_policy_changes() {
    for cancel in [
        "escape",
        "capture",
        "geometry",
        "disabled",
        "hidden",
        "handler",
        "inactive",
        "unmount",
        "window-close",
    ] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, w(), "Cancel", 400., 300.)
            .unwrap();
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
        cx.update(|window, cx| {
            window.activate_window();
            cx.set_reduce_motion(false);
        });
        cx.run_until_parked();
        apply(&owner, cx, initial(Axis::Horizontal));
        cx.run_until_parked();
        requests(&transport);
        // An ordinary nested button owns its press even when moved past drag slop.
        let button = gpui::point(px(5.), px(5.));
        cx.simulate_mouse_move(button, None, Default::default());
        cx.simulate_mouse_down(button, gpui::MouseButton::Left, Default::default());
        cx.simulate_mouse_move(
            gpui::point(px(20.), px(5.)),
            Some(gpui::MouseButton::Left),
            Default::default(),
        );
        assert!(cx.update(|window, _| window.captured_hitbox().is_none()));
        cx.simulate_mouse_up(
            gpui::point(px(20.), px(5.)),
            gpui::MouseButton::Left,
            Default::default(),
        );
        assert!(requests(&transport).is_empty());
        let start = gpui::point(px(80.), px(60.));
        let moved = gpui::point(px(45.), px(60.));
        cx.simulate_mouse_move(start, None, Default::default());
        cx.simulate_mouse_down(start, gpui::MouseButton::Left, Default::default());
        cx.simulate_mouse_move(moved, Some(gpui::MouseButton::Left), Default::default());
        tick(cx, 0);
        let owned = cx
            .update(|window, _| window.captured_hitbox())
            .expect("claimed");
        match cancel {
            "escape" => cx.simulate_keystrokes("escape"),
            "capture" => {
                let other = gpui::HitboxId::placeholder();
                assert_ne!(other, owned);
                cx.update(|window, _| window.capture_pointer(other));
                cx.simulate_keystrokes("escape");
                assert_eq!(cx.update(|window, _| window.captured_hitbox()), Some(other));
                cx.update(|window, _| window.release_pointer());
            }
            "geometry" => apply(
                &owner,
                cx,
                vec![Op::SetStyle(
                    n(3),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(150.)),
                        Field::Height(Length::Px(80.)),
                    ])],
                )],
            ),
            "disabled" => {
                let mut c = config(Axis::Horizontal, 0, 1);
                c.carousel.disabled = true;
                apply(&owner, cx, vec![Op::SetCarouselTrack(n(0), c)]);
            }
            "hidden" => apply(
                &owner,
                cx,
                vec![Op::SetStyle(
                    n(0),
                    vec![Style::Fields(vec![Field::Display(3)])],
                )],
            ),
            "handler" => apply(&owner, cx, vec![Op::Bind(n(0), Some(h(10)))]),
            "inactive" => cx.deactivate_window(),
            "window-close" => {
                let weak = owner.downgrade();
                session.borrow_mut().close(w()).unwrap();
                cx.update(|window, _| window.remove_window());
                drop(owner);
                cx.cx.update(|_| ());
                cx.run_until_parked();
                assert!(weak.upgrade().is_none());
                assert!(requests(&transport).is_empty());
                assert_eq!(session.borrow().retained_bytes(), 0);
                continue;
            }
            "unmount" => apply(
                &owner,
                cx,
                vec![
                    Op::SetRoot(None),
                    Op::Remove(n(5)),
                    Op::Remove(n(6)),
                    Op::Remove(n(7)),
                    Op::Remove(n(2)),
                    Op::Remove(n(3)),
                    Op::Remove(n(4)),
                    Op::Remove(n(1)),
                    Op::Remove(n(0)),
                ],
            ),
            _ => unreachable!(),
        }
        cx.run_until_parked();
        assert!(
            cx.update(|window, _| window.captured_hitbox().is_none()),
            "{cancel}"
        );
        cx.simulate_mouse_up(moved, gpui::MouseButton::Left, Default::default());
        assert!(
            requests(&transport).is_empty(),
            "cancelled {cancel} must not select"
        );
    }
}

#[test]
fn track_drag_does_not_take_pointer_or_keys_from_a_native_editor() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Editor", 400., 300.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.update(|window, cx| {
        window.activate_window();
        cx.set_reduce_motion(false);
    });
    cx.run_until_parked();
    apply(&owner, cx, initial(Axis::Horizontal));
    apply(
        &owner,
        cx,
        vec![
            Op::Create(
                n(8),
                Kind::Input,
                "ordinary native input".into(),
                Some(h(8)),
            ),
            Op::SetEditor(
                n(8),
                EditorConfig {
                    label: "Child editor".into(),
                    placeholder: "".into(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: false,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::SetStyle(
                n(8),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(110.)),
                    Field::Height(Length::Px(30.)),
                ])],
            ),
            Op::Splice(n(3), 0, 1, vec![n(8)]),
            Op::Remove(n(6)),
        ],
    );
    cx.run_until_parked();
    requests(&transport);
    let start = gpui::point(px(90.), px(15.));
    let end = gpui::point(px(60.), px(15.));
    cx.simulate_mouse_move(start, None, Default::default());
    cx.simulate_mouse_down(start, gpui::MouseButton::Left, Default::default());
    cx.simulate_mouse_move(end, Some(gpui::MouseButton::Left), Default::default());
    cx.simulate_mouse_up(end, gpui::MouseButton::Left, Default::default());
    assert!(cx.update(|window, _| window.captured_hitbox().is_none()));
    assert!(requests(&transport).is_empty());
    cx.update(|window, cx| {
        let s = owner.read(cx).editors[&n(8)].snapshot(window, cx);
        assert!(s.focused);
        assert_ne!(s.selection.anchor, s.selection.head);
    });
    cx.simulate_keystrokes("right");
    cx.simulate_keystrokes("x");
    assert!(requests(&transport).is_empty());
    cx.update(|window, cx| {
        assert!(
            owner.read(cx).editors[&n(8)]
                .snapshot(window, cx)
                .text
                .contains('x')
        )
    });
}

fn track_wheel_event(
    cx: &mut VisualTestContext,
    axis: Axis,
    delta: f32,
    precise: bool,
    phase: gpui::TouchPhase,
) {
    let position = gpui::point(px(60.), px(60.));
    cx.simulate_mouse_move(position, None, Default::default());
    let (x, y) = if axis == Axis::Horizontal {
        (delta, 0.)
    } else {
        (0., delta)
    };
    cx.update(|window, cx| {
        window.dispatch_event(
            gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                position,
                delta: if precise {
                    gpui::ScrollDelta::Pixels(gpui::point(px(x), px(y)))
                } else {
                    gpui::ScrollDelta::Lines(gpui::point(x, y))
                },
                touch_phase: phase,
                modifiers: Default::default(),
            }),
            cx,
        )
    });
    cx.run_until_parked();
}
#[test]
fn track_wheel_pixels_paint_on_both_axes_and_finish_without_per_frame_events() {
    use gpui::TouchPhase::*;
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, w(), "Wheel", 400., 300.)
            .unwrap();
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
        cx.simulate_a11y_active(true);
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();
        apply(&owner, cx, initial(axis));
        cx.run_until_parked();
        requests(&transport);
        let other = if axis == Axis::Horizontal {
            Axis::Vertical
        } else {
            Axis::Horizontal
        };
        track_wheel_event(cx, other, -30., true, Started);
        track_wheel_event(cx, other, 0., true, Ended);
        assert!(requests(&transport).is_empty());
        if axis == Axis::Vertical {
            cx.update(|_, cx| cx.set_reduce_motion(true));
        }
        track_wheel_event(cx, axis, -30., true, Started);
        tick(cx, 0);
        assert!(
            (position(cx, axis) - 18.).abs() < 0.1,
            "preview={}",
            position(cx, axis)
        );
        track_wheel_event(cx, axis, -100., true, Moved);
        tick(cx, 0);
        assert!((position(cx, axis) + 82.).abs() < 0.1);
        assert!(
            transport.mailbox.lock().unwrap().drain(100).is_empty(),
            "offsets stay native"
        );
        track_wheel_event(cx, axis, 0., true, Ended);
        assert_eq!(requests(&transport), vec![Request::Select("c".into())]);
        owner.read_with(cx, |v, _| {
            assert_eq!(
                v.carousel_tracks[&n(0)].borrow().wheel_snapshot(),
                (false, false, false)
            )
        });
        tick(cx, 0);
        assert!(
            (position(cx, axis) - 48.).abs() < 0.1,
            "controlled model still selects a"
        );
        track_wheel_event(cx, axis, -80., true, Started);
        tick(cx, 0);
        track_wheel_event(cx, axis, 0., true, Cancelled);
        tick(cx, 0);
        assert!(requests(&transport).is_empty());
        assert!((position(cx, axis) - 48.).abs() < 0.1);
    }
}
#[test]
fn track_wheel_quiet_timer_rearms_after_restart_and_line_echo_steps_once() {
    use gpui::TouchPhase::*;
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Quiet", 400., 300.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    apply(&owner, cx, initial(Axis::Horizontal));
    cx.run_until_parked();
    requests(&transport);
    track_wheel_event(cx, Axis::Horizontal, -30., true, Started);
    tick(cx, 20);
    track_wheel_event(cx, Axis::Horizontal, -20., true, Moved);
    tick(cx, 8);
    assert!(
        requests(&transport).is_empty(),
        "first wake must honor latest delta"
    );
    tick(cx, 20);
    assert_eq!(requests(&transport), vec![Request::Select("b".into())]);
    tick(cx, 0);
    track_wheel_event(cx, Axis::Horizontal, -30., true, Started);
    tick(cx, 0);
    track_wheel_event(cx, Axis::Horizontal, -25., true, Started);
    tick(cx, 0);
    assert_eq!(requests(&transport), vec![Request::Select("b".into())]);
    assert!(
        (position(cx, Axis::Horizontal) + 7.).abs() < 0.1,
        "new preview continues from visible pixels"
    );
    tick(cx, 28);
    assert_eq!(requests(&transport), vec![Request::Select("b".into())]);
    tick(cx, 0);
    track_wheel_event(cx, Axis::Horizontal, -1., false, Moved);
    assert_eq!(requests(&transport), vec![Request::Next]);
    apply(
        &owner,
        cx,
        vec![Op::SetCarouselTrack(n(0), config(Axis::Horizontal, 1, 1))],
    );
    cx.run_until_parked();
    track_wheel_event(cx, Axis::Horizontal, -1., false, Moved);
    assert!(
        requests(&transport).is_empty(),
        "accepted revision preserves burst fence"
    );
    tick(cx, 28);
    track_wheel_event(cx, Axis::Horizontal, -1., false, Moved);
    assert_eq!(requests(&transport), vec![Request::Next]);
    tick(cx, 28);
    owner.read_with(cx, |v, _| {
        assert_eq!(
            v.carousel_tracks[&n(0)].borrow().wheel_snapshot(),
            (false, false, false)
        )
    });
}

#[test]
fn track_wheel_preserves_nested_scroll_and_axis_specific_ancestor_handoff() {
    use gpui::TouchPhase::*;
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, w(), "Nested", 400., 300.)
            .unwrap();
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
        cx.simulate_a11y_active(true);
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();
        apply(&owner, cx, initial(axis));
        cx.run_until_parked();
        let overflow = if axis == Axis::Horizontal {
            Field::OverflowX(3)
        } else {
            Field::OverflowY(3)
        };
        apply(
            &owner,
            cx,
            vec![
                Op::SetCarouselTrack(n(0), config(axis, 1, 1)),
                Op::Create(n(8), Kind::Container, "".into(), None),
                Op::Create(n(9), Kind::Container, "".into(), None),
                Op::SetStyle(
                    n(8),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(80.)),
                        Field::Height(Length::Px(80.)),
                        overflow.clone(),
                    ])],
                ),
                Op::SetStyle(
                    n(9),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(400.)),
                        Field::Height(Length::Px(400.)),
                        Field::Shrink(0.),
                    ])],
                ),
                Op::Splice(n(8), 0, 0, vec![n(9)]),
                Op::Splice(n(3), 0, 1, vec![n(8)]),
                Op::Remove(n(6)),
            ],
        );
        cx.run_until_parked();
        requests(&transport);
        track_wheel_event(cx, axis, -30., true, Started);
        tick(cx, 0);
        owner.read_with(cx, |v, _| {
            let offset = v.scrolls[&n(8)].handle.offset();
            assert_eq!(
                if axis == Axis::Horizontal {
                    offset.x
                } else {
                    offset.y
                },
                px(-30.)
            );
            assert_eq!(
                v.carousel_tracks[&n(0)].borrow().wheel_snapshot(),
                (false, false, false)
            );
        });
        assert!(requests(&transport).is_empty());
        track_wheel_event(cx, axis, 0., true, Ended);
        apply(
            &owner,
            cx,
            vec![
                Op::Splice(n(3), 0, 1, vec![]),
                Op::Remove(n(9)),
                Op::Remove(n(8)),
                Op::SetCarouselTrack(n(0), config(axis, 2, 2)),
                Op::Create(n(10), Kind::Container, "".into(), None),
                Op::Create(n(11), Kind::Container, "".into(), None),
                Op::SetStyle(
                    n(10),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(100.)),
                        Field::Height(Length::Px(100.)),
                        Field::Direction(if axis == Axis::Horizontal { 0 } else { 1 }),
                        overflow,
                    ])],
                ),
                Op::SetStyle(
                    n(11),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(400.)),
                        Field::Height(Length::Px(400.)),
                        Field::Shrink(0.),
                    ])],
                ),
                Op::SetStyle(
                    n(0),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(100.)),
                        Field::Height(Length::Px(100.)),
                        Field::Shrink(0.),
                    ])],
                ),
                Op::Splice(n(10), 0, 0, vec![n(0), n(11)]),
                Op::SetRoot(Some(n(10))),
            ],
        );
        cx.run_until_parked();
        requests(&transport);
        track_wheel_event(cx, axis, -30., true, Started);
        tick(cx, 0);
        owner.read_with(cx, |v, _| {
            let offset = v.scrolls[&n(10)].handle.offset();
            assert_eq!(
                if axis == Axis::Horizontal {
                    offset.x
                } else {
                    offset.y
                },
                if axis == Axis::Horizontal {
                    px(0.)
                } else {
                    px(-30.)
                }
            );
        });
        assert!(requests(&transport).is_empty());
    }
}

#[test]
fn track_wheel_cancels_changed_owners_policy_geometry_and_window_close() {
    use gpui::TouchPhase::*;
    for cancel in [
        "escape",
        "capture",
        "geometry",
        "disabled",
        "hidden",
        "handler",
        "inactive",
        "unmount",
        "window-close",
    ] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, w(), "Cancel wheel", 400., 300.)
            .unwrap();
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();
        apply(&owner, cx, initial(Axis::Horizontal));
        cx.run_until_parked();
        requests(&transport);
        track_wheel_event(cx, Axis::Horizontal, -70., true, Started);
        tick(cx, 0);
        owner.read_with(cx, |v, _| {
            assert_eq!(
                v.carousel_tracks[&n(0)].borrow().wheel_snapshot(),
                (true, true, true),
                "{cancel}"
            )
        });
        match cancel {
            "escape" => cx.simulate_keystrokes("escape"),
            "capture" => {
                cx.update(|window, _| window.capture_pointer(gpui::HitboxId::placeholder()))
            }
            "geometry" => apply(
                &owner,
                cx,
                vec![Op::SetStyle(
                    n(3),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(150.)),
                        Field::Height(Length::Px(80.)),
                    ])],
                )],
            ),
            "disabled" => {
                let mut c = config(Axis::Horizontal, 0, 1);
                c.carousel.disabled = true;
                apply(&owner, cx, vec![Op::SetCarouselTrack(n(0), c)]);
            }
            "hidden" => apply(
                &owner,
                cx,
                vec![Op::SetStyle(
                    n(0),
                    vec![Style::Fields(vec![Field::Display(3)])],
                )],
            ),
            "handler" => apply(&owner, cx, vec![Op::Bind(n(0), Some(h(10)))]),
            "inactive" => cx.deactivate_window(),
            "unmount" => apply(
                &owner,
                cx,
                vec![
                    Op::SetRoot(None),
                    Op::Remove(n(5)),
                    Op::Remove(n(6)),
                    Op::Remove(n(7)),
                    Op::Remove(n(2)),
                    Op::Remove(n(3)),
                    Op::Remove(n(4)),
                    Op::Remove(n(1)),
                    Op::Remove(n(0)),
                ],
            ),
            "window-close" => {
                let weak = owner.downgrade();
                session.borrow_mut().close(w()).unwrap();
                cx.update(|window, _| window.remove_window());
                drop(owner);
                cx.cx.update(|_| ());
                cx.run_until_parked();
                cx.executor()
                    .advance_clock(std::time::Duration::from_millis(100));
                cx.run_until_parked();
                assert!(weak.upgrade().is_none());
                assert!(requests(&transport).is_empty());
                assert_eq!(session.borrow().retained_bytes(), 0);
                continue;
            }
            _ => unreachable!(),
        }
        tick(cx, 0);
        tick(cx, 100);
        assert!(
            requests(&transport).is_empty(),
            "cancelled {cancel} must not commit at old deadline"
        );
        owner.read_with(cx, |v, _| {
            if let Some(state) = v.carousel_tracks.get(&n(0)) {
                let snapshot = state.borrow().wheel_snapshot();
                assert!(!snapshot.0 && !snapshot.2, "{cancel}: {snapshot:?}");
            }
        });
        if cancel == "capture" {
            assert!(cx.update(|window, _| window.captured_hitbox().is_some()));
            cx.update(|window, _| window.release_pointer());
        }
    }
}

fn track_group() -> Vec<Op> {
    vec![
        Op::Create(n(8), Kind::CarouselTrackGroup, String::new(), None),
        Op::Create(n(9), Kind::Container, String::new(), None),
        Op::Create(n(10), Kind::Button, "Previous card".into(), Some(h(10))),
        Op::Create(n(11), Kind::Button, "Next card".into(), Some(h(11))),
        Op::SetControl(n(10), Control::Button(false)),
        Op::SetControl(n(11), Control::Button(false)),
        Op::SetStyle(
            n(8),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(180.)),
                Field::Height(Length::Px(140.)),
            ])],
        ),
        Op::SetStyle(
            n(9),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(180.)),
                Field::Height(Length::Px(40.)),
                Field::Direction(0),
            ])],
        ),
        Op::SetStyle(
            n(10),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(90.)),
                Field::Height(Length::Px(40.)),
            ])],
        ),
        Op::SetStyle(
            n(11),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(90.)),
                Field::Height(Length::Px(40.)),
            ])],
        ),
        Op::Splice(n(9), 0, 0, vec![n(10), n(11)]),
        Op::Splice(n(8), 0, 0, vec![n(0), n(9)]),
        Op::SetRoot(Some(n(8))),
    ]
}
#[test]
fn track_controls_pointer_keyboard_and_semantic_activation_preserve_focus_contract() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, w(), "Controls", 400., 300.)
            .unwrap();
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
        cx.simulate_a11y_active(true);
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();
        apply(&owner, cx, initial(axis));
        apply(&owner, cx, track_group());
        cx.run_until_parked();
        requests(&transport);
        let viewport = owner.read_with(cx, |v, _| v.carousel_tracks[&n(0)].borrow().focus_handle());
        let next = owner.read_with(cx, |v, _| v.buttons[&n(11)].focus.clone());
        let ax = cx.a11y_tree().unwrap();
        for (index, label) in ["Card 0", "Card 1", "Card 2"].into_iter().enumerate() {
            let node = &ax
                .nodes
                .iter()
                .find(|(_, node)| node.label() == Some(label))
                .unwrap()
                .1;
            assert_eq!(node.role(), gpui::accesskit::Role::Group);
            assert_eq!(node.position_in_set(), Some(index + 1));
            assert_eq!(node.size_of_set(), Some(3));
        }
        let (target, node) = ax
            .nodes
            .iter()
            .find(|(_, node)| node.label() == Some("Next card"))
            .unwrap();
        let target = *target;
        let bounds = node.bounds().unwrap();
        let scale = cx.update(|window, _| f64::from(window.scale_factor()));
        let point = gpui::point(
            px(((bounds.x0 + bounds.x1) / 2. / scale) as f32),
            px(((bounds.y0 + bounds.y1) / 2. / scale) as f32),
        );
        cx.simulate_mouse_move(point, None, Default::default());
        cx.simulate_click(point, Default::default());
        assert!(
            cx.update(|window, _| viewport.is_focused(window)),
            "pointer control focuses viewport"
        );
        let events = transport.mailbox.lock().unwrap().drain(100);
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e,Event::Press(_,id,_,_) if *id==n(11)))
                .count(),
            1
        );
        assert!(requests(&transport).is_empty());
        cx.update(|window, cx| window.focus(&next, cx));
        tick(cx, 0);
        cx.simulate_keystrokes("space");
        cx.update(|window, cx| {
            window.dispatch_event(
                gpui::PlatformInput::KeyUp(gpui::KeyUpEvent {
                    keystroke: gpui::Keystroke::parse("space").unwrap(),
                }),
                cx,
            )
        });
        assert!(
            cx.update(|window, _| next.is_focused(window)),
            "keyboard activation retains control focus"
        );
        let events = transport.mailbox.lock().unwrap().drain(100);
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e,Event::Press(_,id,_,_) if *id==n(11)))
                .count(),
            1
        );
        cx.simulate_keystrokes(if axis == Axis::Horizontal {
            "right"
        } else {
            "down"
        });
        assert_eq!(requests(&transport), vec![Request::Next]);
        assert!(cx.update(|window, _| next.is_focused(window)));
        cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
            action: gpui::accesskit::Action::Click,
            target_node: target,
            target_tree: gpui::accesskit::TreeId::ROOT,
            data: None,
        });
        cx.run_until_parked();
        assert!(
            cx.update(|window, _| next.is_focused(window)),
            "semantic activation is not a pointer click"
        );
        let events = transport.mailbox.lock().unwrap().drain(100);
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e,Event::Press(_,id,_,_) if *id==n(11)))
                .count(),
            1
        );
        // Replacing the relationship with an ordinary wrapper removes navigation behavior.
        apply(
            &owner,
            cx,
            vec![
                Op::Create(n(12), Kind::Container, String::new(), None),
                Op::Splice(n(8), 0, 2, vec![]),
                Op::Splice(n(12), 0, 0, vec![n(0), n(9)]),
                Op::SetRoot(Some(n(12))),
                Op::Remove(n(8)),
            ],
        );
        cx.run_until_parked();
        cx.simulate_keystrokes(if axis == Axis::Horizontal {
            "right"
        } else {
            "down"
        });
        assert!(requests(&transport).is_empty());
    }
}

#[test]
fn track_external_controls_pause_auto_for_hover_and_focus_then_resume_a_full_interval() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Scope", 400., 300.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.update(|window, cx| {
        window.activate_window();
        cx.set_reduce_motion(false);
    });
    cx.run_until_parked();
    let outside = gpui::point(px(300.), px(200.));
    cx.simulate_mouse_move(outside, None, Default::default());
    apply(&owner, cx, initial(Axis::Horizontal));
    apply(&owner, cx, track_group());
    apply(
        &owner,
        cx,
        vec![Op::SetCarouselTrack(
            n(0),
            automatic(Axis::Horizontal, 0, 1),
        )],
    );
    cx.run_until_parked();
    requests(&transport);
    let clock = |cx: &mut VisualTestContext| {
        owner.read_with(cx, |v, _| {
            v.carousel_tracks[&n(0)].borrow().clock_snapshot()
        })
    };
    assert_eq!(clock(cx), (true, false));
    tick(cx, 500);
    cx.simulate_mouse_move(gpui::point(px(130.), px(100.)), None, Default::default());
    tick(cx, 0);
    assert_eq!(
        clock(cx),
        (false, false),
        "controls outside viewport still pause hover"
    );
    tick(cx, 1500);
    assert!(requests(&transport).is_empty());
    cx.simulate_mouse_move(outside, None, Default::default());
    tick(cx, 0);
    assert_eq!(clock(cx), (true, false));
    tick(cx, 500);
    let next = owner.read_with(cx, |v, _| v.buttons[&n(11)].focus.clone());
    cx.update(|window, cx| window.focus(&next, cx));
    tick(cx, 0);
    assert_eq!(clock(cx), (false, false));
    tick(cx, 1500);
    assert!(requests(&transport).is_empty());
    cx.update(|window, cx| window.blur(cx));
    tick(cx, 0);
    assert_eq!(clock(cx), (true, false));
    tick(cx, 999);
    assert!(requests(&transport).is_empty());
    tick(cx, 1);
    assert!(matches!(
        requests(&transport).as_slice(),
        [Request::AutoNext(_)]
    ));
}

fn ax_hidden(cx: &mut VisualTestContext, label: &str) -> (gpui::accesskit::NodeId, bool) {
    let tree = cx.a11y_tree().unwrap();
    let id = tree
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some(label))
        .unwrap()
        .0;
    let mut current = id;
    loop {
        let node = &tree.nodes.iter().find(|(id, _)| *id == current).unwrap().1;
        if node.is_hidden() {
            return (id, true);
        }
        let Some(parent) = tree
            .nodes
            .iter()
            .find(|(_, n)| n.children().contains(&current))
        else {
            return (id, false);
        };
        current = parent.0;
    }
}
fn ax_action(
    cx: &mut VisualTestContext,
    id: gpui::accesskit::NodeId,
    action: gpui::accesskit::Action,
) {
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action,
        target_node: id,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    cx.run_until_parked();
}
#[test]
fn track_clipped_cards_exclude_input_and_reveal_after_controlled_navigation_without_remount() {
    for axis in [Axis::Horizontal, Axis::Vertical] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, w(), "Visible cards", 400., 300.)
            .unwrap();
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
        cx.simulate_a11y_active(true);
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();
        apply(&owner, cx, initial(axis));
        apply(&owner, cx, track_group());
        cx.run_until_parked();
        requests(&transport);
        let viewport = owner.read_with(cx, |v, _| v.carousel_tracks[&n(0)].borrow().focus_handle());
        let first = owner.read_with(cx, |v, _| v.buttons[&n(5)].focus.clone());
        let middle = owner.read_with(cx, |v, _| v.buttons[&n(6)].focus.clone());
        let last = owner.read_with(cx, |v, _| v.buttons[&n(7)].focus.clone());
        assert!(
            !ax_hidden(cx, "Action 1").1,
            "partially visible card stays exposed"
        );
        let (last_ax, hidden) = ax_hidden(cx, "Action 2");
        assert!(hidden);
        cx.update(|window, cx| window.focus(&middle, cx));
        tick(cx, 0);
        cx.simulate_keystrokes("tab");
        assert!(
            cx.update(
                |window, cx| owner.read_with(cx, |v, _| v.buttons[&n(10)].focus.is_focused(window))
            ),
            "Tab skips fully clipped card"
        );
        ax_action(cx, last_ax, gpui::accesskit::Action::Focus);
        assert!(!cx.update(|window, _| last.is_focused(window)));
        ax_action(cx, last_ax, gpui::accesskit::Action::Click);
        assert!(transport.mailbox.lock().unwrap().drain(100).is_empty());
        cx.update(|window, cx| window.focus(&first, cx));
        tick(cx, 0);
        apply(
            &owner,
            cx,
            vec![Op::SetCarouselTrack(n(0), config(axis, 2, 1))],
        );
        cx.run_until_parked();
        tick(cx, 0);
        assert!(
            cx.update(|window, _| viewport.is_focused(window)),
            "a fully clipped former target returns to viewport"
        );
        assert!(ax_hidden(cx, "Action 0").1);
        assert_eq!(ax_hidden(cx, "Action 2"), (last_ax, false));
        assert!(
            requests(&transport).is_empty(),
            "focus never invents a model selection"
        );
        owner.read_with(cx, |v, _| assert_eq!(v.buttons[&n(7)].focus, last));
        ax_action(cx, last_ax, gpui::accesskit::Action::Focus);
        assert!(cx.update(|window, _| last.is_focused(window)));
        ax_action(cx, last_ax, gpui::accesskit::Action::Click);
        assert_eq!(
            transport
                .mailbox
                .lock()
                .unwrap()
                .drain(100)
                .iter()
                .filter(|e| matches!(e,Event::Press(_,id,_,_) if *id==n(7)))
                .count(),
            1
        );
        // Focus outside the viewport is never stolen by a new selection.
        let external = owner.read_with(cx, |v, _| v.buttons[&n(11)].focus.clone());
        cx.update(|window, cx| window.focus(&external, cx));
        tick(cx, 0);
        apply(
            &owner,
            cx,
            vec![Op::SetCarouselTrack(n(0), config(axis, 0, 2))],
        );
        cx.run_until_parked();
        assert!(cx.update(|window, _| external.is_focused(window)));
        // Disabled navigation is neither a Tab target nor a semantic/pointer focus target.
        let mut disabled = config(axis, 0, 3);
        disabled.carousel.disabled = true;
        apply(&owner, cx, vec![Op::SetCarouselTrack(n(0), disabled)]);
        cx.run_until_parked();
        assert!(!cx.update(|window, cx| {
            owner.read_with(cx, |v, _| v.focus.borrow().can_focus(&viewport, window))
        }));
        let (root_ax, _) = ax_hidden(cx, "Cards");
        ax_action(cx, root_ax, gpui::accesskit::Action::Focus);
        assert!(cx.update(|window, _| external.is_focused(window)));
        assert!(
            cx.a11y_tree()
                .unwrap()
                .nodes
                .iter()
                .find(|(id, _)| *id == root_ax)
                .unwrap()
                .1
                .is_disabled()
        );
        assert!(
            session
                .borrow()
                .press(
                    w(),
                    n(0),
                    h(0),
                    session.borrow().tree(w()).unwrap().revision()
                )
                .is_none(),
            "track has typed requests, no generic Press"
        );
    }
}

#[test]
fn track_visible_card_keeps_nested_scroll_reveal_and_hidden_editor_cannot_receive_typing() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Retained focus", 400., 300.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    apply(&owner, cx, initial(Axis::Horizontal));
    apply(
        &owner,
        cx,
        vec![
            Op::Create(n(8), Kind::Input, "draft".into(), Some(h(8))),
            Op::SetEditor(
                n(8),
                EditorConfig {
                    label: "Retained editor".into(),
                    placeholder: String::new(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: false,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::SetStyle(
                n(8),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(38.)),
                    Field::Height(Length::Px(30.)),
                ])],
            ),
            Op::Splice(n(2), 0, 1, vec![n(8)]),
            Op::Remove(n(5)),
            Op::Create(n(9), Kind::Container, String::new(), None),
            Op::Create(n(10), Kind::Container, String::new(), None),
            Op::Create(n(11), Kind::Container, String::new(), None),
            Op::Create(n(12), Kind::Button, "Deep control".into(), Some(h(12))),
            Op::SetControl(n(12), Control::Button(false)),
            Op::SetStyle(
                n(9),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(60.)),
                    Field::OverflowY(3),
                ])],
            ),
            Op::SetStyle(
                n(10),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(200.)),
                    Field::Shrink(0.),
                ])],
            ),
            Op::SetStyle(
                n(11),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(80.)),
                    Field::Height(Length::Px(130.)),
                    Field::Shrink(0.),
                ])],
            ),
            Op::SetStyle(
                n(12),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(70.)),
                    Field::Height(Length::Px(30.)),
                    Field::Shrink(0.),
                ])],
            ),
            Op::Splice(n(10), 0, 0, vec![n(11), n(12)]),
            Op::Splice(n(9), 0, 0, vec![n(10)]),
            Op::Splice(n(3), 0, 1, vec![n(9)]),
            Op::Remove(n(6)),
        ],
    );
    cx.run_until_parked();
    requests(&transport);
    let viewport = owner.read_with(cx, |v, _| v.carousel_tracks[&n(0)].borrow().focus_handle());
    let editor = owner.read_with(cx, |v, cx| v.editors[&n(8)].focus_handle(cx));
    cx.update(|window, cx| window.focus(&editor, cx));
    tick(cx, 0);
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| {
            v.editors[&n(8)].mark_test_text("mark", window, cx)
        })
    });
    tick(cx, 0);
    assert!(cx.update(|window, cx| {
        owner.read(cx).editors[&n(8)]
            .snapshot(window, cx)
            .composition
            .is_some()
    }));
    apply(
        &owner,
        cx,
        vec![Op::SetCarouselTrack(n(0), config(Axis::Horizontal, 1, 1))],
    );
    cx.run_until_parked();
    tick(cx, 0);
    assert!(cx.update(|window, _| viewport.is_focused(window)));
    let before = cx.update(|window, cx| owner.read(cx).editors[&n(8)].snapshot(window, cx));
    assert!(!before.focused);
    cx.simulate_keystrokes("x");
    let after = cx.update(|window, cx| owner.read(cx).editors[&n(8)].snapshot(window, cx));
    assert_eq!(after.text, before.text);
    let (deep, _) = ax_hidden(cx, "Deep control");
    ax_action(cx, deep, gpui::accesskit::Action::Focus);
    tick(cx, 0);
    assert!(cx.update(|window, cx| {
        owner.read_with(cx, |v, _| v.buttons[&n(12)].focus.is_focused(window))
    }));
    owner.read_with(cx, |v, _| {
        assert!(
            v.scrolls[&n(9)].handle.offset().y < px(0.),
            "ordinary nested scroller reveals its own target"
        )
    });
    assert!(
        requests(&transport).is_empty(),
        "nested reveal does not select another card"
    );
    apply(
        &owner,
        cx,
        vec![Op::SetCarouselTrack(n(0), config(Axis::Horizontal, 0, 2))],
    );
    cx.run_until_parked();
    tick(cx, 0);
    owner.read_with(cx, |v, cx| {
        assert_eq!(v.editors[&n(8)].focus_handle(cx), editor)
    });
    let (input_ax, hidden) = ax_hidden(cx, "Retained editor");
    assert!(!hidden);
    ax_action(cx, input_ax, gpui::accesskit::Action::Focus);
    assert!(cx.update(|window, _| editor.is_focused(window)));
    assert_eq!(
        cx.update(|window, cx| owner.read(cx).editors[&n(8)].snapshot(window, cx).text),
        before.text
    );
}

#[test]
fn track_clipping_follows_retained_ids_on_reorder_resize_and_unmount() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Reorder", 400., 300.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    apply(&owner, cx, initial(Axis::Horizontal));
    cx.run_until_parked();
    requests(&transport);
    let (last_ax, hidden) = ax_hidden(cx, "Action 2");
    assert!(hidden);
    let last = owner.read_with(cx, |v, _| v.buttons[&n(7)].focus.clone());
    let mut next = config(Axis::Horizontal, 0, 1);
    next.lineage = 1;
    next.carousel.ids.reverse();
    apply(
        &owner,
        cx,
        vec![
            Op::Splice(n(1), 0, 3, vec![n(4), n(3), n(2)]),
            Op::SetCarouselTrack(n(0), next),
        ],
    );
    cx.run_until_parked();
    assert_eq!(ax_hidden(cx, "Action 2"), (last_ax, false));
    assert!(ax_hidden(cx, "Action 0").1);
    ax_action(cx, last_ax, gpui::accesskit::Action::Focus);
    assert!(cx.update(|window, _| last.is_focused(window)));
    apply(
        &owner,
        cx,
        vec![Op::SetStyle(
            n(0),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(0.)),
                Field::Height(Length::Px(80.)),
            ])],
        )],
    );
    cx.run_until_parked();
    tick(cx, 0);
    assert!(
        !cx.update(|window, _| last.is_focused(window)),
        "zero viewport cannot retain child keyboard focus"
    );
    assert!(ax_hidden(cx, "Action 2").1);
    apply(
        &owner,
        cx,
        vec![Op::SetStyle(
            n(0),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(100.)),
                Field::Height(Length::Px(80.)),
            ])],
        )],
    );
    cx.run_until_parked();
    assert_eq!(ax_hidden(cx, "Action 2"), (last_ax, false));
    apply(
        &owner,
        cx,
        vec![
            Op::SetRoot(None),
            Op::Remove(n(5)),
            Op::Remove(n(6)),
            Op::Remove(n(7)),
            Op::Remove(n(2)),
            Op::Remove(n(3)),
            Op::Remove(n(4)),
            Op::Remove(n(1)),
            Op::Remove(n(0)),
        ],
    );
    cx.run_until_parked();
    assert_eq!(session.borrow().retained_bytes(), 0);
    owner.read_with(cx, |v, _| assert!(v.carousel_tracks.is_empty()));
    ax_action(cx, last_ax, gpui::accesskit::Action::Click);
    assert!(requests(&transport).is_empty());
}

#[test]
fn track_visual_clipping_does_not_retire_an_application_owned_modal_portal() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Portal", 400., 300.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    apply(&owner, cx, initial(Axis::Horizontal));
    apply(
        &owner,
        cx,
        vec![
            Op::Create(n(8), Kind::FocusScope, String::new(), Some(h(8))),
            Op::SetFocusScope(
                n(8),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::SetOverlay(
                n(8),
                Some(OverlayConfig {
                    kind: OverlayKind::Dialog,
                    label: "Card dialog".into(),
                    width: 220.,
                    dismiss_on_escape: true,
                    dismiss_on_outside_pointer: false,
                }),
            ),
            Op::SetStyle(n(8), vec![Style::Height(Length::Px(160.))]),
            Op::Create(n(9), Kind::Button, "Dialog action".into(), Some(h(9))),
            Op::SetControl(n(9), Control::Button(false)),
            Op::Splice(n(8), 0, 0, vec![n(9)]),
            Op::Splice(n(2), 1, 0, vec![n(8)]),
        ],
    );
    cx.run_until_parked();
    tick(cx, 0);
    tick(cx, 0);
    requests(&transport);
    let dialog = owner.read_with(cx, |v, _| v.buttons[&n(9)].focus.clone());
    assert!(cx.update(|window, _| dialog.is_focused(window)));
    apply(
        &owner,
        cx,
        vec![Op::SetCarouselTrack(n(0), config(Axis::Horizontal, 2, 1))],
    );
    cx.run_until_parked();
    tick(cx, 0);
    tick(cx, 0);
    assert!(ax_hidden(cx, "Action 0").1);
    let (action, hidden) = ax_hidden(cx, "Dialog action");
    assert!(!hidden, "modal is painted outside the card's clip");
    assert!(
        cx.update(|window, _| dialog.is_focused(window)),
        "navigation cannot steal modal focus"
    );
    ax_action(cx, action, gpui::accesskit::Action::Click);
    assert_eq!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(100)
            .iter()
            .filter(|e| matches!(e,Event::Press(_,id,_,_) if *id==n(9)))
            .count(),
        1
    );
}

#[test]
fn track_clipping_suspends_anchored_popover_and_restores_it_on_reveal() {
    for popup_focused in [true, false] {
        let mut app = TestAppContext::single();
        app.update(gpui_base::init);
        let (_reader, writer) = UnixStream::pair().unwrap();
        let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, w(), "Portal", 400., 300.)
            .unwrap();
        let (owner, cx) =
            app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
        cx.simulate_a11y_active(true);
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();
        apply(&owner, cx, initial(Axis::Horizontal));
        apply(
            &owner,
            cx,
            vec![
                Op::Create(n(8), Kind::FocusScope, String::new(), Some(h(8))),
                Op::SetFocusScope(
                    n(8),
                    FocusScopeConfig {
                        trap: false,
                        auto_focus: true,
                        restore_focus: true,
                    },
                ),
                Op::SetOverlay(
                    n(8),
                    Some(OverlayConfig {
                        kind: OverlayKind::Popover,
                        label: "Card popup".into(),
                        width: 220.,
                        dismiss_on_escape: true,
                        dismiss_on_outside_pointer: false,
                    }),
                ),
                Op::SetStyle(n(8), vec![Style::Height(Length::Px(160.))]),
                Op::Create(n(9), Kind::Button, "Popup action".into(), Some(h(9))),
                Op::SetControl(n(9), Control::Button(false)),
                Op::Splice(n(8), 0, 0, vec![n(9)]),
                Op::Create(n(10), Kind::Container, String::new(), None),
                Op::SetPopover(n(10), true),
                Op::Splice(n(2), 0, 1, vec![n(10)]),
                Op::Splice(n(10), 0, 0, vec![n(5), n(8)]),
            ],
        );
        cx.run_until_parked();
        tick(cx, 0);
        tick(cx, 0);
        requests(&transport);
        let dialog = owner.read_with(cx, |v, _| v.buttons[&n(9)].focus.clone());
        assert!(cx.update(|window, _| dialog.is_focused(window)));
        let (popup_ax, _) = ax_hidden(cx, "Popup action");
        let viewport = owner.read_with(cx, |v, _| v.carousel_tracks[&n(0)].borrow().focus_handle());
        if !popup_focused {
            cx.update(|window, cx| window.focus(&viewport, cx));
            cx.run_until_parked();
        }

        apply(
            &owner,
            cx,
            vec![Op::SetCarouselTrack(n(0), config(Axis::Horizontal, 2, 1))],
        );
        cx.run_until_parked();
        assert!(ax_hidden(cx, "Action 0").1);
        assert!(
            !cx.a11y_tree()
                .unwrap()
                .nodes
                .iter()
                .any(|(_, node)| node.label() == Some("Popup action")),
            "clipped anchor must not leave a floating popup"
        );
        let viewport = owner.read_with(cx, |v, _| v.carousel_tracks[&n(0)].borrow().focus_handle());
        assert!(
            cx.update(|window, _| viewport.is_focused(window)),
            "clipped popup returns focus to viewport"
        );
        ax_action(cx, popup_ax, gpui::accesskit::Action::Click);
        assert!(
            !transport
                .mailbox
                .lock()
                .unwrap()
                .drain(100)
                .iter()
                .any(|event| matches!(event, Event::Press(_,id,_,_) if *id == n(9)))
        );
        apply(
            &owner,
            cx,
            vec![Op::SetCarouselTrack(n(0), config(Axis::Horizontal, 0, 2))],
        );
        cx.run_until_parked();
        assert!(!ax_hidden(cx, "Popup action").1);
        assert!(owner.read_with(cx, |v, _| v.buttons[&n(9)].focus == dialog));
        assert!(
            cx.update(|window, _| viewport.is_focused(window)),
            "revealing a retained popup must not steal focus"
        );
    }
}
