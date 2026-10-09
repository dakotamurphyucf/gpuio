//! Production drawn-menu state on TestPlatform; actual AppKit checks are separate.
use super::*;
use crate::{session::Session, transport::Transport};
use gpui::TestAppContext;
use gpuio_protocol::{
    HandlerId, WindowId,
    menu_command::{Command, Error, Position, Response},
};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn handler(generation: i64) -> HandlerId {
    HandlerId::from_parts(0, generation).unwrap()
}
#[test]
fn positioned_commands_fence_observers_and_close_unavailable_owners() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Menu commands", 600., 400.)
        .unwrap();
    let config = MenuConfig {
        presentation: MenuPresentation::Context,
        menus: vec![MenuDefinition {
            label: "Context".into(),
            disabled: false,
            items: vec![MenuItem::Label("Heading".into())],
        }],
    };
    session
        .borrow_mut()
        .apply(&Transaction {
            window: window_id,
            base: 0,
            revision: 1,
            operations: vec![
                Op::Create(id(0), Kind::Menu, "".into(), Some(handler(1))),
                Op::SetMenu(id(0), config),
                Op::Create(id(1), Kind::Text, "Anchor".into(), None),
                Op::Splice(id(0), 0, 0, vec![id(1)]),
                Op::SetRoot(Some(id(0))),
            ],
        })
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(window_id, session.clone(), transport.clone()));
    cx.simulate_resize(gpui::size(px(600.), px(400.)));
    cx.update(|window, _| window.activate_window());
    cx.run_until_parked();
    let show = Command::Show(Position { x: 120., y: 80. });
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        owner.update(cx, |view, cx| {
            assert_eq!(
                view.menu_command(id(0), handler(2), &show, window, cx),
                Response::Failed(Error::StaleMenu)
            );
            assert_eq!(
                view.menu_command(
                    id(0),
                    handler(1),
                    &Command::Show(Position { x: f64::NAN, y: 0. }),
                    window,
                    cx
                ),
                Response::Failed(Error::InvalidPosition)
            );
            assert_eq!(
                view.menu_command(id(0), handler(1), &show, window, cx),
                Response::Applied
            );
            assert_eq!(
                view.menus[&id(0)].borrow().context_position,
                Some(gpui::point(px(120.), px(80.)))
            );
            assert_eq!(
                view.menu_command(id(0), handler(1), &show, window, cx),
                Response::Failed(Error::Busy)
            );
        });
        window.draw(cx).clear(cx);
    });
    cx.deactivate_window();
    cx.update(|window, cx| {
        owner.update(cx, |view, cx| {
            assert_eq!(
                view.menu_command(id(0), handler(1), &Command::Close, window, cx),
                Response::Applied
            );
            assert_eq!(
                view.menu_command(id(0), handler(1), &Command::Close, window, cx),
                Response::Applied
            );
            assert_eq!(
                view.menu_command(id(0), handler(1), &show, window, cx),
                Response::Failed(Error::Unavailable)
            );
            session
                .borrow_mut()
                .apply(&Transaction {
                    window: window_id,
                    base: 1,
                    revision: 2,
                    operations: vec![Op::Bind(id(0), Some(handler(2)))],
                })
                .unwrap();
            assert_eq!(
                view.menu_command(id(0), handler(1), &Command::Close, window, cx),
                Response::Failed(Error::StaleMenu)
            );
            assert_eq!(
                view.menu_command(id(0), handler(2), &Command::Close, window, cx),
                Response::Applied
            );
        })
    });
    let states: Vec<_> = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(256)
        .into_iter()
        .filter_map(|event| match event {
            Event::MenuOpenChanged(_, _, _, _, open) => Some(open),
            _ => None,
        })
        .collect();
    assert_eq!(&states[..3], &[false, true, false]);
}
