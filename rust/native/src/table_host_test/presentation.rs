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

#[test]
fn pointer_focus_does_not_scroll_a_visible_cell_away_before_release() {
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
        .open(1, window_id(), "Scrolled table", 520., 400.)
        .unwrap();
    let (view, cx) = app.add_window_view(|_, _| View::new(window_id(), session.clone(), transport));
    cx.simulate_a11y_active(true);
    cx.update(|window, cx| {
        window.activate_window();
        window.blur(cx);
    });
    let mut ops = initial();
    ops.extend([
        Op::SetStyle(
            node(0),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(520.)),
                Field::Height(Length::Px(300.)),
                Field::Shrink(0.),
                Field::MarginTop(Length::Px(80.)),
            ])],
        ),
        Op::Create(node(61), Kind::Container, String::new(), None),
        Op::SetStyle(
            node(61),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(520.)),
                Field::Height(Length::Px(250.)),
                Field::OverflowY(3),
            ])],
        ),
        Op::Splice(node(61), 0, 0, vec![node(0)]),
        Op::SetRoot(Some(node(61))),
    ]);
    apply(&view, cx, ops);
    let center = view.read_with(cx, |v, _| v.probes.borrow()[&node(3)].bounds.center());
    let offset = view.read_with(cx, |v, _| v.scrolls[&node(61)].handle.offset());
    cx.simulate_mouse_move(center, None, Default::default());
    cx.simulate_mouse_down(center, gpui::MouseButton::Left, Default::default());
    // A real user holds the button across frames. Sending down/up before one
    // draw hides ancestor focus reveal that can move the target under the hand.
    draw(cx);
    draw(cx);
    let during = view.read_with(cx, |v, _| v.probes.borrow()[&node(3)].bounds.center());
    assert_eq!(
        during, center,
        "pointer focus moved the cell before release"
    );
    assert_eq!(
        view.read_with(cx, |v, _| v.scrolls[&node(61)].handle.offset()),
        offset,
        "pointer focus must preserve the user's ancestor scroll position"
    );
    cx.simulate_mouse_up(center, gpui::MouseButton::Left, Default::default());
    draw(cx);
    let native = view.read_with(cx, |v, _| v.tables[&node(0)].borrow().native.clone());
    assert_eq!(
        native.read_with(cx, |state, _| state.selection().clone()),
        Selection::Cell {
            row: gpuio_table_adapter::table::RowKey(1),
            column: "name".into()
        }
    );
    assert_eq!(
        view.read_with(cx, |v, _| v.scrolls[&node(61)].handle.offset()),
        offset,
        "click completion must also preserve ancestor scroll"
    );
    // An explicit reveal remains effective even without another focus change.
    view.read_with(cx, |v, _| v.focus.borrow().request_reveal());
    draw(cx);
    draw(cx);
    assert_ne!(
        view.read_with(cx, |v, _| v.scrolls[&node(61)].handle.offset()),
        offset,
        "pointer focus must not suppress an explicit reveal"
    );
    // Restore the user's viewport, then focus the table without pointer input.
    // Ordinary keyboard/programmatic focus still performs its automatic reveal.
    view.read_with(cx, |v, _| v.scrolls[&node(61)].handle.set_offset(offset));
    cx.update(|window, cx| window.blur(cx));
    draw(cx);
    draw(cx);
    assert_eq!(
        view.read_with(cx, |v, _| v.scrolls[&node(61)].handle.offset()),
        offset
    );
    cx.update(|window, cx| native.focus_handle(cx).focus(window, cx));
    draw(cx);
    draw(cx);
    assert_ne!(
        view.read_with(cx, |v, _| v.scrolls[&node(61)].handle.offset()),
        offset,
        "non-pointer focus must continue to reveal"
    );
}

#[test]
fn pointer_focus_observation_does_not_reborrow_the_root_view() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id(), "Pointer capture", 520., 300.)
        .unwrap();
    let (view, cx) = app.add_window_view(|_, _| View::new(window_id(), session, transport));
    apply(
        &view,
        cx,
        vec![
            Op::Create(node(0), Kind::Container, String::new(), None),
            Op::SetStyle(
                node(0),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(520.)),
                    Field::Height(Length::Px(300.)),
                ])],
            ),
            Op::SetRoot(Some(node(0))),
        ],
    );
    // Native integration fixtures may dispatch while updating the root. The
    // observer only records focus in its separately owned manager; it must not
    // lease View again merely to obtain that manager.
    cx.update(|window, cx| {
        view.update(cx, |_, cx| {
            window.dispatch_event(
                gpui::PlatformInput::MouseDown(gpui::MouseDownEvent {
                    position: gpui::point(px(10.), px(10.)),
                    button: gpui::MouseButton::Left,
                    modifiers: Default::default(),
                    click_count: 1,
                    first_mouse: false,
                }),
                cx,
            );
            window.dispatch_event(
                gpui::PlatformInput::MouseUp(gpui::MouseUpEvent {
                    position: gpui::point(px(10.), px(10.)),
                    button: gpui::MouseButton::Left,
                    modifiers: Default::default(),
                    click_count: 1,
                }),
                cx,
            );
        });
    });
    cx.run_until_parked();
}
