//! Nested assigned sizes, hidden animation time and deferred popup queries.
use super::*;

pub(super) async fn exercise(cx: &mut gpui::AsyncApp, transport: &Arc<Transport>) {
    let window = cx.update(|cx| {
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, window_id(), "Nested query acceptance", 360., 220.)
            .unwrap();
        session
            .borrow()
            .motion()
            .borrow_mut()
            .set_test_time(Some(Duration::ZERO));
        crate::motion_preference::bind_clocks(&session.borrow().motion(), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(360.), px(220.)),
                    cx,
                ))),
                focus: true,
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(window_id(), session.clone(), transport.clone())),
        )
        .unwrap()
    });
    use gpuio_protocol::{
        animation::{Easing, Property, Repeat, Target},
        animation_program as A,
    };
    let program = A::Config {
        generation: 1,
        playback: A::Playback::Running,
        restart: 0,
        program: A::Program {
            initial: Some(vec![Target {
                property: Property::Width,
                value: 20.,
            }]),
            stages: vec![A::Stage {
                targets: vec![Target {
                    property: Property::Width,
                    value: 100.,
                }],
                timing: A::Timing::Tween(1000, Easing::Linear),
                delay_ms: 0,
            }],
            delay_ms: 0,
            repeat: Repeat::Once,
            clock: A::Clock::Independent,
        },
    };
    apply(
        cx,
        window,
        vec![
            Op::Create(node(0), Kind::ContainerQuery, "".into(), Some(handler(0))),
            Op::Create(node(1), Kind::Container, "".into(), None),
            Op::Create(node(2), Kind::Text, "Wide outer branch".into(), None),
            Op::Create(node(3), Kind::ContainerQuery, "".into(), Some(handler(3))),
            Op::Create(
                node(4),
                Kind::Button,
                "Narrow inner".into(),
                Some(handler(4)),
            ),
            Op::Create(node(5), Kind::Button, "Wide inner".into(), Some(handler(5))),
            Op::Create(node(6), Kind::AnimationProgram, "".into(), None),
            Op::SetControl(node(4), Control::Button(false)),
            Op::SetControl(node(5), Control::Button(false)),
            Op::SetContainerQuery(node(0), config(1, 400.)),
            Op::SetContainerQuery(node(3), config(1, 200.)),
            Op::SetStyle(
                node(3),
                vec![
                    Style::Width(Length::Px(250.)),
                    Style::Height(Length::Px(80.)),
                ],
            ),
            Op::SetStyle(
                node(6),
                vec![
                    Style::Height(Length::Px(30.)),
                    Style::Background(Color::Rgba(0x588bffff)),
                ],
            ),
            Op::SetAnimationProgram(node(6), program),
            Op::Splice(node(3), 0, 0, vec![node(4), node(5)]),
            Op::Splice(node(1), 0, 0, vec![node(3), node(6)]),
            Op::Splice(node(0), 0, 0, vec![node(1), node(2)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    // Admission precedes layout: no branch may accumulate hidden active time
    // while the application waits to obtain its first assigned size/paint.
    window
        .update(cx, |v, w, _| {
            v.session
                .borrow()
                .motion()
                .borrow_mut()
                .set_test_time(Some(Duration::from_millis(500)));
            w.refresh();
        })
        .unwrap();
    frame(cx, window).await;
    window
        .update(cx, |v, _, _| {
            assert!((f32::from(v.probes.borrow()[&node(6)].bounds.size.width) - 20.).abs() < 0.1);
        })
        .unwrap();
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert!(events.iter().any(|e| matches!(e, Event::ContainerSelected(_, n, _, _, s) if *n == node(3) && s.branch == 1 && s.width == 250.)));
    let inner = window
        .update(cx, |v, _, _| {
            assert!(v.focus.borrow().allows(node(5)));
            assert!(!v.focus.borrow().allows(node(4)));
            Rc::downgrade(&v.buttons[&node(5)])
        })
        .unwrap();
    let set_time = |cx: &mut gpui::AsyncApp, millis| {
        window
            .update(cx, |v, w, _| {
                v.session
                    .borrow()
                    .motion()
                    .borrow_mut()
                    .set_test_time(Some(Duration::from_millis(millis)));
                w.refresh();
            })
            .unwrap()
    };
    let width = |cx: &mut gpui::AsyncApp| {
        window
            .update(cx, |v, _, _| {
                f32::from(v.probes.borrow()[&node(6)].bounds.size.width)
            })
            .unwrap()
    };
    set_time(cx, 1000);
    frame(cx, window).await;
    assert!((width(cx) - 60.).abs() < 0.1);
    window
        .update(cx, |_, w, _| w.resize(size(px(480.), px(220.))))
        .unwrap();
    frame(cx, window).await;
    window
        .update(cx, |v, _, _| assert!(!v.focus.borrow().allows(node(5))))
        .unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(80))
        .await;
    let before = window.update(cx, |v, _, _| v.render_count).unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(120))
        .await;
    assert_eq!(
        before,
        window.update(cx, |v, _, _| v.render_count).unwrap(),
        "hidden animated branch stays idle"
    );
    set_time(cx, 2000);
    frame(cx, window).await;
    window
        .update(cx, |_, w, _| w.resize(size(px(360.), px(220.))))
        .unwrap();
    frame(cx, window).await;
    assert!(
        (width(cx) - 60.).abs() < 0.1,
        "hidden interval excluded from independent animation"
    );
    window
        .update(cx, |v, _, _| {
            assert!(Rc::ptr_eq(&inner.upgrade().unwrap(), &v.buttons[&node(5)]))
        })
        .unwrap();
    selections(transport);
    apply(
        cx,
        window,
        vec![
            Op::Create(node(7), Kind::FocusScope, "".into(), Some(handler(7))),
            Op::SetFocusScope(
                node(7),
                FocusScopeConfig {
                    trap: true,
                    auto_focus: true,
                    restore_focus: true,
                },
            ),
            Op::SetOverlay(
                node(7),
                Some(OverlayConfig {
                    kind: OverlayKind::Dialog,
                    label: "Responsive dialog".into(),
                    width: 280.,
                    dismiss_on_escape: true,
                    dismiss_on_outside_pointer: true,
                }),
            ),
            Op::Create(node(8), Kind::ContainerQuery, "".into(), Some(handler(8))),
            Op::Create(node(9), Kind::Text, "Narrow dialog".into(), None),
            Op::Create(node(10), Kind::Text, "Wide dialog".into(), None),
            Op::SetContainerQuery(node(8), config(1, 200.)),
            Op::SetStyle(
                node(8),
                vec![
                    Style::Width(Length::Px(240.)),
                    Style::Height(Length::Px(80.)),
                ],
            ),
            Op::Splice(node(8), 0, 0, vec![node(9), node(10)]),
            Op::Splice(node(7), 0, 0, vec![node(8)]),
            Op::Splice(node(1), 2, 0, vec![node(7)]),
        ],
    );
    frame(cx, window).await;
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert!(events.iter().any(|e| matches!(e, Event::ContainerSelected(_, n, _, _, s) if *n == node(8) && s.branch == 1)), "query paints inside deferred dialog");
    window
        .update(cx, |v, _, _| {
            assert!(v.focus.borrow().allows(node(10)));
            assert!(!v.focus.borrow().allows(node(5)));
        })
        .unwrap();
    window
        .update(cx, |_, w, _| w.resize(size(px(480.), px(220.))))
        .unwrap();
    frame(cx, window).await;
    window
        .update(cx, |v, _, _| {
            assert!(!v.focus.borrow().allows(node(10)));
            assert!(v.focus.borrow().allows(node(2)));
        })
        .unwrap();
    let mut remove = vec![Op::SetRoot(None)];
    remove.extend((0..=10).rev().map(|i| Op::Remove(node(i))));
    apply(cx, window, remove);
    frame(cx, window).await;
    assert!(inner.upgrade().is_none());
    window
        .update(cx, |v, w, _| {
            assert!(v.container_queries.is_empty());
            assert_eq!(v.session.borrow().motion().borrow().counts(), (0, 0, 0));
            assert_eq!(v.session.borrow().retained_bytes(), 0);
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
        })
        .unwrap();
    eprintln!(
        "GPUIO_CONTAINER_QUERY_NESTED_OK: assigned inner size, retained nested state, hidden animation pause/idle, deferred modal selection and disposal"
    );
}
