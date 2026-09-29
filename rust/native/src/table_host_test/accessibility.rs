//! Actual NSAccessibility objects, including actions queued by the Cocoa adapter.
use super::*;
use objc2::{
    msg_send,
    rc::Retained,
    runtime::{AnyObject, Bool},
    sel,
};
use objc2_foundation::{NSRange, NSString};

#[derive(Debug)]
struct Node {
    object: Retained<AnyObject>,
    role: String,
    label: String,
    value: Option<String>,
    row: Option<usize>,
    column: Option<usize>,
    selected: bool,
    focused: bool,
    settable: bool,
}

fn nodes(cx: &mut gpui::AsyncApp, handle: gpui::WindowHandle<View>) -> Vec<Node> {
    unsafe fn visit(object: *mut AnyObject, output: &mut Vec<Node>, depth: usize) {
        assert!(depth <= 40, "bounded AX depth");
        if object.is_null() {
            return;
        }
        unsafe {
            let role: *mut NSString = msg_send![object, accessibilityRole];
            if !role.is_null() {
                let role = (*role).to_string();
                if ["AXTable", "AXRow", "AXCell", "AXButton"].contains(&role.as_str()) {
                    let title: *mut NSString = msg_send![object, accessibilityTitle];
                    let value: *mut NSString = msg_send![object, accessibilityValue];
                    let value = (!value.is_null()).then(|| (*value).to_string());
                    let row_allowed: Bool = msg_send![object, isAccessibilitySelectorAllowed:sel!(accessibilityRowIndexRange)];
                    let column_allowed: Bool = msg_send![object, isAccessibilitySelectorAllowed:sel!(accessibilityColumnIndexRange)];
                    let row = row_allowed.as_bool().then(|| {
                        let range: NSRange = msg_send![object, accessibilityRowIndexRange];
                        assert_eq!(range.length, 1);
                        range.location
                    });
                    let column = column_allowed.as_bool().then(|| {
                        let range: NSRange = msg_send![object, accessibilityColumnIndexRange];
                        assert_eq!(range.length, 1);
                        range.location
                    });
                    let selected: Bool = msg_send![object, isAccessibilitySelected];
                    let focused: Bool = msg_send![object, isAccessibilityFocused];
                    let settable: Bool = msg_send![object, isAccessibilitySelectorAllowed:sel!(setAccessibilitySelected:)];
                    output.push(Node {
                        object: Retained::retain(object).unwrap(),
                        role,
                        label: if title.is_null() {
                            String::new()
                        } else {
                            (*title).to_string()
                        },
                        value,
                        row,
                        column,
                        selected: selected.as_bool(),
                        focused: focused.as_bool(),
                        settable: settable.as_bool(),
                    });
                }
            }
            let children: *mut AnyObject = msg_send![object, accessibilityChildren];
            if children.is_null() {
                return;
            }
            let count: usize = msg_send![children, count];
            assert!(count < 256, "no history-sized AX child array");
            for index in 0..count {
                let child: *mut AnyObject = msg_send![children, objectAtIndex:index];
                visit(child, output, depth + 1);
            }
        }
    }
    let view = super::super::super::editor_test::native_view(cx, handle) as *mut AnyObject;
    let mut output = vec![];
    unsafe {
        let window: *mut AnyObject = msg_send![view, window];
        let content: *mut AnyObject = msg_send![window, contentView];
        visit(content, &mut output, 0);
    }
    output
}
fn cell(nodes: &[Node], row: usize, column: usize) -> &Node {
    nodes
        .iter()
        .find(|node| node.role == "AXCell" && node.row == Some(row) && node.column == Some(column))
        .unwrap_or_else(|| panic!("missing cell ({row}, {column}): {nodes:?}"))
}
fn table(nodes: &[Node]) -> &Node {
    nodes
        .iter()
        .find(|node| node.role == "AXTable" && node.label == "Native table host")
        .unwrap_or_else(|| panic!("missing table: {nodes:?}"))
}
fn header_relationships(nodes: &[Node]) {
    unsafe {
        let root = &*table(nodes).object;
        let headers: *mut AnyObject = msg_send![root, accessibilityColumnHeaderUIElements];
        assert!(!headers.is_null(), "missing column header relationship");
        let count: usize = msg_send![headers, count];
        assert_eq!(count, 2, "only the two painted column headers");
        let group: *mut AnyObject = msg_send![root, accessibilityHeader];
        assert!(!group.is_null(), "missing header container");
        let role: *mut NSString = msg_send![group, accessibilityRole];
        assert_eq!((*role).to_string(), "AXGroup");
        for column in 0..count {
            let header: *mut AnyObject = msg_send![headers, objectAtIndex:column];
            let painted = nodes
                .iter()
                .find(|node| {
                    node.role == "AXCell" && node.row.is_none() && node.column == Some(column)
                })
                .unwrap();
            let same: Bool = msg_send![header, isEqual:&*painted.object];
            assert!(
                same.as_bool(),
                "header must reuse its painted cell identity"
            );
            let mut parent: *mut AnyObject = msg_send![header, accessibilityParent];
            let mut found = false;
            for _ in 0..40 {
                if parent.is_null() {
                    break;
                }
                let same: Bool = msg_send![parent, isEqual:group];
                if same.as_bool() {
                    found = true;
                    break;
                }
                parent = msg_send![parent, accessibilityParent];
            }
            assert!(found, "header group must contain every returned header");
        }
        let row_headers: *mut AnyObject = msg_send![root, accessibilityRowHeaderUIElements];
        assert!(!row_headers.is_null());
        let count: usize = msg_send![row_headers, count];
        assert_eq!(
            count, 0,
            "row selection controls are not semantic row headers"
        );
    }
}

fn retired_headers(node: &Node) {
    unsafe {
        let headers: *mut AnyObject = msg_send![&*node.object, accessibilityColumnHeaderUIElements];
        if !headers.is_null() {
            let count: usize = msg_send![headers, count];
            assert_eq!(count, 0, "retired table exposes old headers");
        }
        let header: *mut AnyObject = msg_send![&*node.object, accessibilityHeader];
        assert!(
            header.is_null(),
            "retired table exposes an old header group"
        );
    }
}
fn press(node: &Node) {
    unsafe {
        let accepted: Bool = msg_send![&*node.object, accessibilityPerformPress];
        assert!(accepted.as_bool(), "AX press rejected: {node:?}");
    }
}
fn selected(node: &Node, value: bool) {
    assert!(node.settable);
    unsafe {
        let _: () = msg_send![&*node.object, setAccessibilitySelected:value];
    }
}
async fn settle(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) {
    cx.background_executor()
        .timer(std::time::Duration::from_millis(25))
        .await;
    frame(cx, window).await;
}
fn requests(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) -> Vec<wire::Request> {
    window
        .update(cx, |view, _, _| {
            view.transport
                .mailbox
                .lock()
                .unwrap()
                .drain(128)
                .into_iter()
                .filter_map(|e| match e {
                    Event::TableInput(_, _, _, _, input) => Some(input.request),
                    _ => None,
                })
                .collect()
        })
        .unwrap()
}

pub(super) async fn exercise(cx: &mut gpui::AsyncApp, window: gpui::WindowHandle<View>) {
    window
        .update(cx, |_, window, cx| {
            window.activate_window();
            cx.activate(true);
        })
        .unwrap();
    // Cocoa lazily enables the accessibility renderer after the first query.
    let _ = nodes(cx, window);
    settle(cx, window).await;
    let initial = nodes(cx, window);
    header_relationships(&initial);
    assert!(
        initial.len() < 70,
        "bounded mounted semantics: {}",
        initial.len()
    );
    unsafe {
        let root = &*table(&initial).object;
        let rows: isize = msg_send![root, accessibilityRowCount];
        let columns: isize = msg_send![root, accessibilityColumnCount];
        assert_eq!((rows, columns), (100_000, 2));
        let row_objects: *mut AnyObject = msg_send![root, accessibilityRows];
        assert!(!row_objects.is_null());
        let count: usize = msg_send![row_objects, count];
        assert!(
            count > 1 && count <= 16,
            "table exposes mounted rows: {count}"
        );
    }
    assert_eq!(cell(&initial, 0, 0).label, "name");
    assert_eq!(
        cell(&initial, 0, 0).value.as_deref(),
        Some("日本語 👨‍👩‍👧‍👦\t\"e\u{301}\"\nnext")
    );
    let name_header = initial
        .iter()
        .find(|n| n.role == "AXCell" && n.row.is_none() && n.column == Some(0))
        .unwrap();
    assert_eq!(name_header.label, "name");
    assert!(name_header.settable);
    requests(cx, window);
    press(cell(&initial, 0, 0));
    settle(cx, window).await;
    assert_eq!(
        requests(cx, window),
        vec![wire::Request::Select(wire::Selection::Cell(
            1,
            "name".into()
        ))]
    );
    let current = nodes(cx, window);
    assert!(cell(&current, 0, 0).selected);
    assert!(
        cell(&current, 0, 0).focused,
        "selected cell exposes composite focus"
    );
    // Repeated press stays a cell selection and never becomes row activation.
    press(cell(&current, 0, 0));
    settle(cx, window).await;
    assert_eq!(
        requests(cx, window),
        vec![wire::Request::Select(wire::Selection::Cell(
            1,
            "name".into()
        ))]
    );
    let current = nodes(cx, window);
    selected(cell(&current, 0, 0), false);
    selected(cell(&current, 0, 0), true);
    selected(cell(&current, 0, 0), false);
    settle(cx, window).await;
    assert_eq!(
        requests(cx, window),
        vec![
            wire::Request::Select(wire::Selection::Empty),
            wire::Request::Select(wire::Selection::Cell(1, "name".into())),
            wire::Request::Select(wire::Selection::Empty),
        ],
        "desired states retain queue order despite unchanged AX snapshot"
    );
    let current = nodes(cx, window);
    let row = current
        .iter()
        .find(|n| n.role == "AXRow" && n.row == Some(1))
        .unwrap();
    selected(row, true);
    settle(cx, window).await;
    assert_eq!(
        requests(cx, window),
        vec![
            wire::Request::Select(wire::Selection::Row(2)),
            wire::Request::Context(wire::Selection::Empty)
        ]
    );
    let current = nodes(cx, window);
    unsafe {
        let rows: *mut AnyObject = msg_send![&*table(&current).object, accessibilitySelectedRows];
        let count: usize = msg_send![rows, count];
        assert_eq!(count, 1);
        let row: *mut AnyObject = msg_send![rows, objectAtIndex:0usize];
        let index: isize = msg_send![row, accessibilityIndex];
        assert_eq!(index, 1);
    }
    // A held mouse down focuses the already-selected row's retained handle.
    // Drawing that intermediate frame must not report the focused row as its
    // own active descendant (GPUI correctly rejects that invalid AX tree).
    let position = window
        .update(cx, |view, _, _| {
            view.probes.borrow()[&node(8)].bounds.center()
        })
        .unwrap();
    super::super::super::native_test::move_mouse(cx, window, position, false);
    super::super::super::native_test::mouse(cx, window, position, true);
    settle(cx, window).await;
    let held = nodes(cx, window);
    assert!(
        held.iter()
            .any(|n| n.role == "AXRow" && n.row == Some(1) && n.focused)
    );
    super::super::super::native_test::mouse(cx, window, position, false);
    settle(cx, window).await;
    requests(cx, window);
    let current = nodes(cx, window);
    unsafe {
        let _: () = msg_send![&*cell(&current, 0, 1).object, setAccessibilityFocused:true];
    }
    settle(cx, window).await;
    assert_eq!(
        requests(cx, window),
        vec![wire::Request::Select(wire::Selection::Cell(
            1,
            "value".into()
        ))]
    );
    let current = nodes(cx, window);
    assert!(
        cell(&current, 0, 1).focused,
        "AX focus follows addressed cell"
    );
    let sort = current
        .iter()
        .find(|n| n.role == "AXButton" && n.label == "Sort name")
        .unwrap();
    press(sort);
    settle(cx, window).await;
    assert_eq!(
        requests(cx, window),
        vec![wire::Request::Sort(
            "name".into(),
            Some(wire::Direction::Descending)
        )]
    );
    let current = nodes(cx, window);
    let header = current
        .iter()
        .find(|n| n.role == "AXCell" && n.row.is_none() && n.column == Some(0))
        .unwrap();
    unsafe {
        let direction: isize = msg_send![&*header.object, accessibilitySortDirection];
        assert_eq!(direction, 2, "native optimistic descending indicator");
    }
    selected(header, true);
    settle(cx, window).await;
    assert_eq!(
        requests(cx, window),
        vec![wire::Request::Select(wire::Selection::Column(
            "name".into()
        ))]
    );
    // Reset optimistic sort through the adapter's explicit application override.
    window
        .update(cx, |view, _, cx| {
            let native = view.tables[&node(0)].borrow().native.clone();
            native.update(cx, |state, cx| state.reset_columns(cx));
        })
        .unwrap();
    settle(cx, window).await;
    // Logical indices are not viewport positions, even before data has arrived.
    window
        .update(cx, |view, _, cx| {
            let native = view.tables[&node(0)].borrow().native.clone();
            native.update(cx, |state, cx| {
                state.replace_selection(Selection::Empty, cx);
                state.scroll_to_row(50_000, cx);
            });
        })
        .unwrap();
    settle(cx, window).await;
    let middle = nodes(cx, window);
    header_relationships(&middle);
    assert!(middle.len() < 70);
    assert!(
        middle
            .iter()
            .any(|n| n.role == "AXRow" && n.row == Some(50_000))
    );
    assert!(middle.iter().filter_map(|n| n.row).all(|r| r > 49_990));
    assert!(!cell(&middle, 50_000, 0).selected);
    assert!(
        cell(&middle, 50_000, 0).value.is_none(),
        "missing data is not a fabricated empty string"
    );
    press(cell(&middle, 50_000, 0));
    settle(cx, window).await;
    assert_eq!(
        requests(cx, window),
        vec![wire::Request::Select(wire::Selection::Cell(
            50_001,
            "name".into()
        ))]
    );
    window
        .update(cx, |view, _, cx| {
            let native = view.tables[&node(0)].borrow().native.clone();
            native.update(cx, |state, cx| {
                state.replace_selection(Selection::Empty, cx);
                state.scroll_to_row(0, cx);
            });
        })
        .unwrap();
    settle(cx, window).await;
    // Pointer suppression must not disable accessibility selection.
    apply(
        cx,
        window,
        vec![Op::SetStyle(
            node(0),
            vec![
                Style::Width(Length::Px(520.)),
                Style::Height(Length::Px(300.)),
                Style::Fields(vec![Field::PointerEvents(false)]),
            ],
        )],
    );
    let current = nodes(cx, window);
    press(cell(&current, 0, 1));
    settle(cx, window).await;
    assert_eq!(
        requests(cx, window),
        vec![wire::Request::Select(wire::Selection::Cell(
            1,
            "value".into()
        ))]
    );
    // Stale painted actions must consult the new hidden/disabled policy.
    let current = nodes(cx, window);
    apply(
        cx,
        window,
        vec![Op::SetStyle(
            node(0),
            vec![Style::Fields(vec![Field::Display(3)])],
        )],
    );
    // Cocoa may already reject the defunct object, or the live route may reject
    // its queued callback. Both paths must preserve application state.
    unsafe {
        let _: Bool = msg_send![&*cell(&current, 0, 0).object, accessibilityPerformPress];
    }
    settle(cx, window).await;
    assert!(requests(cx, window).is_empty());
    assert!(nodes(cx, window).iter().all(|n| n.role != "AXTable"));
    retired_headers(table(&current));
    apply(
        cx,
        window,
        vec![Op::SetStyle(
            node(0),
            vec![
                Style::Width(Length::Px(520.)),
                Style::Height(Length::Px(300.)),
            ],
        )],
    );
    settle(cx, window).await;
    let current = nodes(cx, window);
    let mut disabled = config();
    disabled.disabled = true;
    apply(cx, window, vec![Op::SetTable(node(0), disabled)]);
    selected(cell(&current, 0, 0), true);
    settle(cx, window).await;
    assert!(requests(cx, window).is_empty());
    assert!(nodes(cx, window).iter().all(|n| n.role != "AXTable"));
    apply(cx, window, vec![Op::SetTable(node(0), config())]);
    settle(cx, window).await;
    for (mode, row_selectable, cell_selectable) in [
        (wire::SelectionMode::Rows, true, false),
        (wire::SelectionMode::Cells, false, true),
    ] {
        let mut restricted = config();
        restricted.selection_mode = mode;
        restricted.column_selection = false;
        apply(cx, window, vec![Op::SetTable(node(0), restricted)]);
        settle(cx, window).await;
        let current = nodes(cx, window);
        let row = current
            .iter()
            .find(|n| n.role == "AXRow" && n.row == Some(0))
            .unwrap();
        assert_eq!(row.settable, row_selectable);
        assert_eq!(cell(&current, 0, 0).settable, cell_selectable);
        let header = current
            .iter()
            .find(|n| n.role == "AXCell" && n.row.is_none() && n.column == Some(0))
            .unwrap();
        assert!(
            !header.settable,
            "column selection disabled independently of sorting"
        );
        assert!(
            current
                .iter()
                .any(|n| n.role == "AXButton" && n.label == "Sort name")
        );
    }
    apply(cx, window, vec![Op::SetTable(node(0), config())]);
    // Reset selection silently; subsequent host cases own their command serials.
    window
        .update(cx, |view, _, cx| {
            let native = view.tables[&node(0)].borrow().native.clone();
            native.update(cx, |state, cx| {
                assert!(state.replace_selection(Selection::Empty, cx));
            });
        })
        .unwrap();
    frame(cx, window).await;
    eprintln!(
        "GPUIO_TABLE_AX_OK: logical counts/indexes, bounded rows/cells, desired selection, idempotent press and stale-frame gating"
    );
}
