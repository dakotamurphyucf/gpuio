//! Production hit testing and gesture routing with recorded platform requests.
use super::*;
use crate::{session::Session, transport::Transport};
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpuio_protocol::HandlerId;
use std::os::{fd::AsRawFd, unix::net::UnixStream};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn style(x: f64, width: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Position(1),
        Field::Left(Length::Px(x)),
        Field::Top(Length::Px(0.)),
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(60.)),
    ])]
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn apply(view: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|w, cx| {
        view.update(cx, |view, cx| {
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
            view.update_editors(&applied.dirty, w, cx);
            cx.notify();
        })
    });
    draw(cx);
}
fn mount(
    app: &mut TestAppContext,
    resizable: bool,
) -> (Entity<View>, VisualTestContext, Arc<Transport>, UnixStream) {
    app.update(|cx| {
        gpui_base::init(cx);
        cx.set_global(Recorded::default());
    });
    let (reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let id_window = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, id_window, "Custom", 500., 300.)
        .unwrap();
    let native = app.update(|cx| {
        cx.open_window(
            gpui::WindowOptions {
                is_resizable: resizable,
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(id_window, session.clone(), transport.clone())),
        )
        .unwrap()
    });
    let view = native.root(app).unwrap();
    let mut cx = VisualTestContext::from_window(native.into(), app);
    cx.simulate_resize(gpui::size(px(500.), px(300.)));
    apply(
        &view,
        &mut cx,
        vec![
            Op::Create(id(0), Kind::Container, "".into(), None),
            Op::SetStyle(id(0), style(0., 400.)),
            Op::SetWindowRegion(id(0), Some(Region::TitleBar)),
            Op::Create(
                id(1),
                Kind::Button,
                "Child".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetControl(id(1), Control::Button(false)),
            Op::SetStyle(id(1), style(100., 80.)),
            Op::Create(id(2), Kind::Container, "".into(), None),
            Op::SetStyle(id(2), style(240., 80.)),
            Op::SetWindowRegion(id(2), Some(Region::Exclude)),
            Op::Splice(id(0), 0, 0, vec![id(1), id(2)]),
            Op::SetRoot(Some(id(0))),
        ],
    );
    cx.update(|w, _| w.activate_window());
    draw(&mut cx);
    (view, cx, transport, reader)
}
fn down(cx: &mut VisualTestContext, x: f32, button: MouseButton, clicks: usize) {
    cx.simulate_event(gpui::MouseDownEvent {
        position: gpui::point(px(x), px(20.)),
        button,
        click_count: clicks,
        modifiers: Default::default(),
        first_mouse: false,
    });
}
fn moving(cx: &mut VisualTestContext, x: f32) {
    cx.simulate_event(gpui::MouseMoveEvent {
        position: gpui::point(px(x), px(22.)),
        pressed_button: Some(MouseButton::Left),
        modifiers: Default::default(),
    });
}
fn up(cx: &mut VisualTestContext, x: f32) {
    cx.simulate_event(gpui::MouseUpEvent {
        position: gpui::point(px(x), px(22.)),
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
    });
}
fn take(cx: &mut VisualTestContext) -> Vec<Request> {
    cx.cx
        .update(|cx| std::mem::take(&mut cx.global_mut::<Recorded>().0))
}

#[test]
fn retired_regions_cannot_resume_gestures_and_release_frame_state() {
    let mut app = TestAppContext::single();
    let (view, mut cx, _, _reader) = mount(&mut app, true);
    // Unrelated changes preserve a held gesture and the native region identity.
    down(&mut cx, 20., MouseButton::Left, 1);
    let state = cx
        .cx
        .update(|cx| view.read(cx).window_regions[&id(0)].clone());
    apply(&view, &mut cx, vec![Op::SetText(id(1), "Renamed".into())]);
    assert!(
        cx.cx
            .update(|cx| Rc::ptr_eq(&state, &view.read(cx).window_regions[&id(0)]))
    );
    moving(&mut cx, 25.);
    assert_eq!(take(&mut cx), vec![Request::Move]);
    up(&mut cx, 25.);

    // An equal-valued replacement still invalidates callbacks from the old frame.
    down(&mut cx, 20., MouseButton::Left, 1);
    cx.update(|w, cx| {
        view.update(cx, |view, _| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            view.session
                .borrow_mut()
                .apply(&Transaction {
                    window: view.id,
                    base,
                    revision: base + 1,
                    operations: vec![Op::SetWindowRegion(id(0), Some(Region::TitleBar))],
                })
                .unwrap();
            assert!(!eligible(view, id(0), &state, w));
        })
    });
    moving(&mut cx, 25.);
    assert!(take(&mut cx).is_empty());
    draw(&mut cx);
    assert!(
        cx.cx
            .update(|cx| !Rc::ptr_eq(&state, &view.read(cx).window_regions[&id(0)]))
    );
    drop(state);

    for gate in [
        Field::Inert(true),
        Field::Display(3),
        Field::PointerEvents(false),
    ] {
        down(&mut cx, 20., MouseButton::Left, 1);
        let mut styles = style(0., 400.);
        styles.push(Style::Fields(vec![gate]));
        apply(&view, &mut cx, vec![Op::SetStyle(id(0), styles)]);
        apply(&view, &mut cx, vec![Op::SetStyle(id(0), style(0., 400.))]);
        moving(&mut cx, 25.);
        assert!(
            take(&mut cx).is_empty(),
            "restoring input must not restore an old drag"
        );
        up(&mut cx, 25.);
    }

    down(&mut cx, 20., MouseButton::Left, 1);
    let weak = cx
        .cx
        .update(|cx| Rc::downgrade(&view.read(cx).window_regions[&id(0)]));
    apply(&view, &mut cx, vec![Op::SetWindowRegion(id(0), None)]);
    moving(&mut cx, 25.);
    assert!(take(&mut cx).is_empty());
    assert!(
        cx.cx
            .update(|cx| !view.read(cx).window_regions.contains_key(&id(0)))
    );
    assert!(
        weak.upgrade().is_none(),
        "removed policy releases observer and frame callbacks"
    );
}

#[test]
fn invalid_region_admission_is_atomic_and_reservations_release() {
    let mut app = TestAppContext::single();
    let (view, mut cx, _, _reader) = mount(&mut app, true);
    cx.update(|_, cx| {
        view.update(cx, |view, _| {
            let mut session = view.session.borrow_mut();
            let tree = session.tree(view.id).unwrap();
            let base = tree.revision();
            let bytes = tree.retained_bytes();
            let config = tree.get(id(0)).unwrap().window_region.clone().unwrap();
            let error = session
                .apply(&Transaction {
                    window: view.id,
                    base,
                    revision: base + 1,
                    operations: vec![
                        Op::SetWindowRegion(id(0), None),
                        Op::SetWindowRegion(id(1), Some(Region::TitleBar)),
                    ],
                })
                .err()
                .unwrap();
            assert_eq!(error, ErrorCode::InvalidTree);
            let tree = session.tree(view.id).unwrap();
            assert_eq!(tree.revision(), base);
            assert_eq!(tree.retained_bytes(), bytes);
            assert!(Arc::ptr_eq(
                tree.get(id(0)).unwrap().window_region.as_ref().unwrap(),
                &config
            ));
            // Even a container is invalid when it is a decorative button slot.
            assert_eq!(
                session
                    .apply(&Transaction {
                        window: view.id,
                        base,
                        revision: base + 1,
                        operations: vec![
                            Op::Create(id(3), Kind::Container, "".into(), None),
                            Op::Create(id(4), Kind::Container, "".into(), None),
                            Op::SetWindowRegion(id(3), Some(Region::TitleBar)),
                            Op::Splice(id(1), 0, 0, vec![id(3), id(4)]),
                        ],
                    })
                    .err(),
                Some(ErrorCode::InvalidTree)
            );
            let tree = session.tree(view.id).unwrap();
            assert_eq!(tree.revision(), base);
            assert_eq!(tree.retained_bytes(), bytes);
            assert!(tree.get(id(3)).is_none());
        })
    });
    let before = cx.cx.update(|cx| {
        let view = view.read(cx);
        let session = view.session.borrow();
        session.tree(view.id).unwrap().retained_bytes()
    });
    apply(
        &view,
        &mut cx,
        vec![
            Op::SetWindowRegion(id(0), None),
            Op::SetWindowRegion(id(2), None),
        ],
    );
    let after = cx.cx.update(|cx| {
        let view = view.read(cx);
        let session = view.session.borrow();
        session.tree(view.id).unwrap().retained_bytes()
    });
    assert_eq!(before - after, 2 * RESERVED_BYTES);
}

#[test]
fn inactivity_and_window_close_cancel_and_release_native_regions() {
    let mut app = TestAppContext::single();
    let (view, mut cx, _, _reader) = mount(&mut app, true);
    down(&mut cx, 20., MouseButton::Left, 1);
    let state = cx
        .cx
        .update(|cx| view.read(cx).window_regions[&id(0)].clone());
    assert!(state.armed.get());
    struct Empty;
    impl gpui::Render for Empty {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl gpui::IntoElement {
            gpui::div()
        }
    }
    let other = cx.cx.update(|cx| {
        cx.open_window(Default::default(), |_, cx| cx.new(|_| Empty))
            .unwrap()
    });
    other
        .update(&mut cx.cx, |_, window, _| window.activate_window())
        .unwrap();
    cx.run_until_parked();
    assert!(!state.armed.get());
    down(&mut cx, 20., MouseButton::Left, 1);
    moving(&mut cx, 25.);
    assert!(
        take(&mut cx).is_empty(),
        "inactive windows do not start gestures"
    );
    cx.update(|window, _| window.activate_window());
    draw(&mut cx);
    moving(&mut cx, 25.);
    assert!(
        take(&mut cx).is_empty(),
        "reactivation does not resume an old gesture"
    );
    let weak = Rc::downgrade(&state);
    drop(state);
    cx.update(|window, _| window.remove_window());
    drop(view);
    cx.cx.update(|_| {});
    cx.run_until_parked();
    assert!(
        weak.upgrade().is_none(),
        "window close releases region state and observer"
    );
    other
        .update(&mut cx.cx, |_, window, _| window.remove_window())
        .unwrap();
}

#[test]
fn a_native_editor_inside_the_title_bar_keeps_pointer_input_and_focus() {
    let mut app = TestAppContext::single();
    let (view, mut cx, _, _reader) = mount(&mut app, true);
    apply(
        &view,
        &mut cx,
        vec![
            Op::Create(
                id(3),
                Kind::Input,
                "Draft".into(),
                Some(HandlerId::from_parts(3, 1).unwrap()),
            ),
            Op::SetEditor(
                id(3),
                EditorConfig {
                    label: "Titlebar search".into(),
                    placeholder: "".into(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: true,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::SetStyle(id(3), style(180., 60.)),
            Op::Splice(id(0), 2, 0, vec![id(3)]),
        ],
    );
    down(&mut cx, 195., MouseButton::Left, 1);
    moving(&mut cx, 218.);
    up(&mut cx, 218.);
    assert!(take(&mut cx).is_empty());
    cx.update(|window, cx| {
        let view = view.read(cx);
        assert!(view.editors[&id(3)].focus_handle(cx).is_focused(window));
    });
}
#[test]
fn title_bar_routes_native_gestures_and_excludes_child_controls() {
    let mut app = TestAppContext::single();
    let (view, mut cx, transport, _reader) = mount(&mut app, true);
    down(&mut cx, 20., MouseButton::Left, 1);
    assert!(take(&mut cx).is_empty());
    moving(&mut cx, 25.);
    moving(&mut cx, 30.);
    assert_eq!(take(&mut cx), vec![Request::Move]);
    up(&mut cx, 30.);
    down(&mut cx, 20., MouseButton::Left, 1);
    up(&mut cx, 20.);
    moving(&mut cx, 25.);
    assert!(take(&mut cx).is_empty());
    down(&mut cx, 20., MouseButton::Left, 2);
    moving(&mut cx, 25.);
    up(&mut cx, 25.);
    assert_eq!(take(&mut cx), vec![Request::DoubleClick]);
    for x in [120., 260.] {
        down(&mut cx, x, MouseButton::Left, 1);
        moving(&mut cx, x + 2.);
        up(&mut cx, x + 2.);
        assert!(
            take(&mut cx).is_empty(),
            "child or exclusion armed parent move"
        );
    }
    assert!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(event, Event::Press(_,node,_,_) if *node==id(1)))
    );
    apply(
        &view,
        &mut cx,
        vec![Op::SetControl(id(1), Control::Button(true))],
    );
    down(&mut cx, 120., MouseButton::Left, 1);
    moving(&mut cx, 125.);
    up(&mut cx, 125.);
    assert!(
        take(&mut cx).is_empty(),
        "disabled control is still excluded"
    );
}
#[test]
fn resize_regions_route_all_edges_and_respect_window_policy() {
    for resizable in [true, false] {
        let mut app = TestAppContext::single();
        let (view, mut cx, _, _reader) = mount(&mut app, resizable);
        for edge in [
            Edge::Top,
            Edge::Bottom,
            Edge::Left,
            Edge::Right,
            Edge::TopLeft,
            Edge::TopRight,
            Edge::BottomLeft,
            Edge::BottomRight,
        ] {
            apply(
                &view,
                &mut cx,
                vec![Op::SetWindowRegion(id(0), Some(Region::Resize(edge)))],
            );
            down(&mut cx, 20., MouseButton::Left, 1);
            up(&mut cx, 20.);
            assert_eq!(
                take(&mut cx),
                if resizable && cfg!(target_os = "linux") {
                    vec![Request::Resize(edge)]
                } else {
                    vec![]
                }
            );
        }
    }
}
