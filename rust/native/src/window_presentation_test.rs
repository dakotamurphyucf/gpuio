//! Native policy and presentation observation without actual OS window operations.
use super::*;
use crate::{session::Session, transport::Transport};
use gpui::{TestAppContext, VisualTestContext};
use std::os::{fd::AsRawFd, unix::net::UnixStream};

#[test]
fn native_policy_is_observed_and_unsupported_commands_never_call_the_platform() {
    let mut app = TestAppContext::single();
    app.update(gpui_base::init);
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let id = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, id, "Policy", 400., 200.)
        .unwrap();
    let native = app.update(|cx| {
        cx.open_window(
            WindowOptions {
                is_resizable: false,
                is_minimizable: false,
                ..Default::default()
            },
            |_, cx| cx.new(|_| View::new(id, session.clone(), transport.clone())),
        )
        .unwrap()
    });
    let owner = native.root(&mut app).unwrap();
    let mut cx = VisualTestContext::from_window(native.into(), &app);
    let draw = |cx: &mut VisualTestContext| {
        cx.run_until_parked();
        cx.update(|w, cx| w.draw(cx).clear(cx));
        cx.run_until_parked();
    };
    draw(&mut cx);
    let actual = cx.update(|window, cx| {
        owner.update(cx, |view, _| {
            let expected = presentation(window);
            assert_eq!(expected.decorations, wire::Decorations::Server);
            assert!(
                !expected.resizable && !expected.controls.maximize && !expected.controls.minimize
            );
            // TestWindow deliberately does not implement these OS calls. A missing
            // production guard therefore panics, instead of succeeding in a recorder.
            for action in [wire::Command::Minimize, wire::Command::Zoom] {
                assert_eq!(
                    command(view, &action, window),
                    wire::Response::Failed(wire::Error::Unsupported)
                );
            }
            let wire::Response::Observed(snapshot) = command(view, &wire::Command::Observe, window)
            else {
                panic!("observe failed");
            };
            assert_eq!(snapshot.presentation, expected);
            expected
        })
    });
    transport.mailbox.lock().unwrap().drain(128);
    draw(&mut cx);
    draw(&mut cx);
    assert!(
        transport.mailbox.lock().unwrap().drain(128).is_empty(),
        "unchanged paints publish nothing"
    );

    // Model a previously observed client presentation, then let production
    // rendering read the real TestWindow's server state. Bounds are unchanged.
    for top in [true, false, true] {
        cx.update(|_, cx| {
            owner.update(cx, |view, _| {
                view.last_presentation = Some(wire::Presentation {
                    decorations: wire::Decorations::Client(wire::Tiling {
                        top,
                        right: false,
                        bottom: false,
                        left: false,
                    }),
                    ..actual
                });
            })
        });
        draw(&mut cx);
    }
    let events = transport.mailbox.lock().unwrap().drain(128);
    assert_eq!(events.len(), 1, "metadata observations coalesce per window");
    let Event::WindowChanged(observed, snapshot) = &events[0] else {
        panic!("missing presentation observation");
    };
    assert_eq!(*observed, id);
    assert_eq!(snapshot.presentation, actual);
    draw(&mut cx);
    assert!(transport.mailbox.lock().unwrap().drain(128).is_empty());
}
