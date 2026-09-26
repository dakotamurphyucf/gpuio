//! Native dispatch regression: stale frames/drags cannot target replacement data.
use super::*;
use gpuio_table_adapter::table::ColumnSort;

pub(super) fn draw(cx: &mut AsyncApp, handle: WindowHandle<Probe>) {
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
pub(super) fn bounds(
    cx: &mut AsyncApp,
    handle: WindowHandle<Probe>,
    id: impl Into<ElementId>,
) -> Bounds<Pixels> {
    handle
        .update(cx, |_, window, _| {
            let id = id.into();
            gpui_base::test_support::find(window, &[], &id)
                .unwrap_or_else(|| {
                    panic!(
                        "missing painted element {id:?}; observed: {}",
                        gpui_base::test_support::registered_paths(window)
                    )
                })
                .bounds()
        })
        .unwrap()
}
pub(super) fn column_bounds(
    cx: &mut AsyncApp,
    handle: WindowHandle<Probe>,
    index: usize,
) -> Bounds<Pixels> {
    let key = handle
        .update(cx, |view, _, cx| {
            view.table.read(cx).delegate().columns[index].key.clone()
        })
        .unwrap();
    bounds(cx, handle, format!("col-header:{key}"))
}

fn mouse(
    window: &mut Window,
    cx: &mut App,
    position: Point<Pixels>,
    button: MouseButton,
    down: bool,
    count: usize,
) {
    let event = if down {
        PlatformInput::MouseDown(MouseDownEvent {
            position,
            button,
            modifiers: Default::default(),
            click_count: count,
            first_mouse: false,
        })
    } else {
        PlatformInput::MouseUp(MouseUpEvent {
            position,
            button,
            modifiers: Default::default(),
            click_count: count,
        })
    };
    window.dispatch_event(event, cx);
}
fn click(
    cx: &mut AsyncApp,
    handle: WindowHandle<Probe>,
    position: Point<Pixels>,
    button: MouseButton,
    count: usize,
) {
    handle
        .update(cx, |_, window, cx| {
            mouse(window, cx, position, button, true, count);
            mouse(window, cx, position, button, false, count);
        })
        .unwrap();
}
fn drag_move(cx: &mut AsyncApp, handle: WindowHandle<Probe>, position: Point<Pixels>) {
    handle
        .update(cx, |_, window, cx| {
            window.dispatch_event(
                PlatformInput::MouseMove(MouseMoveEvent {
                    position,
                    pressed_button: Some(MouseButton::Left),
                    modifiers: Default::default(),
                }),
                cx,
            );
        })
        .unwrap();
}
fn reverse(table: &mut TableState<Delegate>, cx: &mut Context<TableState<Delegate>>) {
    let delegate = table.delegate_mut();
    delegate.keys.reverse();
    delegate.positions = delegate
        .keys
        .iter()
        .enumerate()
        .map(|(index, key)| (*key, index))
        .collect();
    delegate.columns[2..].reverse();
    table.refresh(cx);
    table.replace_selection(Selection::Empty, cx);
    table.scroll_to_row(0, cx);
    table.scroll_to_col(2, cx);
}

pub(super) fn exercise(cx: &mut AsyncApp, handle: WindowHandle<Probe>) {
    let events = Rc::new(RefCell::new(Vec::<TableEvent>::new()));
    let observed = events.clone();
    let _subscription = handle
        .update(cx, |view, _, cx| {
            cx.subscribe(&view.table, move |_, _, event, _| {
                observed.borrow_mut().push(event.clone())
            })
        })
        .unwrap();
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                let delegate = table.delegate_mut();
                delegate.keys = (1..=200).collect();
                delegate.positions = delegate
                    .keys
                    .iter()
                    .enumerate()
                    .map(|(index, key)| (*key, index))
                    .collect();
                delegate.columns = (0..8)
                    .map(|index| {
                        let column =
                            Column::new(format!("event-{index}"), format!("Column {index}"))
                                .width(px(120.))
                                .sortable();
                        if index < 2 {
                            column.fixed_left()
                        } else {
                            column
                        }
                    })
                    .collect();
                table.refresh(cx);
                table.replace_selection(Selection::Empty, cx);
                table.scroll_to_row(0, cx);
                table.scroll_to_col(2, cx);
            })
        })
        .unwrap();
    draw(cx, handle);
    let row = bounds(cx, handle, ("row", 2usize));
    let column = column_bounds(cx, handle, 0);
    let position = point(column.center().x, row.center().y);
    click(cx, handle, position, MouseButton::Left, 1);
    assert!(
        events
            .borrow()
            .contains(&TableEvent::SelectCell(RowKey(3), "event-0".into())),
        "fresh cell click: {:?}",
        events.borrow()
    );
    let captured = events.borrow().clone();
    events.borrow_mut().clear();
    // No draw between replacement and click: invoke exactly the old frame's listeners.
    handle
        .update(cx, |view, window, cx| {
            view.table.update(cx, reverse);
            mouse(window, cx, position, MouseButton::Left, true, 2);
            mouse(window, cx, position, MouseButton::Left, false, 2);
            mouse(window, cx, position, MouseButton::Right, true, 1);
            mouse(window, cx, position, MouseButton::Right, false, 1);
            assert_eq!(view.table.read(cx).selection(), &Selection::Empty);
        })
        .unwrap();
    assert!(
        events.borrow().is_empty(),
        "obsolete layout emitted {:?}",
        events.borrow()
    );
    assert!(
        captured.contains(&TableEvent::SelectCell(RowKey(3), "event-0".into())),
        "queued events must keep original identity"
    );
    draw(cx, handle);
    let row = bounds(cx, handle, ("row", 2usize));
    let column = column_bounds(cx, handle, 0);
    let position = point(column.center().x, row.center().y);
    click(cx, handle, position, MouseButton::Left, 2);
    click(cx, handle, position, MouseButton::Right, 1);
    assert!(
        events
            .borrow()
            .contains(&TableEvent::ActivatedCell(RowKey(198), "event-0".into())),
        "double click: {:?}",
        events.borrow()
    );
    assert!(
        events
            .borrow()
            .contains(&TableEvent::RightClickedCell(RowKey(198), "event-0".into())),
        "context click: {:?}",
        events.borrow()
    );
    draw(cx, handle);
    events.borrow_mut().clear();
    let sort = bounds(cx, handle, ("icon-sort", 2usize)).center();
    click(cx, handle, sort, MouseButton::Left, 1);
    assert!(
        events.borrow().contains(&TableEvent::SortRequested(
            "event-7".into(),
            ColumnSort::Descending
        )),
        "sort request: {:?}",
        events.borrow()
    );
    // Sort is a request, never an implicit sort of the loaded subset.
    handle
        .update(cx, |view, _, cx| {
            assert_eq!(view.table.read(cx).delegate().keys[0], 200)
        })
        .unwrap();
    // Reject an optimistic sort using the same retained schema value.
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| table.reset_columns(cx))
        })
        .unwrap();
    draw(cx, handle);
    events.borrow_mut().clear();
    let sort = bounds(cx, handle, ("icon-sort", 2usize)).center();
    click(cx, handle, sort, MouseButton::Left, 1);
    assert!(
        events.borrow().contains(&TableEvent::SortRequested(
            "event-7".into(),
            ColumnSort::Descending
        )),
        "explicit reset must clear optimistic sort"
    );
    draw(cx, handle);
    events.borrow_mut().clear();
    let resize = bounds(cx, handle, ("resizable-handle", 2usize)).center();
    handle
        .update(cx, |_, window, cx| {
            mouse(window, cx, resize, MouseButton::Left, true, 1)
        })
        .unwrap();
    drag_move(cx, handle, resize + point(px(25.), px(0.)));
    cx.update(|cx| {
        assert!(
            cx.has_active_drag(),
            "resize fixture must start a native drag"
        )
    });
    // Replace schema while an old drag is alive, then build fresh listeners.
    handle
        .update(cx, |view, _, cx| view.table.update(cx, reverse))
        .unwrap();
    draw(cx, handle);
    drag_move(cx, handle, resize + point(px(55.), px(0.)));
    handle
        .update(cx, |_, window, cx| {
            mouse(window, cx, resize, MouseButton::Left, false, 1)
        })
        .unwrap();
    assert!(
        !events
            .borrow()
            .iter()
            .any(|event| matches!(event, TableEvent::ColumnWidthsChanged(_))),
        "retired resize emitted {:?}",
        events.borrow()
    );
    draw(cx, handle);
    events.borrow_mut().clear();
    let resize = bounds(cx, handle, ("resizable-handle", 2usize)).center();
    handle
        .update(cx, |_, window, cx| {
            mouse(window, cx, resize, MouseButton::Left, true, 1)
        })
        .unwrap();
    drag_move(cx, handle, resize + point(px(25.), px(0.)));
    drag_move(cx, handle, resize + point(px(35.), px(0.)));

    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                table.update_source(cx, |delegate| {
                    delegate.keys.push(201);
                    delegate.positions.insert(201, 200);
                });
            })
        })
        .unwrap();
    draw(cx, handle);
    assert!(
        column_bounds(cx, handle, 2).size.width > px(120.),
        "row page must preserve preview width"
    );

    drag_move(cx, handle, resize + point(px(45.), px(0.)));
    handle
        .update(cx, |_, window, cx| {
            mouse(
                window,
                cx,
                resize + point(px(45.), px(0.)),
                MouseButton::Left,
                false,
                1,
            )
        })
        .unwrap();
    assert!(events.borrow().iter().any(|event| matches!(event, TableEvent::ColumnWidthsChanged(widths) if widths.len() == 8 && widths[2].0 == "event-2" && widths[2].1 > px(120.))), "fresh keyed resize: {:?}", events.borrow());
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                table.update_source(cx, |delegate| {
                    assert_eq!(delegate.keys.pop(), Some(201));
                    delegate.positions.remove(&201);
                });
            })
        })
        .unwrap();
    draw(cx, handle);
    assert!(column_bounds(cx, handle, 2).size.width > px(120.));
    events.borrow_mut().clear();
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| table.reset_columns(cx))
        })
        .unwrap();
    draw(cx, handle);
    assert_eq!(column_bounds(cx, handle, 2).size.width, px(120.));
    assert!(
        events.borrow().is_empty(),
        "explicit schema reconciliation must not echo input events"
    );
    draw(cx, handle);
    events.borrow_mut().clear();
    let source = column_bounds(cx, handle, 2).center();
    let target = column_bounds(cx, handle, 3);
    let destination = point(target.right() - px(8.), target.center().y);
    handle
        .update(cx, |_, window, cx| {
            mouse(window, cx, source, MouseButton::Left, true, 1)
        })
        .unwrap();
    drag_move(cx, handle, source + point(px(20.), px(0.)));
    cx.update(|cx| {
        assert!(
            cx.has_active_drag(),
            "reorder fixture must start a native drag"
        )
    });
    handle
        .update(cx, |view, _, cx| view.table.update(cx, reverse))
        .unwrap();
    draw(cx, handle);
    drag_move(cx, handle, destination);
    handle
        .update(cx, |_, window, cx| {
            mouse(window, cx, destination, MouseButton::Left, false, 1)
        })
        .unwrap();
    assert!(
        !events
            .borrow()
            .iter()
            .any(|event| matches!(event, TableEvent::MoveColumn { .. })),
        "retired reorder emitted {:?}",
        events.borrow()
    );
    draw(cx, handle);
    events.borrow_mut().clear();
    let (moved, before) = handle
        .update(cx, |view, _, cx| {
            let table = view.table.read(cx);
            (
                table.delegate().columns[2].key.clone(),
                table.delegate().columns[4].key.clone(),
            )
        })
        .unwrap();
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                assert!(table.replace_selection(
                    Selection::Cell {
                        row: RowKey(198),
                        column: moved.clone()
                    },
                    cx
                ));
            })
        })
        .unwrap();
    let source = column_bounds(cx, handle, 2).center();
    let target = column_bounds(cx, handle, 3);
    let destination = point(target.right() - px(8.), target.center().y);
    handle
        .update(cx, |_, window, cx| {
            mouse(window, cx, source, MouseButton::Left, true, 1)
        })
        .unwrap();
    drag_move(cx, handle, source + point(px(20.), px(0.)));
    handle
        .update(cx, |view, _, cx| {
            view.table.update(cx, |table, cx| {
                table.update_source(cx, |delegate| {
                    delegate.keys.push(201);
                    delegate.positions.insert(201, 200);
                });
            })
        })
        .unwrap();
    draw(cx, handle);

    drag_move(cx, handle, destination);
    handle
        .update(cx, |_, window, cx| {
            mouse(window, cx, destination, MouseButton::Left, false, 1)
        })
        .unwrap();
    assert!(
        events.borrow().contains(&TableEvent::MoveColumn {
            column: moved.clone(),
            before: Some(before)
        }),
        "fresh keyed reorder: {:?}",
        events.borrow()
    );
    handle
        .update(cx, |view, _, cx| {
            let table = view.table.read(cx);
            assert_eq!(table.delegate().columns[3].key, moved);
            assert_eq!(table.selected_cell(), Some((2, 3)));
        })
        .unwrap();
    eprintln!(
        "TABLE_EVENT_IDENTITY_OK: fresh pointer/double/context/sort, stale frame suppression, retired resize/reorder, keyed widths/moves, row-page arrival preserves active resize/reorder and preview, selection follows moved column"
    );
}
