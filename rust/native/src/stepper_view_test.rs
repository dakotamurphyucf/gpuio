//! The actual public OCaml composition, decoded and rendered on TestPlatform.
//! This verifies native routing/AX/layout; it does not simulate an OS desktop.
use super::*;
use gpui::{KeyDownEvent, KeyUpEvent, Keystroke, PlatformInput, TestAppContext};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

#[test]
fn public_workflow_stepper_keeps_one_focus_target_per_step_and_routes_native_activation() {
    let hex = include_str!("../../../test/view_api/stepper-public-view.hex").trim();
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let Message::Apply(tx) = gpuio_protocol::decode(&bytes).unwrap() else {
        panic!("expected public view transaction")
    };
    let steps: Vec<_> = tx
        .operations
        .iter()
        .filter_map(|op| match op {
            Op::Create(node, Kind::Button, label, handler) => {
                Some((*node, label.clone(), *handler))
            }
            _ => None,
        })
        .collect();
    assert_eq!(steps.len(), 4);
    let node = |label: &str| steps.iter().find(|(_, text, _)| text == label).unwrap().0;
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, tx.window, "Workflow", 800., 240.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(tx.window, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let result = view.session.borrow_mut().apply(&tx).unwrap();
            view.update_editors(&result.dirty, window, cx);
            view.list_actions(&result.lists, window, cx);
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    let tree = cx.a11y_tree().unwrap();
    let review = tree
        .nodes
        .iter()
        .find(|(_, n)| n.role() == gpui::accesskit::Role::Button && n.label() == Some("Review"))
        .unwrap();
    assert_eq!(
        review.1.aria_current(),
        Some(gpui::accesskit::AriaCurrent::Step)
    );
    assert_eq!(review.1.description(), Some("Step 3 of 4 · Current step"));
    assert_eq!(
        tree.nodes
            .iter()
            .filter(|(_, n)| n.role() == gpui::accesskit::Role::Button)
            .count(),
        4
    );
    let approval = tree
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Approval") && n.role() == gpui::accesskit::Role::Button)
        .unwrap();
    assert!(approval.1.is_disabled());
    owner.read_with(cx, |view, _| {
        assert_eq!(
            view.buttons.len(),
            4,
            "passive glyphs and connectors own no focus"
        );
        for (id, _, _) in &steps {
            let bounds = view.probes.borrow()[id].bounds;
            assert!(bounds.size.width > px(0.) && bounds.size.height > px(0.));
        }
    });
    for name in ["Draft", "Review", "Publish", "Draft"] {
        cx.simulate_keystrokes("tab");
        cx.update(|window, cx| {
            owner.read_with(cx, |view, _| {
                assert!(
                    view.buttons[&node(name)].focus.is_focused(window),
                    "Tab to {name}"
                );
            })
        });
    }
    cx.simulate_keystrokes("shift-tab");
    cx.update(|window, cx| {
        owner.read_with(cx, |view, _| {
            assert!(view.buttons[&node("Publish")].focus.is_focused(window));
        })
    });
    transport.mailbox.lock().unwrap().drain(256);
    for key in ["space", "enter"] {
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
    let presses: Vec<_> = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| match event {
            Event::Press(_, node, _, _) => Some(node),
            _ => None,
        })
        .collect();
    assert_eq!(presses, vec![node("Publish"); 2]);
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: review.0,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    cx.run_until_parked();
    assert!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .into_iter()
            .any(|event| matches!(event, Event::Press(_, id, _, _) if id == node("Review")))
    );
    // Queue AX activation, then disable before asynchronous delivery. The old
    // painted action must use current admission, not its captured enabled state.
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: review.0,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
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
                    operations: vec![Op::SetControl(node("Review"), Control::Button(true))],
                })
                .unwrap();
            view.update_editors(&result.dirty, window, cx);
            view.list_actions(&result.lists, window, cx);
            cx.notify();
        });
    });
    cx.run_until_parked();
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(256)
            .into_iter()
            .any(|event| matches!(event, Event::Press(..)))
    );
}
