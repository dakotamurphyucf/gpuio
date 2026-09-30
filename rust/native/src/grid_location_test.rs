//! Retained native grid geometry and GPU paint; no foreground OS input.
use super::styled_text_test::{apply, draw};
use super::*;
use gpui::point;
use gpuio_protocol::grid_location::{Axis, Edge, Location};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn axis(start: Edge, end: Edge) -> Axis {
    Axis { start, end }
}
fn auto() -> Axis {
    axis(Edge::Auto, Edge::Auto)
}
fn full() -> Axis {
    axis(Edge::Line(1), Edge::Line(-1))
}
fn location(column: Axis, row: Axis) -> Location {
    Location { column, row }
}
fn parent(width: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Position(1),
        Field::Left(Length::Px(20.)),
        Field::Top(Length::Px(20.)),
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(300.)),
        Field::Display(2),
        Field::GridColumns(4),
        Field::GridRows(3),
    ])]
}
fn child(value: Option<Location>) -> Vec<Style> {
    let mut fields = vec![Field::Background(Fill::Solid(Color::Rgba(0xff0000ff)))];
    if let Some(value) = value {
        fields.push(Field::GridLocation(value));
    }
    vec![Style::Fields(fields)]
}
fn check(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, expected: [f64; 4]) {
    draw(cx, handle);
    let actual = handle
        .update(cx, |view, _, _| {
            let bounds = view.probes.borrow()[&id(1)].bounds;
            [
                f64::from(bounds.origin.x),
                f64::from(bounds.origin.y),
                f64::from(bounds.size.width),
                f64::from(bounds.size.height),
            ]
        })
        .unwrap();
    for (value, target) in actual.into_iter().zip(expected) {
        assert!(
            (value - target).abs() < 0.1,
            "grid bounds {actual:?}, expected {expected:?}"
        );
    }
    // Read the actual GPU scene too, not only a recorded layout rectangle.
    handle
        .update(cx, |_, window, _| {
            let image = window.render_to_image().expect("grid GPU readback");
            let scale = f64::from(window.scale_factor());
            let x = ((expected[0] + expected[2] / 2.) * scale) as u32;
            let y = ((expected[1] + expected[3] / 2.) * scale) as u32;
            let pixel = image.get_pixel(x, y).0;
            assert!(
                pixel[0] > 240 && pixel[1] < 10 && pixel[2] < 10,
                "grid fill {pixel:?}"
            );
        })
        .unwrap();
}
async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    use Edge::*;
    apply(
        cx,
        handle,
        vec![
            Op::Create(id(0), Kind::Container, String::new(), None),
            Op::Create(id(1), Kind::Container, String::new(), None),
            Op::Splice(id(0), 0, 0, vec![id(1)]),
            Op::SetRoot(Some(id(0))),
        ],
    );
    let cases = [
        (location(auto(), auto()), [0., 0., 1., 1.]),
        (location(full(), full()), [0., 0., 4., 3.]),
        (
            location(axis(Line(2), Line(4)), axis(Line(2), Line(4))),
            [1., 1., 2., 2.],
        ),
        (location(axis(Line(-3), Line(-1)), auto()), [2., 0., 2., 1.]),
        (location(axis(Line(3), Line(1)), auto()), [0., 0., 2., 1.]),
        (location(axis(Line(2), Line(2)), auto()), [1., 0., 1., 1.]),
        (
            location(axis(Span(2), Span(3)), axis(Auto, Span(2))),
            [0., 0., 2., 2.],
        ),
        (location(axis(Line(2), Span(2)), auto()), [1., 0., 2., 1.]),
        (location(axis(Span(2), Line(4)), auto()), [1., 0., 2., 1.]),
        (location(axis(Auto, Line(4)), auto()), [2., 0., 1., 1.]),
    ];
    for width in [400., 480.] {
        apply(cx, handle, vec![Op::SetStyle(id(0), parent(width))]);
        for (value, [x, y, w, h]) in cases {
            apply(cx, handle, vec![Op::SetStyle(id(1), child(Some(value)))]);
            check(
                cx,
                handle,
                [
                    20. + x * width / 4.,
                    20. + y * 100.,
                    w * width / 4.,
                    h * 100.,
                ],
            );
        }
    }
    apply(cx, handle, vec![Op::SetStyle(id(0), parent(400.))]);
    let mut styles = child(Some(location(auto(), axis(Span(2), Span(2)))));
    styles.push(Style::State(
        2,
        vec![Field::GridLocation(location(
            axis(Span(2), Span(2)),
            auto(),
        ))],
    ));
    apply(cx, handle, vec![Op::SetStyle(id(1), styles)]);
    for (point, expected) in [
        (point(px(510.), px(350.)), [20., 20., 100., 200.]),
        (point(px(40.), px(40.)), [20., 20., 200., 100.]),
        (point(px(510.), px(350.)), [20., 20., 100., 200.]),
    ] {
        native_test::move_mouse(cx, handle, point, false);
        check(cx, handle, expected);
    }
    // Absolute Auto edges use the containing grid's edges, rather than the
    // one-track automatic placement used for in-flow items.
    let mut absolute = child(Some(location(axis(Line(2), Auto), axis(Auto, Line(3)))));
    absolute.push(Style::Fields(vec![
        Field::Position(1),
        Field::Left(Length::Px(0.)),
        Field::Right(Length::Px(0.)),
        Field::Top(Length::Px(0.)),
        Field::Bottom(Length::Px(0.)),
    ]));
    apply(cx, handle, vec![Op::SetStyle(id(1), absolute)]);
    check(cx, handle, [120., 20., 300., 200.]);
    apply(cx, handle, vec![Op::SetStyle(id(1), child(None))]);
    check(cx, handle, [20., 20., 100., 100.]);
    apply(
        cx,
        handle,
        vec![Op::SetRoot(None), Op::Remove(id(1)), Op::Remove(id(0))],
    );
    draw(cx, handle);
    handle
        .update(cx, |view, _, _| {
            assert_eq!(
                view.session
                    .borrow()
                    .tree(view.id)
                    .unwrap()
                    .retained_bytes(),
                0
            );
        })
        .unwrap();
    eprintln!(
        "GPUIO_GRID_LOCATION_OK: 20 geometry/GPU cases, signed/equal/reversed lines, span normalization, resize, absolute Auto edges, atomic hover replacement/restoration, unset and teardown"
    );
}
pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        let window_id = WindowId::from_parts(0, 1).unwrap();
        session
            .borrow_mut()
            .open(1, window_id, "Grid placement check", 520., 380.)
            .unwrap();
        let handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(520.), px(380.)),
                        cx,
                    ))),
                    focus: false,
                    ..Default::default()
                },
                |_, cx| cx.new(|_| View::new(window_id, session, transport)),
            )
            .unwrap();
        cx.spawn(async move |cx| {
            let checked = native_test::protect(exercise(cx, handle)).await;
            *task_failure.borrow_mut() = checked.err();
            let _ = handle.update(cx, |_, window, _| window.remove_window());
            cx.update(stop_application);
        })
        .detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
