//! Public OCaml sidebar transactions replayed through native layout and routing.
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
// AccessKit retains hidden subtrees in its raw update. Platform consumers
// suppress a node when any ancestor is hidden, not only its own hidden flag.
fn exposed(
    nodes: &[(gpui::accesskit::NodeId, gpui::accesskit::Node)],
    id: gpui::accesskit::NodeId,
) -> bool {
    let node = &nodes
        .iter()
        .find(|(candidate, _)| *candidate == id)
        .unwrap()
        .1;
    if node.is_hidden() {
        return false;
    }
    nodes
        .iter()
        .find(|(_, parent)| parent.children().contains(&id))
        .is_none_or(|(parent, _)| exposed(nodes, *parent))
}
#[test]
fn public_sidebar_labels_retain_focus_paint_once_and_route_one_activation() {
    let transactions = [
        include_str!("../../../test/fixtures/sidebar-labels-0.hex"),
        include_str!("../../../test/fixtures/sidebar-labels-1.hex"),
        include_str!("../../../test/fixtures/sidebar-labels-2.hex"),
        include_str!("../../../test/fixtures/sidebar-labels-3.hex"),
        include_str!("../../../test/fixtures/sidebar-labels-4.hex"),
        include_str!("../../../test/fixtures/sidebar-labels-5.hex"),
    ]
    .map(transaction);
    let initial = &transactions[0];
    let destination = initial
        .operations
        .iter()
        .find_map(|op| match op {
            Op::SetLink(node, _) => Some(*node),
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
        .open(1, initial.window, "Sidebar", 800., 600.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(initial.window, session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    apply(&owner, cx, initial);
    cx.simulate_keystrokes("tab");
    let focus = cx.update(|window, cx| {
        owner.read_with(cx, |view, _| {
            let focus = view.buttons[&destination].focus.clone();
            assert!(focus.is_focused(window));
            focus
        })
    });
    for (index, tx) in transactions.iter().enumerate().take(5) {
        if index != 0 {
            apply(&owner, cx, tx);
        }
        let ax = cx.a11y_tree().unwrap();
        let links: Vec<_> = ax
            .nodes
            .iter()
            .filter(|(_, n)| n.role() == gpui::accesskit::Role::Link)
            .collect();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].1.label(), Some("Destination"));
        assert_eq!(links[0].1.is_disabled(), index == 3);
        assert_eq!(
            ax.nodes
                .iter()
                .filter(|(id, n)| n.label() == Some("Destination") && exposed(&ax.nodes, *id))
                .count(),
            1
        );
        if index == 4 {
            assert!(!ax.nodes.iter().any(|(_, n)| n.label() == Some("Suffix")));
        }
        let click = cx.update(|window, cx| {
            owner.read_with(cx, |view, _| {
                assert_eq!(view.buttons[&destination].focus, focus);
                if index < 3 {
                    assert!(focus.is_focused(window));
                }
                if index == 3 {
                    assert!(!focus.is_focused(window));
                }
            });
            window.draw(cx).clear(cx);
            let quads = window.painted_quads();
            let opacity = if index == 3 { 0.5 } else { 1. };
            let item_background = gpui::Background::from(rgba(0x112244ff)).opacity(opacity);
            assert_eq!(
                quads
                    .iter()
                    .filter(|q| q.background == item_background)
                    .count(),
                1,
                "item paint at {index}"
            );
            let label_background: gpui::Background = rgba(match index {
                1 => 0x22cc44ff,
                3 => 0x5522ffff,
                _ => 0xff2255ff,
            })
            .into();
            let label_background = label_background.opacity(opacity);
            let labels: Vec<_> = quads
                .iter()
                .filter(|q| q.background == label_background)
                .collect();
            assert_eq!(
                labels.len(),
                usize::from(index != 2),
                "label paint at {index}"
            );
            let bounds = if index == 2 {
                quads
                    .iter()
                    .find(|q| q.background == item_background)
                    .unwrap()
                    .bounds
            } else {
                labels[0].bounds
            };
            let scale = window.scale_factor();
            gpui::point(
                px((bounds.origin.x.0 + 4. * scale) / scale),
                px((bounds.origin.y.0 + 4. * scale) / scale),
            )
        });
        transport.mailbox.lock().unwrap().drain(256);
        cx.simulate_click(click, Default::default());
        for key in ["enter", "space"] {
            let events = transport.mailbox.lock().unwrap().drain(256);
            assert_eq!(
                events
                    .iter()
                    .filter(|e| matches!(e, Event::Press(_, node, _, _) if *node == destination))
                    .count(),
                usize::from(index != 3),
                "activation {index} before {key}"
            );
            press(cx, key);
        }
        assert_eq!(
            transport
                .mailbox
                .lock()
                .unwrap()
                .drain(256)
                .iter()
                .filter(|e| matches!(e, Event::Press(_, node, _, _) if *node == destination))
                .count(),
            usize::from(index != 3)
        );
        cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
            action: gpui::accesskit::Action::Click,
            target_node: links[0].0,
            target_tree: gpui::accesskit::TreeId::ROOT,
            data: None,
        });
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(
            transport
                .mailbox
                .lock()
                .unwrap()
                .drain(256)
                .iter()
                .filter(|e| matches!(e, Event::Press(_, node, _, _) if *node == destination))
                .count(),
            usize::from(index != 3),
            "accessibility activation at {index}"
        );
    }
    apply(&owner, cx, &transactions[5]);
    cx.update(|_, cx| {
        owner.read_with(cx, |view, _| {
            assert!(view.buttons.is_empty());
            assert!(view.selections.is_empty());
        })
    });
    assert!(
        !cx.a11y_tree()
            .unwrap()
            .nodes
            .iter()
            .any(|(_, n)| n.label() == Some("Destination"))
    );
}
