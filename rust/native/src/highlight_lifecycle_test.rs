//! Production virtual-list reuse and window-scoped highlight ownership.
use super::*;
use gpuio_protocol::list::{
    Config as ListConfig, IdRun, Order, Row, ScrollPolicy, ScrollRequest, ScrollTarget,
};

fn dimensions(width: f64, height: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(height)),
        Field::Shrink(0.),
        Field::Background(Fill::Solid(Color::Rgba(0xffffffff))),
        Field::FontSize(20.),
        Field::LineHeight(Length::Px(30.)),
    ])]
}

fn window_tree(text: &str, color: i64) -> Vec<Op> {
    vec![
        Op::Create(id(6), Kind::HighlightScope, "".into(), Some(handler(500))),
        Op::SetHighlightScope(id(6), config(color)),
        Op::SetStyle(id(6), dimensions(400., 200.)),
        Op::Create(id(7), Kind::Text, text.into(), None),
        Op::SetStyle(id(7), row(20., false, false)),
        Op::Splice(id(6), 0, 0, vec![id(7)]),
        Op::SetRoot(Some(id(6))),
    ]
}

fn window_id(cx: &mut AsyncApp, window: WindowHandle<View>) -> WindowId {
    window.update(cx, |view, _, _| view.id).unwrap()
}

fn current_result(cx: &mut AsyncApp, window: WindowHandle<View>) -> Arc<jobs::Ready> {
    window
        .update(cx, |view, _, _| {
            let state = view.highlights[&id(6)].borrow();
            let jobs::Status::Ready(ready) = state.job.as_ref().unwrap().status() else {
                panic!("ready window scope")
            };
            ready
        })
        .unwrap()
}

async fn window_samples(
    cx: &mut AsyncApp,
    windows: &[WindowHandle<View>],
    transport: &Transport,
    wanted: &[(WindowId, HandlerId, i64)],
) -> Vec<Observation> {
    let mut samples = vec![None; wanted.len()];
    for _ in 0..500 {
        let events = transport.mailbox.lock().unwrap().drain(128);
        for event in events {
            if let Event::HighlightObserved(window, node, handler, _, sample) = event
                && node == id(6)
            {
                for (index, (w, h, count)) in wanted.iter().enumerate() {
                    if window == *w && handler == *h && sample.state == ready(*count) {
                        samples[index] = Some(sample.clone());
                    }
                }
            }
        }
        for window in windows {
            draw(cx, *window);
        }
        if samples.iter().all(Option::is_some) {
            return samples.into_iter().map(Option::unwrap).collect();
        }
        pause(cx).await;
    }
    panic!("missing window-scoped observations: {wanted:?}, received {samples:?}");
}

fn colors(cx: &mut AsyncApp, window: WindowHandle<View>) -> (usize, usize) {
    window
        .update(cx, |_, window, _| {
            window
                .render_to_image()
                .unwrap()
                .pixels()
                .fold((0, 0), |(red, green), p| {
                    (
                        red + usize::from(p.0 == [255, 0, 0, 255]),
                        green + usize::from(p.0 == [0, 255, 0, 255]),
                    )
                })
        })
        .unwrap()
}

fn row_config(offset: i64) -> Config {
    let mut config = config(0xff0000ff);
    config.0[0].appearance.active_color = 0x00ff00ff;
    config.0[0].match_index_offset = offset;
    config.0[0].active_index = Some(if offset < 50_000 { 0 } else { 50_000 });
    config
}

async fn virtual_rows(cx: &mut AsyncApp, window: WindowHandle<View>, transport: &Transport) {
    let mut ops = vec![
        Op::Remove(id(7)),
        Op::Remove(id(6)),
        Op::Create(id(8), Kind::VirtualList, "".into(), Some(handler(900))),
        Op::SetStyle(id(8), dimensions(400., 200.)),
        Op::SetListConfig(
            id(8),
            ListConfig {
                estimated_height: 40.,
                overscan: 80.,
                max_active: 16,
                scroll_policy: ScrollPolicy::KeepPosition,
                scrollbar: true,
                managed: true,
            },
        ),
        Op::SetListOrder(
            id(8),
            Order {
                revision: 1,
                runs: vec![IdRun {
                    first: 1,
                    count: 100_000,
                }],
            },
        ),
    ];
    for row in 0..12 {
        ops.extend([
            Op::Create(
                id(9 + row * 2),
                Kind::HighlightScope,
                "".into(),
                Some(handler(1000 + row)),
            ),
            Op::SetHighlightScope(id(9 + row * 2), row_config(row)),
            Op::SetStyle(id(9 + row * 2), dimensions(375., 40.)),
            Op::Create(id(10 + row * 2), Kind::Text, format!("aaa row {row}"), None),
            Op::SetStyle(id(10 + row * 2), dimensions(350., 30.)),
            Op::Splice(id(9 + row * 2), 0, 0, vec![id(10 + row * 2)]),
        ]);
    }
    ops.extend([
        Op::SetListRows(
            id(8),
            (0..12)
                .map(|row| Row {
                    id: row + 1,
                    node: id(9 + row * 2),
                })
                .collect(),
        ),
        Op::Splice(id(8), 0, 0, (0..12).map(|row| id(9 + row * 2)).collect()),
        Op::SetRoot(Some(id(8))),
    ]);
    apply(cx, window, ops);
    wait_rows(cx, window, transport, 1).await;
    let (red, green) = colors(cx, window);
    assert!(
        red > 20 && green > 20,
        "initial virtual rows have normal and active paints: {red}/{green}"
    );
    let retired = window
        .update(cx, |view, _, _| {
            view.highlights
                .values()
                .map(Rc::downgrade)
                .collect::<Vec<_>>()
        })
        .unwrap();
    apply(
        cx,
        window,
        vec![Op::ScrollList(
            id(8),
            ScrollRequest {
                serial: 1,
                target: ScrollTarget::Offset(50_001, 0.),
            },
        )],
    );
    let mut cleared = false;
    for _ in 0..500 {
        transport.mailbox.lock().unwrap().drain(128);
        draw(cx, window);
        pause(cx).await;
        cleared = window
            .update(cx, |view, _, _| {
                view.lists[&id(8)]
                    .borrow()
                    .observed
                    .as_ref()
                    .is_some_and(|v| v.visible_first == 50_000)
                    && view.highlights.is_empty()
            })
            .unwrap();
        if cleared {
            break;
        }
    }
    assert!(cleared, "distant unloaded page retires old row scopes");
    assert!(
        retired.iter().all(|s| s.upgrade().is_none()),
        "painted row scopes are reclaimed after scrolling away"
    );
    assert_eq!(
        colors(cx, window),
        (0, 0),
        "placeholder page has no stale highlight paint"
    );
    // Reuse the same native node IDs for a different logical page. Bindings and
    // offsets change together, as in the public managed-list reconciler.
    let mut ops = vec![Op::SetListRows(
        id(8),
        (0..12)
            .map(|row| Row {
                id: 50_001 + row,
                node: id(9 + row * 2),
            })
            .collect(),
    )];
    for row in 0..12 {
        ops.extend([
            Op::Bind(id(9 + row * 2), Some(handler(2000 + row))),
            Op::SetHighlightScope(id(9 + row * 2), row_config(50_000 + row * 2)),
            Op::SetText(id(10 + row * 2), format!("aaa aaa distant row {row}")),
        ]);
    }
    apply(cx, window, ops);
    wait_rows(cx, window, transport, 2).await;
    let (red, green) = colors(cx, window);
    assert!(
        red > 20 && green > 20,
        "reused rows have new offsets and match paints: {red}/{green}"
    );
    assert!(
        window
            .update(cx, |view, _, _| view.highlights.len() <= 12)
            .unwrap()
    );
}

async fn wait_rows(
    cx: &mut AsyncApp,
    window: WindowHandle<View>,
    transport: &Transport,
    count: i64,
) {
    let expected_window = window_id(cx, window);
    let mut observed = false;
    for _ in 0..500 {
        for event in transport.mailbox.lock().unwrap().drain(128) {
            if let Event::HighlightObserved(w, _, _, _, sample) = event
                && w == expected_window
                && sample.state == ready(count)
            {
                observed = true;
            }
        }
        draw(cx, window);
        pause(cx).await;
        let ready = window
            .update(cx, |view, _, _| {
                !view.highlights.is_empty()
                    && view
                        .highlights
                        .values()
                        .all(|scope| scope.borrow().observation().state == ready(count))
            })
            .unwrap();
        if ready && observed {
            draw(cx, window);
            return;
        }
    }
    panic!("virtual row queries did not reach {count}");
}

pub(super) async fn exercise(
    cx: &mut AsyncApp,
    first: WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
    transport: &Arc<Transport>,
) {
    let first_id = window_id(cx, first);
    let second_id = WindowId::from_parts(1, 1).unwrap();
    session
        .borrow_mut()
        .open(2, second_id, "Independent highlight window", 400., 200.)
        .unwrap();
    let bounds = cx.update(|cx| Bounds::centered(None, size(px(400.), px(200.)), cx));
    let second = cx
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                focus: false,
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(second_id, session.clone(), transport.clone())),
        )
        .unwrap();
    let checked = crate::host::native_test::protect(async {
        apply(cx, first, window_tree("aaa", 0xff0000ff));
        // Allocate and retire the first six slots to match the first window's
        // allocator history. Native node IDs are dense and generation checked.
        let mut second_tree = Vec::new();
        for slot in 0..6 {
            second_tree.push(Op::Create(id(slot), Kind::Container, "".into(), None));
            second_tree.push(Op::Remove(id(slot)));
        }
        second_tree.extend(window_tree("aaa aaa", 0x00ff00ff));
        apply(cx, second, second_tree);
        let samples = window_samples(
            cx,
            &[first, second],
            transport,
            &[(first_id, handler(500), 1), (second_id, handler(500), 2)],
        )
        .await;
        let survivor = current_result(cx, second);
        let first_colors = colors(cx, first);
        let second_colors = colors(cx, second);
        assert!(first_colors.0 > 20 && first_colors.1 == 0);
        assert!(second_colors.1 > 20 && second_colors.0 == 0);
        apply(
            cx,
            first,
            vec![
                Op::Bind(id(6), Some(handler(501))),
                Op::SetText(id(7), "aaa aaa aaa".into()),
            ],
        );
        window_samples(
            cx,
            &[first, second],
            transport,
            &[(first_id, handler(501), 3)],
        )
        .await;
        assert!(
            Arc::ptr_eq(&survivor, &current_result(cx, second)),
            "one window's source update does not restart the other"
        );
        let survivor_epoch = second
            .update(cx, |view, _, _| view.highlights[&id(6)].borrow().epoch)
            .unwrap();
        assert_eq!(survivor_epoch, samples[1].epoch);
        virtual_rows(cx, first, transport).await;
        assert!(
            Arc::ptr_eq(&survivor, &current_result(cx, second)),
            "virtual-list churn is window-local"
        );
        apply(cx, first, vec![Op::SetText(id(10), "aaa ".repeat(8192))]);
        draw(cx, first);
        let retiring = first
            .update(cx, |view, _, _| {
                view.highlights
                    .values()
                    .map(Rc::downgrade)
                    .collect::<Vec<_>>()
            })
            .unwrap();
        transport.mailbox.lock().unwrap().drain(128);
        first
            .update(cx, |_, window, _| window.remove_window())
            .unwrap();
        session.borrow_mut().close(first_id).unwrap();
        for _ in 0..8 {
            draw(cx, second);
            pause(cx).await;
        }
        assert!(
            retiring.iter().all(|scope| scope.upgrade().is_none()),
            "window close drops its scopes and pending work"
        );
        let late_events = transport.mailbox.lock().unwrap().drain(128);
        assert!(
            !late_events.iter().any(|event| {
                matches!(event, Event::HighlightObserved(window, _, _, _, _) if *window == first_id)
            }),
            "closed window cannot publish late highlight observations"
        );
        apply(
            cx,
            second,
            vec![
                Op::Bind(id(6), Some(handler(501))),
                Op::SetText(id(7), "aaa aaa aaa aaa".into()),
            ],
        );
        window_samples(cx, &[second], transport, &[(second_id, handler(501), 4)]).await;
        let (red, green) = colors(cx, second);
        assert!(
            green > 20 && red == 0,
            "surviving window continues independently"
        );
        eprintln!(
            "GPUIO_NATIVE_HIGHLIGHT_LIFECYCLE_OK: window-scoped IDs/counts/colors/epochs, \
             100k logical rows, distant placeholders, node reuse and offsets, \
             row cleanup, window close and surviving updates"
        );
    })
    .await;
    let _ = second.update(cx, |_, window, _| window.remove_window());
    let _ = session.borrow_mut().close(second_id);
    if let Err(error) = checked {
        std::panic::resume_unwind(error);
    }
}
