//! Passive native content shares one link action while retaining its own motion.
use super::*;
use gpuio_protocol::{
    animation::{self as motion, Easing, Property, Repeat, Target},
    animation_program as program, avatar, loading,
};
use std::time::Duration;

fn target(width: f64) -> Vec<Target> {
    vec![Target {
        property: Property::Width,
        value: width,
    }]
}
fn loading_config(animated: bool) -> loading::Config {
    loading::Config {
        kind: loading::Kind::Spinner,
        label: "Passive loading".into(),
        animated,
        period_ms: 800,
    }
}
fn dimensions(width: f64, height: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(width)),
        Field::Height(Length::Px(height)),
        Field::Shrink(0.),
    ])]
}
pub(super) async fn frame(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    super::super::editor_test::frame(cx, handle).await;
    super::super::editor_test::frame(cx, handle).await;
}
async fn pause(cx: &mut gpui::AsyncApp) {
    cx.background_executor()
        .timer(Duration::from_millis(140))
        .await;
}
pub(super) async fn idle(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    frame(cx, handle).await;
    pause(cx).await;
    let before = handle.update(cx, |view, _, _| view.render_count).unwrap();
    pause(cx).await;
    assert_eq!(
        handle.update(cx, |view, _, _| view.render_count).unwrap(),
        before
    );
}

#[track_caller]
pub(super) fn events(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
    rendered: bool,
    pressed: Option<NodeId>,
) {
    let expected = handle
        .update(cx, |view, _, _| {
            let session = view.session.borrow();
            let tree = session.tree(view.id).unwrap();
            let mut expected = Vec::new();
            if rendered {
                expected.push(Event::Rendered(view.id, tree.revision()));
            }
            if let Some(node) = pressed {
                expected.push(Event::Press(
                    view.id,
                    node,
                    tree.get(node).unwrap().handler.unwrap(),
                    tree.revision(),
                ));
            }
            expected
        })
        .unwrap();
    let actual = transport.mailbox.lock().unwrap().drain(256);
    assert!(
        actual.len() < 256,
        "passive content must not flood the bridge"
    );
    assert_eq!(actual, expected, "validate the entire bridge mailbox");
}

async fn animation_geometry(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    for (millis, tween_width, program_width) in [
        (0, 80., 90.),
        (200, 100., 112.5),
        (400, 120., 135.),
        (600, 140., 157.5),
        (800, 80., 90.),
    ] {
        handle
            .update(cx, |view, window, _| {
                let now = Some(Duration::from_millis(millis));
                view.animations[&id(18)].borrow_mut().set_test_time(now);
                view.session
                    .borrow()
                    .motion()
                    .borrow_mut()
                    .set_test_time(now);
                window.refresh();
            })
            .unwrap();
        frame(cx, handle).await;
        handle
            .update(cx, |view, _, _| {
                for (slot, expected) in [(18, tween_width), (20, program_width)] {
                    let actual = f32::from(view.probes.borrow()[&id(slot)].bounds.size.width);
                    assert!(
                        (actual - expected).abs() < 0.1,
                        "passive animation {slot} at {millis}ms: expected {expected}, got {actual}"
                    );
                }
            })
            .unwrap();
    }
    handle
        .update(cx, |view, _, _| {
            view.animations[&id(18)].borrow_mut().set_test_time(None);
            view.session
                .borrow()
                .motion()
                .borrow_mut()
                .set_test_time(None);
        })
        .unwrap();
}

#[cfg(feature = "native-image-tests")]
fn avatar_pixels(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    handle
        .update(cx, |view, window, _| {
            let bounds = view.probes.borrow()[&id(16)].bounds;
            let scale = window.scale_factor();
            let image = window.render_to_image().unwrap();
            let x = (f32::from(bounds.left()) * scale) as u32;
            let y = (f32::from(bounds.top()) * scale) as u32;
            let width = (f32::from(bounds.size.width) * scale) as u32;
            let height = (f32::from(bounds.size.height) * scale) as u32;
            let mut glyph_pixels = 0;
            for row in height / 4..height * 3 / 4 {
                for col in width / 4..width * 3 / 4 {
                    let pixel = image.get_pixel(x + col, y + row).0;
                    if pixel[0] > 220 && pixel[1] > 220 && pixel[2] > 220 {
                        glyph_pixels += 1;
                    }
                }
            }
            assert!(glyph_pixels > 8, "avatar fallback must paint inside link");
        })
        .unwrap();
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    events(cx, handle, transport, false, None);
    cx.update(|cx| crate::motion_preference::set(motion::Preference::Full, cx));
    handle
        .update(cx, |view, _, _| {
            view.session
                .borrow()
                .motion()
                .borrow_mut()
                .set_test_time(Some(Duration::ZERO));
        })
        .unwrap();
    // Earlier fixture slots 10..15 have retired generations. New slots avoid
    // accidentally testing stale IDs instead of the passive-content contract.
    let animated = motion::Config {
        generation: 1,
        targets: target(160.),
        initial: Some(target(80.)),
        duration_ms: 800,
        delay_ms: 0,
        easing: Easing::Linear,
        repeat: Repeat::Loop,
    };
    let sequence = program::Config {
        generation: 1,
        playback: program::Playback::Running,
        restart: 0,
        program: program::Program {
            initial: Some(target(90.)),
            stages: vec![program::Stage {
                targets: target(180.),
                timing: program::Timing::Tween(800, Easing::Linear),
                delay_ms: 0,
            }],
            delay_ms: 0,
            repeat: Repeat::Loop,
            clock: program::Clock::Independent,
        },
    };
    apply(
        cx,
        handle,
        vec![
            Op::SetStyle(
                id(0),
                vec![Style::Fields(vec![Field::Display(1), Field::Direction(1)])],
            ),
            Op::SetStyle(id(1), vec![Style::Fields(vec![Field::Display(3)])]),
            Op::SetStyle(id(2), vec![Style::Fields(vec![Field::Display(3)])]),
            Op::SetStyle(id(3), vec![Style::Fields(vec![Field::Display(3)])]),
            Op::SetStyle(id(4), vec![Style::Fields(vec![Field::Display(3)])]),
            Op::SetStyle(id(5), dimensions(200., 40.)),
            Op::Create(id(16), Kind::Avatar, String::new(), None),
            Op::SetAvatar(
                id(16),
                avatar::Config {
                    source: None,
                    fit: gpuio_protocol::image::ImageFit::Cover,
                    label: Some("Passive avatar".into()),
                    fallback: "DM".into(),
                },
            ),
            Op::SetStyle(
                id(16),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(64.)),
                    Field::Height(Length::Px(64.)),
                    Field::Shrink(0.),
                    Field::FontSize(24.),
                    Field::Foreground(Color::Rgba(0xffffffff)),
                    Field::Background(Fill::Solid(Color::Rgba(0x334455ff))),
                ])],
            ),
            Op::Create(id(17), Kind::Loading, String::new(), None),
            Op::SetLoading(id(17), loading_config(true)),
            Op::SetStyle(id(17), dimensions(32., 32.)),
            Op::Create(id(18), Kind::Animated, String::new(), None),
            Op::SetAnimation(id(18), animated),
            Op::SetStyle(id(18), dimensions(80., 32.)),
            Op::Create(id(19), Kind::Text, "Animated detail".into(), None),
            Op::Splice(id(18), 0, 0, vec![id(19)]),
            Op::Create(id(20), Kind::AnimationProgram, String::new(), None),
            Op::SetAnimationProgram(id(20), sequence),
            Op::SetStyle(id(20), dimensions(90., 32.)),
            Op::Create(id(21), Kind::Text, "Sequence detail".into(), None),
            Op::Splice(id(20), 0, 0, vec![id(21)]),
            Op::Create(
                id(22),
                Kind::Link,
                String::new(),
                Some(gpuio_protocol::HandlerId::from_parts(22, 1).unwrap()),
            ),
            Op::SetLink(id(22), config(22, 0)),
            Op::SetStyle(
                id(22),
                vec![Style::Fields(vec![
                    Field::Display(1),
                    Field::Direction(1),
                    Field::Width(Length::Px(300.)),
                    Field::Height(Length::Px(230.)),
                    Field::Shrink(0.),
                    Field::Background(Fill::Solid(Color::Rgba(0x182638ff))),
                ])],
            ),
            Op::Create(id(23), Kind::Text, "Passive details 世界".into(), None),
            Op::Splice(id(22), 0, 0, vec![id(16), id(17), id(18), id(20), id(23)]),
            Op::Splice(id(0), 0, 0, vec![id(22)]),
        ],
    );
    handle
        .update(cx, |view, _, _| {
            view.animations[&id(18)]
                .borrow_mut()
                .set_test_time(Some(Duration::ZERO));
        })
        .unwrap();
    frame(cx, handle).await;
    events(cx, handle, transport, true, None);
    animation_geometry(cx, handle).await;
    events(cx, handle, transport, false, None);
    let (retained, loading, tween, sequence, revision) = handle
        .update(cx, |view, _, _| {
            assert_eq!(
                view.buttons.keys().copied().collect::<Vec<_>>(),
                vec![id(1), id(2), id(3), id(4), id(5), id(22)],
                "hidden earlier controls retain owners; passive children add none"
            );
            assert!(view.selections.is_empty() && view.images.is_empty());
            (
                view.buttons[&id(22)].focus.clone(),
                Rc::downgrade(&view.loading_probes[&id(17)]),
                Rc::downgrade(&view.animations[&id(18)]),
                Rc::downgrade(&view.animation_programs[&id(20)]),
                view.session.borrow().tree(view.id).unwrap().revision(),
            )
        })
        .unwrap();
    let before = loading.upgrade().unwrap().get();
    pause(cx).await;
    let after = loading.upgrade().unwrap().get();
    assert!(after.count > before.count);
    assert_ne!(after.phase, before.phase);
    events(cx, handle, transport, false, None);
    handle
        .update(cx, |view, _, _| {
            assert_eq!(
                view.session.borrow().tree(view.id).unwrap().revision(),
                revision,
                "native content motion needs no tree updates"
            );
        })
        .unwrap();
    #[cfg(feature = "native-image-tests")]
    avatar_pixels(cx, handle);
    for child in [16, 17, 19, 21] {
        focus(cx, handle, 5);
        draw(cx, handle);
        focused(cx, handle, 5);
        click(cx, handle, child);
        focused(cx, handle, 22);
        events(cx, handle, transport, false, Some(id(22)));
        apply(
            cx,
            handle,
            vec![Op::SetText(id(23), format!("Activated preview {child}"))],
        );
        frame(cx, handle).await;
        focused(cx, handle, 22);
        events(cx, handle, transport, true, None);
        #[cfg(target_os = "macos")]
        assert_eq!(
            accessible(cx, handle, 22, AxAction::InspectFocus),
            Some(true)
        );
    }
    key(cx, handle, "tab");
    focused(cx, handle, 5);
    key(cx, handle, "shift-tab");
    focused(cx, handle, 22);
    events(cx, handle, transport, false, None);
    cx.update(|cx| crate::motion_preference::set(motion::Preference::Reduce, cx));
    idle(cx, handle).await;
    events(cx, handle, transport, false, None);
    let override_label = "Updated composed name 世界";
    apply(
        cx,
        handle,
        vec![Op::SetAccessibility(
            id(22),
            Some(gpuio_protocol::accessibility::Config {
                role: None,
                label: Some(override_label.into()),
                description: Some("Opens the local guide".into()),
                live: gpuio_protocol::accessibility::Live::Off,
                current: None,
                field: None,
            }),
        )],
    );
    frame(cx, handle).await;
    events(cx, handle, transport, true, None);
    #[cfg(target_os = "macos")]
    {
        assert_eq!(
            accessible_named(cx, handle, override_label, AxAction::Inspect),
            Some(true)
        );
        assert!(accessible(cx, handle, 22, AxAction::Inspect).is_none());
        accessible_named(cx, handle, override_label, AxAction::Press);
        frame(cx, handle).await;
        events(cx, handle, transport, false, Some(id(22)));
    }
    apply(cx, handle, vec![Op::SetAccessibility(id(22), None)]);
    frame(cx, handle).await;
    events(cx, handle, transport, true, None);
    #[cfg(target_os = "macos")]
    assert_eq!(accessible(cx, handle, 22, AxAction::Inspect), Some(true));
    // Native mouse-down followed by disable must not activate on the later up.
    let center = handle
        .update(cx, |view, _, _| {
            view.probes.borrow()[&id(16)].bounds.center()
        })
        .unwrap();
    super::super::native_test::move_mouse(cx, handle, center, false);
    draw(cx, handle);
    super::super::native_test::mouse(cx, handle, center, true);
    apply(
        cx,
        handle,
        vec![Op::SetLink(
            id(22),
            Config {
                disabled: true,
                ..config(22, 0)
            },
        )],
    );
    draw(cx, handle);
    super::super::native_test::mouse(cx, handle, center, false);
    draw(cx, handle);
    events(cx, handle, transport, true, None);
    focus(cx, handle, 5);
    draw(cx, handle);
    click(cx, handle, 17);
    focused(cx, handle, 5);
    events(cx, handle, transport, false, None);
    apply(cx, handle, vec![Op::SetLink(id(22), config(22, 0))]);
    draw(cx, handle);
    events(cx, handle, transport, true, None);
    super::super::native_test::mouse(cx, handle, center, true);
    let mut remove = vec![Op::Splice(id(0), 0, 1, vec![])];
    remove.extend((16..=23).rev().map(|slot| Op::Remove(id(slot))));
    apply(cx, handle, remove);
    draw(cx, handle);
    super::super::native_test::mouse(cx, handle, center, false);
    draw(cx, handle);
    events(cx, handle, transport, true, None);
    assert!(
        loading.upgrade().is_none() && tween.upgrade().is_none() && sequence.upgrade().is_none()
    );
    let replacement = NodeId::from_parts(22, 2).unwrap();
    let text = NodeId::from_parts(23, 2).unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                replacement,
                Kind::Link,
                String::new(),
                Some(gpuio_protocol::HandlerId::from_parts(22, 2).unwrap()),
            ),
            Op::SetLink(replacement, config(22, 0)),
            Op::SetStyle(replacement, dimensions(280., 60.)),
            Op::Create(text, Kind::Text, "Replacement content".into(), None),
            Op::Splice(replacement, 0, 0, vec![text]),
            Op::Splice(id(0), 0, 0, vec![replacement]),
        ],
    );
    frame(cx, handle).await;
    events(cx, handle, transport, true, None);
    let remounted = handle
        .update(cx, |view, window, cx| {
            let focus = view.buttons[&replacement].focus.clone();
            window.focus(&focus, cx);
            focus
        })
        .unwrap();
    assert_ne!(retained, remounted, "unmount retires native focus identity");
    key(cx, handle, "enter");
    events(cx, handle, transport, false, Some(replacement));
    apply(
        cx,
        handle,
        vec![Op::SetText(text, "Updated content".into())],
    );
    frame(cx, handle).await;
    events(cx, handle, transport, true, None);
    handle
        .update(cx, |view, _, _| {
            assert_eq!(
                view.buttons[&replacement].focus, remounted,
                "content update retains root"
            );
        })
        .unwrap();
    apply(
        cx,
        handle,
        vec![
            Op::Splice(id(0), 0, 1, vec![]),
            Op::Remove(text),
            Op::Remove(replacement),
        ],
    );
    idle(cx, handle).await;
    events(cx, handle, transport, true, None);
    handle
        .update(cx, |view, _, _| {
            assert!(view.loading_probes.is_empty());
            assert!(view.animations.is_empty() && view.animation_programs.is_empty());
        })
        .unwrap();
    cx.update(|cx| crate::motion_preference::set(motion::Preference::System, cx));
    events(cx, handle, transport, false, None);
    eprintln!(
        "GPUIO_LINK_CONTENT_OK: passive avatar/loading/tween/program input, sampled native geometry, raw bridge events, reduced idle, AX override/reset, disable/unmount pointer fencing, content retirement and remount"
    );
}
