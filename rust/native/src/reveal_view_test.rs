use super::*;
use crate::{session::Session, transport::Transport};
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpuio_protocol::{animation::Spring, reveal::Config};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream, time::Duration};

fn transaction(hex: &str) -> Transaction {
    let hex = hex.trim();
    let bytes: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let Message::Apply(tx) = gpuio_protocol::decode(&bytes).unwrap() else {
        panic!("Apply")
    };
    tx
}
fn config(expanded: bool, retain: bool) -> Config {
    Config {
        expanded,
        retain,
        spring: Spring {
            stiffness: 400.,
            damping: 40.,
            mass: 1.,
            epsilon: 0.1,
            max_duration_ms: 2000,
        },
    }
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, ops: Vec<Op>) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let result = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: view.id,
                    base,
                    revision: base + 1,
                    operations: ops,
                })
                .unwrap();
            view.update_editors(&result.dirty, window, cx);
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}
fn tick(cx: &mut VisualTestContext, ms: u64) {
    cx.executor().advance_clock(Duration::from_millis(ms));
    cx.update(|window, cx| {
        window.simulate_next_frame(cx);
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
}

#[test]
fn retained_close_is_measured_but_inert_and_reversal_keeps_editor_identity() {
    let mut initial = transaction(include_str!("../../../test/fixtures/disclosure-rich-0.hex"));
    let mut closed = transaction(include_str!("../../../test/fixtures/disclosure-rich-1.hex"));
    let mut reopened = transaction(include_str!("../../../test/fixtures/disclosure-rich-2.hex"));
    let find = |kind, text: &str| {
        initial
            .operations
            .iter()
            .find_map(|op| match op {
                Op::Create(id, k, label, _) if *k == kind && label == text => Some(*id),
                _ => None,
            })
            .unwrap()
    };
    let panel = find(Kind::Panel, "first");
    let button = find(Kind::Button, "first");
    let editor = initial
        .operations
        .iter()
        .find_map(|op| match op {
            Op::SetEditor(id, _) => Some(*id),
            _ => None,
        })
        .unwrap();
    initial
        .operations
        .push(Op::SetReveal(panel, Some(config(true, true))));
    closed
        .operations
        .push(Op::SetReveal(panel, Some(config(false, true))));
    reopened
        .operations
        .push(Op::SetReveal(panel, Some(config(true, true))));
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_read, write) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(write.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, initial.window, "Reveal", 800., 600.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(initial.window, session.clone(), transport.clone()));
    cx.update(|window, cx| {
        window.activate_window();
        cx.set_reduce_motion(false);
    });
    cx.simulate_a11y_active(true);
    apply(&owner, cx, initial.operations);
    let shared = cx.update(|_, cx| owner.read_with(cx, |view, _| view.reveals[&panel].clone()));
    let natural = shared.borrow().measurement.unwrap();
    assert!(natural.bounds.size.height > px(0.));
    let editor_focus = cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let focus = view.editors[&editor].focus_handle(cx);
            window.focus(&focus, cx);
            focus
        })
    });
    cx.simulate_keystrokes("x");
    let draft = cx.update(|window, cx| {
        owner.read_with(cx, |view, cx| {
            view.editors[&editor].snapshot(window, cx).text
        })
    });
    apply(&owner, cx, closed.operations);
    assert!(shared.borrow().motion.is_animating());
    tick(cx, 40);
    let partial = shared.borrow().measurement.unwrap();
    assert!(
        partial.bounds.size.height > px(0.)
            && partial.bounds.size.height < natural.bounds.size.height
    );
    assert_eq!(partial.natural, natural.natural);
    cx.update(|window, cx| {
        owner.read_with(cx, |view, cx| {
            assert!(view.buttons[&button].focus.is_focused(window));
            assert!(!view.focus.borrow().visible(editor));
            assert_eq!(view.editors[&editor].focus_handle(cx), editor_focus);
            assert_eq!(view.editors[&editor].snapshot(window, cx).text, draft);
        })
    });
    let accessibility = cx.a11y_tree().unwrap();
    let (editor_ax, _) = accessibility
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Accordion draft"))
        .unwrap();
    let mut cursor = *editor_ax;
    loop {
        let (_, node) = accessibility
            .nodes
            .iter()
            .find(|(id, _)| *id == cursor)
            .unwrap();
        if node.is_hidden() {
            break;
        }
        cursor = accessibility
            .nodes
            .iter()
            .find(|(_, parent)| parent.children().contains(&cursor))
            .expect("outgoing editor must have a hidden accessibility ancestor")
            .0;
    }
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Focus,
        target_tree: gpui::accesskit::TreeId::ROOT,
        target_node: *editor_ax,
        data: None,
    });
    cx.simulate_click(
        partial.bounds.origin + gpui::point(px(8.), px(8.)),
        Default::default(),
    );
    cx.simulate_keystrokes("y");
    cx.update(|window, cx| {
        owner.read_with(cx, |view, cx| {
            assert!(view.buttons[&button].focus.is_focused(window));
            assert_eq!(view.editors[&editor].snapshot(window, cx).text, draft);
        })
    });
    apply(&owner, cx, reopened.operations);
    assert_eq!(
        shared.borrow().measurement.unwrap().bounds.size.height,
        partial.bounds.size.height
    );
    tick(cx, 2100);
    assert!(!shared.borrow().motion.is_animating());
    assert!(shared.borrow().pending.is_none());
    assert_eq!(
        shared.borrow().measurement.unwrap().bounds.size,
        natural.bounds.size
    );
    cx.update(|window, cx| {
        owner.read_with(cx, |view, cx| {
            assert_eq!(view.editors[&editor].focus_handle(cx), editor_focus);
            assert_eq!(view.editors[&editor].snapshot(window, cx).text, draft);
        })
    });
    // Removing the configuration cancels pending weak work without destroying
    // the panel or retained editor.
    apply(&owner, cx, vec![Op::SetReveal(panel, None)]);
    cx.update(|_, cx| owner.read_with(cx, |view, _| assert!(!view.reveals.contains_key(&panel))));
}

#[test]
fn unmount_reopen_reduced_motion_inactivity_and_removal_retire_native_work() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_read, write) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(write.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    let id = |slot| NodeId::from_parts(slot, 1).unwrap();
    let panel = id(1);
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "Lifetime", 400., 300.)
        .unwrap();
    let (owner, cx) = app.add_window_view(|_, _| View::new(wid, session.clone(), transport));
    cx.update(|window, cx| {
        window.activate_window();
        cx.set_reduce_motion(false);
    });
    apply(
        &owner,
        cx,
        vec![
            Op::Create(id(0), Kind::Container, "".into(), None),
            Op::Create(panel, Kind::Panel, "Details".into(), None),
            Op::Create(id(2), Kind::Text, "Content".into(), None),
            Op::SetStyle(
                id(2),
                vec![Style::Height(gpuio_protocol::v1::Length::Px(120.))],
            ),
            Op::SetStyle(
                id(0),
                vec![Style::Width(gpuio_protocol::v1::Length::Px(320.))],
            ),
            Op::Splice(panel, 0, 0, vec![id(2)]),
            Op::Splice(id(0), 0, 0, vec![panel]),
            Op::SetRoot(Some(id(0))),
            Op::SetReveal(panel, Some(config(true, false))),
        ],
    );
    let shared = cx.update(|_, cx| owner.read_with(cx, |view, _| view.reveals[&panel].clone()));
    apply(
        &owner,
        cx,
        vec![
            Op::Splice(panel, 0, 1, vec![]),
            Op::Remove(id(2)),
            Op::SetStyle(panel, vec![Style::Fields(vec![Field::Display(3)])]),
            Op::SetReveal(panel, Some(config(false, false))),
        ],
    );
    assert!(session.borrow().tree(wid).unwrap().get(id(2)).is_none());
    assert!(!shared.borrow().motion.is_animating());
    assert!(shared.borrow().pending.is_none());
    assert!(shared.borrow().measurement.unwrap().natural.is_none());
    let replacement = NodeId::from_parts(2, 2).unwrap();
    apply(
        &owner,
        cx,
        vec![
            Op::Create(replacement, Kind::Text, "New content".into(), None),
            Op::SetStyle(
                replacement,
                vec![Style::Height(gpuio_protocol::v1::Length::Px(120.))],
            ),
            Op::Splice(panel, 0, 0, vec![replacement]),
            Op::SetStyle(panel, vec![]),
            Op::SetReveal(panel, Some(config(true, false))),
        ],
    );
    assert_eq!(
        shared.borrow().measurement.unwrap().bounds.size.height,
        px(0.)
    );
    assert!(shared.borrow().pending.is_some());
    tick(cx, 40);
    assert!(shared.borrow().measurement.unwrap().bounds.size.height > px(0.));
    cx.update(|_, cx| cx.set_reduce_motion(true));
    tick(cx, 16);
    assert!(!shared.borrow().motion.is_animating());
    assert!(shared.borrow().pending.is_none());
    assert_eq!(
        shared.borrow().measurement.unwrap().bounds.size.height,
        px(120.)
    );
    cx.update(|_, cx| cx.set_reduce_motion(false));
    apply(
        &owner,
        cx,
        vec![Op::SetReveal(panel, Some(config(true, true)))],
    );
    apply(
        &owner,
        cx,
        vec![
            Op::SetStyle(panel, vec![Style::Fields(vec![Field::Display(3)])]),
            Op::SetReveal(panel, Some(config(false, true))),
        ],
    );
    assert!(shared.borrow().pending.is_some());
    cx.deactivate_window();
    assert!(!shared.borrow().motion.is_animating());
    assert!(shared.borrow().pending.is_none());
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    apply(
        &owner,
        cx,
        vec![
            Op::SetStyle(panel, vec![]),
            Op::SetReveal(panel, Some(config(true, true))),
        ],
    );
    assert!(shared.borrow().pending.is_some());
    // Move the still-animating panel outside the viewport without constraining
    // its flow parent height. Paint clipping must retire its frame lease.
    apply(
        &owner,
        cx,
        vec![Op::SetStyle(
            id(0),
            vec![
                Style::Width(gpuio_protocol::v1::Length::Px(320.)),
                Style::Fields(vec![Field::PaddingTop(gpuio_protocol::v1::Length::Px(
                    5000.,
                ))]),
            ],
        )],
    );
    assert!(!shared.borrow().motion.is_animating());
    assert!(shared.borrow().pending.is_none());
    apply(
        &owner,
        cx,
        vec![Op::SetStyle(
            id(0),
            vec![Style::Width(gpuio_protocol::v1::Length::Px(320.))],
        )],
    );
    assert_eq!(
        shared.borrow().measurement.unwrap().bounds.size.height,
        px(120.)
    );
    apply(
        &owner,
        cx,
        vec![
            Op::SetStyle(panel, vec![Style::Fields(vec![Field::Display(3)])]),
            Op::SetReveal(panel, Some(config(false, true))),
        ],
    );
    assert!(shared.borrow().pending.is_some());
    apply(&owner, cx, vec![Op::SetReveal(panel, None)]);
    assert!(
        shared.borrow().pending.is_none(),
        "removal cancels even a separately retained test owner"
    );
    assert!(!shared.borrow().motion.is_animating());
    tick(cx, 2000);
    // A newly configured owner starts settled; window-close cleanup explicitly
    // retires it even if a caller still holds a diagnostic reference.
    apply(
        &owner,
        cx,
        vec![Op::SetReveal(panel, Some(config(false, true)))],
    );
    let new_owner = cx.update(|_, cx| owner.read_with(cx, |view, _| view.reveals[&panel].clone()));
    apply(
        &owner,
        cx,
        vec![
            Op::SetStyle(panel, vec![]),
            Op::SetReveal(panel, Some(config(true, true))),
        ],
    );
    assert!(new_owner.borrow().pending.is_some());
    cx.update(|_, cx| owner.update(cx, |view, _| view.close_reveals()));
    assert!(!new_owner.borrow().motion.is_animating());
    assert!(new_owner.borrow().pending.is_none());
}
