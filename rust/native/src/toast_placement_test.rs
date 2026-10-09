//! Production host geometry on TestPlatform, not physical desktop acceptance.
use super::*;
use gpui::{Entity, TestAppContext, VisualTestContext};
use gpuio_protocol::HandlerId;
use gpuio_protocol::toast_placement::{Anchor, Placement};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
fn n(i: i64) -> NodeId {
    NodeId::from_parts(i, 1).unwrap()
}
fn w() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|window, cx| {
        owner.update(cx, |v, cx| {
            let base = v.session.borrow().tree(w()).unwrap().revision();
            let result = v
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: w(),
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            v.update_editors(&result.dirty, window, cx);
            cx.notify();
        })
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}
#[test]
fn all_anchors_insets_reset_and_tiny_viewports_preserve_native_owners() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, w(), "Toasts", 400., 300.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(w(), session.clone(), transport.clone()));
    cx.simulate_a11y_active(true);
    cx.update(|window, _| window.activate_window());
    apply(
        &owner,
        cx,
        vec![
            Op::Create(n(0), Kind::ToastStack, "".into(), None),
            Op::SetToastStack(
                n(0),
                ToastStackConfig {
                    label: "Toasts".into(),
                    corner: ToastCorner::BottomRight,
                    width: 120.,
                    max_visible: 3,
                },
            ),
            Op::Create(
                n(1),
                Kind::Toast,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetToast(
                n(1),
                ToastConfig {
                    label: "Saved".into(),
                    close_label: "Dismiss saved".into(),
                    timeout_ns: Some(60_000_000_000),
                    politeness: ToastPoliteness::Polite,
                },
            ),
            Op::SetStyle(
                n(1),
                vec![Style::Fields(vec![Field::Height(Length::Px(60.))])],
            ),
            Op::Create(
                n(2),
                Kind::Input,
                "draft".into(),
                Some(HandlerId::from_parts(1, 1).unwrap()),
            ),
            Op::SetEditor(
                n(2),
                EditorConfig {
                    label: "Draft".into(),
                    placeholder: String::new(),
                    read_only: false,
                    disabled: false,
                    submit_on_enter: false,
                    auto_focus: false,
                    min_rows: 1,
                    max_rows: 1,
                },
            ),
            Op::Splice(n(1), 0, 0, vec![n(2)]),
            Op::Splice(n(0), 0, 0, vec![n(1)]),
            Op::SetRoot(Some(n(0))),
        ],
    );
    let close = owner.read_with(cx, |v, _| v.toasts[&n(1)].close_focus.clone());
    assert_eq!(
        owner.read_with(cx, |v, _| v.toasts[&n(1)].status()),
        (false, true)
    );
    let editor = owner.read_with(cx, |v, cx| v.editors[&n(2)].focus_handle(cx));
    cx.update(|window, cx| window.focus(&editor, cx));
    for anchor in [
        Anchor::TopLeft,
        Anchor::TopRight,
        Anchor::BottomLeft,
        Anchor::BottomRight,
        Anchor::TopCenter,
        Anchor::BottomCenter,
        Anchor::LeftCenter,
        Anchor::RightCenter,
    ] {
        apply(
            &owner,
            cx,
            vec![Op::SetToastPlacement(
                n(0),
                Some(Placement {
                    anchor,
                    top: 34.,
                    right: 8.,
                    bottom: 12.,
                    left: 20.,
                }),
            )],
        );
        let bounds = owner.read_with(cx, |v, _| v.toasts[&n(1)].bounds.get());
        let viewport = cx.update(|window, _| window.viewport_size());
        let right = f32::from(viewport.width) - 8. - f32::from(bounds.size.width);
        let bottom = f32::from(viewport.height) - 12. - f32::from(bounds.size.height);
        let x = match anchor {
            Anchor::TopLeft | Anchor::BottomLeft | Anchor::LeftCenter => 20.,
            Anchor::TopRight | Anchor::BottomRight | Anchor::RightCenter => right,
            _ => (20. + right) / 2.,
        };
        let y = match anchor {
            Anchor::TopLeft | Anchor::TopRight | Anchor::TopCenter => 34.,
            Anchor::BottomLeft | Anchor::BottomRight | Anchor::BottomCenter => bottom,
            _ => (34. + bottom) / 2.,
        };
        assert!(
            (f32::from(bounds.origin.x) - x).abs() <= 1.
                && (f32::from(bounds.origin.y) - y).abs() <= 1.,
            "{anchor:?}: {bounds:?}, expected {x},{y}"
        );
        owner.read_with(cx, |v, cx| {
            assert_eq!(v.toasts[&n(1)].close_focus, close);
            assert_eq!(v.editors[&n(2)].focus_handle(cx), editor);
        });
        assert!(cx.update(|window, _| editor.is_focused(window)));
        // A focused native editor may schedule caret frames; placement itself
        // does not own an animation or recurring frame callback.
    }
    apply(&owner, cx, vec![Op::SetToastPlacement(n(0), None)]);
    let b = owner.read_with(cx, |v, _| v.toasts[&n(1)].bounds.get());
    let size = cx.update(|window, _| window.viewport_size());
    assert!((f32::from(b.bottom_right().x - size.width) + 16.).abs() < 1.);
    assert!((f32::from(b.bottom_right().y - size.height) + 16.).abs() < 1.);
    apply(
        &owner,
        cx,
        vec![Op::SetToastPlacement(
            n(0),
            Some(Placement {
                anchor: Anchor::BottomCenter,
                top: 16384.,
                right: 16384.,
                bottom: 16384.,
                left: 16384.,
            }),
        )],
    );
    assert!(
        !cx.update(|window, _| editor.is_focused(window)),
        "zero usable area must retire focus"
    );
    assert_eq!(
        owner.read_with(cx, |v, _| v.toasts[&n(1)].status()),
        (false, false),
        "unusable placement pauses the deadline even after editor focus has gone"
    );
    assert!(!owner.read_with(cx, |v, _| v.focus.borrow().allows(n(2))));
    cx.simulate_keystrokes("x");
    assert_eq!(
        cx.update(|window, cx| owner.read(cx).editors[&n(2)].snapshot(window, cx).text),
        "draft"
    );
    let nodes = cx.a11y_tree().unwrap().nodes;
    if let Some((id, _)) = nodes.iter().find(|(_, node)| node.label() == Some("Draft")) {
        let mut cursor = *id;
        let mut hidden = false;
        loop {
            hidden |= nodes
                .iter()
                .find(|(id, _)| *id == cursor)
                .unwrap()
                .1
                .is_hidden();
            let Some((parent, _)) = nodes
                .iter()
                .find(|(_, node)| node.children().contains(&cursor))
            else {
                break;
            };
            cursor = *parent;
        }
        assert!(
            hidden,
            "zero-area editor must have a hidden accessibility ancestor"
        );
    }
    apply(&owner, cx, vec![Op::SetToastPlacement(n(0), None)]);
    assert!(owner.read_with(cx, |v, _| v.focus.borrow().allows(n(2))));
    assert_eq!(
        owner.read_with(cx, |v, _| v.toasts[&n(1)].status()),
        (false, true)
    );
    cx.update(|window, cx| window.focus(&editor, cx));
    cx.simulate_keystrokes("end x");
    assert_eq!(
        cx.update(|window, cx| owner.read(cx).editors[&n(2)].snapshot(window, cx).text),
        "draftx"
    );
    apply(
        &owner,
        cx,
        vec![
            Op::SetRoot(None),
            Op::Remove(n(2)),
            Op::Remove(n(1)),
            Op::Remove(n(0)),
        ],
    );
    owner.read_with(cx, |v, _| {
        assert!(v.toasts.is_empty());
        assert!(v.toast_stacks.is_empty());
        assert!(v.editors.is_empty());
    });
    cx.update(|window, _| window.remove_window());
}
