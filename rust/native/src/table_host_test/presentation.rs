//! Production table layout/state styling on TestPlatform; no physical GPU claim.
use super::behavior::{apply, draw};
use super::*;
use crate::session::Session;
use gpui::{TestAppContext, VisualTestContext};
use std::os::unix::net::UnixStream;

fn foreground(rgba: i64) -> Field {
    Field::Foreground(Color::Rgba(rgba))
}
fn text_color(view: &Entity<View>, cx: &VisualTestContext, slot: i64) -> gpui::Hsla {
    view.read_with(cx, |v, _| v.probes.borrow()[&node(slot)].color)
}
fn row_bounds(cx: &mut VisualTestContext) -> gpui::accesskit::Rect {
    cx.a11y_tree()
        .unwrap()
        .nodes
        .into_iter()
        .find(|(_, n)| n.role() == gpui::accesskit::Role::Row && n.label() == Some("Row 1"))
        .expect("painted first native row")
        .1
        .bounds()
        .unwrap()
}

#[test]
fn scoped_presentation_preserves_geometry_and_refines_native_interaction_states() {
    let mut app = TestAppContext::single();
    app.update(|cx| {
        gpui_base::init(cx);
        gpuio_table_adapter::init(cx);
    });
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id(), "Presentation", 520., 300.)
        .unwrap();
    let (view, cx) = app.add_window_view(|_, _| View::new(window_id(), session.clone(), transport));
    cx.simulate_a11y_active(true);
    cx.update(|window, cx| {
        window.activate_window();
        window.blur(cx);
    });
    let mut ops = initial();
    ops.extend([
        Op::Create(node(61), Kind::Container, String::new(), None),
        Op::SetTableHeader(
            node(61),
            Some(gpuio_protocol::table_header::Target::Column("name".into())),
        ),
        Op::Create(node(62), Kind::Text, "Custom header".into(), None),
        Op::Splice(node(61), 0, 0, vec![node(62)]),
        Op::Splice(node(0), 0, 0, vec![node(61)]),
    ]);
    apply(&view, cx, ops);
    let native = view.read_with(cx, |v, _| v.tables[&node(0)].borrow().native.clone());
    let before = row_bounds(cx);
    let baseline = text_color(&view, cx, 3);
    let row_styles = vec![
        Style::Fields(vec![foreground(0xff0000ff), Field::FontWeight(600)]),
        Style::State(7, vec![foreground(0x00ff00ff)]),
        Style::State(1, vec![foreground(0xffff00ff)]),
        Style::State(2, vec![foreground(0x0000ffff)]),
        Style::State(3, vec![foreground(0xff00ffff)]),
        Style::State(6, vec![foreground(0x888888ff)]),
    ];
    apply(
        &view,
        cx,
        vec![
            Op::SetTableHeaderStyle(
                node(0),
                vec![
                    Style::Fields(vec![foreground(0x00ffffff)]),
                    Style::State(2, vec![foreground(0xff0000ff)]),
                ],
            ),
            Op::SetTableRowStyle(node(1), row_styles),
        ],
    );
    let expected = |rgba| crate::host::color(&Color::Rgba(rgba));
    assert_eq!(text_color(&view, cx, 3), expected(0xff0000ff));
    assert_eq!(text_color(&view, cx, 62), expected(0x00ffffff));
    assert_eq!(
        row_bounds(cx),
        before,
        "scoped font/paint cannot alter native row geometry"
    );
    let header_center = view.read_with(cx, |v, _| v.probes.borrow()[&node(62)].bounds.center());
    cx.simulate_mouse_move(header_center, None, Default::default());
    draw(cx);
    assert_eq!(text_color(&view, cx, 62), expected(0xff0000ff));
    cx.simulate_mouse_move(gpui::point(px(700.), px(500.)), None, Default::default());
    draw(cx);
    native.update(cx, |t, cx| {
        assert!(t.replace_selection(
            Selection::Cell {
                row: RowKey(1),
                column: "value".into()
            },
            cx
        ));
    });
    draw(cx);
    assert_eq!(
        text_color(&view, cx, 3),
        expected(0xff0000ff),
        "cell selection is not whole-row styling"
    );
    native.update(cx, |t, cx| {
        assert!(t.replace_selection(Selection::Row(RowKey(1)), cx));
    });
    draw(cx);
    assert_eq!(text_color(&view, cx, 3), expected(0x00ff00ff));
    cx.update(|window, cx| native.focus_handle(cx).focus(window, cx));
    draw(cx);
    assert_eq!(text_color(&view, cx, 3), expected(0xffff00ff));
    let center = view.read_with(cx, |v, _| v.probes.borrow()[&node(3)].bounds.center());
    cx.simulate_mouse_move(center, None, Default::default());
    draw(cx);
    assert_eq!(text_color(&view, cx, 3), expected(0x0000ffff));
    cx.simulate_mouse_down(center, gpui::MouseButton::Left, Default::default());
    draw(cx);
    assert_eq!(text_color(&view, cx, 3), expected(0xff00ffff));
    cx.simulate_mouse_up(center, gpui::MouseButton::Left, Default::default());
    let mut disabled = config();
    disabled.disabled = true;
    apply(&view, cx, vec![Op::SetTable(node(0), disabled)]);
    assert_eq!(text_color(&view, cx, 3), expected(0x888888ff));
    assert_eq!(row_bounds(cx), before);
    cx.simulate_mouse_move(gpui::point(px(700.), px(500.)), None, Default::default());
    apply(
        &view,
        cx,
        vec![
            Op::SetTable(node(0), config()),
            Op::SetTableHeaderStyle(node(0), vec![]),
            Op::SetTableRowStyle(node(1), vec![]),
        ],
    );
    assert_eq!(text_color(&view, cx, 3), baseline);
    assert_eq!(
        native.entity_id(),
        view.read_with(cx, |v, _| v.tables[&node(0)].borrow().native.entity_id())
    );
    assert_eq!(row_bounds(cx), before);
    let mut remove = vec![Op::SetRoot(None)];
    remove.extend((0..63).rev().map(|slot| Op::Remove(node(slot))));
    apply(&view, cx, remove);
    assert_eq!(
        session.borrow().tree(window_id()).unwrap().retained_bytes(),
        0
    );
}
