//! Physical GPU readback through native_images. Compilation is not acceptance.
use super::*;
use gpuio_protocol::{animation::Easing, spinner};

fn root() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn indicator() -> NodeId {
    NodeId::from_parts(1, 1).unwrap()
}
fn appearance(opacity: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(96.)),
        Field::Height(Length::Px(96.)),
        Field::Opacity(opacity),
    ])]
}
async fn frame(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, ms: u64) {
    let previous = window
        .update(cx, |view, window, _| {
            view.spinner_clock.set_test_time(Duration::from_millis(ms));
            window.refresh();
            view.render_count
        })
        .unwrap();
    for _ in 0..200 {
        pause(cx).await;
        if window.update(cx, |view, _, _| view.render_count).unwrap() > previous {
            return;
        }
    }
    panic!("spinner did not render requested clock phase {ms}ms");
}
async fn decoded(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    for _ in 0..200 {
        let ready = window
            .update(cx, |view, _, _| {
                let Some(state) = view.images.get(&indicator()) else {
                    return false;
                };
                let binding = state.binding.borrow();
                matches!(state.emitted, Some((_, ImageState::Ready(_))))
                    && binding.requested == Some(binding.rendered)
                    && binding.pending.is_none()
                    && binding.resize_error.is_none()
            })
            .unwrap();
        if ready {
            frame(cx, window, 0).await;
            return;
        }
        pause(cx).await;
    }
    panic!("spinner SVG did not reach measured ready state");
}
fn pixels_at_phase(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    quarter: usize,
    expected: [u8; 3],
) {
    window
        .update(cx, |view, window, _| {
            let bounds = view.probes.borrow()[&indicator()].bounds;
            assert_eq!(bounds.size, size(px(96.), px(96.)));
            let image = window.render_to_image().expect("spinner GPU readback");
            let scale = window.scale_factor();
            // Independent interior sample positions for a right-side rectangle
            // rotated clockwise about the center. Source RGB is red; paint is green.
            for (index, (x, y)) in [(72., 48.), (48., 72.), (24., 48.), (48., 24.)]
                .into_iter()
                .enumerate()
            {
                let x = ((f32::from(bounds.origin.x) + x) * scale) as u32;
                let y = ((f32::from(bounds.origin.y) + y) * scale) as u32;
                let wanted = if index == quarter {
                    expected
                } else {
                    [0, 0, 0]
                };
                let actual = image.get_pixel(x, y).0;
                assert!(
                    actual[..3]
                        .iter()
                        .zip(wanted)
                        .all(|(a, b)| a.abs_diff(b) <= 3),
                    "quarter {quarter}, probe {index}: {actual:?}, expected {wanted:?}"
                );
            }
        })
        .unwrap();
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    session: &Rc<RefCell<Session>>,
    transport: &Arc<Transport>,
) {
    let id = WindowId::from_parts(1, 1).unwrap();
    session
        .borrow_mut()
        .open(2, id, "Spinner pixels", 96., 96.)
        .unwrap();
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect x="40" y="24" width="16" height="16" fill="red"/></svg>"#;
    let source = {
        let mut session = session.borrow_mut();
        let store = session.assets().unwrap();
        let source = store.begin(Format::Svg, svg.len()).unwrap();
        store.append(source, 0, svg).unwrap();
        store.finish(source).unwrap();
        source
    };
    let window = cx.update(|cx| {
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(96.), px(96.)),
                    cx,
                ))),
                focus: false,
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(id, session.clone(), transport.clone())),
        )
        .unwrap()
    });
    window
        .update(cx, |view, _, _| {
            view.spinner_clock.set_test_time(Duration::ZERO);
        })
        .unwrap();
    let mut config = spinner::Config {
        label: "Rotating mask".into(),
        animated: true,
        period_ms: 1000,
        easing: Easing::Linear,
        source: Some(ImageSource::Reference(source)),
    };
    apply(
        cx,
        window,
        vec![
            Op::Create(root(), Kind::Container, "".into(), None),
            Op::Create(indicator(), Kind::Loading, "".into(), Some(handler(1))),
            Op::SetStyle(
                root(),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(96.)),
                    Field::Height(Length::Px(96.)),
                    Field::Background(Fill::Solid(Color::Rgba(0x000000ff))),
                    Field::Foreground(Color::Rgba(0x0adc1eff)),
                ])],
            ),
            Op::SetStyle(indicator(), appearance(1.)),
            Op::SetSpinner(indicator(), config.clone()),
            Op::Splice(root(), 0, 0, vec![indicator()]),
            Op::SetRoot(Some(root())),
        ],
    );
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(source)
        .unwrap();
    decoded(cx, window).await;
    for quarter in 0..4 {
        frame(cx, window, quarter as u64 * 250).await;
        pixels_at_phase(cx, window, quarter, [10, 220, 30]);
    }
    // Label updates preserve phase; restyling inherits tint and multiplies alpha.
    config.label = "Still rotating".into();
    apply(
        cx,
        window,
        vec![
            Op::SetSpinner(indicator(), config.clone()),
            Op::SetStyle(indicator(), appearance(0.5)),
        ],
    );
    frame(cx, window, 750).await;
    pixels_at_phase(cx, window, 3, [5, 110, 15]);
    // Explicit static mode resets to zero; pending native wakes must drain.
    config.animated = false;
    apply(cx, window, vec![Op::SetSpinner(indicator(), config)]);
    frame(cx, window, 1000).await;
    pixels_at_phase(cx, window, 0, [5, 110, 15]);
    for _ in 0..12 {
        pause(cx).await;
    }
    let renders = window.update(cx, |view, _, _| view.render_count).unwrap();
    for _ in 0..12 {
        pause(cx).await;
    }
    assert_eq!(
        window.update(cx, |view, _, _| view.render_count).unwrap(),
        renders,
        "static spinner must not sustain a native frame loop"
    );
    let driver = window
        .update(cx, |view, _, _| view.spinners[&indicator()].driver())
        .unwrap();
    apply(
        cx,
        window,
        vec![
            Op::SetRoot(None),
            Op::Splice(root(), 0, 1, vec![]),
            Op::Remove(indicator()),
            Op::Remove(root()),
        ],
    );
    assert!(driver.sample(false).is_none());
    window
        .update(cx, |view, window, _| {
            assert!(view.spinners.is_empty() && view.images.is_empty());
            assert_eq!(view.session.borrow().retained_bytes(), 0);
            window.remove_window();
        })
        .unwrap();
    assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 0);
    eprintln!(
        "GPUIO_SPINNER_GPU_OK: four mask rotations, inherited tint/alpha, label phase, static idle and teardown at native display scale"
    );
}
