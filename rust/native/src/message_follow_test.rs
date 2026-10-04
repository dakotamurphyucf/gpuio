//! Public composition primitives on TestPlatform, without an OS window.
use super::*;
use crate::session::Session;
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpuio_protocol::{HandlerId, animation as a, list as l};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream, time::Duration};

fn n(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| {
        w.simulate_next_frame(cx);
        w.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|w, cx| {
        owner.update(cx, |v, cx| {
            let base = v.session.borrow().tree(v.id).unwrap().revision();
            let result = v
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: v.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            v.update_editors(&result.dirty, w, cx);
            v.list_actions(&result.lists, w, cx);
            cx.notify();
        });
    });
    draw(cx);
}
fn at(owner: &Entity<View>, cx: &mut VisualTestContext, ms: u64) {
    owner.update(cx, |v, cx| {
        for state in v.animations.values() {
            state
                .borrow_mut()
                .set_test_time(Some(Duration::from_millis(ms)));
        }
        cx.notify();
    });
    draw(cx);
}
fn config(generation: i64, visible: bool, jump: bool) -> a::Config {
    let mut targets = vec![a::Target {
        property: a::Property::Opacity,
        value: if visible { 1. } else { 0. },
    }];
    if jump {
        targets.push(a::Target {
            property: a::Property::Bottom,
            value: if visible { 16. } else { -48. },
        });
    }
    targets.sort_by_key(|t| t.property);
    a::Config {
        generation,
        targets,
        initial: None,
        duration_ms: 200,
        delay_ms: 0,
        easing: a::Easing::Linear,
        repeat: a::Repeat::Once,
    }
}
fn style(slot: i64, fields: Vec<Field>) -> Op {
    Op::SetStyle(n(slot), vec![Style::Fields(fields)])
}
fn gate(hidden: bool) -> Op {
    style(5, vec![Field::PointerEvents(true), Field::Inert(hidden)])
}

#[test]
fn message_follow_overlays_retain_scroll_and_settle_outside_the_input_clip() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    app.update(|cx| cx.set_reduce_motion(false));
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window, "Follow", 400., 300.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(window, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    let mut operations = (0..=6)
        .map(|i| {
            Op::Create(
                n(i),
                match i {
                    1 => Kind::VirtualList,
                    2 | 3 => Kind::Animated,
                    6 => Kind::Button,
                    _ => Kind::Container,
                },
                if i == 6 {
                    "Follow latest".into()
                } else {
                    String::new()
                },
                matches!(i, 1 | 6).then(|| HandlerId::from_parts(i, 1).unwrap()),
            )
        })
        .collect::<Vec<_>>();
    operations.extend([
        style(
            0,
            vec![
                Field::Display(1),
                Field::Direction(1),
                Field::Width(Length::Px(300.)),
                Field::Height(Length::Px(200.)),
                Field::OverflowX(2),
                Field::OverflowY(2),
            ],
        ),
        style(
            1,
            vec![
                Field::Width(Length::Px(300.)),
                Field::Height(Length::Px(200.)),
                Field::Shrink(0.),
            ],
        ),
        Op::SetListConfig(
            n(1),
            l::Config {
                estimated_height: 32.,
                overscan: 32.,
                max_active: 12,
                scroll_policy: l::ScrollPolicy::FollowTailWhenAtEnd,
                scrollbar: true,
                managed: true,
            },
        ),
        Op::SetListOrder(
            n(1),
            l::Order {
                revision: 1,
                runs: vec![l::IdRun {
                    first: 1,
                    count: 1000,
                }],
            },
        ),
        style(
            2,
            vec![
                Field::Position(1),
                Field::Left(Length::Px(0.)),
                Field::Right(Length::Px(18.)),
                Field::Bottom(Length::Px(0.)),
                Field::Height(Length::Px(48.)),
                Field::PointerEvents(false),
            ],
        ),
        style(
            3,
            vec![
                Field::OverflowX(2),
                Field::OverflowY(2),
                Field::Position(1),
                Field::Left(Length::Px(0.)),
                Field::Right(Length::Px(18.)),
                Field::Height(Length::Px(48.)),
                Field::PointerEvents(false),
            ],
        ),
        style(
            4,
            vec![
                Field::Display(1),
                Field::Direction(0),
                Field::Height(Length::Percent(100.)),
                Field::JustifyContent(4),
                Field::AlignItems(4),
            ],
        ),
        gate(true),
        style(
            6,
            vec![
                Field::Width(Length::Px(100.)),
                Field::Height(Length::Px(32.)),
            ],
        ),
        Op::SetAnimation(n(2), config(1, false, false)),
        Op::SetAnimation(n(3), config(1, false, true)),
        Op::Splice(n(0), 0, 0, vec![n(1), n(2), n(3)]),
        Op::Splice(n(3), 0, 0, vec![n(4)]),
        Op::Splice(n(4), 0, 0, vec![n(5)]),
        Op::Splice(n(5), 0, 0, vec![n(6)]),
        Op::SetRoot(Some(n(0))),
    ]);
    apply(&owner, cx, operations);
    at(&owner, cx, 0);
    apply(
        &owner,
        cx,
        vec![Op::ScrollList(
            n(1),
            l::ScrollRequest {
                serial: 1,
                target: l::ScrollTarget::Offset(500, 7.),
            },
        )],
    );
    let retained = owner.read_with(cx, |v, _| v.lists[&n(1)].clone());
    let original = retained.borrow().native.handle().viewport_bounds();
    let anchor = retained.borrow().observed.as_ref().unwrap().anchor;
    assert_eq!(anchor, Some((500, 7.)));
    apply(
        &owner,
        cx,
        vec![
            Op::SetAnimation(n(2), config(2, true, false)),
            Op::SetAnimation(n(3), config(2, true, true)),
            gate(false),
        ],
    );
    at(&owner, cx, 200);
    let button = cx
        .a11y_tree()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.role() == gpui::accesskit::Role::Button)
        .unwrap()
        .0;
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: button,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    cx.run_until_parked();
    assert!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|e| matches!(e, Event::Press(_, id, _, _) if *id == n(6)))
    );
    apply(
        &owner,
        cx,
        vec![
            Op::SetAnimation(n(2), config(3, false, false)),
            Op::SetAnimation(n(3), config(3, false, true)),
            gate(true),
        ],
    );
    owner.read_with(cx, |v, _| assert!(!v.focus.borrow().allows(n(6))));
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: button,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    cx.run_until_parked();
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|e| matches!(e, Event::Press(_, id, _, _) if *id == n(6)))
    );
    at(&owner, cx, 300);
    owner.read_with(cx, |v, _| {
        assert_eq!(
            retained.borrow().native.handle().viewport_bounds(),
            original
        );
        assert!(Rc::ptr_eq(&v.lists[&n(1)], &retained));
        assert_eq!(retained.borrow().observed.as_ref().unwrap().anchor, anchor);
        assert!(
            v.probes.borrow()[&n(3)].bounds.top() < original.bottom(),
            "outgoing control still paints during exit"
        );
    });
    at(&owner, cx, 400);
    owner.read_with(cx, |v, _| {
        assert!(v.probes.borrow()[&n(3)].bounds.top() >= original.bottom())
    });
    let requests = owner.read_with(cx, |v, _| {
        v.animations
            .values()
            .map(|s| s.borrow().frame_requests)
            .sum::<u64>()
    });
    at(&owner, cx, 1000);
    owner.read_with(cx, |v, _| {
        assert_eq!(
            v.animations
                .values()
                .map(|s| s.borrow().frame_requests)
                .sum::<u64>(),
            requests
        )
    });
    // A hidden settled overlay must not shield the transcript's bottom-center.
    let before = retained
        .borrow()
        .native
        .handle()
        .scroll_px_offset_for_scrollbar();
    cx.simulate_event(gpui::ScrollWheelEvent {
        position: original.origin + gpui::point(px(140.), px(180.)),
        delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(-25.))),
        modifiers: Default::default(),
        touch_phase: gpui::TouchPhase::Moved,
    });
    draw(cx);
    assert_ne!(
        retained
            .borrow()
            .native
            .handle()
            .scroll_px_offset_for_scrollbar(),
        before
    );
    cx.update(|_, cx| cx.set_reduce_motion(true));
    apply(
        &owner,
        cx,
        vec![
            Op::SetAnimation(n(2), config(4, true, false)),
            Op::SetAnimation(n(3), config(4, true, true)),
            gate(false),
        ],
    );
    owner.read_with(cx, |v, _| {
        assert_eq!(
            v.probes.borrow()[&n(3)].bounds.bottom(),
            original.bottom() - px(16.)
        )
    });
    apply(
        &owner,
        cx,
        vec![
            Op::SetAnimation(n(2), config(5, false, false)),
            Op::SetAnimation(n(3), config(5, false, true)),
            gate(true),
        ],
    );
    owner.read_with(cx, |v, _| {
        assert!(v.probes.borrow()[&n(3)].bounds.top() >= original.bottom())
    });
    let weak = owner.read_with(cx, |v, _| Rc::downgrade(&v.animations[&n(3)]));
    let mut removal = vec![Op::SetRoot(None)];
    removal.extend((0..=6).rev().map(|i| Op::Remove(n(i))));
    apply(&owner, cx, removal);
    assert!(weak.upgrade().is_none());
    owner.read_with(cx, |v, _| {
        assert!(v.lists.is_empty() && v.animations.is_empty())
    });
}
