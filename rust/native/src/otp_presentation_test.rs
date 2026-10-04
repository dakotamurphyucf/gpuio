//! Real retained OTP layout and edit state on TestPlatform, without OS windows.
use super::*;
use crate::session::Session;
use gpui::TestAppContext;
use gpuio_protocol::{HandlerId, NodeId, WindowId, otp_presentation::Appearance};
use std::{cell::RefCell, os::fd::AsRawFd, os::unix::net::UnixStream, rc::Rc};
fn apply(view: &mut View, window: &mut Window, cx: &mut Context<View>, operations: Vec<Op>) {
    let base = view.session.borrow().tree(view.id).unwrap().revision();
    let result = view
        .session
        .borrow_mut()
        .apply(&Transaction {
            window: view.id,
            base,
            revision: base + 1,
            operations,
        })
        .unwrap();
    view.update_editors(&result.dirty, window, cx);
    view.list_actions(&result.lists, window, cx);
    cx.notify();
}
#[::core::prelude::v1::test]
fn grouping_updates_shared_hit_geometry_while_composition_history_and_owner_survive() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "OTP presentation", 480., 160.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    let config = o::Config {
        policy: o::Policy::new(6, o::Alphabet::Digits).unwrap(),
        label: "Code".into(),
        masked: false,
        disabled: false,
        read_only: false,
        auto_focus: false,
    };
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![
                    Op::Create(node, Kind::OtpInput, "".into(), Some(handler)),
                    Op::SetOtpInput(node, config, "12".into()),
                    Op::SetRoot(Some(node)),
                ],
            )
        });
        window.draw(cx).clear(cx);
    });
    let input = owner.read_with(cx, |view, _| view.otps[&node].state.clone());
    cx.update(|window, cx| {
        input.update(cx, |input, cx| {
            window.focus(&input.focus, cx);
            input.replace_text_in_range(None, "34", window, cx);
        });
        window.draw(cx).clear(cx);
    });
    let before = input.read_with(cx, |input, _| input.model.snapshot());
    assert_eq!(before.value, "1234");
    assert!(before.can_undo);
    let appearance = Appearance {
        groups: 2,
        cell_width: Some(40.),
        cell_gap: 4.,
        group_gap: 24.,
        background: Some(0xff0000ff),
        focus_border: Some(0x00ff00ff),
        ..Appearance::default()
    };
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            apply(
                view,
                window,
                cx,
                vec![Op::SetOtpAppearance(node, Some(appearance.clone()))],
            )
        });
        assert!(
            input.read(cx).layout.is_none(),
            "retired geometry must not answer platform queries"
        );
        input.update(cx, |input, cx| {
            assert!(
                input
                    .bounds_for_range(0..1, Bounds::default(), window, cx)
                    .is_none()
            )
        });
        window.draw(cx).clear(cx);
        let red: gpui::Background = rgba(0xff0000ff).into();
        let quads = window.painted_quads();
        let cells: Vec<_> = quads.iter().filter(|quad| quad.background == red).collect();
        assert_eq!(cells.len(), 6);
        assert!(
            cells
                .iter()
                .all(|quad| quad.border_color == rgba(0x00ff00ff).into())
        );
        input.read_with(cx, |input, _| {
            assert_eq!(input.model.snapshot(), before);
            let layout = input.layout.as_ref().unwrap();
            assert_eq!(layout.cells.len(), 6);
            assert_eq!(layout.cells[1].left() - layout.cells[0].right(), px(4.));
            assert_eq!(layout.cells[3].left() - layout.cells[2].right(), px(24.));
            let p = point(layout.positions[3].1, layout.bounds.center().y);
            assert_eq!(layout.byte_for_point(p), 3);
            assert_eq!(layout.range_bounds(3..3).left(), layout.positions[3].1);
            assert_eq!(layout.cells[5].right() - layout.cells[0].left(), px(280.));
        });
    });
    assert_eq!(
        owner.read_with(cx, |view, _| view.otps[&node].state.entity_id()),
        input.entity_id()
    );
    cx.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.replace_and_mark_text_in_range(None, "é", Some(0..1), window, cx)
        });
        window.draw(cx).clear(cx);
    });
    let composing = input.read_with(cx, |input, _| input.model.snapshot());
    assert!(composing.composition.is_some());
    for policy in [
        Some(Appearance {
            groups: 32,
            cell_width: Some(28.),
            ..appearance
        }),
        None,
    ] {
        cx.update(|window, cx| {
            owner.update(cx, |view, cx| {
                apply(view, window, cx, vec![Op::SetOtpAppearance(node, policy)])
            });
            window.draw(cx).clear(cx);
            input.read_with(cx, |input, _| {
                assert_eq!(input.model.snapshot(), composing);
                assert_eq!(
                    input.layout.as_ref().unwrap().cells.len(),
                    1,
                    "preedit shapes as one continuous field"
                );
                assert!(input.focus.is_focused(window));
            });
        });
    }
}
