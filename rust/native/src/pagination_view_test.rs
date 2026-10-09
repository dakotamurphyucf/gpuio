//! Exact public OCaml pager transactions on TestPlatform; no desktop window.
use super::*;
use gpui::{
    Entity, KeyDownEvent, KeyUpEvent, Keystroke, PlatformInput, TestAppContext, VisualTestContext,
    accesskit,
};
use gpuio_protocol::number_input as n;
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
            view.list_actions(&result.lists, window, cx);
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

fn gap_state(cx: &VisualTestContext, label: &str, expanded: bool) -> accesskit::NodeId {
    let tree = cx.a11y_tree().unwrap();
    let (id, node) = tree
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some(label))
        .unwrap();
    assert_eq!(node.role(), accesskit::Role::Button);
    assert_eq!(node.has_popup(), Some(accesskit::HasPopup::Dialog));
    assert_eq!(node.is_expanded(), Some(expanded));
    *id
}

fn gap_geometry(owner: &Entity<View>, cx: &mut VisualTestContext, gap: NodeId, tx: &Transaction) {
    let panel = tx
        .operations
        .iter()
        .find_map(|op| match op {
            Op::SetOverlay(node, Some(_)) => Some(*node),
            _ => None,
        })
        .unwrap();
    cx.update(|_, cx| {
        owner.read_with(cx, |view, _| {
            let bounds = view.probes.borrow()[&gap].bounds;
            let anchor = view.focus.borrow().anchor(panel).unwrap().get();
            // The probe is inside the button's one-pixel border; the anchor records
            // the whole gap wrapper, whose only laid-out child is that button.
            assert_eq!(anchor.center(), bounds.center());
            assert!((anchor.size.width - bounds.size.width).abs() <= gpui::px(2.));
            assert!((anchor.size.height - bounds.size.height).abs() <= gpui::px(2.));
        })
    });
}

#[test]
fn public_pagination_popup_routes_keyboard_and_restores_gap_focus_on_close() {
    let initial = transaction(include_str!("../../../test/fixtures/pagination-view-0.hex"));
    let opened = transaction(include_str!("../../../test/fixtures/pagination-view-1.hex"));
    let closed = transaction(include_str!("../../../test/fixtures/pagination-view-2.hex"));
    let second_opened = transaction(include_str!("../../../test/fixtures/pagination-view-3.hex"));
    let first_reopened = transaction(include_str!("../../../test/fixtures/pagination-view-4.hex"));
    let final_closed = transaction(include_str!("../../../test/fixtures/pagination-view-5.hex"));
    let button = |tx: &Transaction, text: &str| {
        tx.operations
            .iter()
            .find_map(|op| match op {
                Op::Create(node, Kind::Button, label, _) if label == text => Some(*node),
                _ => None,
            })
            .unwrap()
    };
    let gap = button(&initial, "…");
    let shortcut = button(&opened, "2");
    let number = opened
        .operations
        .iter()
        .find_map(|op| match op {
            Op::SetNumberInput(node, _, _) => Some(*node),
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
        .open(1, initial.window, "Pagination", 1000., 600.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(initial.window, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    apply(&owner, cx, &initial);
    let labels: Vec<_> = initial
        .operations
        .iter()
        .filter_map(|op| match op {
            Op::SetAccessibility(_, Some(config)) => config
                .label
                .as_deref()
                .filter(|label| label.starts_with("Pages ")),
            _ => None,
        })
        .collect();
    assert_eq!(labels.len(), 2);
    let gap_ids = [
        gap_state(cx, labels[0], false),
        gap_state(cx, labels[1], false),
    ];
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| window.focus(&view.buttons[&gap].focus, cx))
    });
    press(cx, "space");
    assert!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(event, Event::Press(_, node, _, _) if *node == gap))
    );
    apply(&owner, cx, &opened);
    assert_eq!(gap_state(cx, labels[0], true), gap_ids[0]);
    assert_eq!(gap_state(cx, labels[1], false), gap_ids[1]);
    gap_geometry(&owner, cx, gap, &opened);
    let tree = cx.a11y_tree().unwrap();
    assert!(
        tree.nodes
            .iter()
            .any(|(_, node)| node.label() == Some("Page number"))
    );
    assert!(
        tree.nodes
            .iter()
            .any(|(_, node)| node.label() == Some("Go to page") && node.is_disabled())
    );
    assert!(
        tree.nodes.len() < 100,
        "one billion pages do not produce an unbounded tree"
    );
    cx.update(|window, cx| {
        owner.read_with(cx, |view, cx| {
            assert_eq!(view.numbers.len(), 1);
            assert!(
                view.numbers[&number].focus_handle(cx).is_focused(window),
                "page field gets opening focus"
            );
        })
    });
    transport.mailbox.lock().unwrap().drain(128);
    cx.simulate_keystrokes("up");
    cx.simulate_keystrokes("enter");
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert!(events.iter().any(|event| matches!(event,
        Event::NumberInputEvent(_, node, _, _, n::Event::Committed(n::Source::Keyboard, snapshot))
        if *node == number && snapshot.committed == n::Value::Number(3.))));
    assert!(
        !events.iter().any(|event| matches!(event, Event::Press(..))),
        "editing does not choose a shortcut"
    );
    cx.simulate_keystrokes("shift-tab");
    press(cx, "space");
    assert!(
        transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(event, Event::Press(..))),
        "shortcut reachable with keyboard"
    );
    // Queue an AX shortcut, then apply actual OCaml closure before delivery.
    let tree = cx.a11y_tree().unwrap();
    let target = tree
        .nodes
        .iter()
        .find(|(_, node)| node.role() == gpui::accesskit::Role::Button && node.label() == Some("2"))
        .unwrap()
        .0;
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: target,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    apply(&owner, cx, &closed);
    assert_eq!(gap_state(cx, labels[0], false), gap_ids[0]);
    assert_eq!(gap_state(cx, labels[1], false), gap_ids[1]);
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, Event::Press(_, node, _, _) if *node == shortcut))
    );
    cx.update(|window, cx| {
        owner.read_with(cx, |view, _| {
            assert!(view.numbers.is_empty());
            assert!(
                view.buttons[&gap].focus.is_focused(window),
                "closing restores the opening gap"
            );
        })
    });

    let gaps: Vec<_> = initial
        .operations
        .iter()
        .filter_map(|op| match op {
            Op::Create(id, Kind::Button, text, _) if text == "…" => Some(*id),
            _ => None,
        })
        .collect();
    cx.simulate_a11y_action(accesskit::ActionRequest {
        action: accesskit::Action::Click,
        target_node: gap_ids[1],
        target_tree: accesskit::TreeId::ROOT,
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
            .any(|event| matches!(event, Event::Press(_, node, _, _) if *node == gaps[1]))
    );
    apply(&owner, cx, &second_opened);
    assert_eq!(gap_state(cx, labels[0], false), gap_ids[0]);
    assert_eq!(gap_state(cx, labels[1], true), gap_ids[1]);
    gap_geometry(&owner, cx, gaps[1], &second_opened);
    apply(&owner, cx, &first_reopened);
    assert_eq!(gap_state(cx, labels[0], true), gap_ids[0]);
    assert_eq!(gap_state(cx, labels[1], false), gap_ids[1]);
    gap_geometry(&owner, cx, gap, &first_reopened);
    owner.read_with(cx, |view, _| assert_eq!(view.numbers.len(), 1));
    apply(&owner, cx, &final_closed);
    assert_eq!(gap_state(cx, labels[0], false), gap_ids[0]);
    assert_eq!(gap_state(cx, labels[1], false), gap_ids[1]);
    owner.read_with(cx, |view, _| assert!(view.numbers.is_empty()));
    cx.update(|window, cx| {
        owner.read_with(cx, |view, _| {
            assert!(
                view.buttons[&gap].focus.is_focused(window),
                "closing the replacement chooser restores its own gap"
            );
        })
    });
}
