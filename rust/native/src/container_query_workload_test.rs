//! Many simultaneously laid-out queries, bounded rules and retained alternatives.
use super::*;
use std::time::Instant;

fn query(item: i64) -> NodeId {
    node(17 + item * 3)
}
fn compact(item: i64) -> NodeId {
    node(18 + item * 3)
}
fn wide(item: i64) -> NodeId {
    node(19 + item * 3)
}

pub(super) async fn exercise(cx: &mut gpui::AsyncApp, transport: &Arc<Transport>) {
    let window = cx.update(|cx| {
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, window_id(), "Container query workload", 400., 300.)
            .unwrap();
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(400.), px(300.)),
                    cx,
                ))),
                focus: true,
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(window_id(), session.clone(), transport.clone())),
        )
        .unwrap()
    });
    let mut config = config(1, 23.);
    // Exercise the maximum rule count with the successful rule last.
    let mut never_at_this_size = config.rules[0].clone();
    never_at_this_size.condition.width.minimum = 1000.;
    config
        .rules
        .splice(0..0, std::iter::repeat_n(never_at_this_size, 31));
    let mut ops = vec![Op::Create(node(0), Kind::Container, "".into(), None)];
    for row in 1..=16 {
        ops.extend([
            Op::Create(node(row), Kind::Container, "".into(), None),
            Op::SetStyle(
                node(row),
                vec![Style::Fields(vec![
                    Field::Direction(0),
                    Field::Height(Length::Px(12.)),
                    Field::Shrink(0.),
                ])],
            ),
        ]);
    }
    for item in 0..256 {
        ops.extend([
            Op::Create(query(item), Kind::ContainerQuery, "".into(), None),
            Op::Create(compact(item), Kind::Container, "".into(), None),
            Op::Create(wide(item), Kind::Container, "".into(), None),
            Op::SetContainerQuery(query(item), config.clone()),
            Op::SetStyle(
                query(item),
                vec![Style::Fields(vec![
                    Field::Width(Length::Percent(6.25)),
                    Field::Height(Length::Px(12.)),
                    Field::Shrink(0.),
                ])],
            ),
            Op::SetStyle(
                compact(item),
                vec![Style::Background(Color::Rgba(0x336688ff))],
            ),
            Op::SetStyle(wide(item), vec![Style::Background(Color::Rgba(0x33aa88ff))]),
            Op::Splice(query(item), 0, 0, vec![compact(item), wide(item)]),
        ]);
    }
    for row in 1..=16 {
        let start = (row - 1) * 16;
        ops.push(Op::Splice(
            node(row),
            0,
            0,
            (start..start + 16).map(query).collect(),
        ));
    }
    let button = node(785);
    ops.extend([
        Op::Create(
            button,
            Kind::Button,
            "Responsive input".into(),
            Some(handler(785)),
        ),
        Op::SetControl(button, Control::Button(false)),
        Op::SetStyle(
            button,
            vec![
                Style::Width(Length::Px(180.)),
                Style::Height(Length::Px(32.)),
            ],
        ),
        Op::Splice(node(0), 0, 0, (1..=16).map(node).chain([button]).collect()),
        Op::SetRoot(Some(node(0))),
    ]);
    let started = Instant::now();
    apply(cx, window, ops);
    let admission = started.elapsed();
    let started = Instant::now();
    frame(cx, window).await;
    let first_frame_barrier = started.elapsed();
    let retained = window
        .update(cx, |v, _, _| {
            assert_eq!(v.container_queries.len(), 256);
            for item in 0..256 {
                assert!(v.focus.borrow().allows(wide(item)));
                assert!(!v.focus.borrow().allows(compact(item)));
                assert!(
                    v.probes.borrow().contains_key(&wide(item)),
                    "all selected alternatives painted"
                );
            }
            v.session.borrow().retained_bytes()
        })
        .unwrap();
    assert!(selections(transport).is_empty());
    let revision = window
        .update(cx, |v, _, _| {
            v.session.borrow().tree(v.id).unwrap().revision()
        })
        .unwrap();
    let mut barriers = Vec::new();
    for width in [320., 400., 320., 400.] {
        let started = Instant::now();
        window
            .update(cx, |_, w, _| w.resize(size(px(width), px(300.))))
            .unwrap();
        frame(cx, window).await;
        barriers.push(started.elapsed());
        window
            .update(cx, |v, _, _| {
                for item in 0..256 {
                    assert!(v.focus.borrow().allows(if width < 368. {
                        compact(item)
                    } else {
                        wide(item)
                    }));
                }
                assert_eq!(v.session.borrow().tree(v.id).unwrap().revision(), revision);
                assert_eq!(v.session.borrow().retained_bytes(), retained);
            })
            .unwrap();
        assert!(
            selections(transport).is_empty(),
            "unobserved native queries produce no bridge selection traffic"
        );
    }
    let started = Instant::now();
    click(cx, window, button);
    let input_dispatch = started.elapsed();
    assert_eq!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .iter()
            .filter(|e| matches!(e, Event::Press(_, id, _, _) if *id == button))
            .count(),
        1
    );
    frame(cx, window).await;
    let renders = window.update(cx, |v, _, _| v.render_count).unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(150))
        .await;
    assert_eq!(
        window.update(cx, |v, _, _| v.render_count).unwrap(),
        renders
    );
    let mut remove = vec![Op::SetRoot(None)];
    remove.extend((0..=785).rev().map(|i| Op::Remove(node(i))));
    let started = Instant::now();
    apply(cx, window, remove);
    let disposal = started.elapsed();
    frame(cx, window).await;
    window
        .update(cx, |v, w, _| {
            assert!(v.container_queries.is_empty());
            assert_eq!(v.session.borrow().retained_bytes(), 0);
            v.session.borrow_mut().close(v.id).unwrap();
            w.remove_window();
        })
        .unwrap();
    eprintln!(
        "GPUIO_CONTAINER_QUERY_WORKLOAD_OK: 256 visible queries, 32 rules each, 512 retained alternatives, retained_bytes={retained}, admission={admission:?}, first_frame_barrier={first_frame_barrier:?}, resize_barriers={barriers:?}, native_input_dispatch={input_dispatch:?}, disposal={disposal:?}; debug wall times include scheduling, not frame CPU or OS input latency"
    );
}
