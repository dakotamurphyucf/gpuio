//! Snapshot construction uses the production View and registry on TestPlatform.
//! This does not run AppKit or certify physical interaction.
use super::*;
use crate::{session::Session, transport::Transport};
use gpui::{TestAppContext, px};
use gpuio_protocol::{HandlerId, WindowId};
use std::{os::fd::AsRawFd, os::unix::net::UnixStream};

fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn command(name: &str, enabled: bool) -> CommandConfig {
    CommandConfig {
        id: name.into(),
        label: format!("Action {name}"),
        generation: 1,
        enabled,
        checked: Some(true),
        shortcuts: vec![],
        target: CommandTarget::Callback,
    }
}

#[test]
fn popup_snapshot_preserves_registry_state_and_disabled_ancestry() {
    let mut app = TestAppContext::single();
    let (_reader, writer) = UnixStream::pair().unwrap();
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    let session = Rc::new(RefCell::new(Session::default()));
    let window_id = WindowId::from_parts(0, 1).unwrap();
    session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
    session
        .borrow_mut()
        .open(1, window_id, "Popup snapshot", 400., 300.)
        .unwrap();
    let config = MenuConfig {
        presentation: MenuPresentation::PlatformContext,
        menus: vec![MenuDefinition {
            label: "Root".into(),
            disabled: false,
            items: vec![
                MenuItem::Label("Section".into()),
                MenuItem::Command("run".into()),
                MenuItem::Submenu(MenuDefinition {
                    label: "Disabled".into(),
                    disabled: true,
                    items: vec![MenuItem::Command("run".into())],
                }),
                MenuItem::Separator,
                MenuItem::Command("off".into()),
            ],
        }],
    };
    session
        .borrow_mut()
        .apply(&Transaction {
            window: window_id,
            base: 0,
            revision: 1,
            operations: vec![
                Op::Create(
                    id(0),
                    Kind::CommandScope,
                    "".into(),
                    Some(HandlerId::from_parts(0, 1).unwrap()),
                ),
                Op::SetCommands(id(0), vec![command("run", true), command("off", false)]),
                Op::Create(id(1), Kind::Menu, "".into(), None),
                Op::SetMenu(id(1), config.clone()),
                Op::Create(id(2), Kind::Text, "Target".into(), None),
                Op::Splice(id(1), 0, 0, vec![id(2)]),
                Op::Splice(id(0), 0, 0, vec![id(1)]),
                Op::SetRoot(Some(id(0))),
            ],
        })
        .unwrap();
    let (owner, cx) =
        app.add_window_view(|_, _| View::new(window_id, session.clone(), transport.clone()));
    cx.simulate_resize(gpui::size(px(400.), px(300.)));
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        owner.update(cx, |view, cx| {
            let session = session.borrow();
            let tree = session.tree(window_id).unwrap();
            let mut routes = Vec::new();
            let items = view.popup_items(tree, id(1), &config.menus[0], (window, cx), &mut routes, false);
            assert!(matches!(&items[0], popup::Item::Row { label, enabled: false, action: None, .. } if label == "Section"));
            assert!(matches!(&items[1], popup::Item::Row { label, enabled: true, checked: true, action: Some(0) } if label == "Action run"));
            let popup::Item::Submenu { enabled: false, items: children, .. } = &items[2] else { panic!("disabled submenu") };
            assert!(matches!(&children[0], popup::Item::Row { enabled: false, action: Some(1), .. }));
            assert!(matches!(&items[3], popup::Item::Separator));
            assert!(matches!(&items[4], popup::Item::Row { enabled: false, action: Some(2), .. }));
            assert_eq!(routes.len(), 3);
            assert_eq!(routes[0].request().command, "run");
            assert_eq!(routes[1].request().command, "run");
            assert_eq!(routes[2].request().command, "off");
            assert_eq!(routes[0].request().source, CommandSource::Menu(id(1)));
            let mut disabled_routes = Vec::new();
            let disabled = view.popup_items(tree, id(1), &config.menus[0], (window, cx), &mut disabled_routes, true);
            assert!(matches!(&disabled[1], popup::Item::Row { enabled: false, .. }));
        });
    });
}
