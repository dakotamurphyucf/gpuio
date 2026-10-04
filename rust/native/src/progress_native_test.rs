//! Real GPU/AX fixture in native_images. Linking this does not establish acceptance.
use super::*;
use gpuio_protocol::{
    animation::Easing,
    progress_presentation::{Config, Shape, Transition},
};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn config(fraction: Option<f64>, transition: Transition) -> Config {
    Config {
        progress: ProgressConfig {
            label: "Measured circle".into(),
            fraction,
        },
        shape: Shape::Circle,
        transition,
    }
}
fn appearance(opacity: f64, inert: bool) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(128.)),
        Field::Height(Length::Px(96.)),
        Field::Foreground(Color::Rgba(0xc80000ff)),
        Field::Opacity(opacity),
        Field::Inert(inert),
    ])]
}
async fn frame(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, ms: u64) {
    let before = window
        .update(cx, |view, window, _| {
            view.progress_clock.set_test_time(Duration::from_millis(ms));
            window.refresh();
            view.render_count
        })
        .unwrap();
    for _ in 0..200 {
        pause(cx).await;
        if window.update(cx, |view, _, _| view.render_count).unwrap() > before {
            return;
        }
    }
    panic!("progress did not render clock phase {ms}ms");
}
fn pixels(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, filled: usize, alpha: bool) {
    window.update(cx,|view,window,_| {
        let probes=view.probes.borrow();
        let bounds=probes[&id(1)].bounds;
        assert_eq!(bounds.size,size(px(128.),px(96.)));
        assert_eq!(bounds.center(),probes[&id(2)].bounds.center(),"center content layout");
        let image=window.render_to_image().expect("progress GPU readback");
        let scale=window.scale_factor();
        let check=|x:f32,y:f32,wanted:[u8;3]| {
            let x=((f32::from(bounds.origin.x)+x)*scale) as u32;
            let y=((f32::from(bounds.origin.y)+y)*scale) as u32;
            let actual=image.get_pixel(x,y).0;
            assert!(actual[..3].iter().zip(wanted).all(|(a,b)|a.abs_diff(b)<=4),
                "point({x},{y}), filled={filled}, half_alpha={alpha}: {actual:?}, expected{wanted:?}");
        };
        // Midpoints of the four clockwise quadrants, safely inside the 5px
        // annulus. A stretched ellipse or wrong sweep direction fails these.
        let offset=45.5_f32/std::f32::consts::SQRT_2;
        for (index,(x,y)) in [(offset,offset),(-offset,offset),(-offset,-offset),(offset,-offset)].into_iter().enumerate() {
            // Paths independently inherit opacity: half-alpha fill over a
            // 10%-alpha track is .5 + .1*(1-.5) = .55 of red over black.
            let red=match (index<filled,alpha) {(true,false)=>200,(false,false)=>40,(true,true)=>110,(false,true)=>20};
            check(64.+x,48.+y,[red,0,0]);
        }
        check(8.,48.,[0,0,0]);
        check(120.,48.,[0,0,0]);
        check(64.,48.,[0,0,if alpha {100}else{200}]);
    }).unwrap();
}
fn linear_pixels(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, probes: &[(f32, f32, bool)]) {
    window
        .update(cx, |view, window, _| {
            let bounds = view.probes.borrow()[&id(1)].bounds;
            assert_eq!(bounds.size, size(px(128.), px(16.)));
            let image = window
                .render_to_image()
                .expect("linear progress GPU readback");
            let scale = window.scale_factor();
            for &(x, y, filled) in probes {
                let actual = image
                    .get_pixel(
                        ((f32::from(bounds.origin.x) + x) * scale) as u32,
                        ((f32::from(bounds.origin.y) + y) * scale) as u32,
                    )
                    .0;
                let wanted = if filled { [200, 0, 0] } else { [0, 0, 0] };
                assert!(
                    actual[..3]
                        .iter()
                        .zip(wanted)
                        .all(|(a, b)| a.abs_diff(b) <= 4),
                    "linear point({x},{y}): {actual:?}, expected{wanted:?}"
                );
            }
        })
        .unwrap();
}

async fn quiet(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    for _ in 0..12 {
        pause(cx).await;
    }
    let before = window.update(cx, |view, _, _| view.render_count).unwrap();
    for _ in 0..12 {
        pause(cx).await;
    }
    assert_eq!(
        window.update(cx, |view, _, _| view.render_count).unwrap(),
        before,
        "settled/static progress sustained a native frame loop"
    );
}
#[cfg(target_os = "macos")]
async fn roles(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, visible: bool) {
    for _ in 0..200 {
        let root = crate::host::control_test::accessible_role(cx, window, "Measured circle");
        let child = crate::host::control_test::accessible_role(cx, window, "Cancel progress");
        if visible
            && root.as_deref() == Some("AXProgressIndicator")
            && child.as_deref() == Some("AXButton")
            || !visible && root.is_none() && child.is_none()
        {
            return;
        }
        pause(cx).await;
    }
    panic!("progress/center AX roles did not reach visible={visible}");
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    session: &Rc<RefCell<Session>>,
    transport: &Arc<Transport>,
) {
    let original_reduce_motion = cx.update(|cx| {
        let original = cx.reduce_motion();
        cx.set_reduce_motion(false);
        original
    });
    let window_id = WindowId::from_parts(2, 1).unwrap();
    session
        .borrow_mut()
        .open(3, window_id, "Progress pixels", 128., 96.)
        .unwrap();
    let window = cx.update(|cx| {
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(128.), px(96.)),
                    cx,
                ))),
                focus: false,
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(window_id, session.clone(), transport.clone())),
        )
        .unwrap()
    });
    window
        .update(cx, |view, _, _| {
            view.progress_clock.set_test_time(Duration::ZERO)
        })
        .unwrap();
    apply(
        cx,
        window,
        vec![
            Op::Create(id(0), Kind::Container, "".into(), None),
            Op::SetStyle(
                id(0),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(128.)),
                    Field::Height(Length::Px(96.)),
                    Field::Background(Fill::Solid(Color::Rgba(0x000000ff))),
                ])],
            ),
            Op::Create(id(1), Kind::Progress, "".into(), None),
            Op::SetProgressPresentation(id(1), config(Some(0.), Transition::Immediate)),
            Op::SetStyle(id(1), appearance(1., false)),
            Op::Create(id(2), Kind::Button, "".into(), Some(handler(1))),
            Op::SetStyle(
                id(2),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(16.)),
                    Field::Height(Length::Px(16.)),
                    Field::Background(Fill::Solid(Color::Rgba(0x0000c8ff))),
                    Field::AccessibleName("Cancel progress".into()),
                ])],
            ),
            Op::Splice(id(1), 0, 0, vec![id(2)]),
            Op::Splice(id(0), 0, 0, vec![id(1)]),
            Op::SetRoot(Some(id(0))),
        ],
    );
    for (fraction, filled) in [(0., 0), (0.25, 1), (0.5, 2), (1., 4)] {
        apply(
            cx,
            window,
            vec![Op::SetProgressPresentation(
                id(1),
                config(Some(fraction), Transition::Immediate),
            )],
        );
        frame(cx, window, 0).await;
        pixels(cx, window, filled, false);
    }
    #[cfg(target_os = "macos")]
    roles(cx, window, true).await;
    apply(
        cx,
        window,
        vec![
            Op::SetProgressPresentation(id(1), config(Some(0.25), Transition::Immediate)),
            Op::SetStyle(id(1), appearance(0.5, false)),
        ],
    );
    frame(cx, window, 0).await;
    pixels(cx, window, 1, true);
    apply(
        cx,
        window,
        vec![
            Op::SetProgressPresentation(id(1), config(Some(0.), Transition::Immediate)),
            Op::SetStyle(id(1), appearance(1., false)),
        ],
    );
    frame(cx, window, 0).await;
    apply(
        cx,
        window,
        vec![Op::SetProgressPresentation(
            id(1),
            config(
                Some(1.),
                Transition::Tween {
                    duration_ms: 1000,
                    easing: Easing::Linear,
                },
            ),
        )],
    );
    // First paint starts the interval; changing fake time before it would defer
    // the interval start and test scheduling rather than interpolation.
    frame(cx, window, 0).await;
    for (ms, filled) in [(250, 1), (500, 2), (1000, 4)] {
        frame(cx, window, ms).await;
        pixels(cx, window, filled, false);
        assert_eq!(
            session
                .borrow()
                .tree(window_id)
                .unwrap()
                .get(id(1))
                .unwrap()
                .progress
                .as_ref()
                .unwrap()
                .fraction,
            Some(1.)
        );
    }
    quiet(cx, window).await;
    apply(
        cx,
        window,
        vec![
            Op::SetProgressPresentation(id(1), config(None, Transition::Immediate)),
            Op::SetStyle(id(1), appearance(1., true)),
        ],
    );
    frame(cx, window, 2000).await;
    pixels(cx, window, 1, false);
    #[cfg(target_os = "macos")]
    roles(cx, window, false).await;
    quiet(cx, window).await;
    cx.update(|cx| cx.set_reduce_motion(true));
    apply(cx, window, vec![Op::SetStyle(id(1), appearance(1., false))]);
    frame(cx, window, 3000).await;
    pixels(cx, window, 1, false);
    #[cfg(target_os = "macos")]
    roles(cx, window, true).await;
    quiet(cx, window).await;
    // Same semantic node returns to legacy linear presentation atomically,
    // removing its center before final-tree validation. Short fills must obey
    // both root corners and their own moving-edge corners.
    apply(
        cx,
        window,
        vec![
            Op::Splice(id(1), 0, 1, vec![]),
            Op::Remove(id(2)),
            Op::SetProgress(
                id(1),
                ProgressConfig {
                    label: "Rounded bar".into(),
                    fraction: Some(0.),
                },
            ),
            Op::SetStyle(
                id(1),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(128.)),
                    Field::Height(Length::Px(16.)),
                    Field::Shrink(0.),
                    Field::TopLeftRadius(8.),
                    Field::TopRightRadius(8.),
                    Field::BottomLeftRadius(8.),
                    Field::BottomRightRadius(8.),
                    Field::Foreground(Color::Rgba(0xc80000ff)),
                    Field::Background(Fill::Solid(Color::Rgba(0x000000ff))),
                ])],
            ),
        ],
    );
    for (fraction, probes) in [
        (0., vec![(4., 8., false), (64., 8., false)]),
        (
            0.0625,
            vec![
                (4., 8., true),
                (1., 1., false),
                (7., 0., false),
                (10., 8., false),
            ],
        ),
        (
            0.5,
            vec![
                (4., 8., true),
                (60., 8., true),
                (63., 1., false),
                (70., 8., false),
            ],
        ),
        (
            1.,
            vec![(64., 8., true), (1., 1., false), (126., 1., false)],
        ),
    ] {
        apply(
            cx,
            window,
            vec![Op::SetProgress(
                id(1),
                ProgressConfig {
                    label: "Rounded bar".into(),
                    fraction: Some(fraction),
                },
            )],
        );
        frame(cx, window, 3000).await;
        linear_pixels(cx, window, &probes);
    }
    // An explicit clip ancestor keeps the backdrop black below its 8px edge;
    // the 16px progress element must retain its own layout without shrinking.
    apply(
        cx,
        window,
        vec![
            Op::Create(id(3), Kind::Container, "".into(), None),
            Op::SetStyle(
                id(3),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(128.)),
                    Field::Height(Length::Px(8.)),
                    Field::Shrink(0.),
                    Field::OverflowY(1),
                ])],
            ),
            Op::Splice(id(0), 0, 1, vec![id(3)]),
            Op::Splice(id(3), 0, 0, vec![id(1)]),
        ],
    );
    frame(cx, window, 3000).await;
    linear_pixels(cx, window, &[(64., 3., true), (64., 12., false)]);
    #[cfg(target_os = "macos")]
    assert!(crate::host::control_test::accessible_role(cx, window, "Cancel progress").is_none());
    quiet(cx, window).await;
    let driver = window
        .update(cx, |view, _, _| view.progresses[&id(1)].driver())
        .unwrap();
    apply(
        cx,
        window,
        vec![
            Op::SetRoot(None),
            Op::Splice(id(0), 0, 1, vec![]),
            Op::Splice(id(3), 0, 1, vec![]),
            Op::Remove(id(3)),
            Op::Remove(id(1)),
            Op::Remove(id(0)),
        ],
    );
    assert!(driver.sample(false).is_none());
    window
        .update(cx, |view, window, _| {
            assert!(view.progresses.is_empty());
            assert_eq!(view.session.borrow().retained_bytes(), 0);
            window.remove_window();
        })
        .unwrap();
    session.borrow_mut().close(window_id).unwrap();
    cx.update(|cx| cx.set_reduce_motion(original_reduce_motion));
    eprintln!(
        "GPUIO_PROGRESS_GPU_OK: inscribed ring/sweep/track alpha, center pixels, native tween, rounded short/full linear fill, ancestor clip, inert/reduced idle and cleanup at display scale; macOS AX roles checked={}",
        cfg!(target_os = "macos")
    );
}
