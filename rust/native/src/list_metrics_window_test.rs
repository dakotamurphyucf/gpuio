//! Actual platform layout and painted row bounds; wheel input is dispatched at
//! GPUI's platform-event boundary, not synthesized through the OS input service.
use super::*;

pub(super) async fn exercise(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    let ancestor_style = |height| {
        vec![Style::Fields(vec![
            Field::Width(Length::Px(420.)),
            Field::Height(Length::Px(300.)),
            Field::FontSize(12.),
            Field::LineHeight(Length::Px(height)),
            Field::Foreground(Color::Rgba(0x182030ff)),
            Field::Background(Fill::Solid(Color::Rgba(0xf5f6f8ff))),
        ])]
    };
    let rows: Vec<_> = (0..100)
        .map(|i| Row {
            id: i + 1,
            node: node(i + 2),
        })
        .collect();
    let mut ops = vec![
        Op::Create(node(0), Kind::Container, "".into(), None),
        Op::SetStyle(node(0), ancestor_style(20.)),
        Op::Create(
            node(1),
            Kind::VirtualList,
            "".into(),
            Some(gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetStyle(node(1), dimensions(400., 200.)),
        Op::SetListConfig(
            node(1),
            Config {
                estimated_height: 20.,
                overscan: 100.,
                max_active: 128,
                scroll_policy: ScrollPolicy::KeepPosition,
                scrollbar: false,
                managed: false,
            },
        ),
        Op::SetListOrder(
            node(1),
            Order {
                revision: 1,
                runs: vec![IdRun {
                    first: 1,
                    count: 100,
                }],
            },
        ),
    ];
    for row in &rows {
        ops.push(Op::Create(
            row.node,
            Kind::Text,
            format!("Row {} · inherited text sizing", row.id),
            None,
        ));
    }
    ops.extend([
        Op::Splice(node(1), 0, 0, rows.iter().map(|r| r.node).collect()),
        Op::SetListRows(node(1), rows),
        Op::Splice(node(0), 0, 0, vec![node(1)]),
        Op::SetRoot(Some(node(0))),
    ]);
    apply(cx, window, ops);
    frame(cx, window).await;
    let handle = window
        .update(cx, |v, _, _| {
            v.lists[&node(1)].borrow().native.handle().clone()
        })
        .unwrap();
    let painted = |cx: &mut gpui::AsyncApp| {
        window
            .update(cx, |v, _, _| v.probes.borrow()[&node(52)].bounds)
            .unwrap()
    };
    for (serial, height) in [40., 12., 28.].into_iter().enumerate() {
        apply(
            cx,
            window,
            vec![Op::ScrollList(
                node(1),
                ScrollRequest {
                    serial: serial as i64 + 1,
                    target: ScrollTarget::Offset(51, 5.),
                },
            )],
        );
        frame(cx, window).await;
        apply(
            cx,
            window,
            vec![Op::SetStyle(node(0), ancestor_style(height))],
        );
        frame(cx, window).await;
        assert_eq!(handle.logical_scroll_top().item_ix, 50);
        assert_eq!(handle.logical_scroll_top().offset_in_item, px(5.));
        let before = painted(cx);
        assert_eq!(before.size.height, px(height as f32));
        let position = handle.viewport_bounds().origin + gpui::point(px(20.), px(20.));
        super::super::native_test::move_mouse(cx, window, position, false);
        window
            .update(cx, |view, w, cx| {
                view.probes.borrow_mut().clear();
                w.dispatch_event(
                    gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                        position,
                        delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(24.))),
                        touch_phase: gpui::TouchPhase::Moved,
                        modifiers: Default::default(),
                    }),
                    cx,
                );
            })
            .unwrap();
        frame(cx, window).await;
        let after = painted(cx);
        assert_eq!(
            after.origin.y - before.origin.y,
            px(24.),
            "painted row must follow the wheel exactly"
        );
        assert_eq!(after, handle.bounds_for_item(50).unwrap());
        eprintln!("LIST_INHERITED_METRICS height={height} painted_delta=24 anchor_before=50:5");
    }
    eprintln!(
        "GPUIO_LIST_INHERITED_METRICS_OK: three line heights, retained anchor, exact wheel displacement and fresh painted bounds"
    );
}
