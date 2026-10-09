//! Retained branches and deferred list layout must preserve or retire native
//! timelines according to node lifetime, not merely whether a row was painted.
use super::*;
use gpuio_protocol::{
    container_query::{Config as Rules, Predicate, Range, Rule},
    list::{Config as ListConfig, IdRun, Order, Row, ScrollPolicy, ScrollRequest, ScrollTarget},
};

fn dimensions(width: Length, height: f64, hidden: bool) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(width),
        Field::Height(Length::Px(height)),
        Field::Shrink(0.),
        Field::Display(if hidden { 3 } else { 1 }),
        Field::FontSize(24.),
        Field::LineHeight(Length::Px(30.)),
        Field::Foreground(Color::Rgba(0x000000ff)),
        Field::Background(Fill::Solid(Color::Rgba(0xffffffff))),
    ])]
}
fn advance(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    clock: &Clock,
    now: &mut u64,
    ms: u64,
) {
    *now += ms;
    clock.set_time(*now);
    draw(cx, handle);
}
fn remove_range(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, first: i64, last: i64) {
    let mut ops = vec![Op::Splice(id(0), 1, 1, vec![])];
    ops.extend((first..=last).rev().map(|slot| Op::Remove(id(slot))));
    apply(cx, handle, ops);
    draw(cx, handle);
}
fn panels(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, clock: &Clock, now: &mut u64) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(id(4), Kind::Container, String::new(), None),
            Op::SetStyle(id(4), dimensions(Length::Px(360.), 260., false)),
            Op::Create(id(5), Kind::TabPanel, "First".into(), None),
            Op::SetStyle(id(5), dimensions(Length::Px(350.), 80., false)),
            Op::Create(id(6), Kind::Text, "First retained text".into(), None),
            Op::SetTextShimmer(id(6), Some(config())),
            Op::Create(id(7), Kind::TabPanel, "Second".into(), None),
            Op::SetStyle(id(7), dimensions(Length::Px(350.), 80., true)),
            Op::Create(id(8), Kind::Text, "Second retained text".into(), None),
            Op::SetTextShimmer(id(8), Some(config())),
            Op::Splice(id(5), 0, 0, vec![id(6)]),
            Op::Splice(id(7), 0, 0, vec![id(8)]),
            Op::Splice(id(4), 0, 0, vec![id(5), id(7)]),
            Op::Splice(id(0), 1, 0, vec![id(4)]),
        ],
    );
    draw(cx, handle);
    let first = probe(cx, handle, id(6));
    advance(cx, handle, clock, now, 250);
    assert_eq!(first.snapshot().unwrap().phase, 0.25);
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(id(5), dimensions(Length::Px(350.), 80., true)),
            Op::SetStyle(id(7), dimensions(Length::Px(350.), 80., false)),
        ],
    );
    draw(cx, handle);
    let second = probe(cx, handle, id(8));
    assert_eq!(second.snapshot().unwrap().phase, 0.);
    idle(cx, handle, &[&first]);
    advance(cx, handle, clock, now, 500);
    assert_eq!(first.snapshot().unwrap().phase, 0.25);
    assert_eq!(second.snapshot().unwrap().phase, 0.5);
    assert!(colors(&image(cx, handle))[0] > 20);
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(id(5), dimensions(Length::Px(350.), 80., false)),
            Op::SetStyle(id(7), dimensions(Length::Px(350.), 80., true)),
        ],
    );
    draw(cx, handle);
    assert_eq!(
        first.snapshot().unwrap().phase,
        0.25,
        "retained tab resumes"
    );
    idle(cx, handle, &[&second]);
    // Source changes while hidden restart only that retained owner.
    apply(
        cx,
        handle,
        vec![Op::SetText(id(8), "Updated hidden text".into())],
    );
    draw(cx, handle);
    advance(cx, handle, clock, now, 2000);
    assert_eq!(second.snapshot().unwrap().phase, 0.);
    assert!(!second.snapshot().unwrap().running);
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(id(5), dimensions(Length::Px(350.), 80., true)),
            Op::SetStyle(id(7), dimensions(Length::Px(350.), 80., false)),
        ],
    );
    draw(cx, handle);
    assert_eq!(second.snapshot().unwrap().phase, 0.);
    assert!(second.snapshot().unwrap().running);
    remove_range(cx, handle, 4, 8);
    assert!(first.snapshot().is_none() && second.snapshot().is_none());
}

async fn responsive(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    clock: &Clock,
    now: &mut u64,
) {
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(id(0), dimensions(Length::Percent(100.), 260., false)),
            Op::Create(id(9), Kind::ContainerQuery, String::new(), None),
            Op::SetStyle(id(9), dimensions(Length::Percent(100.), 120., false)),
            Op::SetContainerQuery(
                id(9),
                Rules {
                    generation: 1,
                    branches: vec!["compact".into(), "wide".into()],
                    default: 0,
                    rules: vec![Rule {
                        condition: Predicate {
                            width: Range {
                                minimum: 450.,
                                maximum: None,
                            },
                            height: Range::ALL,
                        },
                        branch: 1,
                    }],
                },
            ),
            Op::Create(id(10), Kind::Text, "Compact branch text".into(), None),
            Op::SetTextShimmer(id(10), Some(config())),
            Op::Create(id(11), Kind::Text, "Wide branch text".into(), None),
            Op::SetTextShimmer(id(11), Some(config())),
            Op::Splice(id(9), 0, 0, vec![id(10), id(11)]),
            Op::Splice(id(0), 1, 0, vec![id(9)]),
        ],
    );
    draw(cx, handle);
    let compact = probe(cx, handle, id(10));
    advance(cx, handle, clock, now, 250);
    let revision = handle
        .update(cx, |v, _, _| {
            v.session.borrow().tree(v.id).unwrap().revision()
        })
        .unwrap();
    handle
        .update(cx, |_, w, _| w.resize(size(px(520.), px(260.))))
        .unwrap();
    for _ in 0..100 {
        draw(cx, handle);
        if handle
            .update(cx, |v, _, _| {
                v.text_shimmers
                    .get(&id(11))
                    .is_some_and(|o| o.probe().snapshot().unwrap().running)
            })
            .unwrap()
        {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    let wide = probe(cx, handle, id(11));
    assert!(wide.snapshot().unwrap().running);
    assert_eq!(wide.snapshot().unwrap().phase, 0.);
    idle(cx, handle, &[&compact]);
    advance(cx, handle, clock, now, 500);
    assert_eq!(compact.snapshot().unwrap().phase, 0.25);
    assert_eq!(wide.snapshot().unwrap().phase, 0.5);
    assert!(colors(&image(cx, handle))[0] > 20);
    handle
        .update(cx, |_, w, _| w.resize(size(px(360.), px(260.))))
        .unwrap();
    for _ in 0..100 {
        draw(cx, handle);
        if compact.snapshot().unwrap().running {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    assert!(compact.snapshot().unwrap().running);
    assert_eq!(compact.snapshot().unwrap().phase, 0.25);
    idle(cx, handle, &[&wide]);
    assert_eq!(
        handle
            .update(cx, |v, _, _| v
                .session
                .borrow()
                .tree(v.id)
                .unwrap()
                .revision())
            .unwrap(),
        revision,
        "resize switches branch with no bridge transaction"
    );
    remove_range(cx, handle, 9, 11);
    assert!(compact.snapshot().is_none() && wide.snapshot().is_none());
}

fn list_apply(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, operations: Vec<Op>) {
    handle
        .update(cx, |view, window, cx| {
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
            view.list_actions(&applied.lists, window, cx);
            cx.notify();
        })
        .unwrap();
}
fn rows(generation: i64, first: i64) -> Vec<Row> {
    (0..12)
        .map(|i| Row {
            id: first + i,
            node: NodeId::from_parts(13 + i, generation).unwrap(),
        })
        .collect()
}
fn row_ops(generation: i64, first: i64) -> Vec<Op> {
    let mut ops = Vec::new();
    for row in rows(generation, first) {
        ops.extend([
            Op::Create(
                row.node,
                Kind::Text,
                format!("Shimmer row {}", row.id),
                None,
            ),
            Op::SetStyle(row.node, dimensions(Length::Px(330.), 40., false)),
            Op::SetTextShimmer(row.node, Some(config())),
        ]);
    }
    ops.extend([
        Op::SetListRows(id(12), rows(generation, first)),
        Op::Splice(
            id(12),
            0,
            0,
            rows(generation, first).iter().map(|r| r.node).collect(),
        ),
    ]);
    ops
}
async fn list_position(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, first: i64) {
    for _ in 0..100 {
        draw(cx, handle);
        if handle
            .update(cx, |v, _, _| {
                v.lists[&id(12)]
                    .borrow()
                    .observed
                    .as_ref()
                    .is_some_and(|s| s.visible_first == first)
            })
            .unwrap()
        {
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    panic!("list never reached logical position {first}");
}
async fn virtual_rows(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    clock: &Clock,
    now: &mut u64,
) {
    let mut ops = vec![
        Op::Create(id(12), Kind::VirtualList, String::new(), None),
        Op::SetStyle(id(12), dimensions(Length::Px(350.), 240., false)),
        Op::SetListConfig(
            id(12),
            ListConfig {
                estimated_height: 40.,
                overscan: 80.,
                max_active: 16,
                scroll_policy: ScrollPolicy::KeepPosition,
                scrollbar: false,
                managed: true,
            },
        ),
        Op::SetListOrder(
            id(12),
            Order {
                revision: 1,
                runs: vec![IdRun {
                    first: 1,
                    count: 100_000,
                }],
            },
        ),
    ];
    ops.extend(row_ops(1, 1));
    ops.push(Op::Splice(id(0), 1, 0, vec![id(12)]));
    list_apply(cx, handle, ops);
    list_position(cx, handle, 0).await;
    let first = probe(cx, handle, id(13));
    advance(cx, handle, clock, now, 500);
    assert_eq!(first.snapshot().unwrap().phase, 0.5);
    assert!(colors(&image(cx, handle))[0] > 20);
    let old = handle
        .update(cx, |v, _, _| {
            (13..25)
                .filter_map(|n| v.text_shimmers.get(&id(n)).map(|o| o.probe()))
                .collect::<Vec<_>>()
        })
        .unwrap();
    assert!(!old.is_empty() && old.len() <= 12);
    list_apply(
        cx,
        handle,
        vec![Op::ScrollList(
            id(12),
            ScrollRequest {
                serial: 1,
                target: ScrollTarget::Offset(50_001, 0.),
            },
        )],
    );
    list_position(cx, handle, 50_000).await;
    idle(cx, handle, &old.iter().collect::<Vec<_>>());
    advance(cx, handle, clock, now, 2000);
    assert_eq!(
        first.snapshot().unwrap().phase,
        0.5,
        "offscreen materialized rows pause"
    );
    assert_eq!(
        colors(&image(cx, handle))[0],
        0,
        "unloaded page has no old row paint"
    );
    // Actual eviction removes the described nodes; cached list elements and
    // queued wakes must not retain their owners across a new logical page.
    let mut ops = vec![
        Op::SetListRows(id(12), vec![]),
        Op::Splice(id(12), 0, 12, vec![]),
    ];
    ops.extend((13..25).rev().map(|n| Op::Remove(id(n))));
    list_apply(cx, handle, ops);
    assert!(old.iter().all(|p| p.snapshot().is_none()));
    list_apply(cx, handle, row_ops(2, 50_001));
    list_position(cx, handle, 50_000).await;
    let replacement = probe(cx, handle, NodeId::from_parts(13, 2).unwrap());
    assert_eq!(replacement.snapshot().unwrap().phase, 0.);
    advance(cx, handle, clock, now, 250);
    assert_eq!(replacement.snapshot().unwrap().phase, 0.25);
    assert!(old.iter().all(|p| p.snapshot().is_none()));
    let mut ops = vec![
        Op::Splice(id(0), 1, 1, vec![]),
        Op::SetListRows(id(12), vec![]),
        Op::Splice(id(12), 0, 12, vec![]),
    ];
    ops.extend(
        (13..25)
            .rev()
            .map(|n| Op::Remove(NodeId::from_parts(n, 2).unwrap())),
    );
    ops.push(Op::Remove(id(12)));
    list_apply(cx, handle, ops);
    draw(cx, handle);
    delivery(cx, handle);
    assert!(replacement.snapshot().is_none());
}

async fn scope_visibility(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    native_test::move_mouse(cx, handle, gpui::point(px(380.), px(280.)), false);
    let mut styles = dimensions(Length::Px(360.), 260., false);
    styles.push(Style::State(2, vec![Field::Visibility(1)]));
    apply(cx, handle, vec![Op::SetStyle(id(3), styles)]);
    draw(cx, handle);
    let old = handle
        .update(cx, |v, _, _| Rc::downgrade(&v.highlights[&id(3)]))
        .unwrap();
    native_test::move_mouse(cx, handle, gpui::point(px(30.), px(30.)), false);
    draw(cx, handle);
    assert!(
        old.upgrade().is_none(),
        "hidden outer search scope retires work"
    );
    handle
        .update(cx, |v, _, _| assert!(v.highlights.is_empty()))
        .unwrap();
    native_test::move_mouse(cx, handle, gpui::point(px(380.), px(280.)), false);
    let mut restored = false;
    for _ in 0..100 {
        draw(cx, handle);
        restored = handle
            .update(cx, |v, _, _| v.highlights.contains_key(&id(3)))
            .unwrap();
        if restored {
            break;
        }
        cx.background_executor()
            .timer(Duration::from_millis(10))
            .await;
    }
    assert!(
        restored,
        "native state restoration works even after every scope was retired"
    );
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    clock: &Clock,
    now: &mut u64,
) {
    scope_visibility(cx, handle).await;
    let renders = handle.update(cx, |v, _, _| v.render_count).unwrap();
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id(3),
            dimensions(Length::Px(360.), 260., true),
        )],
    );
    draw(cx, handle);
    handle
        .update(cx, |view, _, _| {
            assert!(
                view.render_count - renders <= 8,
                "hiding a populated search scope must settle"
            );
            assert!(
                !view.highlights.contains_key(&id(3)),
                "hidden scope owns no matching work"
            );
        })
        .unwrap();
    panels(cx, handle, clock, now);
    responsive(cx, handle, clock, now).await;
    virtual_rows(cx, handle, clock, now).await;
    let first = NodeId::from_parts(1, 2).unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(id(0), root(vec![])),
            Op::SetStyle(id(3), dimensions(Length::Px(360.), 260., false)),
            Op::SetText(first, "Fresh after lifecycle checks".into()),
        ],
    );
    draw(cx, handle);
    assert_eq!(probe(cx, handle, first).snapshot().unwrap().phase, 0.);
    eprintln!(
        "GPUIO_NATIVE_TEXT_SHIMMER_LIFECYCLE_OK: retained tabs, hidden source replacement, native responsive branches, 100k logical list placeholders, materialized pause, eviction and generation remount"
    );
}
