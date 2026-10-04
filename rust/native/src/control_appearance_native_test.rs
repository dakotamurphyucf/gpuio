//! Physical GPU/AX checks in native_image_views; compiling is not acceptance.
use super::*;
use gpuio_protocol::control_appearance::Config;

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn control(kind: Kind) -> NodeId {
    let generation = match kind {
        Kind::Checkbox => 1,
        Kind::Switch => 2,
        Kind::RadioGroup => 3,
        _ => unreachable!(),
    };
    NodeId::from_parts(1, generation).unwrap()
}
fn appearance(size: f64, opacity: f64) -> Config {
    Config {
        size,
        switch_width: size * 2.,
        indicator_style: vec![
            Style::Fields(vec![
                Field::Background(Fill::Solid(Color::Rgba(0xc80000ff))),
                Field::Foreground(Color::Rgba(0xc80000ff)),
                Field::Opacity(opacity),
            ]),
            Style::State(
                6,
                vec![
                    Field::Background(Fill::Solid(Color::Rgba(0x0000c8ff))),
                    Field::Foreground(Color::Rgba(0x0000c8ff)),
                ],
            ),
        ],
        mark_style: vec![Style::Fields(vec![
            Field::Foreground(Color::Rgba(0x00dc00ff)),
            Field::Opacity(opacity),
        ])],
        ..Config::default()
    }
}
fn style(opacity: f64, disabled: bool) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Position(1),
        Field::Left(Length::Px(24.)),
        Field::Top(Length::Px(24.)),
        Field::Width(Length::Px(420.)),
        Field::Height(Length::Px(160.)),
        Field::Display(1),
        Field::Direction(0),
        Field::AlignItems(0),
        // Keep the label line shorter than the smallest indicator so its
        // center is independent of platform font ascent in pixel assertions.
        Field::FontSize(4.),
        Field::LineHeight(Length::Px(4.)),
        Field::Foreground(Color::Rgba(0x00dc00ff)),
        Field::Opacity(opacity),
        Field::Disabled(disabled),
        Field::AccessibleName("Appearance control".into()),
    ])]
}
fn value(kind: Kind, state: CheckState) -> Op {
    match kind {
        Kind::Checkbox => Op::SetControl(control(kind), Control::Checkbox(state, false)),
        Kind::Switch => Op::SetControl(
            control(kind),
            Control::Switch(state == CheckState::Checked, false),
        ),
        Kind::RadioGroup => Op::SetChoice(
            control(kind),
            ChoiceConfig {
                label: "Appearance control".into(),
                items: vec![ChoiceItem {
                    id: "one".into(),
                    label: "Choice".into(),
                    disabled: false,
                }],
                selected: (state == CheckState::Checked).then(|| "one".into()),
                disabled: false,
            },
        ),
        _ => unreachable!(),
    }
}
async fn frame(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    let before = handle
        .update(cx, |view, window, _| {
            window.refresh();
            view.render_count
        })
        .unwrap();
    for _ in 0..200 {
        pause(cx).await;
        if handle.update(cx, |view, _, _| view.render_count).unwrap() > before {
            return;
        }
    }
    panic!("control appearance did not paint within five seconds");
}
fn pixels(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    kind: Kind,
    extent: f64,
    state: CheckState,
    alpha: bool,
    disabled: bool,
) {
    handle.update(cx, |view, window, _| {
        let bounds = view.probes.borrow()[&control(kind)].bounds;
        let origin = bounds.origin + gpui::point(px(if kind == Kind::RadioGroup {6.} else {0.}), px(if kind == Kind::RadioGroup {6.} else {0.}));
        let image = window.render_to_image().expect("control indicator GPU readback");
        let scale = window.scale_factor();
        let sample = |x:f32,y:f32| image.get_pixel(
            ((f32::from(origin.x)+x)*scale) as u32,
            ((f32::from(origin.y)+y)*scale) as u32).0;
        let s = extent as f32;
        let track = if disabled {[0,0,200]} else if alpha {[50,0,0]} else {[200,0,0]};
        let check = |x,y,wanted:[u8;3]| {
            let actual = sample(x,y);
            assert!(actual[..3].iter().zip(wanted).all(|(a,b)| a.abs_diff(b)<=4),
                "{kind:?}/{state:?}, size{extent}, alpha{alpha}, disabled{disabled}, ({x},{y}): {actual:?} expected {wanted:?}");
        };
        let mark = if alpha {[44,28,0]} else {[0,220,0]};
        match kind {
            Kind::Switch => {
                let checked=state==CheckState::Checked;
                check(s*0.5,s*0.5,if checked {track} else {mark});
                check(s*1.5,s*0.5,if checked {mark} else {track});
                check(s*2.+3.,s*0.5,[0,0,0]);
            }
            Kind::RadioGroup => {
                check(s*0.5,s*0.5,if state==CheckState::Checked {mark} else {track});
                check(s*0.18,s*0.5,track);
            }
            Kind::Checkbox => {
                check(s*0.75,s*0.18,track);
                // At the smallest size the check/dash is subpixel. Require
                // nonzero green coverage rather than a fully opaque center pixel.
                let mut green=0;
                for y in 0..(s*scale) as u32 {
                    for x in 0..(s*scale) as u32 {
                        let color=sample(x as f32/scale,y as f32/scale);
                        if color[1] > 15 {green+=1;}
                    }
                }
                assert_eq!(green>0,state!=CheckState::Unchecked,"{state:?} mark coverage at size{extent}");
                check(s+3.,s*0.5,[0,0,0]);
            }
            _=>unreachable!(),
        }
    }).unwrap();
}
async fn quiet(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    for _ in 0..8 {
        pause(cx).await;
    }
    let before = handle.update(cx, |view, _, _| view.render_count).unwrap();
    for _ in 0..8 {
        pause(cx).await;
    }
    assert_eq!(
        handle.update(cx, |view, _, _| view.render_count).unwrap(),
        before,
        "static control sustained a frame loop"
    );
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    session: &Rc<RefCell<Session>>,
    transport: &Arc<Transport>,
) {
    let window_id = WindowId::from_parts(3, 1).unwrap();
    session
        .borrow_mut()
        .open(4, window_id, "Control appearance pixels", 480., 240.)
        .unwrap();
    let window = cx.update(|cx| {
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(480.), px(240.)),
                    cx,
                ))),
                focus: false,
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(window_id, session.clone(), transport.clone())),
        )
        .unwrap()
    });
    eprintln!("GPUIO_CONTROL_APPEARANCE_PHASE: background window opened");
    apply(
        cx,
        window,
        vec![
            Op::Create(id(0), Kind::Container, "".into(), None),
            Op::SetStyle(
                id(0),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(480.)),
                    Field::Height(Length::Px(240.)),
                    Field::Background(Fill::Solid(Color::Rgba(0x000000ff))),
                ])],
            ),
            Op::SetRoot(Some(id(0))),
        ],
    );
    let mut cases = 0;
    for kind in [Kind::Checkbox, Kind::Switch, Kind::RadioGroup] {
        apply(
            cx,
            window,
            vec![
                Op::Create(control(kind), kind, "".into(), Some(handler(1))),
                value(kind, CheckState::Unchecked),
                Op::SetStyle(control(kind), style(1., false)),
                Op::Splice(id(0), 0, 0, vec![control(kind)]),
            ],
        );
        frame(cx, window).await;
        let owner = window
            .update(cx, |view, _, _| view.buttons[&control(kind)].clone())
            .unwrap();
        for extent in [8., 18., 128.] {
            for state in [
                CheckState::Unchecked,
                CheckState::Checked,
                CheckState::Indeterminate,
            ] {
                if kind != Kind::Checkbox && state == CheckState::Indeterminate {
                    continue;
                }
                apply(
                    cx,
                    window,
                    vec![
                        value(kind, state),
                        Op::SetControlAppearance(control(kind), Some(appearance(extent, 1.))),
                    ],
                );
                frame(cx, window).await;
                pixels(cx, window, kind, extent, state, false, false);
                cases += 1;
            }
        }
        #[cfg(target_os = "macos")]
        {
            let expected = if kind == Kind::RadioGroup {
                "AXRadioGroup"
            } else {
                "AXCheckBox"
            };
            let mut observed = None;
            for _ in 0..200 {
                observed =
                    crate::host::control_test::accessible_role(cx, window, "Appearance control");
                if observed.as_deref() == Some(expected) {
                    break;
                }
                pause(cx).await;
            }
            assert_eq!(observed.as_deref(), Some(expected));
        }
        // Radio and thumb centers provide an antialias-free opacity sample.
        if kind != Kind::Checkbox {
            apply(
                cx,
                window,
                vec![
                    value(kind, CheckState::Checked),
                    Op::SetStyle(control(kind), style(0.5, false)),
                    Op::SetControlAppearance(control(kind), Some(appearance(32., 0.5))),
                ],
            );
            frame(cx, window).await;
            pixels(cx, window, kind, 32., CheckState::Checked, true, false);
        }
        apply(
            cx,
            window,
            vec![
                value(kind, CheckState::Unchecked),
                Op::SetStyle(control(kind), style(1., true)),
                Op::SetControlAppearance(control(kind), Some(appearance(32., 1.))),
            ],
        );
        frame(cx, window).await;
        pixels(cx, window, kind, 32., CheckState::Unchecked, false, true);
        window
            .update(cx, |view, _, _| {
                assert!(Rc::ptr_eq(&owner, &view.buttons[&control(kind)]));
                assert!(!view.focus.borrow().allows(control(kind)));
            })
            .unwrap();
        apply(
            cx,
            window,
            vec![
                Op::SetStyle(control(kind), style(1., false)),
                Op::SetControlAppearance(control(kind), None),
            ],
        );
        frame(cx, window).await;
        window
            .update(cx, |view, _, _| {
                assert!(Rc::ptr_eq(&owner, &view.buttons[&control(kind)]));
                assert!(view.focus.borrow().allows(control(kind)));
                assert!(
                    view.session
                        .borrow()
                        .tree(view.id)
                        .unwrap()
                        .get(control(kind))
                        .unwrap()
                        .control_appearance
                        .is_none()
                );
            })
            .unwrap();
        window
            .update(cx, |view, window, _| {
                let bounds = view.probes.borrow()[&control(kind)].bounds;
                let offset = if kind == Kind::RadioGroup { 6. } else { 0. };
                let scale = window.scale_factor();
                let image = window
                    .render_to_image()
                    .expect("default appearance readback");
                let width = if kind == Kind::Switch { 30. } else { 18. };
                let x = (f32::from(bounds.origin.x) + offset + width + 2.) * scale;
                let y = (f32::from(bounds.origin.y) + offset + 9.) * scale;
                assert_eq!(
                    &image.get_pixel(x as u32, y as u32).0[..3],
                    &[0, 0, 0],
                    "default dimensions reset"
                );
            })
            .unwrap();
        if kind == Kind::Checkbox {
            let mut child_style = style(1., false);
            child_style.push(Style::Fields(vec![
                Field::Position(0),
                Field::Left(Length::Px(0.)),
                Field::Top(Length::Px(0.)),
            ]));
            apply(
                cx,
                window,
                vec![
                    Op::Create(id(2), Kind::Container, "".into(), None),
                    Op::SetStyle(
                        id(2),
                        vec![Style::Fields(vec![
                            Field::Position(1),
                            Field::Left(Length::Px(24.)),
                            Field::Top(Length::Px(24.)),
                            Field::Width(Length::Px(16.)),
                            Field::Height(Length::Px(32.)),
                            Field::OverflowX(1),
                            Field::OverflowY(1),
                        ])],
                    ),
                    Op::SetStyle(control(kind), child_style),
                    Op::SetControlAppearance(control(kind), Some(appearance(32., 1.))),
                    Op::Splice(id(0), 0, 1, vec![id(2)]),
                    Op::Splice(id(2), 0, 0, vec![control(kind)]),
                ],
            );
            frame(cx, window).await;
            window
                .update(cx, |view, window, _| {
                    let bounds = view.probes.borrow()[&id(2)].bounds;
                    let scale = window.scale_factor();
                    let image = window
                        .render_to_image()
                        .expect("clipped indicator GPU readback");
                    for (x, expected) in [(8., [200, 0, 0]), (24., [0, 0, 0])] {
                        let pixel = image
                            .get_pixel(
                                ((f32::from(bounds.origin.x) + x) * scale) as u32,
                                ((f32::from(bounds.origin.y) + 16.) * scale) as u32,
                            )
                            .0;
                        assert!(
                            pixel[..3]
                                .iter()
                                .zip(expected)
                                .all(|(a, b)| a.abs_diff(b) <= 4),
                            "ancestor clipping at{x}: {pixel:?}"
                        );
                    }
                })
                .unwrap();
            apply(
                cx,
                window,
                vec![
                    Op::Splice(id(2), 0, 1, vec![]),
                    Op::Splice(id(0), 0, 1, vec![control(kind)]),
                    Op::Remove(id(2)),
                    Op::SetStyle(control(kind), style(1., false)),
                ],
            );
            frame(cx, window).await;
        }
        quiet(cx, window).await;
        apply(
            cx,
            window,
            vec![Op::Splice(id(0), 0, 1, vec![]), Op::Remove(control(kind))],
        );
        frame(cx, window).await;
        window
            .update(cx, |view, _, _| {
                assert!(view.buttons.is_empty() && view.radios.is_empty())
            })
            .unwrap();
        #[cfg(target_os = "macos")]
        assert!(
            crate::host::control_test::accessible_role(cx, window, "Appearance control").is_none()
        );
    }
    apply(cx, window, vec![Op::SetRoot(None), Op::Remove(id(0))]);
    frame(cx, window).await;
    quiet(cx, window).await;
    assert_eq!(
        session.borrow().tree(window_id).unwrap().retained_bytes(),
        0
    );
    window
        .update(cx, |_, window, _| window.remove_window())
        .unwrap();
    session.borrow_mut().close(window_id).unwrap();
    eprintln!(
        "GPUIO_CONTROL_APPEARANCE_GPU_OK: {cases} size/value cases, mark coverage, thumb endpoints, nested alpha, ancestor clipping, disabled parts, native owner retention, reset and idle/teardown; macOS AX roles checked={}",
        cfg!(target_os = "macos")
    );
}
