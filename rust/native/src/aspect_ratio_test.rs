//! Native retained layout plus GPU pixels, without foreground input.
use super::styled_text_test::{apply, draw};
use super::*;
use gpui::point;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn parent(width: f64, padding: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Position(1),
        Field::Left(Length::Px(20.)),
        Field::Top(Length::Px(20.)),
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(260.)),
        Field::Direction(1),
        Field::AlignItems(0),
        Field::PaddingTop(Length::Px(padding)),
        Field::PaddingRight(Length::Px(padding)),
        Field::PaddingBottom(Length::Px(padding)),
        Field::PaddingLeft(Length::Px(padding)),
    ])]
}
fn child(fields: Vec<Field>) -> Vec<Style> {
    let mut defaults = vec![
        Field::Shrink(0.),
        Field::Background(Fill::Solid(Color::Rgba(0xff0000ff))),
    ];
    defaults.extend(fields);
    vec![Style::Fields(defaults)]
}
fn painted_bounds(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) -> Option<[f64; 4]> {
    handle
        .update(cx, |_, window, _| {
            let image = window.render_to_image().expect("aspect GPU readback");
            let mut bounds = None::<[u32; 4]>;
            for (x, y, color) in image.enumerate_pixels() {
                if color[0] > 240 && color[1] < 10 && color[2] < 10 {
                    let b = bounds.get_or_insert([x, y, x, y]);
                    b[0] = b[0].min(x);
                    b[1] = b[1].min(y);
                    b[2] = b[2].max(x);
                    b[3] = b[3].max(y);
                }
            }
            let scale = f64::from(window.scale_factor());
            bounds.map(|[x, y, r, b]| {
                [
                    f64::from(x) / scale,
                    f64::from(y) / scale,
                    f64::from(r - x + 1) / scale,
                    f64::from(b - y + 1) / scale,
                ]
            })
        })
        .unwrap()
}
fn check(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, expected: [f64; 4]) {
    draw(cx, handle);
    let actual = painted_bounds(cx, handle).expect("painted ratio rectangle");
    for (a, e) in actual.into_iter().zip(expected) {
        assert!(
            (a - e).abs() <= 1.,
            "bounds {actual:?}, expected {expected:?}"
        );
    }
}
async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    apply(
        cx,
        handle,
        vec![
            Op::Create(id(0), Kind::Container, String::new(), None),
            Op::Create(id(1), Kind::Container, String::new(), None),
            Op::SetStyle(id(0), parent(160., 0.)),
            Op::Splice(id(0), 0, 0, vec![id(1)]),
            Op::SetRoot(Some(id(0))),
        ],
    );
    let mut cases = 0;
    for width in [120., 200.] {
        for padding in [0., 12.] {
            for ratio in [1., 2., 4.] {
                apply(
                    cx,
                    handle,
                    vec![
                        Op::SetStyle(id(0), parent(width, padding)),
                        Op::SetStyle(
                            id(1),
                            child(vec![
                                Field::Width(Length::Percent(100.)),
                                Field::AspectRatio(ratio),
                            ]),
                        ),
                    ],
                );
                let inner = width - 2. * padding;
                check(
                    cx,
                    handle,
                    [20. + padding, 20. + padding, inner, inner / ratio],
                );
                cases += 1;
            }
        }
    }
    apply(cx, handle, vec![Op::SetStyle(id(0), parent(240., 0.))]);
    for (fields, width, height) in [
        (
            vec![Field::Height(Length::Px(80.)), Field::AspectRatio(2.)],
            160.,
            80.,
        ),
        (
            vec![
                Field::Width(Length::Px(120.)),
                Field::Height(Length::Px(80.)),
                Field::AspectRatio(2.),
            ],
            120.,
            80.,
        ),
        (
            vec![
                Field::Width(Length::Px(120.)),
                Field::MinHeight(Length::Px(100.)),
                Field::AspectRatio(2.),
            ],
            200.,
            100.,
        ),
        (
            vec![
                Field::Width(Length::Px(120.)),
                Field::MaxHeight(Length::Px(90.)),
                Field::AspectRatio(1.),
            ],
            90.,
            90.,
        ),
    ] {
        apply(cx, handle, vec![Op::SetStyle(id(1), child(fields))]);
        check(cx, handle, [20., 20., width, height]);
        cases += 1;
    }
    let mut styles = child(vec![Field::Width(Length::Px(120.)), Field::AspectRatio(2.)]);
    styles.push(Style::State(2, vec![Field::AspectRatio(1.)]));
    apply(cx, handle, vec![Op::SetStyle(id(1), styles)]);
    for (point, height) in [
        (point(px(300.), px(290.)), 60.),
        (point(px(40.), px(40.)), 120.),
        (point(px(300.), px(290.)), 60.),
    ] {
        native_test::move_mouse(cx, handle, point, false);
        check(cx, handle, [20., 20., 120., height]);
    }
    apply(
        cx,
        handle,
        vec![Op::SetStyle(
            id(1),
            child(vec![Field::Width(Length::Px(120.))]),
        )],
    );
    draw(cx, handle);
    assert_eq!(
        painted_bounds(cx, handle),
        None,
        "removing ratio restores empty auto height"
    );
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
            )
        })
        .unwrap();
    eprintln!(
        "GPUIO_ASPECT_RATIO_OK: {cases} GPU layout cases, percentage/padded resize, width/height authority, min/max constraints, hover restoration, unset and teardown"
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
            .open(1, window_id, "Aspect ratio check", 360., 320.)
            .unwrap();
        let handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(360.), px(320.)),
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
