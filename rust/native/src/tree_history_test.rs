//! Full tree-row traversal through production GPUI layout/paint, not a mock viewport.
use super::*;
use gpuio_protocol::{
    accessibility::{Config as Metadata, Live, Role, TreeItem},
    list::{Config, IdRun, Order, Row, ScrollPolicy, ScrollRequest, ScrollTarget},
};

const COUNT: i64 = 100_000;
const ACTIVE: usize = 256;
const DEPTH: i64 = 128;

fn dimensions(width: f64, height: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(height)),
        Field::Shrink(0.),
    ])]
}
fn metadata(role: Role, label: String) -> Metadata {
    Metadata {
        role: Some(role),
        label: Some(label),
        description: None,
        live: Live::Off,
        field: None,
        current: None,
    }
}
// Explicit frames isolate cache retention from display-link throttling when the
// window is occluded. Other native tree suites exercise real platform input.
fn paint(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    for _ in 0..3 {
        let arena = cx
            .update_window(handle.into(), |_, window, cx| {
                window.refresh();
                window.draw(cx)
            })
            .unwrap();
        cx.update(|cx| arena.clear(cx));
    }
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
) -> Vec<std::sync::Weak<str>> {
    let transport = handle
        .update(cx, |view, _, _| view.transport.clone())
        .unwrap();
    transport.mailbox.lock().unwrap().drain(128);
    let root = node(20);
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                root,
                Kind::VirtualList,
                String::new(),
                Some(gpuio_protocol::HandlerId::from_parts(30, 1).unwrap()),
            ),
            Op::SetListConfig(
                root,
                Config {
                    estimated_height: 1.,
                    overscan: 0.,
                    max_active: ACTIVE as i64,
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
            Op::SetAccessibility(
                root,
                Some(metadata(Role::Tree(true), "Tree history".into())),
            ),
            Op::SetTreeInput(root, true),
            Op::SetTreeMoves(root, true),
            Op::SetStyle(root, dimensions(380., ACTIVE as f64)),
            Op::SetRoot(Some(root)),
        ],
    );
    let mut previous = Vec::new();
    let mut text = Vec::<std::sync::Weak<str>>::new();
    let mut selections = Vec::<std::rc::Weak<RefCell<crate::selection::State>>>::new();
    let mut serial = 0;
    let mut generations = [0_i64; ACTIVE];
    let mut visits = 0;
    let mut peak_cached = 0;
    for pass in 0..2 {
        for first in (1..=COUNT).step_by(ACTIVE) {
            serial += 1;
            let last = (first + ACTIVE as i64 - 1).min(COUNT);
            let current: Vec<_> = (first..=last)
                .enumerate()
                .map(|(slot, id)| {
                    generations[slot] += 1;
                    (
                        id,
                        NodeId::from_parts(21 + 2 * slot as i64, generations[slot]).unwrap(),
                        NodeId::from_parts(22 + 2 * slot as i64, generations[slot]).unwrap(),
                    )
                })
                .collect();
            let mut operations: Vec<_> = previous.iter().copied().map(Op::Remove).collect();
            for &(id, row, content) in &current {
                let level = (id - 1) % DEPTH + 1;
                let label = format!("Item {id}, depth {level}, pass {pass}");
                let mut text_style = dimensions(370., 1.);
                text_style.push(Style::Fields(vec![
                    Field::UserSelect(true),
                    Field::OverflowY(1),
                ]));
                operations.extend([
                    Op::Create(row, Kind::Container, String::new(), None),
                    Op::SetStyle(row, dimensions(380., 1.)),
                    Op::SetAccessibility(
                        row,
                        Some(metadata(
                            Role::TreeItem(TreeItem {
                                level,
                                index: if level == 1 { (id - 1) / DEPTH } else { 0 },
                                count: Some(if level == 1 {
                                    (COUNT + DEPTH - 1) / DEPTH
                                } else {
                                    1
                                }),
                                expanded: (level < DEPTH && id < COUNT).then_some(true),
                                selected: true,
                                disabled: false,
                                busy: false,
                            }),
                            label.clone(),
                        )),
                    ),
                    Op::Create(
                        content,
                        Kind::Text,
                        format!("{label}: {}", "payload ".repeat(32)),
                        None,
                    ),
                    Op::SetStyle(content, text_style),
                    Op::Splice(row, 0, 0, vec![content]),
                ]);
            }
            operations.extend([
                Op::Splice(
                    root,
                    0,
                    (previous.len() / 2) as i64,
                    current.iter().map(|(_, row, _)| *row).collect(),
                ),
                Op::SetListRows(
                    root,
                    current
                        .iter()
                        .map(|(id, row, _)| Row {
                            id: *id,
                            node: *row,
                        })
                        .collect(),
                ),
                Op::SetStyle(root, dimensions(380., current.len() as f64)),
                Op::ScrollList(
                    root,
                    ScrollRequest {
                        serial,
                        target: ScrollTarget::Offset(first, 0.),
                    },
                ),
            ]);
            handle
                .update(cx, |view, _, _| view.probes.borrow_mut().clear())
                .unwrap();
            apply(cx, handle, operations);
            paint(cx, handle);
            // Emulate the OCaml event consumer between batches. This test does
            // not intentionally saturate the bounded viewport/frame mailbox.
            transport.mailbox.lock().unwrap().drain(128);
            futures_lite::future::yield_now().await;
            text.retain(|weak| weak.strong_count() != 0);
            peak_cached = peak_cached.max(text.len());
            assert!(
                text.len() <= 2 * ACTIVE,
                "tree text cache grows with history: {}",
                text.len()
            );
            assert!(
                selections.iter().all(|weak| weak.upgrade().is_none()),
                "evicted selection retained at row {first}"
            );
            let (payloads, current_selections): (Vec<_>, Vec<_>) = handle
                .update(cx, |view, _, _| {
                    let state = view.lists[&root].borrow();
                    assert_eq!(state.native.index().len(), COUNT as usize);
                    assert_eq!(state.resource_counts(), (current.len(), current.len()));
                    assert!(state.tree_focus.is_some());
                    assert_eq!(state.pending_focus_row(), None);
                    assert!(
                        state.observed.as_ref().unwrap().pinned.is_empty(),
                        "selected rows must not create focus pins"
                    );
                    assert_eq!(
                        view.selections.len(),
                        current.len(),
                        "every child text must actually paint"
                    );
                    assert!(view.tree_drag.upgrade().is_none());
                    assert!(
                        view.editors.is_empty()
                            && view.buttons.is_empty()
                            && view.images.is_empty()
                    );
                    let session = view.session.borrow();
                    let tree = session.tree(view.id).unwrap();
                    assert_eq!(tree.len(), 1 + 2 * current.len());
                    (
                        current
                            .iter()
                            .map(|(_, _, content)| {
                                Arc::downgrade(&tree.get(*content).unwrap().text)
                            })
                            .collect(),
                        view.selections.values().map(Rc::downgrade).collect(),
                    )
                })
                .unwrap();
            text.extend(payloads);
            selections = current_selections;
            previous = current
                .iter()
                .flat_map(|(_, row, content)| [*row, *content])
                .collect();
            visits += current.len();
            if first == 1 || first % (ACTIVE as i64 * 100) == 1 {
                eprintln!(
                    "TREE_HISTORY pass={} visited={visits} active_cap={ACTIVE} max_depth={DEPTH}",
                    pass + 1
                );
            }
        }
    }
    let weak_list = handle
        .update(cx, |view, _, _| Rc::downgrade(&view.lists[&root]))
        .unwrap();
    let mut operations = vec![Op::SetRoot(None), Op::Remove(root)];
    operations.extend(previous.into_iter().map(Op::Remove));
    apply(cx, handle, operations);
    paint(cx, handle);
    futures_lite::future::yield_now().await;
    assert!(
        weak_list.upgrade().is_none(),
        "tree list state retained after unmount"
    );
    assert!(selections.iter().all(|weak| weak.upgrade().is_none()));
    text.retain(|weak| weak.strong_count() != 0);
    assert!(text.len() <= 2 * ACTIVE);
    handle
        .update(cx, |view, _, _| {
            assert!(view.lists.is_empty() && view.selections.is_empty());
            assert!(view.session.borrow().tree(view.id).unwrap().is_empty());
        })
        .unwrap();
    assert_eq!(visits, 2 * COUNT as usize);
    eprintln!(
        "GPUIO_TREE_HISTORY_OK: visits={visits} logical_rows={COUNT} max_depth={DEPTH} active_rows={ACTIVE} peak_cached_payloads={peak_cached} after_unmount_cached={}; evicted selections and list state released",
        text.len()
    );
    text
}
