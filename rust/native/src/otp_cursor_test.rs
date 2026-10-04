//! Deterministic native caret timing; no OS window or real input-method claim.
use super::*;
use crate::session::Session;
use gpui::{TestAppContext, VisualTestContext};
use gpuio_protocol::v1::{Field as StyleField, Length as WireLength, Style as WireStyle};
use std::{cell::RefCell, os::fd::AsRawFd, os::unix::net::UnixStream, rc::Rc, time::Duration};

fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn apply(owner: &Entity<View>, cx: &mut VisualTestContext, operations: Vec<Op>) {
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
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
                .unwrap();
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
    });
    draw(cx);
}
fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.run_until_parked();
}
fn advance(cx: &mut VisualTestContext, millis: u64) {
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(millis));
    cx.run_until_parked();
    draw(cx);
}
fn state(input: &Entity<Input>, cx: &VisualTestContext) -> (bool, bool) {
    input.read_with(cx, |input, _| {
        (input.cursor.visible, input.cursor.running())
    })
}
fn mount(
    app: &mut TestAppContext,
) -> (
    Entity<View>,
    Entity<Input>,
    &mut VisualTestContext,
    Arc<Transport>,
    UnixStream,
) {
    let (reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let wid = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, wid, "OTP caret", 480., 160.)
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(wid, session.clone(), transport.clone()));
    apply(
        &owner,
        cx,
        vec![
            Op::Create(
                node(),
                Kind::OtpInput,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetOtpInput(
                node(),
                o::Config {
                    policy: o::Policy::new(6, o::Alphabet::Digits).unwrap(),
                    label: "Code".into(),
                    masked: false,
                    disabled: false,
                    read_only: false,
                    auto_focus: false,
                },
                "12".into(),
            ),
            Op::SetOtpAppearance(
                node(),
                Some(gpuio_protocol::otp_presentation::Appearance {
                    caret: Some(0xff0000ff),
                    ..Default::default()
                }),
            ),
            Op::SetRoot(Some(node())),
        ],
    );
    let input = owner.read_with(cx, |view, _| view.otps[&node()].state.clone());
    cx.update(|window, cx| {
        window.activate_window();
        input.update(cx, |input, cx| window.focus(&input.focus, cx));
    });
    cx.run_until_parked();
    draw(cx);
    assert_eq!(state(&input, cx), (true, true));
    (owner, input, cx, transport, reader)
}
fn red_carets(cx: &mut VisualTestContext) -> usize {
    cx.update(|window, _| {
        let red: Background = rgba(0xff0000ff).into();
        window
            .painted_quads()
            .iter()
            .filter(|q| q.background == red)
            .count()
    })
}

#[::core::prelude::v1::test]
fn otp_caret_phase_is_paint_only_and_editing_restarts_without_stale_ticks() {
    let mut app = TestAppContext::single();
    let (owner, input, cx, transport, _reader) = mount(&mut app);
    transport.mailbox.lock().unwrap().drain(256);
    let snapshot = input.read_with(cx, |input, _| input.model.snapshot());
    assert_eq!(red_carets(cx), 1);
    advance(cx, 499);
    assert_eq!(state(&input, cx), (true, true));
    advance(cx, 1);
    assert_eq!(state(&input, cx), (false, true));
    assert_eq!(red_carets(cx), 0);
    input.read_with(cx, |input, _| {
        assert_eq!(input.model.snapshot(), snapshot);
        assert!(
            input.layout.as_ref().unwrap().caret.is_some(),
            "logical IME geometry survives off phase"
        );
    });
    assert!(transport.mailbox.lock().unwrap().drain(256).is_empty());
    // Presentation changes must not restart a running phase.
    apply(
        &owner,
        cx,
        vec![Op::SetOtpAppearance(
            node(),
            Some(gpuio_protocol::otp_presentation::Appearance {
                groups: 2,
                caret: Some(0xff0000ff),
                ..Default::default()
            }),
        )],
    );
    assert_eq!(state(&input, cx), (false, true));
    advance(cx, 500);
    assert_eq!(state(&input, cx), (true, true));
    // A new accepted edit cancels the previous phase even near its deadline.
    for text in ["3", "4", "5"] {
        advance(cx, 400);
        cx.update(|window, cx| {
            input.update(cx, |input, cx| {
                input.replace_text_in_range(None, text, window, cx)
            })
        });
        draw(cx);
        advance(cx, 100);
        assert_eq!(state(&input, cx), (true, true));
    }
    advance(cx, 399);
    assert_eq!(red_carets(cx), 1);
    advance(cx, 1);
    assert_eq!(red_carets(cx), 0);
    cx.update(|window, cx| {
        input.update(cx, |input, cx| {
            assert!(
                input
                    .bounds_for_range(0..1, Bounds::default(), window, cx)
                    .is_some()
            );
        });
    });
    // Composition is steady and has no timer; cancellation resumes a full phase.
    cx.update(|window, cx| {
        input.update(cx, |input, cx| {
            input.replace_and_mark_text_in_range(None, "é", Some(0..1), window, cx)
        })
    });
    draw(cx);
    assert_eq!(state(&input, cx), (true, false));
    advance(cx, 2000);
    assert_eq!(state(&input, cx), (true, false));
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            let result = view.otps[&node()].command(&o::Command::CancelComposition, window, cx);
            assert!(matches!(result, o::Response::Applied(_)));
        })
    });
    draw(cx);
    assert_eq!(state(&input, cx), (true, true));
}

#[::core::prelude::v1::test]
fn otp_caret_stops_for_inactive_policy_hidden_and_removed_owners() {
    let mut app = TestAppContext::single();
    let (owner, input, cx, _transport, _reader) = mount(&mut app);
    // Effective application motion preference, without modifying the editor.
    cx.update(|_, cx| cx.set_reduce_motion(true));
    // A suspended paint cannot let an already queued deadline keep blinking.
    cx.executor().advance_clock(Duration::from_millis(500));
    cx.run_until_parked();
    assert_eq!(state(&input, cx), (true, false));
    draw(cx);
    assert_eq!(state(&input, cx), (true, false));
    advance(cx, 2000);
    assert_eq!(state(&input, cx), (true, false));
    cx.update(|_, cx| cx.set_reduce_motion(false));
    draw(cx);
    assert_eq!(state(&input, cx), (true, true));
    cx.deactivate_window();
    assert_eq!(state(&input, cx), (true, false));
    advance(cx, 2000);
    assert_eq!(state(&input, cx), (true, false));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    draw(cx);
    assert_eq!(state(&input, cx), (true, true));
    // Read-only and nonempty selections have no blinking deadline.
    let config = input.read_with(cx, |input, _| input.model.config().clone());
    let mut read_only = config.clone();
    read_only.read_only = true;
    apply(
        &owner,
        cx,
        vec![Op::SetOtpInput(node(), read_only, "12".into())],
    );
    assert_eq!(state(&input, cx), (true, false));
    apply(
        &owner,
        cx,
        vec![Op::SetOtpInput(node(), config.clone(), "12".into())],
    );
    cx.update(|_, cx| input.update(cx, |input, cx| input.select_all(cx)));
    draw(cx);
    assert_eq!(state(&input, cx), (true, false));
    cx.update(|_, cx| {
        input.update(cx, |input, cx| {
            input.move_caret(edit::Movement::Right, false, cx)
        })
    });
    draw(cx);
    assert_eq!(state(&input, cx), (true, true));
    apply(
        &owner,
        cx,
        vec![Op::SetStyle(
            node(),
            vec![WireStyle::Fields(vec![StyleField::Display(3)])],
        )],
    );
    assert_eq!(state(&input, cx), (true, false));
    advance(cx, 2000);
    assert_eq!(state(&input, cx), (true, false));
    apply(&owner, cx, vec![Op::SetStyle(node(), vec![])]);
    cx.update(|window, cx| input.update(cx, |input, cx| window.focus(&input.focus, cx)));
    draw(cx);
    assert_eq!(state(&input, cx), (true, true));
    apply(&owner, cx, vec![Op::SetRoot(None), Op::Remove(node())]);
    assert_eq!(
        state(&input, cx),
        (true, false),
        "retirement cancels even with an external strong test reference"
    );
    let weak = input.downgrade();
    drop(input);
    draw(cx);
    advance(cx, 2000);
    assert!(
        weak.upgrade().is_none(),
        "timer cannot retain retired editor"
    );
}

#[::core::prelude::v1::test]
fn otp_caret_clipping_permission_and_window_close_release_the_timer() {
    let mut app = TestAppContext::single();
    let (owner, input, cx, transport, _reader) = mount(&mut app);
    apply(
        &owner,
        cx,
        vec![Op::SetStyle(
            node(),
            vec![WireStyle::Fields(vec![
                StyleField::Width(WireLength::Px(0.)),
                StyleField::MinWidth(WireLength::Px(0.)),
            ])],
        )],
    );
    assert!(!input.read_with(cx, |input, _| input.cursor.in_view));
    assert_eq!(state(&input, cx), (true, false));
    transport.mailbox.lock().unwrap().drain(256);
    advance(cx, 2000);
    assert!(transport.mailbox.lock().unwrap().drain(256).is_empty());
    apply(&owner, cx, vec![Op::SetStyle(node(), vec![])]);
    assert_eq!(state(&input, cx), (true, true));
    // The field still has nonzero layout bounds when an ancestor clips it out.
    let parent = NodeId::from_parts(1, 1).unwrap();
    let clip = |width| {
        Op::SetStyle(
            parent,
            vec![WireStyle::Fields(vec![
                StyleField::Width(WireLength::Px(width)),
                StyleField::Height(WireLength::Px(40.)),
                StyleField::OverflowX(1),
                StyleField::OverflowY(1),
            ])],
        )
    };
    apply(
        &owner,
        cx,
        vec![
            Op::Create(parent, Kind::Container, "".into(), None),
            Op::SetRoot(None),
            Op::Splice(parent, 0, 0, vec![node()]),
            clip(0.),
            Op::SetRoot(Some(parent)),
        ],
    );
    input.read_with(cx, |input, _| {
        assert!(input.layout.as_ref().unwrap().bounds.size.width > px(0.));
        assert!(!input.cursor.in_view);
    });
    assert_eq!(state(&input, cx), (true, false));
    apply(&owner, cx, vec![clip(320.)]);
    assert_eq!(state(&input, cx), (true, true));
    for native_disabled in [true, false] {
        let config = input.read_with(cx, |input, _| input.model.config().clone());
        let operation = if native_disabled {
            let mut config = config.clone();
            config.disabled = true;
            Op::SetOtpInput(node(), config, "12".into())
        } else {
            Op::SetStyle(
                node(),
                vec![WireStyle::Fields(vec![StyleField::Disabled(true)])],
            )
        };
        apply(&owner, cx, vec![operation]);
        assert_eq!(state(&input, cx), (true, false));
        advance(cx, 2000);
        assert_eq!(state(&input, cx), (true, false));
        apply(
            &owner,
            cx,
            vec![
                Op::SetOtpInput(node(), config, "12".into()),
                Op::SetStyle(node(), vec![]),
            ],
        );
        cx.update(|window, cx| input.update(cx, |input, cx| window.focus(&input.focus, cx)));
        draw(cx);
        assert_eq!(state(&input, cx), (true, true));
    }
    let weak = input.downgrade();
    let weak_owner = owner.downgrade();
    let (session, wid) = owner.read_with(cx, |view, _| (view.session.clone(), view.id));
    session.borrow_mut().close(wid).unwrap();
    drop(input);
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| view.sync_otps(&[], window, cx));
        window.remove_window();
    });
    drop(owner);
    cx.cx.update(|_| ());
    cx.run_until_parked();
    assert!(weak_owner.upgrade().is_none());
    // TestPlatform retains its last platform input handler for inspection after
    // closing a window. That handler can still hold the editor; the host must
    // cancel timing immediately even while this external owner survives.
    if let Some(input) = weak.upgrade() {
        assert_eq!(state(&input, cx), (true, false));
    }
    assert!(session.borrow().tree(wid).is_none());
    transport.mailbox.lock().unwrap().drain(256);
    cx.executor().advance_clock(Duration::from_secs(20));
    cx.run_until_parked();
    assert!(transport.mailbox.lock().unwrap().drain(256).is_empty());
}
