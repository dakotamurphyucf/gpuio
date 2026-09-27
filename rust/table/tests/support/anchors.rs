//! Keyed native viewport position, tested after actual layout/paint.
use super::events::{bounds, column_bounds, draw};
use super::*;

fn positions(delegate: &mut Delegate) {
    delegate.positions = delegate
        .keys
        .iter()
        .enumerate()
        .map(|(index, key)| (*key, index))
        .collect();
}
fn same_pixel(actual: Pixels, expected: Pixels, what: &str) {
    assert!(
        (actual - expected).abs() < px(0.1),
        "{what}: {actual:?} != {expected:?}"
    );
}

pub(super) fn exercise(cx: &mut AsyncApp, handle: WindowHandle<Probe>) {
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                table.update_source(cx, |delegate| {
                    delegate.keys = (1..=100_000).collect();
                    positions(delegate);
                    delegate.columns = (0..64)
                        .map(|index| {
                            let column =
                                Column::new(format!("anchor-{index}"), format!("Column {index}"))
                                    .width(px(160.));
                            if index < 2 {
                                column.fixed_left()
                            } else {
                                column
                            }
                        })
                        .collect();
                });
                table.replace_selection(Selection::Empty, cx);
                table.scroll_to_row(12_345, cx);
                table.scroll_to_col(12, cx);
            })
        })
        .unwrap();
    draw(cx, handle);
    // Keep seven pixels of the top row and seventeen pixels of the first
    // scrolling column clipped, to test more than integer-index restoration.
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                table
                    .vertical_scroll_handle
                    .0
                    .borrow()
                    .base_handle
                    .set_offset(point(px(0.), -px(32. * 12_345. + 7.)));
                table
                    .horizontal_scroll_handle
                    .set_offset(point(-px(160. * 10. + 17.), px(0.)));
                cx.notify();
            })
        })
        .unwrap();
    draw(cx, handle);
    let before_row = bounds(cx, handle, ("row", 12_345usize));
    let before_col = column_bounds(cx, handle, 12);
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                table.update_source(cx, |delegate| {
                    delegate.keys.reverse();
                    positions(delegate);
                    delegate.columns[2..].reverse();
                });
            })
        })
        .unwrap();
    draw(cx, handle);
    let after_row = bounds(cx, handle, ("row", 87_654usize));
    let after_col = column_bounds(cx, handle, 53);
    same_pixel(
        after_row.origin.y,
        before_row.origin.y,
        "row anchor through 100k reversal",
    );
    same_pixel(
        after_col.origin.x,
        before_col.origin.x,
        "column anchor through reorder",
    );
    handle
        .update(cx, |view, _, cx| {
            assert_eq!(view.table.read(cx).delegate().keys[87_654], 12_346);
            assert_eq!(view.table.read(cx).delegate().columns[53].key, "anchor-12");
        })
        .unwrap();

    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                table.update_source(cx, |delegate| {
                    delegate.keys.splice(0..0, 100_001..=100_100);
                    positions(delegate);
                });
            })
        })
        .unwrap();
    draw(cx, handle);
    same_pixel(
        bounds(cx, handle, ("row", 87_754usize)).origin.y,
        before_row.origin.y,
        "prepend page anchor",
    );
    same_pixel(
        column_bounds(cx, handle, 53).origin.x,
        before_col.origin.x,
        "unchanged columns on prepend",
    );

    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                table.update_source(cx, |delegate| {
                    delegate.columns.retain(|column| column.key != "anchor-12");
                });
            })
        })
        .unwrap();
    draw(cx, handle);
    same_pixel(
        column_bounds(cx, handle, 53).origin.x,
        before_col.origin.x,
        "removed column anchor fallback",
    );
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                table.scroll_to_col(0, cx);
            })
        })
        .unwrap();
    draw(cx, handle);
    same_pixel(
        column_bounds(cx, handle, 53).origin.x,
        before_col.origin.x,
        "pinned column reveal leaves horizontal scroll alone",
    );

    // A pending command follows its target key through the next replacement,
    // instead of overriding the anchor with a stale positional target.
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                let target = table.delegate().positions[&50_001];
                table.scroll_to_row(target, cx);
                table.update_source(cx, |delegate| {
                    delegate.keys.reverse();
                    positions(delegate);
                });
            })
        })
        .unwrap();
    draw(cx, handle);
    let commanded = handle
        .update(cx, |view, _, cx| {
            view.table.read(cx).delegate().positions[&50_001]
        })
        .unwrap();
    let commanded_y = bounds(cx, handle, ("row", commanded)).origin.y;
    same_pixel(
        commanded_y,
        before_row.origin.y + px(7.),
        "pending keyed command wins over painted anchor",
    );
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                table.update_source(cx, |delegate| {
                    delegate.keys.reverse();
                    positions(delegate);
                });
                table.scroll_to_row(1000, cx);
            })
        })
        .unwrap();
    draw(cx, handle);
    same_pixel(
        bounds(cx, handle, ("row", 1000usize)).origin.y,
        commanded_y,
        "new explicit command wins",
    );

    // Removal of a pending target cancels it. Removing the painted anchor has
    // a separately documented nearest-position fallback.
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                let target = table.delegate().keys[2000];
                table.scroll_to_row(2000, cx);
                table.update_source(cx, |delegate| {
                    delegate.keys.retain(|key| *key != target);
                    positions(delegate);
                });
                assert!(
                    table
                        .vertical_scroll_handle
                        .0
                        .borrow()
                        .deferred_scroll_to_item
                        .is_none()
                );
            })
        })
        .unwrap();
    draw(cx, handle);
    same_pixel(
        bounds(cx, handle, ("row", 1000usize)).origin.y,
        commanded_y,
        "removed command retains painted anchor",
    );
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                table.update_source(cx, |delegate| {
                    delegate.keys.remove(1000);
                    positions(delegate);
                });
            })
        })
        .unwrap();
    draw(cx, handle);
    same_pixel(
        bounds(cx, handle, ("row", 1000usize)).origin.y,
        commanded_y,
        "removed anchor falls back to old position",
    );
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                table.update_source(cx, |delegate| {
                    delegate.keys.clear();
                    delegate.positions.clear();
                    delegate.columns.clear();
                });
            })
        })
        .unwrap();
    draw(cx, handle);
    handle
        .update(cx, |view, _, cx| {
            let table = view.table.read(cx);
            assert_eq!(
                table.vertical_scroll_handle.0.borrow().base_handle.offset(),
                point(px(0.), px(0.))
            );
            assert_eq!(
                table.horizontal_scroll_handle.offset(),
                point(px(0.), px(0.))
            );
        })
        .unwrap();
    eprintln!(
        "TABLE_ANCHORS_OK: 100k row/column reversal at fractional offsets, prepend page, pending/new command precedence, removal fallback, empty reset"
    );
}
