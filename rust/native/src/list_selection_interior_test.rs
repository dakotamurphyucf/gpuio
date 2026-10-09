//! Interior materialization may change while surviving geometric endpoints remain.
use super::*;

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    interior: NodeId,
) -> NodeId {
    let mut style = dimensions(400., 100.);
    style.push(Style::Fields(vec![Field::UserSelect(true)]));
    apply(
        cx,
        window,
        vec![
            Op::SetStyle(node(0), dimensions(420., 340.)),
            Op::SetStyle(node(5), style.clone()),
        ],
    );
    frame(cx, window).await;
    select(cx, window, node(5), "Row 5".len()).await;
    assert_eq!(
        copy(cx, window),
        "Row 3\nnew Row 4 β\nRow 5",
        "three materialized rows are selected"
    );
    let (children, rows, owner, order) = window
        .update(cx, |view, w, cx| {
            let pins = view.list_pins(w, cx);
            let pinned = pins.iter().find(|pin| pin.node == node(0)).unwrap();
            assert!(pinned.rows.contains(&3));
            assert!(
                !pinned.rows.contains(&4),
                "historical interior range is not a permanent admission pin"
            );
            let (base, children, rows, order) = {
                let session = view.session.borrow();
                let tree = session.tree(view.id).unwrap();
                let list = tree.get(node(0)).unwrap();
                (
                    tree.revision(),
                    list.children.to_vec(),
                    list.list_rows.to_vec(),
                    list.list_order.clone().unwrap(),
                )
            };
            let owner = Rc::downgrade(&view.selections[&interior]);
            let tx = Transaction {
                window: view.id,
                base,
                revision: base + 1,
                operations: vec![
                    Op::Splice(
                        node(0),
                        0,
                        children.len() as i64,
                        children
                            .iter()
                            .copied()
                            .filter(|id| *id != interior)
                            .collect(),
                    ),
                    Op::SetListRows(
                        node(0),
                        rows.iter().filter(|row| row.id != 4).cloned().collect(),
                    ),
                    Op::Remove(interior),
                ],
            };
            let applied = view.session.borrow_mut().apply_guarded(&tx, &pins).unwrap();
            view.update_editors(&applied.dirty, w, cx);
            view.list_actions(&applied.lists, w, cx);
            cx.notify();
            (children, rows, owner, order)
        })
        .unwrap();
    frame(cx, window).await;
    assert!(
        owner.upgrade().is_none(),
        "evicted interior owner releases despite surviving selection"
    );
    window
        .update(cx, |view, _, _| {
            let session = view.session.borrow();
            let list = session.tree(view.id).unwrap().get(node(0)).unwrap();
            assert!(
                Arc::ptr_eq(list.list_order.as_ref().unwrap(), &order),
                "only materialization changed; logical row 4 remains"
            );
        })
        .unwrap();
    assert_eq!(
        copy(cx, window),
        "Row 3\nRow 5",
        "live endpoints survive; Copy does not retain evicted payloads"
    );
    let recycled = NodeId::from_parts(4, 3).unwrap();
    apply(
        cx,
        window,
        vec![
            Op::Create(recycled, Kind::Text, "returning Row 4 λ".into(), None),
            Op::SetStyle(recycled, style),
            Op::Splice(
                node(0),
                0,
                children.len() as i64 - 1,
                children
                    .into_iter()
                    .map(|id| if id == interior { recycled } else { id })
                    .collect(),
            ),
            Op::SetListRows(
                node(0),
                rows.into_iter()
                    .map(|row| {
                        if row.id == 4 {
                            Row {
                                id: 4,
                                node: recycled,
                            }
                        } else {
                            row
                        }
                    })
                    .collect(),
            ),
        ],
    );
    frame(cx, window).await;
    assert_eq!(
        copy(cx, window),
        "Row 3\nreturning Row 4 λ\nRow 5",
        "new interior joins retained endpoints using current generation/payload"
    );
    apply(
        cx,
        window,
        vec![Op::SetText(recycled, "changed interior ζ".into())],
    );
    frame(cx, window).await;
    assert_eq!(
        copy(cx, window),
        "unchanged",
        "changing selected interior source retires shared geometry"
    );
    select(cx, window, node(5), "Row 5".len()).await;
    assert_eq!(
        copy(cx, window),
        "Row 3\nchanged interior ζ\nRow 5",
        "fresh selection after source change contains current text"
    );
    eprintln!(
        "GPUIO_LIST_SELECTION_INTERIOR_OK: guarded unpinned interior eviction, unchanged logical order, weak release, surviving endpoint Copy, generation reuse with fresh payload and source-change retirement"
    );
    recycled
}
