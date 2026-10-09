//! Actual public rich-header composition, with retained native editing.
use super::*;
use gpui::{
    Entity, KeyDownEvent, KeyUpEvent, Keystroke, PlatformInput, TestAppContext, VisualTestContext,
};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
fn transaction(hex: &str) -> Transaction {
    let hex = hex.trim();
    let bytes: Vec<_> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let Message::Apply(tx) = gpuio_protocol::decode(&bytes).unwrap() else {
        panic!("expected Apply")
    };
    tx
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, tx: &Transaction) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let result = view.session.borrow_mut().apply(tx).unwrap();
            view.update_editors(&result.dirty, window, cx);
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}
fn press(cx: &mut VisualTestContext, key: &str) {
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        let keystroke = Keystroke::parse(key).unwrap();
        window.dispatch_event(
            PlatformInput::KeyDown(KeyDownEvent {
                keystroke: keystroke.clone(),
                is_held: false,
                prefer_character_input: false,
            }),
            cx,
        );
        window.dispatch_event(PlatformInput::KeyUp(KeyUpEvent { keystroke }), cx);
    });
}
#[test]
fn public_rich_headers_keep_heading_semantics_roving_keys_and_collapse_focus() {
    let initial = transaction(include_str!("../../../test/fixtures/disclosure-rich-0.hex"));
    let closed = transaction(include_str!("../../../test/fixtures/disclosure-rich-1.hex"));
    let reopened = transaction(include_str!("../../../test/fixtures/disclosure-rich-2.hex"));
    let button = |text: &str| {
        initial
            .operations
            .iter()
            .find_map(|op| match op {
                Op::Create(id, Kind::Button, label, _) if label == text => Some(*id),
                _ => None,
            })
            .unwrap()
    };
    let first = button("first");
    let last = button("last");
    let editor = initial
        .operations
        .iter()
        .find_map(|op| match op {
            Op::SetEditor(node, _) => Some(*node),
            _ => None,
        })
        .unwrap();
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, initial.window, "Disclosure", 800., 600.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(initial.window, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    apply(&owner, cx, &initial);
    let tree = cx.a11y_tree().unwrap();
    assert_eq!(
        tree.nodes
            .iter()
            .filter(|(_, n)| n.role() == gpui::accesskit::Role::Heading && n.level() == Some(3))
            .count(),
        3
    );
    assert_eq!(
        tree.nodes
            .iter()
            .filter(|(_, n)| n.role() == gpui::accesskit::Role::Button)
            .count(),
        3
    );
    assert!(
        tree.nodes
            .iter()
            .any(|(_, n)| n.label() == Some("first") && n.is_expanded() == Some(true))
    );
    assert!(
        tree.nodes
            .iter()
            .any(|(_, n)| n.label() == Some("locked") && n.is_disabled())
    );
    cx.simulate_keystrokes("tab");
    cx.simulate_keystrokes("down");
    cx.update(|window, cx| {
        owner.read_with(cx, |view, _| {
            assert!(view.buttons[&last].focus.is_focused(window))
        })
    });
    cx.simulate_keystrokes("home");
    cx.update(|window, cx| {
        owner.read_with(cx, |view, _| {
            assert!(view.buttons[&first].focus.is_focused(window))
        })
    });
    transport.mailbox.lock().unwrap().drain(128);
    press(cx, "enter");
    assert_eq!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .filter(|event| matches!(event, Event::Press(_, node, _, _) if *node == first))
            .count(),
        1
    );
    let original_focus = cx.update(|window, cx| {
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
    assert!(draft.contains('x'));
    apply(&owner, cx, &closed);
    cx.update(|window, cx| {
        owner.read_with(cx, |view, cx| {
            assert!(view.buttons[&first].focus.is_focused(window));
            assert_eq!(view.editors[&editor].focus_handle(cx), original_focus);
            assert_eq!(view.editors[&editor].snapshot(window, cx).text, draft);
        })
    });
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Accordion draft"))
    );
    apply(&owner, cx, &reopened);
    cx.update(|window, cx| {
        owner.read_with(cx, |view, cx| {
            assert_eq!(view.editors[&editor].focus_handle(cx), original_focus);
            assert_eq!(view.editors[&editor].snapshot(window, cx).text, draft);
        })
    });
}
