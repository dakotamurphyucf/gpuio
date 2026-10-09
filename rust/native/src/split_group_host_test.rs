//! Production Host on TestPlatform. No physical desktop acceptance.
use super::*;
use gpui::{Entity, TestAppContext, VisualTestContext};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
#[path = "../tests/support/split_group.rs"]
mod support;
use support::*;
fn draw(cx: &mut VisualTestContext) {
    cx.run_until_parked();
    cx.update(|w, cx| w.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| {
            let base = v.session.borrow().tree(w()).unwrap().revision();
            let applied = v
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: w(),
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            v.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
    });
    draw(cx);
}
#[test]
fn production_group_observes_native_resize_and_retains_children_and_focus_on_reorder() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Split", 400., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, _| window.activate_window());
    apply(&owner, cx, initial());
    let state = owner.read_with(cx, |v, _| v.split_groups[&n(0)].clone());
    let button = owner.read_with(cx, |v, _| Rc::downgrade(&v.buttons[&n(4)]));
    let focus = state.borrow().focus("p0").unwrap().clone();
    transport.mailbox.lock().unwrap().drain(100);
    cx.update(|w, cx| w.focus(&focus, cx));
    assert!(cx.update(|w, _| focus.is_focused(w)));
    assert!(owner.read_with(cx, |v, _| v.focus.borrow().allows(n(0))));
    cx.simulate_keystrokes("right");
    draw(cx);
    let events = transport.mailbox.lock().unwrap().drain(100);
    assert!(events.iter().any(|e| matches!(e,Event::SplitGroupResized(_,_,_,_,_,s) if s.sizes==vec![("p0".into(),110.),("p1".into(),90.),("p2".into(),100.)])),"{events:?}; focused={} nodes={:?}",cx.update(|w,_| focus.is_focused(w)),cx.a11y_tree().unwrap().nodes);
    let child = owner.read_with(cx, |v, _| v.buttons[&n(4)].focus.clone());
    apply(
        &owner,
        cx,
        vec![Op::SetStyle(
            n(4),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(30.)),
                Field::Height(Length::Px(24.)),
                Field::MarginLeft(Length::Px(160.)),
            ])],
        )],
    );
    assert!(
        !cx.update(|window, cx| owner.read(cx).focus.borrow().can_focus(&child, window)),
        "a child beyond its own panel cannot become a focus stop in the next panel"
    );
    apply(
        &owner,
        cx,
        vec![Op::SetStyle(
            n(4),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(30.)),
                Field::Height(Length::Px(24.)),
            ])],
        )],
    );
    let mut c = config();
    c.panels.swap(0, 1);
    apply(
        &owner,
        cx,
        vec![
            Op::SetSplitGroup(n(0), c.clone(), Default::default()),
            Op::Splice(n(0), 0, 3, vec![n(6), n(1), n(11)]),
        ],
    );
    assert!(owner.read_with(cx, |v, _| Rc::ptr_eq(&v.split_groups[&n(0)], &state)));
    assert!(owner.read_with(cx, |v, _| Rc::ptr_eq(
        &v.buttons[&n(4)],
        &button.upgrade().unwrap()
    )));
    assert_eq!(focus, state.borrow().focus("p0").unwrap().clone());
    assert_eq!(cx.update(|w, cx| w.simulate_next_frame(cx)), 0);
    c.panels.iter_mut().find(|p| p.id == "p0").unwrap().visible = false;
    apply(
        &owner,
        cx,
        vec![Op::SetSplitGroup(n(0), c, Default::default())],
    );
    assert!(!cx.update(|w, _| focus.is_focused(w)));
    assert!(!owner.read_with(cx, |v, _| v.focus.borrow().allows(n(4))));
    assert!(button.upgrade().is_some());
    let child_focus = owner.read_with(cx, |v, _| v.buttons[&n(9)].focus.clone());
    cx.update(|w, cx| w.focus(&child_focus, cx));
    let mut clipped = config();
    clipped.reset_generation = 1;
    for panel in &mut clipped.panels {
        panel.minimum_size = 200.;
        panel.initial_size = Some(200.);
    }
    apply(
        &owner,
        cx,
        vec![
            Op::SetSplitGroup(n(0), clipped, Default::default()),
            Op::Splice(n(0), 0, 3, vec![n(1), n(6), n(11)]),
            Op::SetStyle(
                n(0),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(100.)),
                    Field::Height(Length::Px(120.)),
                ])],
            ),
        ],
    );
    assert!(!owner.read_with(cx, |v, _| v.focus.borrow().allows(n(9))));
    assert!(!cx.update(|w, _| child_focus.is_focused(w)));
    let nodes = cx.a11y_tree().unwrap().nodes;
    let button_id = nodes
        .iter()
        .find(|(_, n)| n.label() == Some("Button 1"))
        .unwrap()
        .0;
    let mut cursor = button_id;
    let mut hidden = false;
    loop {
        let node = &nodes.iter().find(|(id, _)| *id == cursor).unwrap().1;
        hidden |= node.is_hidden();
        let Some((parent, _)) = nodes.iter().find(|(_, n)| n.children().contains(&cursor)) else {
            break;
        };
        cursor = *parent;
    }
    assert!(
        hidden,
        "clipped button needs a hidden accessibility ancestor"
    );
    transport.mailbox.lock().unwrap().drain(100);
    cx.simulate_a11y_action(gpui::accesskit::ActionRequest {
        action: gpui::accesskit::Action::Click,
        target_node: button_id,
        target_tree: gpui::accesskit::TreeId::ROOT,
        data: None,
    });
    draw(cx);
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(100)
            .iter()
            .any(|e| matches!(e, Event::Press(..)))
    );
    cx.update(|w, cx| owner.update(cx, |v, cx| v.close_split_groups(w, cx)));
    cx.update(|w, _| w.remove_window());
}

#[test]
fn public_core_frames_apply_request_once_and_release_the_native_owner() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Public split", 400., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.update(|window, _| window.activate_window());
    let mut weak = None;
    let mut button = None;
    let mut requests = vec![];
    for (index, line) in include_str!("../../../test/fixtures/split-group-public.hex")
        .lines()
        .enumerate()
    {
        let bytes: Vec<_> = (0..line.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&line[i..i + 2], 16).unwrap())
            .collect();
        let Message::Apply(tx) = gpuio_protocol::decode(&bytes).unwrap() else {
            panic!("expected transaction")
        };
        apply(&owner, cx, tx.operations);
        if index == 0 {
            weak = Some(owner.read_with(cx, |v, _| Rc::downgrade(&v.split_groups[&n(0)])));
            button = Some(owner.read_with(cx, |v, _| Rc::downgrade(&v.buttons[&n(3)])));
        } else if index < 5 {
            assert!(owner.read_with(cx, |v, _| Rc::ptr_eq(
                &v.buttons[&n(3)],
                &button.as_ref().unwrap().upgrade().unwrap()
            )));
        }
        requests.extend(
            transport
                .mailbox
                .lock()
                .unwrap()
                .drain(100)
                .into_iter()
                .filter_map(|e| match e {
                    Event::SplitGroupResized(_, _, _, _, _, snapshot) => Some(snapshot),
                    _ => None,
                }),
        );
    }
    assert_eq!(requests.len(), 1, "{requests:?}");
    assert_eq!(
        requests[0].source,
        gpuio_protocol::split_group::Source::Request(1)
    );
    assert_eq!(
        requests[0].sizes,
        vec![("c".into(), 100.), ("a".into(), 130.), ("b".into(), 70.)]
    );
    assert!(weak.unwrap().upgrade().is_none());
    assert!(button.unwrap().upgrade().is_none());
    assert!(owner.read_with(cx, |v, _| v.split_groups.is_empty()));
    cx.update(|window, _| window.remove_window());
}

#[test]
fn retained_editor_survives_reorder_hide_and_resize_without_hidden_input() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Draft", 400., 200.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.update(|window, _| window.activate_window());
    let mut operations = initial();
    operations.extend([
        Op::Create(n(16), Kind::Input, "draft".into(), Some(h(16))),
        Op::SetEditor(
            n(16),
            EditorConfig {
                label: "Retained draft".into(),
                placeholder: String::new(),
                read_only: false,
                disabled: false,
                submit_on_enter: false,
                auto_focus: false,
                min_rows: 1,
                max_rows: 1,
            },
        ),
        Op::SetStyle(
            n(16),
            vec![Style::Fields(vec![
                Field::Width(Length::Px(80.)),
                Field::Height(Length::Px(30.)),
            ])],
        ),
        Op::Splice(n(2), 0, 1, vec![n(16)]),
        Op::Remove(n(4)),
    ]);
    apply(&owner, cx, operations);
    let editor = owner.read_with(cx, |v, cx| v.editors[&n(16)].focus_handle(cx));
    cx.update(|window, cx| window.focus(&editor, cx));
    cx.simulate_keystrokes("end !");
    draw(cx);
    let original = cx.update(|window, cx| owner.read(cx).editors[&n(16)].snapshot(window, cx));
    assert_eq!(original.text, "draft!");
    let mut c = config();
    c.panels.swap(0, 1);
    apply(
        &owner,
        cx,
        vec![
            Op::SetSplitGroup(n(0), c.clone(), Default::default()),
            Op::Splice(n(0), 0, 3, vec![n(6), n(1), n(11)]),
        ],
    );
    assert!(cx.update(|window, _| editor.is_focused(window)));
    owner.read_with(cx, |v, cx| {
        assert_eq!(v.editors[&n(16)].focus_handle(cx), editor)
    });
    c.panels[1].visible = false;
    apply(
        &owner,
        cx,
        vec![Op::SetSplitGroup(n(0), c.clone(), Default::default())],
    );
    assert!(!cx.update(|window, _| editor.is_focused(window)));
    cx.simulate_keystrokes("x");
    draw(cx);
    assert_eq!(
        cx.update(|window, cx| owner.read(cx).editors[&n(16)].snapshot(window, cx).text),
        original.text
    );
    c.panels[1].visible = true;
    c.resize = Some(gpuio_protocol::split_group::ResizeRequest {
        id: "p0".into(),
        size: 130.,
        serial: 1,
    });
    apply(
        &owner,
        cx,
        vec![Op::SetSplitGroup(n(0), c, Default::default())],
    );
    owner.read_with(cx, |v, cx| {
        assert_eq!(v.editors[&n(16)].focus_handle(cx), editor)
    });
    cx.update(|window, cx| window.focus(&editor, cx));
    cx.simulate_keystrokes("?");
    draw(cx);
    assert_eq!(
        cx.update(|window, cx| owner.read(cx).editors[&n(16)].snapshot(window, cx).text),
        "draft!?"
    );
    let mut remove = vec![Op::SetRoot(None), Op::Remove(n(16))];
    remove.extend((0..16).rev().filter(|i| *i != 4).map(|i| Op::Remove(n(i))));
    apply(&owner, cx, remove);
    owner.read_with(cx, |v, _| {
        assert!(v.editors.is_empty());
        assert!(v.split_groups.is_empty());
    });
    cx.update(|window, _| window.remove_window());
}
