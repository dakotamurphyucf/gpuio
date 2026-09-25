//! Retained virtual rows and observation/configuration lifetimes.
use super::*;
use gpuio_protocol::list::{
    Config as ListConfig, IdRun, Order, Row, ScrollPolicy, ScrollRequest, ScrollTarget,
};

fn query(row: i64) -> NodeId {
    node(row * 3 - 2)
}
fn compact(row: i64) -> NodeId {
    node(row * 3 - 1)
}
fn wide(row: i64) -> NodeId {
    node(row * 3)
}
fn scroll(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, serial: i64, row: i64) {
    apply(
        cx,
        window,
        vec![Op::ScrollList(
            node(0),
            ScrollRequest {
                serial,
                target: ScrollTarget::Offset(row, 0.),
            },
        )],
    );
}
fn observations(transport: &Transport) -> Vec<(NodeId, gpuio_protocol::HandlerId, Snapshot)> {
    transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| match event {
            Event::ContainerSelected(_, node, handler, _, snapshot) => {
                Some((node, handler, snapshot))
            }
            _ => None,
        })
        .collect()
}
pub(super) async fn exercise(cx: &mut gpui::AsyncApp, transport: &Arc<Transport>) {
    let window = cx.update(|cx| {
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, window_id(), "Virtual query retention", 360., 220.)
            .unwrap();
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
    let mut ops = vec![
        Op::Create(node(0), Kind::VirtualList, "".into(), None),
        Op::SetListConfig(
            node(0),
            ListConfig {
                estimated_height: 40.,
                overscan: 0.,
                max_active: 64,
                scroll_policy: ScrollPolicy::KeepPosition,
                scrollbar: false,
                managed: false,
            },
        ),
        Op::SetListOrder(
            node(0),
            Order {
                revision: 1,
                runs: vec![IdRun {
                    first: 1,
                    count: 32,
                }],
            },
        ),
        Op::SetStyle(
            node(0),
            vec![
                Style::Width(Length::Px(320.)),
                Style::Height(Length::Px(160.)),
            ],
        ),
    ];
    for row in 1..=32 {
        ops.extend([
            Op::Create(
                query(row),
                Kind::ContainerQuery,
                "".into(),
                Some(handler(row)),
            ),
            Op::Create(
                compact(row),
                Kind::Button,
                format!("Compact {row}"),
                Some(handler(100 + row)),
            ),
            Op::Create(
                wide(row),
                Kind::Button,
                format!("Wide {row}"),
                Some(handler(200 + row)),
            ),
            Op::SetControl(compact(row), Control::Button(false)),
            Op::SetControl(wide(row), Control::Button(false)),
            Op::SetContainerQuery(query(row), config(1, 300.)),
            Op::SetStyle(
                query(row),
                vec![Style::Fields(vec![
                    Field::Height(Length::Px(40.)),
                    Field::Shrink(0.),
                ])],
            ),
            Op::Splice(query(row), 0, 0, vec![compact(row), wide(row)]),
        ]);
    }
    ops.extend([
        Op::SetListRows(
            node(0),
            (1..=32)
                .map(|row| Row {
                    id: row,
                    node: query(row),
                })
                .collect(),
        ),
        Op::Splice(node(0), 0, 0, (1..=32).map(query).collect()),
        Op::SetRoot(Some(node(0))),
    ]);
    apply(cx, window, ops);
    frame(cx, window).await;
    let initial = observations(transport);
    let first = initial.iter().find(|(id, _, _)| *id == query(1)).unwrap().2;
    assert_eq!(first.branch, 1);
    assert!(
        !initial.iter().any(|(id, _, _)| *id == query(20)),
        "offscreen queries do not publish selections"
    );
    let retained = window
        .update(cx, |v, _, _| {
            assert!(v.focus.borrow().allows(wide(1)));
            assert!(!v.focus.borrow().allows(compact(1)));
            assert!(!v.focus.borrow().allows(wide(20)));
            Rc::downgrade(&v.buttons[&wide(1)])
        })
        .unwrap();
    scroll(cx, window, 1, 20);
    frame(cx, window).await;
    window
        .update(cx, |v, _, _| {
            assert_eq!(
                v.lists[&node(0)]
                    .borrow()
                    .observed
                    .as_ref()
                    .unwrap()
                    .visible_first,
                19
            );
            assert!(
                !v.focus.borrow().allows(wide(1)),
                "virtualized branch loses eligibility"
            );
            assert!(v.focus.borrow().allows(wide(20)));
            assert!(Rc::ptr_eq(
                &retained.upgrade().unwrap(),
                &v.buttons[&wide(1)]
            ));
        })
        .unwrap();
    observations(transport);
    scroll(cx, window, 2, 1);
    frame(cx, window).await;
    assert!(
        !observations(transport)
            .iter()
            .any(|(id, _, _)| *id == query(1)),
        "returning unchanged branch does not replay its selection"
    );

    // A config/observer change while absent must wait for an actual selected paint.
    scroll(cx, window, 3, 20);
    frame(cx, window).await;
    observations(transport);
    let mut reordered = config(2, 300.);
    reordered.branches = vec!["wide".into(), "compact".into()];
    reordered.default = 1;
    reordered.rules[0].branch = 0;
    apply(
        cx,
        window,
        vec![
            Op::SetContainerQuery(query(1), reordered),
            Op::Splice(query(1), 0, 2, vec![wide(1), compact(1)]),
            Op::Bind(query(1), Some(handler(500))),
        ],
    );
    frame(cx, window).await;
    assert!(
        !observations(transport)
            .iter()
            .any(|(id, _, _)| *id == query(1))
    );
    scroll(cx, window, 4, 1);
    frame(cx, window).await;
    let events = observations(transport);
    let selected = events
        .iter()
        .filter(|(id, _, _)| *id == query(1))
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].1, handler(500));
    assert_eq!(selected[0].2.generation, 2);
    assert_eq!(selected[0].2.branch, 0);
    assert!(selected[0].2.sequence > first.sequence);
    window
        .update(cx, |v, _, _| {
            assert!(Rc::ptr_eq(
                &retained.upgrade().unwrap(),
                &v.buttons[&wide(1)]
            ));
        })
        .unwrap();

    apply(cx, window, vec![Op::Bind(query(1), None)]);
    frame(cx, window).await;
    apply(
        cx,
        window,
        vec![
            Op::SetContainerQuery(query(1), config(3, 400.)),
            Op::Splice(query(1), 0, 2, vec![compact(1), wide(1)]),
        ],
    );
    frame(cx, window).await;
    assert!(
        !observations(transport)
            .iter()
            .any(|(id, _, _)| *id == query(1))
    );
    apply(cx, window, vec![Op::Bind(query(1), Some(handler(501)))]);
    frame(cx, window).await;
    assert!(
        !observations(transport)
            .iter()
            .any(|(id, _, _)| *id == query(1)),
        "new observer receives future selections only"
    );
    apply(
        cx,
        window,
        vec![Op::SetContainerQuery(query(1), config(4, 300.))],
    );
    frame(cx, window).await;
    let events = observations(transport);
    let selected = events.iter().find(|(id, _, _)| *id == query(1)).unwrap();
    assert_eq!(selected.1, handler(501));
    assert_eq!(selected.2.generation, 4);
    assert_eq!(selected.2.branch, 1);
    click(cx, window, wide(1));
    assert!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .iter()
            .any(|e| matches!(e, Event::Press(_, id, _, _) if *id == wide(1)))
    );

    // Declared logical sizes, including fractional boundaries, select natively.
    apply(
        cx,
        window,
        vec![Op::SetContainerQuery(query(1), config(5, 300.5))],
    );
    for (width, branch) in [(300.25, 0), (300.5, 1), (300.75, 1)] {
        apply(
            cx,
            window,
            vec![Op::SetStyle(
                node(0),
                vec![
                    Style::Width(Length::Px(width)),
                    Style::Height(Length::Px(160.)),
                ],
            )],
        );
        frame(cx, window).await;
        window
            .update(cx, |v, _, _| {
                assert!(
                    v.focus
                        .borrow()
                        .allows(if branch == 0 { compact(1) } else { wide(1) }),
                    "fractional assigned width {width}"
                );
            })
            .unwrap();
        observations(transport);
    }
    #[cfg(any(feature = "native-image-tests", feature = "native-canvas-tests"))]
    {
        // Exercise GPUI's scale-change path, not a physical monitor transition.
        // Predicates use assigned logical pixels after GPUI device-pixel snapping.
        let original = window.update(cx, |_, w, _| w.scale_factor()).unwrap();
        for (scale_index, scale) in [1., 1.5, 2., original].into_iter().enumerate() {
            window
                .update(cx, |_, w, _| w.set_scale_factor(scale))
                .unwrap();
            for (width_index, width) in [300.25, 300.5, 300.75].into_iter().enumerate() {
                apply(
                    cx,
                    window,
                    vec![
                        Op::SetContainerQuery(
                            query(1),
                            config(6 + (scale_index * 3 + width_index) as i64, 300.5),
                        ),
                        Op::SetStyle(
                            node(0),
                            vec![
                                Style::Width(Length::Px(width)),
                                Style::Height(Length::Px(160.)),
                            ],
                        ),
                    ],
                );
                frame(cx, window).await;
                window
                    .update(cx, |v, w, _| {
                        let assigned = f32::from(w.pixel_snap(px(width as f32))) as f64;
                        let branch = i64::from(assigned >= 300.5);
                        assert!(
                            v.focus
                                .borrow()
                                .allows(if branch == 0 { compact(1) } else { wide(1) }),
                            "declared width {width}, assigned {assigned}, scale {scale}"
                        );
                    })
                    .unwrap();
                let observed = observations(transport);
                let selected = &observed
                    .iter()
                    .find(|(id, _, _)| *id == query(1))
                    .unwrap()
                    .2;
                let assigned = window
                    .update(cx, |_, w, _| {
                        f32::from(w.pixel_snap(px(width as f32))) as f64
                    })
                    .unwrap();
                assert!((selected.width - assigned).abs() < 0.001);
                assert_eq!(selected.branch, i64::from(assigned >= 300.5));
            }
        }
        eprintln!(
            "GPUIO_CONTAINER_QUERY_SCALE_OK: logical fractional thresholds at GPUI test scales 1, 1.5, 2 and restored display scale"
        );
    }
    let before = window.update(cx, |v, _, _| v.render_count).unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(160))
        .await;
    assert_eq!(
        before,
        window.update(cx, |v, _, _| v.render_count).unwrap(),
        "static queries must settle without layout oscillation"
    );

    let mut ops = vec![Op::SetRoot(None)];
    ops.extend((0..=96).rev().map(|id| Op::Remove(node(id))));
    apply(cx, window, ops);
    frame(cx, window).await;
    assert!(retained.upgrade().is_none());
    window
        .update(cx, |v, w, _| {
            assert!(v.container_queries.is_empty());
            assert!(v.lists.is_empty());
            assert_eq!(v.session.borrow().retained_bytes(), 0);
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
        })
        .unwrap();
    eprintln!(
        "GPUIO_CONTAINER_QUERY_LIFECYCLE_OK: virtualized retention/input gating, config reorder, future-only observers, fractional thresholds, idle and disposal"
    );
}
