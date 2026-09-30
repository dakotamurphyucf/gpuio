//! Background GPU checks through the retained production renderer. Pointer state
//! is dispatched inside GPUI; no desktop keyboard/focus/IME claim is made.
use super::*;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::time::Duration;

fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn apply(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, operations: Vec<Op>) {
    handle
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let changed = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: view.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            view.update_editors(&changed.dirty, window, cx);
            cx.notify();
        })
        .unwrap();
}
fn draw(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    cx.update_window(handle.into(), |_, window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    })
    .unwrap();
}
#[derive(Clone, Copy, Debug)]
struct Panel {
    width: f64,
    height: f64,
    stroke: f64,
    radius: f64,
}
impl Panel {
    fn styles(self, pattern: Option<i64>) -> Vec<Style> {
        let mut fields = vec![
            Field::Position(1),
            Field::Left(Length::Px(20.)),
            Field::Top(Length::Px(20.)),
            Field::Width(Length::Px(self.width)),
            Field::Height(Length::Px(self.height)),
            Field::Background(Fill::Solid(Color::Rgba(0x000000ff))),
            Field::BorderColor(Color::Rgba(0xffffffff)),
            Field::BorderTopWidth(self.stroke),
            Field::BorderRightWidth(self.stroke),
            Field::BorderBottomWidth(self.stroke),
            Field::BorderLeftWidth(self.stroke),
            Field::TopLeftRadius(self.radius),
            Field::TopRightRadius(self.radius),
            Field::BottomLeftRadius(self.radius),
            Field::BottomRightRadius(self.radius),
        ];
        if let Some(pattern) = pattern {
            fields.push(Field::BorderStyle(pattern));
        }
        vec![Style::Fields(fields)]
    }
}
fn pixels(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    panel: Panel,
    dashed: bool,
    only_side: Option<usize>,
) {
    handle
        .update(cx, |_, window, _| {
            let image = window.render_to_image().expect("border GPU readback");
            let scale = f64::from(window.scale_factor());
            let read = |x: f64, y: f64| image.get_pixel((x * scale) as u32, (y * scale) as u32).0;
            let white = |p: [u8; 4]| p[0] > 230 && p[1] > 230 && p[2] > 230;
            let black = |p: [u8; 4]| p[0] < 20 && p[1] < 20 && p[2] < 20;
            let inset = panel.radius.max(panel.stroke) + 5.;
            for side in 0..4 {
                let length = if side % 2 == 0 {
                    panel.width
                } else {
                    panel.height
                };
                let (mut on, mut off, mut gaps, mut total) = (0, 0, 0, 0);
                let mut samples = Vec::new();
                for i in (inset * scale) as u32..((length - inset) * scale) as u32 {
                    let along = f64::from(i) / scale;
                    let half = panel.stroke / 2.;
                    let (x, y) = match side {
                        0 => (20. + along, 20. + half),
                        1 => (20. + panel.width - half, 20. + along),
                        2 => (20. + along, 20. + panel.height - half),
                        _ => (20. + half, 20. + along),
                    };
                    let pixel = read(x, y);
                    samples.push(pixel[0]);
                    on += usize::from(white(pixel));
                    off += usize::from(black(pixel));
                    // A one-device-pixel gap can straddle adjacent samples.
                    // The antialiased valley need not contain a pure-black
                    // pixel; require at most half coverage instead. Solid and
                    // zero-width/omitted edges retain their strict assertions.
                    gaps += usize::from(if panel.stroke * scale <= 1. {
                        pixel[0] <= 128 && pixel[1] <= 128 && pixel[2] <= 128
                    } else {
                        black(pixel)
                    });
                    total += 1;
                }
                assert!(total > 20);
                if panel.stroke == 0. || only_side.is_some_and(|selected| side != selected) {
                    assert_eq!(off, total, "zero-width border: {panel:?}, side {side}");
                } else if dashed {
                    assert!(
                        on > 2 && gaps > 2,
                        "dash/gap missing: {panel:?}, side {side}, bright={on}/gaps={gaps}/black={off}/total={total}, samples={samples:?}"
                    );
                } else {
                    assert_eq!(on, total, "solid border: {panel:?}, side {side}");
                }
            }
            assert!(black(read(10., 10.)), "border must remain within its box");
            assert!(
                black(read(20. + panel.width / 2., 20. + panel.height / 2.)),
                "border does not replace its background"
            );
            if panel.radius > 0. {
                assert!(
                    black(read(20.5, 20.5)),
                    "rounded corner clips the outer corner"
                );
            }
        })
        .unwrap();
}
async fn exercise(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    if std::env::args().any(|arg| arg == "--scale-one") {
        handle
            .update(cx, |_, window, _| window.set_scale_factor(1.))
            .unwrap();
    }
    apply(
        cx,
        handle,
        vec![
            Op::Create(node(0), Kind::Container, String::new(), None),
            Op::Create(node(1), Kind::Container, String::new(), None),
            Op::SetStyle(
                node(0),
                vec![Style::Fields(vec![
                    Field::Width(Length::Percent(100.)),
                    Field::Height(Length::Percent(100.)),
                    Field::Background(Fill::Solid(Color::Rgba(0x000000ff))),
                ])],
            ),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    );
    let mut cases = 0;
    for (width, height) in [(240., 100.), (300., 180.)] {
        for radius in [0., 12.] {
            for stroke in [0., 1., 3., 8.] {
                let panel = Panel {
                    width,
                    height,
                    stroke,
                    radius,
                };
                for pattern in [Some(0), Some(1), None, Some(1)] {
                    apply(
                        cx,
                        handle,
                        vec![Op::SetStyle(node(1), panel.styles(pattern))],
                    );
                    draw(cx, handle);
                    pixels(cx, handle, panel, pattern == Some(1), None);
                    cases += 1;
                }
            }
        }
    }
    let panel = Panel {
        width: 240.,
        height: 120.,
        stroke: 3.,
        radius: 12.,
    };
    for only_side in 0..4 {
        for pattern in [0, 1] {
            let mut styles = panel.styles(Some(pattern));
            styles.push(Style::Fields(vec![
                Field::BorderTopWidth(if only_side == 0 { panel.stroke } else { 0. }),
                Field::BorderRightWidth(if only_side == 1 { panel.stroke } else { 0. }),
                Field::BorderBottomWidth(if only_side == 2 { panel.stroke } else { 0. }),
                Field::BorderLeftWidth(if only_side == 3 { panel.stroke } else { 0. }),
            ]));
            apply(cx, handle, vec![Op::SetStyle(node(1), styles)]);
            draw(cx, handle);
            pixels(cx, handle, panel, pattern == 1, Some(only_side));
            cases += 1;
        }
    }
    let mut styles = panel.styles(Some(0));
    styles.push(Style::State(2, vec![Field::BorderStyle(1)]));
    styles.push(Style::State(3, vec![Field::BorderStyle(0)]));
    apply(cx, handle, vec![Op::SetStyle(node(1), styles)]);
    draw(cx, handle);
    for (x, y, dashed) in [(330., 230., false), (100., 60., true), (330., 230., false)] {
        native_test::move_mouse(cx, handle, gpui::point(px(x), px(y)), false);
        draw(cx, handle);
        pixels(cx, handle, panel, dashed, None);
    }
    let point = gpui::point(px(100.), px(60.));
    native_test::move_mouse(cx, handle, point, false);
    draw(cx, handle);
    pixels(cx, handle, panel, true, None);
    native_test::mouse(cx, handle, point, true);
    draw(cx, handle);
    pixels(cx, handle, panel, false, None);
    native_test::mouse(cx, handle, point, false);
    draw(cx, handle);
    pixels(cx, handle, panel, true, None);
    apply(
        cx,
        handle,
        vec![Op::SetRoot(None), Op::Remove(node(1)), Op::Remove(node(0))],
    );
    draw(cx, handle);
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
    // Let an already queued platform frame settle, then observe another
    // interval without explicit draw/refresh calls.
    cx.background_executor()
        .timer(Duration::from_millis(120))
        .await;
    let renders = handle.update(cx, |view, _, _| view.render_count).unwrap();
    cx.background_executor()
        .timer(Duration::from_millis(120))
        .await;
    let settled = handle.update(cx, |view, _, _| view.render_count).unwrap();
    assert_eq!(
        settled, renders,
        "border teardown must not schedule animation"
    );
    eprintln!(
        "GPUIO_BORDER_STYLE_OK: {cases} GPU cases, four edges, width/radius/resize, solid/dash/unset replacement, dispatched hover/press precedence and zero retained tree bytes after removal"
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
            .open(1, window_id, "Border style check", 360., 260.)
            .unwrap();
        let handle = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(360.), px(260.)),
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
