//! Retained slot/resource lifecycle on TestPlatform; no GPU/OS AX acceptance.
use super::*;
use gpui::TestAppContext;
use gpuio_protocol::{
    ResourceId,
    animation::{Config as MotionConfig, Easing, Property, Repeat, Target},
    animation_program as program,
    asset::Format,
    avatar::Config,
};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn config(source: Option<ImageSource>) -> Config {
    Config {
        source,
        fit: ImageFit::Cover,
        label: Some("Only semantic owner".into()),
        fallback: "?".into(),
    }
}
fn apply(view: &mut View, window: &mut Window, cx: &mut Context<View>, operations: Vec<Op>) {
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
        .unwrap_or_else(|error| panic!("avatar slot transaction base={base}: {error:?}"));
    view.update_editors(&applied.dirty, window, cx);
    cx.notify();
}
fn source(session: &SharedSession, format: Format, bytes: &[u8]) -> ResourceId {
    let mut session = session.borrow_mut();
    let assets = session.assets().unwrap();
    let id = assets.begin(format, bytes.len()).unwrap();
    assets.append(id, 0, bytes).unwrap();
    assets.finish(id).unwrap();
    id
}

#[test]
fn native_selection_retains_fallback_leases_and_releases_on_unmount() {
    for format in [Format::Pnm, Format::Svg] {
        retained_slot(format, Teardown::Unmount);
    }
}

#[test]
fn window_close_releases_nested_fallbacks_and_pending_motion() {
    for format in [Format::Pnm, Format::Svg] {
        retained_slot(format, Teardown::WindowClose);
    }
}

enum Teardown {
    Unmount,
    WindowClose,
}

#[test]
fn avatar_group_members_contribute_their_intrinsic_width_before_overflow() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Group", 400., 100.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(window_id, session.clone(), transport.clone()));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let mut operations = vec![];
            for slot in 0..7 {
                let kind = if matches!(slot, 0 | 1 | 5) {
                    Kind::Container
                } else {
                    Kind::Avatar
                };
                operations.push(Op::Create(id(slot), kind, "".into(), None));
            }
            for slot in [0, 1, 5] {
                operations.push(Op::SetStyle(
                    id(slot),
                    vec![Style::Fields(vec![
                        Field::Display(1),
                        Field::Direction(0),
                        Field::Shrink(0.),
                        Field::AlignItems(1),
                    ])],
                ));
            }
            operations.push(Op::SetStyle(
                id(5),
                vec![Style::Fields(vec![
                    Field::Display(1),
                    Field::Direction(0),
                    Field::Shrink(0.),
                    Field::MarginLeft(Length::Px(4.)),
                ])],
            ));
            for slot in [2, 3, 4, 6] {
                operations.push(Op::SetAvatar(id(slot), config(None)));
                operations.push(Op::SetStyle(
                    id(slot),
                    vec![Style::Fields(vec![
                        Field::Width(Length::Px(48.)),
                        Field::Height(Length::Px(48.)),
                        Field::MinWidth(Length::Px(48.)),
                        Field::MaxWidth(Length::Px(48.)),
                        Field::MinHeight(Length::Px(48.)),
                        Field::MaxHeight(Length::Px(48.)),
                        Field::Grow(0.),
                        Field::Shrink(0.),
                        Field::MarginLeft(Length::Px(if slot == 3 || slot == 4 {
                            -14.4
                        } else {
                            0.
                        })),
                        Field::Background(Fill::Solid(Color::Rgba(if slot == 6 {
                            0x0000ffff
                        } else {
                            0xff0000ff
                        }))),
                    ])],
                ));
            }
            operations.extend([
                Op::Splice(id(1), 0, 0, vec![id(2), id(3), id(4)]),
                Op::Splice(id(5), 0, 0, vec![id(6)]),
                Op::Splice(id(0), 0, 0, vec![id(1), id(5)]),
                Op::SetRoot(Some(id(0))),
            ]);
            apply(view, window, cx, operations);
        });
        window.draw(cx).clear(cx);
        let quads = window.painted_quads();
        let colored = |color| {
            let expected: gpui::Background = rgba(color).into();
            quads
                .iter()
                .filter(|quad| quad.background == expected)
                .map(|quad| quad.bounds)
                .collect::<Vec<_>>()
        };
        let members = colored(0xff0000ff);
        let overflow = colored(0x0000ffff);
        assert_eq!(members.len(), 3);
        assert_eq!(overflow.len(), 1);
        assert!(
            ((overflow[0].left() - members[2].right()).as_f32() / window.scale_factor() - 4.).abs()
                < 1.,
            "members={members:?}, overflow={overflow:?}"
        );
    });
}

fn retained_slot(format: Format, teardown: Teardown) {
    let mut app = TestAppContext::single();
    app.update(crate::image_host::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Slot", 100., 100.)
        .unwrap();
    let pixels = b"P6\n1 1\n255\n\xff\x00\x00";
    let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="red"/></svg>"#;
    let primary = source(
        &session,
        format,
        if format == Format::Svg { svg } else { pixels },
    );
    let decorative = source(&session, Format::Pnm, pixels);
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(window_id, session.clone(), transport.clone()));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(id(0), Kind::Avatar, "".into(), None),
                    Op::SetAvatar(id(0), config(None)),
                    Op::Create(id(1), Kind::Animated, "".into(), None),
                    Op::SetAnimation(
                        id(1),
                        MotionConfig {
                            generation: 1,
                            targets: vec![Target {
                                property: Property::Opacity,
                                value: 0.5,
                            }],
                            initial: Some(vec![Target {
                                property: Property::Opacity,
                                value: 1.,
                            }]),
                            duration_ms: 1000,
                            delay_ms: 10000,
                            easing: Easing::Linear,
                            repeat: Repeat::Once,
                        },
                    ),
                    Op::Create(id(2), Kind::Image, "".into(), None),
                    Op::SetImage(
                        id(2),
                        ImageConfig {
                            source: ImageSource::Reference(decorative),
                            fit: ImageFit::Contain,
                            label: Some("Decorative child".into()),
                        },
                    ),
                    Op::SetStyle(
                        id(2),
                        vec![Style::Fields(vec![
                            Field::Width(Length::Px(16.)),
                            Field::Height(Length::Px(16.)),
                        ])],
                    ),
                    Op::Create(id(3), Kind::Container, "".into(), None),
                    Op::Create(id(4), Kind::Avatar, "".into(), None),
                    Op::SetAvatar(id(4), config(None)),
                    Op::Create(id(5), Kind::AnimationProgram, "".into(), None),
                    Op::SetAnimationProgram(
                        id(5),
                        program::Config {
                            generation: 1,
                            playback: program::Playback::Running,
                            restart: 0,
                            program: program::Program {
                                initial: Some(vec![Target {
                                    property: Property::Opacity,
                                    value: 1.,
                                }]),
                                stages: vec![program::Stage {
                                    targets: vec![Target {
                                        property: Property::Opacity,
                                        value: 0.5,
                                    }],
                                    timing: program::Timing::Tween(1000, Easing::Linear),
                                    delay_ms: 0,
                                }],
                                delay_ms: 10000,
                                repeat: Repeat::Once,
                                clock: program::Clock::Independent,
                            },
                        },
                    ),
                    Op::Splice(id(5), 0, 0, vec![id(2)]),
                    Op::Splice(id(4), 0, 0, vec![id(5)]),
                    Op::Splice(id(1), 0, 0, vec![id(4)]),
                    Op::Splice(id(0), 0, 0, vec![id(1)]),
                    Op::Splice(id(3), 0, 0, vec![id(0)]),
                    Op::SetRoot(Some(id(3))),
                ],
            )
        })
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    owner.read_with(cx, |view, _| {
        assert!(view.avatar_fallbacks[&id(0)].visible);
        assert!(view.focus.borrow().visible(id(2)));
        assert!(view.images.contains_key(&id(2)));
    });
    let animation = owner.read_with(cx, |view, _| {
        let animation = view.animations[&id(1)].clone();
        assert!(animation.borrow().has_deadline());
        animation
    });
    let program = owner.read_with(cx, |view, _| {
        assert!(view.avatar_fallbacks[&id(4)].visible);
        let program = view.animation_programs[&id(5)].clone();
        assert!(program.borrow().has_deadline());
        program
    });
    // A source registration can retire while its mounted fallback still owns a
    // lease. Selecting the primary must retain that lease and the child identity.
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(decorative)
        .unwrap();
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetAvatar(
                    id(0),
                    config(Some(ImageSource::Reference(primary))),
                )],
            )
        })
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    owner.read_with(cx, |view, _| {
        assert!(!view.avatar_fallbacks[&id(0)].visible);
        assert!(!view.avatar_fallbacks[&id(4)].visible);
        assert!(!view.focus.borrow().visible(id(2)));
        assert!(view.images.contains_key(&id(2)));
        assert!(view.visited.contains(&id(2)));
    });
    assert!(!animation.borrow().has_deadline());
    assert!(!program.borrow().has_deadline());
    let requests = animation.borrow().frame_requests;
    let program_requests = program.borrow().wake_count;
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(20));
    cx.run_until_parked();
    assert_eq!(
        animation.borrow().frame_requests,
        requests,
        "hidden fallback must not wake its tween"
    );
    assert_eq!(
        program.borrow().wake_count,
        program_requests,
        "hidden nested fallback must not wake its animation program"
    );
    assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 1);
    if format == Format::Svg {
        // Size admission fails in prepaint, not during OCaml reconciliation.
        // Check the first drawn frame, then first-frame recovery at normal size.
        for (width, fallback) in [(20000., true), (48., false)] {
            cx.update(|window, cx| {
                owner.update(cx, |view, cx| {
                    apply(
                        view,
                        window,
                        cx,
                        vec![Op::SetStyle(
                            id(0),
                            vec![Style::Fields(vec![
                                Field::Width(Length::Px(width)),
                                Field::Shrink(0.),
                            ])],
                        )],
                    )
                });
                window.draw(cx).clear(cx);
            });
            owner.read_with(cx, |view, _| {
                assert_eq!(view.avatar_fallbacks[&id(0)].visible, fallback)
            });
            cx.run_until_parked();
        }
    }
    // A query update owns a different hidden set and cannot reveal the avatar's
    // dormant subtree (the original proposed reuse would lose this invariant).
    owner.update(cx, |view, _| view.sync_container_queries(&[]));
    owner.read_with(cx, |view, _| assert!(!view.focus.borrow().visible(id(2))));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetAvatar(
                    id(0),
                    config(Some(ImageSource::Unavailable(ImageError::InvalidData))),
                )],
            )
        })
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    owner.read_with(cx, |view, _| {
        assert!(view.avatar_fallbacks[&id(0)].visible);
        assert!(view.avatar_fallbacks[&id(4)].visible);
        assert!(view.focus.borrow().visible(id(2)));
        assert!(view.images.contains_key(&id(2)));
        assert!(Rc::ptr_eq(&animation, &view.animations[&id(1)]));
        assert!(animation.borrow().has_deadline());
        assert!(Rc::ptr_eq(&program, &view.animation_programs[&id(5)]));
        assert!(program.borrow().has_deadline());
    });
    for (name, fields, painted) in [
        ("inert ancestor", vec![Field::Inert(true)], true),
        ("hidden ancestor", vec![Field::Display(3)], false),
        ("restored ancestor", vec![], true),
    ] {
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                apply(
                    view,
                    window,
                    cx,
                    vec![Op::SetStyle(id(3), vec![Style::Fields(fields)])],
                )
            });
            window.draw(cx).clear(cx);
        });
        cx.run_until_parked();
        owner.read_with(cx, |view, _| {
            assert_eq!(
                view.avatar_fallbacks[&id(0)].visible,
                painted,
                "{name}: paint eligibility must differ from interaction"
            );
            assert!(Rc::ptr_eq(&animation, &view.animations[&id(1)]));
            assert_eq!(
                view.avatar_fallbacks[&id(4)].visible,
                painted,
                "{name}: nested slot"
            );
            assert!(Rc::ptr_eq(&program, &view.animation_programs[&id(5)]));
        });
        assert_eq!(
            animation.borrow().has_deadline(),
            name == "restored ancestor",
            "{name}: clock eligibility"
        );
        assert_eq!(
            program.borrow().has_deadline(),
            name == "restored ancestor",
            "{name}: nested program clock eligibility"
        );
    }
    let weak_animation = Rc::downgrade(&animation);
    let weak_program = Rc::downgrade(&program);
    drop(animation);
    drop(program);
    match teardown {
        Teardown::Unmount => {
            cx.update(|window, cx| {
                owner.update(cx, |view, cx| {
                    apply(
                        view,
                        window,
                        cx,
                        vec![
                            Op::SetRoot(None),
                            Op::Remove(id(2)),
                            Op::Remove(id(1)),
                            Op::Remove(id(0)),
                            Op::Remove(id(3)),
                            Op::Remove(id(4)),
                            Op::Remove(id(5)),
                        ],
                    )
                })
            });
            cx.run_until_parked();
            owner.read_with(cx, |view, _| {
                assert!(view.avatar_fallbacks.is_empty());
                assert!(view.images.is_empty());
                assert!(view.animation_programs.is_empty());
            });
        }
        Teardown::WindowClose => {
            let weak_owner = owner.downgrade();
            // Match production ordering: Session retires the tree and motion
            // declarations, then GPUI removes the window. Do not unmount nodes
            // or shut down the application-wide image service first.
            session.borrow_mut().close(window_id).unwrap();
            cx.update(|window, _| window.remove_window());
            drop(owner);
            cx.cx.update(|_| ());
            cx.run_until_parked();
            assert!(weak_owner.upgrade().is_none());
            assert_eq!(session.borrow().motion().borrow().counts(), (0, 0, 0));
            assert!(session.borrow().tree(window_id).is_none());
            assert_eq!(session.borrow().retained_bytes(), 0);
            transport.mailbox.lock().unwrap().drain(128);
            cx.executor()
                .advance_clock(std::time::Duration::from_secs(20));
            cx.run_until_parked();
            assert!(
                transport.mailbox.lock().unwrap().drain(128).is_empty(),
                "closed window must not deliver delayed image/motion events"
            );
        }
    }
    assert!(weak_animation.upgrade().is_none());
    assert!(weak_program.upgrade().is_none());
    assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 0);
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(primary)
        .unwrap();
    // Global cleanup follows per-window assertions, so it cannot hide leaks.
    cx.cx.update(crate::image_host::finish_before_quit);
}
