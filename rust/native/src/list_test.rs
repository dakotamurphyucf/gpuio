//! Real GPUI layout through the production host, including sparse placeholders.
use super::*;
use gpuio_protocol::list::{
    Config, IdRun, Order, Row, ScrollPolicy, ScrollRequest, ScrollTarget, Viewport,
};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn window_id() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn dimensions(width: f64, height: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(height)),
        Field::Shrink(0.),
    ])]
}
fn apply(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, operations: Vec<Op>) {
    window
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let tx = Transaction {
                window: view.id,
                base,
                revision: base + 1,
                operations,
            };
            let applied = view
                .session
                .borrow_mut()
                .apply(&tx)
                .unwrap_or_else(|error| panic!("{error:?}: {tx:?}"));
            view.update_editors(&applied.dirty, window, cx);
            view.list_actions(&applied.lists);
            cx.notify();
        })
        .unwrap();
}
fn viewport(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) -> Viewport {
    window
        .update(cx, |view, _, _| {
            view.lists[&node(0)]
                .borrow()
                .observed
                .clone()
                .expect("list laid out")
        })
        .unwrap()
}
async fn frame(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    for _ in 0..4 {
        let (sender, receiver) = async_channel::bounded(1);
        window
            .update(cx, |_, window, _| {
                window.refresh();
                window.on_next_frame(move |_, _| {
                    let _ = sender.try_send(());
                });
            })
            .unwrap();
        let mut painted = false;
        for attempt in 0..50 {
            let timer = cx
                .background_executor()
                .timer(std::time::Duration::from_millis(100));
            if futures_lite::future::or(
                async {
                    receiver.recv().await.unwrap();
                    true
                },
                async {
                    timer.await;
                    false
                },
            )
            .await
            {
                painted = true;
                break;
            }
            // The direct native host has no OCaml tick/wake loop. Refreshing a
            // delayed frame preserves actual paint confirmation while bounding
            // a platform frame-source stall instead of waiting forever.
            window
                .update(cx, |_, window, cx| {
                    if attempt == 0 {
                        eprintln!("LIST_FRAME retry active={}", window.is_window_active());
                    }
                    cx.notify();
                    window.refresh();
                })
                .unwrap();
        }
        assert!(painted, "native list frame did not arrive within 5 seconds");
    }
}
fn initial() -> Vec<Op> {
    let mut ops = vec![
        Op::Create(
            node(0),
            Kind::VirtualList,
            "".into(),
            Some(gpuio_protocol::HandlerId::from_parts(0, 1).unwrap()),
        ),
        Op::SetStyle(node(0), dimensions(420., 300.)),
        Op::SetListConfig(
            node(0),
            Config {
                estimated_height: 100.,
                overscan: 200.,
                max_active: 32,
                scroll_policy: ScrollPolicy::FollowTailWhenAtEnd,
                scrollbar: true,
                managed: true,
            },
        ),
        Op::SetListOrder(
            node(0),
            Order {
                revision: 1,
                runs: vec![IdRun {
                    first: 1,
                    count: 100_000,
                }],
            },
        ),
        Op::ScrollList(
            node(0),
            ScrollRequest {
                serial: 1,
                target: ScrollTarget::Offset(1, 0.),
            },
        ),
    ];
    for id in 1..=10 {
        ops.extend([
            Op::Create(
                node(id),
                match id {
                    1 => Kind::Button,
                    2 => Kind::Input,
                    _ => Kind::Text,
                },
                if id == 2 {
                    String::new()
                } else {
                    format!("Row {id}")
                },
                if id <= 2 {
                    Some(gpuio_protocol::HandlerId::from_parts(1, 1).unwrap())
                } else {
                    None
                },
            ),
            Op::SetStyle(node(id), dimensions(400., 100.)),
        ]);
    }
    ops.push(Op::SetEditor(
        node(2),
        EditorConfig {
            label: "Pinned row editor".into(),
            placeholder: "Compose here".into(),
            read_only: false,
            disabled: false,
            submit_on_enter: true,
            auto_focus: false,
            min_rows: 1,
            max_rows: 1,
        },
    ));
    let mut selectable = dimensions(400., 100.);
    selectable.push(Style::Fields(vec![Field::UserSelect(true)]));
    ops.push(Op::SetStyle(node(3), selectable));
    ops.extend([
        Op::SetListRows(
            node(0),
            (1..=10).map(|id| Row { id, node: node(id) }).collect(),
        ),
        Op::Splice(node(0), 0, 0, (1..=10).map(node).collect()),
        Op::SetRoot(Some(node(0))),
    ]);
    ops
}
async fn exercise(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    eprintln!("LIST_TEST waiting initial frame");
    frame(cx, window).await;
    eprintln!("LIST_TEST initial layout ready");
    let first = viewport(cx, window);
    assert_eq!(first.visible_first, 0, "{first:?}");
    assert!((2..=4).contains(&first.visible_last), "{first:?}");
    assert!(
        !first.requested.is_empty() && first.requested.len() <= 10,
        "{first:?}"
    );
    assert!(
        first.at_start && !first.at_end && !first.following_tail,
        "{first:?}"
    );
    // The UI gains focus after the last observed range, immediately before an
    // eviction arrives. Native admission must retain it without a layout roundtrip.
    window
        .update(cx, |view, window, cx| {
            view.buttons[&node(1)].focus.focus(window, cx);
            let pins = view.list_pins(window, cx);
            assert!(
                pins.iter()
                    .any(|pin| pin.node == node(0) && pin.rows.contains(&1))
            );
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let tx = Transaction {
                window: view.id,
                base,
                revision: base + 1,
                operations: vec![
                    Op::Remove(node(1)),
                    Op::Splice(node(0), 0, 1, vec![]),
                    Op::SetListRows(
                        node(0),
                        (2..=10).map(|id| Row { id, node: node(id) }).collect(),
                    ),
                ],
            };
            assert!(matches!(
                view.session.borrow_mut().apply_guarded(&tx, &pins),
                Err(crate::tree::ApplyFailure::Retained(_))
            ));
            assert_eq!(
                view.session.borrow().tree(view.id).unwrap().revision(),
                base
            );
        })
        .unwrap();
    apply(
        cx,
        window,
        vec![Op::ScrollList(
            node(0),
            ScrollRequest {
                serial: 2,
                target: ScrollTarget::Offset(50_001, 37.5),
            },
        )],
    );
    frame(cx, window).await;
    let jumped = viewport(cx, window);
    assert_eq!(jumped.anchor, Some((50_001, 37.5)), "{jumped:?}");
    assert_eq!(jumped.visible_first, 50_000);
    assert_eq!(jumped.pinned, vec![1]);
    window
        .update(cx, |view, window, cx| {
            assert!(view.buttons[&node(1)].focus.is_focused(window));
            window.focus(view.root_focus.as_ref().unwrap(), cx);
        })
        .unwrap();
    assert!(
        jumped.requested.contains(&50_001) && jumped.requested.len() <= 10,
        "{jumped:?}"
    );
    apply(
        cx,
        window,
        vec![Op::SetListOrder(
            node(0),
            Order {
                revision: 2,
                runs: vec![
                    IdRun {
                        first: 100_001,
                        count: 1,
                    },
                    IdRun {
                        first: 1,
                        count: 100_000,
                    },
                ],
            },
        )],
    );
    frame(cx, window).await;
    let prepended = viewport(cx, window);
    assert_eq!(prepended.anchor, Some((50_001, 37.5)), "{prepended:?}");
    assert_eq!(prepended.visible_first, 50_001);
    apply(
        cx,
        window,
        vec![Op::SetListOrder(
            node(0),
            Order {
                revision: 3,
                runs: vec![
                    IdRun {
                        first: 50_001,
                        count: 50_000,
                    },
                    IdRun {
                        first: 1,
                        count: 50_000,
                    },
                    IdRun {
                        first: 100_001,
                        count: 1,
                    },
                ],
            },
        )],
    );
    frame(cx, window).await;
    let reordered = viewport(cx, window);
    assert_eq!(reordered.anchor, Some((50_001, 37.5)), "{reordered:?}");
    assert_eq!(reordered.visible_first, 0);
    // Materialize a previously absent row with a different measured height.
    apply(
        cx,
        window,
        vec![
            Op::Create(node(11), Kind::Text, "streaming response".into(), None),
            Op::SetStyle(node(11), dimensions(400., 150.)),
            Op::SetListRows(
                node(0),
                (1..=10)
                    .map(|id| Row { id, node: node(id) })
                    .chain([Row {
                        id: 50_001,
                        node: node(11),
                    }])
                    .collect(),
            ),
            Op::Splice(node(0), 10, 0, vec![node(11)]),
        ],
    );
    frame(cx, window).await;
    window
        .update(cx, |view, _, _| {
            let state = view.lists[&node(0)].borrow();
            assert_eq!(
                state
                    .native
                    .handle()
                    .bounds_for_item(0)
                    .unwrap()
                    .size
                    .height,
                px(150.)
            );
            assert_eq!(
                state.observed.as_ref().unwrap().anchor,
                Some((50_001, 37.5))
            );
        })
        .unwrap();
    apply(
        cx,
        window,
        vec![Op::SetStyle(node(11), dimensions(400., 220.))],
    );
    frame(cx, window).await;
    window
        .update(cx, |view, _, _| {
            let state = view.lists[&node(0)].borrow();
            assert_eq!(
                state
                    .native
                    .handle()
                    .bounds_for_item(0)
                    .unwrap()
                    .size
                    .height,
                px(220.)
            );
            assert_eq!(
                state.observed.as_ref().unwrap().anchor,
                Some((50_001, 37.5))
            );
        })
        .unwrap();
    apply(
        cx,
        window,
        vec![Op::SetStyle(node(0), dimensions(360., 180.))],
    );
    frame(cx, window).await;
    assert_eq!(viewport(cx, window).anchor, Some((50_001, 37.5)));
    apply(
        cx,
        window,
        vec![Op::ScrollList(
            node(0),
            ScrollRequest {
                serial: 3,
                target: ScrollTarget::End,
            },
        )],
    );
    frame(cx, window).await;
    let tail = viewport(cx, window);
    assert!(tail.following_tail && tail.at_end, "{tail:?}");
    // Real event routing, rather than calling ListState::scroll_to directly.
    let position = gpui::point(px(150.), px(80.));
    super::native_test::move_mouse(cx, window, position, false);
    window
        .update(cx, |_, window, cx| {
            window.dispatch_event(
                gpui::PlatformInput::ScrollWheel(gpui::ScrollWheelEvent {
                    position,
                    delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(75.))),
                    touch_phase: gpui::TouchPhase::Started,
                    modifiers: Default::default(),
                }),
                cx,
            );
        })
        .unwrap();
    frame(cx, window).await;
    let away = viewport(cx, window);
    assert!(
        !away.following_tail && !away.at_end,
        "wheel did not pause: {away:?}"
    );
    apply(
        cx,
        window,
        vec![Op::SetListOrder(
            node(0),
            Order {
                revision: 4,
                runs: vec![
                    IdRun {
                        first: 50_001,
                        count: 50_000,
                    },
                    IdRun {
                        first: 1,
                        count: 50_000,
                    },
                    IdRun {
                        first: 100_001,
                        count: 2,
                    },
                ],
            },
        )],
    );
    frame(cx, window).await;
    let appended = viewport(cx, window);
    assert_eq!(appended.anchor, away.anchor, "append moved paused anchor");
    assert!(!appended.following_tail);
    apply(
        cx,
        window,
        vec![Op::ScrollList(
            node(0),
            ScrollRequest {
                serial: 4,
                target: ScrollTarget::End,
            },
        )],
    );
    frame(cx, window).await;
    assert!(viewport(cx, window).following_tail);
    // The scrollbar thumb at the bottom is at least 48 logical pixels high.
    let thumb = gpui::point(px(355.), px(155.));
    super::native_test::move_mouse(cx, window, thumb, false);
    frame(cx, window).await;
    super::native_test::mouse(cx, window, thumb, true);
    let dragged = gpui::point(px(355.), px(70.));
    super::native_test::move_mouse(cx, window, dragged, true);
    super::native_test::mouse(cx, window, dragged, false);
    frame(cx, window).await;
    let dragged_viewport = viewport(cx, window);
    assert!(
        !dragged_viewport.at_end && !dragged_viewport.following_tail,
        "scrollbar did not move: {dragged_viewport:?}"
    );
    window
        .update(cx, |view, _, _| {
            assert_eq!(view.session.borrow().tree(view.id).unwrap().len(), 12);
            assert!(
                view.lists[&node(0)]
                    .borrow()
                    .observed
                    .as_ref()
                    .unwrap()
                    .requested
                    .len()
                    <= 32
            );
        })
        .unwrap();
    // Composition must retain the same native editor while its row is outside
    // the visible range. macOS enters through the actual NSTextInputClient.
    apply(
        cx,
        window,
        vec![Op::ScrollList(
            node(0),
            ScrollRequest {
                serial: 5,
                target: ScrollTarget::Offset(2, 0.),
            },
        )],
    );
    frame(cx, window).await;
    let editor_alive = window
        .update(cx, |view, window, cx| {
            let editor = view.editors.get_mut(&node(2)).unwrap();
            assert!(matches!(
                editor.command(&EditorCommand::Focus, window, cx),
                EditorResult::Applied(_)
            ));
            editor.liveness_probe()
        })
        .unwrap();
    frame(cx, window).await;
    #[cfg(target_os = "macos")]
    super::editor_test::native_text(cx, window, "に", true);
    apply(
        cx,
        window,
        vec![Op::ScrollList(
            node(0),
            ScrollRequest {
                serial: 6,
                target: ScrollTarget::Offset(60_000, 0.),
            },
        )],
    );
    frame(cx, window).await;
    assert!(viewport(cx, window).pinned.contains(&2));
    window
        .update(cx, |view, window, cx| {
            let snapshot = view.editors[&node(2)].snapshot(window, cx);
            assert!(snapshot.focused);
            #[cfg(target_os = "macos")]
            assert!(snapshot.composition.is_some());
        })
        .unwrap();
    #[cfg(target_os = "macos")]
    {
        super::editor_test::native_text(cx, window, "日本", false);
        window
            .update(cx, |view, window, cx| {
                let snapshot = view.editors[&node(2)].snapshot(window, cx);
                assert!(snapshot.composition.is_none());
                assert_eq!(snapshot.text, "日本");
            })
            .unwrap();
    }
    // Native guard vetoes stale virtualization but allows intentional deletion
    // of the logical source record, even while its editor is focused.
    window
        .update(cx, |view, window, cx| {
            let pins = view.list_pins(window, cx);
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let mut tx = Transaction {
                window: view.id,
                base,
                revision: base + 1,
                operations: vec![
                    Op::Remove(node(2)),
                    Op::Splice(node(0), 1, 1, vec![]),
                    Op::SetListRows(
                        node(0),
                        (1..=10)
                            .filter(|id| *id != 2)
                            .map(|id| Row { id, node: node(id) })
                            .chain([Row {
                                id: 50_001,
                                node: node(11),
                            }])
                            .collect(),
                    ),
                ],
            };
            assert!(matches!(
                view.session.borrow_mut().apply_guarded(&tx, &pins),
                Err(crate::tree::ApplyFailure::Retained(_))
            ));
            tx.operations.push(Op::SetListOrder(
                node(0),
                Order {
                    revision: 5,
                    runs: vec![
                        IdRun {
                            first: 50_001,
                            count: 50_000,
                        },
                        IdRun { first: 1, count: 1 },
                        IdRun {
                            first: 3,
                            count: 49_998,
                        },
                        IdRun {
                            first: 100_001,
                            count: 2,
                        },
                    ],
                },
            ));
            let applied = view.session.borrow_mut().apply_guarded(&tx, &pins).unwrap();
            view.update_editors(&applied.dirty, window, cx);
            view.list_actions(&applied.lists);
            cx.notify();
        })
        .unwrap();
    frame(cx, window).await;
    assert!(
        !editor_alive(),
        "deleted row retained its native editor entity"
    );
    apply(
        cx,
        window,
        vec![Op::ScrollList(
            node(0),
            ScrollRequest {
                serial: 7,
                target: ScrollTarget::Offset(3, 0.),
            },
        )],
    );
    frame(cx, window).await;
    let text_position = window
        .update(cx, |view, _, _| {
            view.probes.borrow()[&node(3)].bounds.origin + gpui::point(px(5.), px(8.))
        })
        .unwrap();
    super::native_test::move_mouse(cx, window, text_position, false);
    super::native_test::mouse(cx, window, text_position, true);
    let selection = window
        .update(cx, |view, _, _| {
            let selection = &view.selections[&node(3)];
            assert!(selection.borrow().is_dragging());
            Rc::downgrade(selection)
        })
        .unwrap();
    apply(
        cx,
        window,
        vec![Op::ScrollList(
            node(0),
            ScrollRequest {
                serial: 8,
                target: ScrollTarget::Offset(60_000, 0.),
            },
        )],
    );
    frame(cx, window).await;
    assert!(viewport(cx, window).pinned.contains(&3));
    assert!(selection.upgrade().unwrap().borrow().is_dragging());
    super::native_test::mouse(cx, window, text_position, false);
    frame(cx, window).await;
    assert!(!selection.upgrade().unwrap().borrow().is_dragging());
    let owner = window
        .update(cx, |view, _, _| Rc::downgrade(&view.lists[&node(0)]))
        .unwrap();
    let mut remove = vec![Op::SetRoot(None)];
    remove.extend(
        (1..=11)
            .filter(|id| *id != 2)
            .map(|id| Op::Remove(node(id))),
    );
    remove.push(Op::Remove(node(0)));
    apply(cx, window, remove);
    frame(cx, window).await;
    assert!(owner.upgrade().is_none(), "unmounted list owner retained");
    assert!(
        selection.upgrade().is_none(),
        "unmounted selection retained"
    );
    eprintln!(
        "GPUIO_NATIVE_LIST_OK: 100k logical rows, sparse layout, programmatic viewport, exact prepend/reorder/height/resize anchor, wheel pause/append, scrollbar drag, tail jump, editor/selection retention and disposal"
    );
}
// Resource traversal measures real GPUI layout/paint caches, independently of
// display-link cadence or whether the user's desktop occludes this test window.
// Interaction/IME checks above still require actual platform frames.
fn layout_frames(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    for _ in 0..3 {
        let arena = cx
            .update_window(window.into(), |_, window, cx| {
                window.refresh();
                window.draw(cx)
            })
            .unwrap();
        cx.update(|cx| arena.clear(cx));
    }
}

// Traverse every logical row twice with disjoint native identities. Weak probes
// distinguish dropped payloads/resources from counters that merely look bounded.
async fn history(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) -> Vec<std::sync::Weak<str>> {
    const COUNT: i64 = 100_000;
    const ACTIVE: i64 = 256;
    let root = node(12);
    apply(
        cx,
        window,
        vec![
            Op::Create(root, Kind::VirtualList, String::new(), None),
            Op::SetStyle(root, dimensions(420., ACTIVE as f64)),
            Op::SetListConfig(
                root,
                Config {
                    estimated_height: 1.,
                    overscan: 0.,
                    max_active: ACTIVE,
                    scroll_policy: ScrollPolicy::KeepPosition,
                    scrollbar: false,
                    managed: true,
                },
            ),
            Op::SetListOrder(
                root,
                Order {
                    revision: 1,
                    runs: vec![IdRun {
                        first: 1,
                        count: COUNT,
                    }],
                },
            ),
            Op::SetRoot(Some(root)),
        ],
    );
    let mut previous_nodes = Vec::new();
    let mut previous_text = Vec::<std::sync::Weak<str>>::new();
    let mut previous_selections = Vec::<std::rc::Weak<RefCell<crate::selection::State>>>::new();
    let mut serial = 0;
    let mut generations = [0_i64; ACTIVE as usize];
    let mut visits = 0;
    let mut peak_cached = 0;
    for pass in 0..2 {
        for first in (1..=COUNT).step_by(ACTIVE as usize) {
            let last = (first + ACTIVE - 1).min(COUNT);
            let current: Vec<_> = (first..=last)
                .enumerate()
                .map(|(slot, _)| {
                    generations[slot] += 1;
                    NodeId::from_parts(13 + slot as i64, generations[slot]).unwrap()
                })
                .collect();
            let mut ops: Vec<_> = previous_nodes.iter().copied().map(Op::Remove).collect();
            for (id, node) in (first..=last).zip(current.iter().copied()) {
                let mut styles = dimensions(400., 1.);
                styles.push(Style::Fields(vec![
                    Field::UserSelect(true),
                    Field::OverflowY(1),
                ]));
                ops.extend([
                    Op::Create(
                        node,
                        Kind::Text,
                        format!("row {id}: {}", "payload ".repeat(64)),
                        None,
                    ),
                    Op::SetStyle(node, styles),
                ]);
            }
            serial += 1;
            ops.extend([
                Op::Splice(root, 0, previous_nodes.len() as i64, current.clone()),
                Op::SetListRows(
                    root,
                    (first..=last)
                        .zip(current.iter().copied())
                        .map(|(id, node)| Row { id, node })
                        .collect(),
                ),
                Op::SetStyle(root, dimensions(420., current.len() as f64)),
                Op::ScrollList(
                    root,
                    ScrollRequest {
                        serial,
                        target: ScrollTarget::Offset(first, 0.),
                    },
                ),
            ]);
            // Test-only geometry probes otherwise intentionally remember prior
            // nodes for unrelated interaction tests; they are not product caches.
            window
                .update(cx, |view, _, _| view.probes.borrow_mut().clear())
                .unwrap();
            apply(cx, window, ops);
            if first == 1 || first % (256 * 100) == 1 {
                eprintln!("LIST_HISTORY start pass={} row={first}", pass + 1);
            }
            layout_frames(cx, window);
            futures_lite::future::yield_now().await;
            // GPUI may retain shaped text independently of row resources.
            // Keep only live weak probes so the test itself cannot retain an
            // allocation for every previously visited row. Bound these cached
            // payloads separately from the exact active selection/view counts.
            previous_text.retain(|weak| weak.strong_count() != 0);
            peak_cached = peak_cached.max(previous_text.len());
            assert!(
                previous_text.len() <= 2 * ACTIVE as usize,
                "text cache grew with visited history: {}",
                previous_text.len()
            );
            assert!(
                previous_selections
                    .iter()
                    .all(|weak| weak.upgrade().is_none()),
                "old selection retained at row {first}"
            );
            let (current_text, selections): (Vec<_>, Vec<_>) = window
                .update(cx, |view, _, _| {
                    let state = view.lists[&root].borrow();
                    assert_eq!(state.native.index().len(), COUNT as usize);
                    assert_eq!(state.resource_counts(), (current.len(), current.len()));
                    assert_eq!(
                        view.selections.len(),
                        current.len(),
                        "every row must actually render"
                    );
                    assert!(
                        view.buttons.is_empty()
                            && view.editors.is_empty()
                            && view.images.is_empty()
                    );
                    let session = view.session.borrow();
                    let tree = session.tree(view.id).unwrap();
                    assert_eq!(tree.len(), current.len() + 1);
                    (
                        current
                            .iter()
                            .map(|id| Arc::downgrade(&tree.get(*id).unwrap().text))
                            .collect(),
                        view.selections.values().map(Rc::downgrade).collect(),
                    )
                })
                .unwrap();
            previous_text.extend(current_text);
            previous_selections = selections;
            visits += current.len();
            previous_nodes = current;
        }
        eprintln!(
            "LIST_HISTORY pass={} visits={visits} active_cap={ACTIVE} metadata_rows={COUNT}",
            pass + 1
        );
    }
    let mut ops = vec![Op::SetRoot(None), Op::Remove(root)];
    ops.extend(previous_nodes.iter().copied().map(Op::Remove));
    apply(cx, window, ops);
    layout_frames(cx, window);
    futures_lite::future::yield_now().await;
    previous_text.retain(|weak| weak.strong_count() != 0);
    assert!(previous_text.len() <= 2 * ACTIVE as usize);
    eprintln!(
        "LIST_HISTORY peak_cached_payloads={peak_cached} after_unmount={}",
        previous_text.len()
    );
    assert!(
        previous_selections
            .iter()
            .all(|weak| weak.upgrade().is_none())
    );
    assert_eq!(visits, 2 * COUNT as usize);
    eprintln!(
        "GPUIO_NATIVE_LIST_HISTORY_OK: 100k rows visited and revisited, 256 active views/selection caches, evicted row resources released, frame text cache bounded; logical metadata remains O(100k)"
    );
    previous_text
}
pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _read = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let write = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(write.as_raw_fd()).unwrap());
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        gpui_base::init(cx);
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session
            .borrow_mut()
            .open(1, window_id(), "Managed lists", 480., 420.)
            .unwrap();
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(480.), px(420.)),
                        cx,
                    ))),
                    focus: true,
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(window_id(), session.clone(), transport.clone())),
            )
            .unwrap();
        cx.activate(true);
        cx.spawn(async move |cx| {
            let result = super::native_test::protect(async {
                eprintln!("LIST_TEST applying initial source");
                apply(cx, window, initial());
                exercise(cx, window).await;
                let cached_text = history(cx, window).await;
                window
                    .update(cx, |_, window, _| window.remove_window())
                    .unwrap();
                assert!(
                    cached_text.iter().all(|weak| weak.strong_count() == 0),
                    "closed window retained text cache"
                );
            })
            .await;
            // Close on both success and caught failure; a failed frame assertion
            // must not leave a test window on the developer's desktop.
            let _ = window.update(cx, |_, window, _| window.remove_window());
            *task_failure.borrow_mut() = result.err();
            cx.update(super::stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
