//! Cross-row Copy and legal endpoint eviction through managed-list admission.
use super::*;
#[path = "list_selection_interior_test.rs"]
mod interior;
use crate::host::{
    editor_test::key,
    native_test::{mouse, move_mouse},
};

fn copy(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) -> String {
    cx.update(|cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string("unchanged".into())));
    key(cx, window, "secondary-c");
    cx.update(|cx| {
        cx.read_from_clipboard()
            .and_then(|item| item.text())
            .unwrap()
    })
}
async fn select(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, last: NodeId, end: usize) {
    let (start, end) = window
        .update(cx, |view, _, _| {
            (
                view.selections[&node(3)].borrow().point(0),
                view.selections[&last].borrow().point(end),
            )
        })
        .unwrap();
    move_mouse(cx, window, start, false);
    mouse(cx, window, start, true);
    move_mouse(cx, window, end, true);
    mouse(cx, window, end, false);
    frame(cx, window).await;
}
pub(super) async fn exercise(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) -> NodeId {
    let saved = cx.update(|cx| cx.read_from_clipboard());
    let mut style = dimensions(400., 100.);
    style.push(Style::Fields(vec![Field::UserSelect(true)]));
    apply(
        cx,
        window,
        vec![
            Op::SetStyle(node(4), style.clone()),
            Op::ScrollList(
                node(0),
                ScrollRequest {
                    serial: 9,
                    target: ScrollTarget::Offset(3, 0.),
                },
            ),
        ],
    );
    frame(cx, window).await;
    select(cx, window, node(4), "Row 4".len()).await;
    assert_eq!(
        copy(cx, window),
        "Row 3\nRow 4",
        "cross-row Copy before virtualization"
    );
    let (children, rows, owner) = window
        .update(cx, |view, w, cx| {
            let pins = view.list_pins(w, cx);
            let pinned = pins.iter().find(|pin| pin.node == node(0)).unwrap();
            assert!(pinned.rows.contains(&3), "focused anchor stays pinned");
            assert!(
                !pinned.rows.contains(&4),
                "historical unfocused endpoint does not pin forever"
            );
            let (base, children, rows) = {
                let session = view.session.borrow();
                let tree = session.tree(view.id).unwrap();
                let list = tree.get(node(0)).unwrap();
                (
                    tree.revision(),
                    list.children.to_vec(),
                    list.list_rows.to_vec(),
                )
            };
            let owner = Rc::downgrade(&view.selections[&node(4)]);
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
                            .filter(|id| *id != node(4))
                            .collect(),
                    ),
                    Op::SetListRows(
                        node(0),
                        rows.iter().filter(|row| row.id != 4).cloned().collect(),
                    ),
                    Op::Remove(node(4)),
                ],
            };
            let applied = view.session.borrow_mut().apply_guarded(&tx, &pins).unwrap();
            view.update_editors(&applied.dirty, w, cx);
            view.list_actions(&applied.lists, w, cx);
            cx.notify();
            (children, rows, owner)
        })
        .unwrap();
    frame(cx, window).await;
    assert!(
        owner.upgrade().is_none(),
        "evicted endpoint releases native selection owner"
    );
    assert_eq!(
        copy(cx, window),
        "unchanged",
        "eviction must not leave a partial stale Copy"
    );
    let recycled = NodeId::from_parts(4, 2).unwrap();
    let current_count = children.len() - 1;
    apply(
        cx,
        window,
        vec![
            Op::Create(recycled, Kind::Text, "new Row 4 β".into(), None),
            Op::SetStyle(recycled, style),
            Op::Splice(
                node(0),
                0,
                current_count as i64,
                children
                    .into_iter()
                    .map(|id| if id == node(4) { recycled } else { id })
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
        "unchanged",
        "rematerializing the same logical row cannot resurrect selection"
    );
    select(cx, window, recycled, "new Row 4 β".len()).await;
    assert_eq!(
        copy(cx, window),
        "Row 3\nnew Row 4 β",
        "fresh gesture uses the new node generation"
    );
    let recycled = interior::exercise(cx, window, recycled).await;
    window
        .update(cx, |_, w, cx| gpui_base::TextSelection::clear(w, cx))
        .unwrap();
    cx.update(|cx| {
        cx.write_to_clipboard(
            saved.unwrap_or_else(|| gpui::ClipboardItem::new_string(String::new())),
        )
    });
    eprintln!(
        "GPUIO_LIST_SELECTION_LIFECYCLE_OK: cross-row Copy, focused anchor admission pin, legal unfocused endpoint eviction, owner release, no stale Copy/revival and fresh node generation"
    );
    recycled
}
