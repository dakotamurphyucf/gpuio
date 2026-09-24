use gpuio_native::session::{CommandInvocation, Session};
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn menu(disabled: bool) -> MenuConfig {
    MenuConfig {
        presentation: MenuPresentation::Context,
        menus: vec![MenuDefinition {
            label: "File".into(),
            disabled: false,
            items: vec![MenuItem::Submenu(MenuDefinition {
                label: "More".into(),
                disabled,
                items: vec![MenuItem::Command("run".into())],
            })],
        }],
    }
}
#[test]
fn menu_sources_validate_enabled_paths_and_survive_atomic_graph_changes() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "menus", 400., 300.).unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(node(0), Kind::CommandScope, "".into(), Some(handler)),
                Op::SetCommands(
                    node(0),
                    vec![CommandConfig {
                        id: "run".into(),
                        label: "Run".into(),
                        generation: 1,
                        enabled: true,
                        checked: None,
                        shortcuts: vec![],
                        target: CommandTarget::Callback,
                    }],
                ),
                Op::Create(node(1), Kind::Menu, "".into(), None),
                Op::SetMenu(node(1), menu(true)),
                Op::Create(node(2), Kind::Text, "Child".into(), None),
                Op::Splice(node(1), 0, 0, vec![node(2)]),
                Op::Splice(node(0), 0, 0, vec![node(1)]),
                Op::SetRoot(Some(node(0))),
            ],
        ))
        .unwrap();
    let request = CommandInvocation {
        scope: node(0),
        handler,
        revision: 1,
        command: "run",
        generation: 1,
        source: CommandSource::Menu(node(1)),
    };
    assert!(session.invoke_command(window, request).is_none());
    session
        .apply(&tx(1, vec![Op::SetMenu(node(1), menu(false))]))
        .unwrap();
    assert!(session.invoke_command(window, request).is_some());
    let bytes = session.retained_bytes();
    let mut missing = menu(false);
    missing.menus[0].items = vec![MenuItem::Command("missing".into())];
    let mut bar = menu(false);
    bar.presentation = MenuPresentation::PlatformBar;
    for operations in [
        vec![Op::SetMenu(node(1), missing)],
        vec![Op::SetCommands(node(0), vec![])],
        vec![Op::SetMenu(node(1), bar.clone())], // Context child invalid on a bar.
        vec![
            Op::Create(node(3), Kind::Menu, "".into(), None),
            Op::SetMenu(node(3), bar.clone()),
            Op::Create(node(4), Kind::Menu, "".into(), None),
            Op::SetMenu(node(4), bar),
            Op::Splice(node(0), 1, 0, vec![node(3), node(4)]),
        ],
    ] {
        assert_eq!(
            session.apply(&tx(2, operations)),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(session.retained_bytes(), bytes);
        assert_eq!(session.tree(window).unwrap().revision(), 2);
    }
    session
        .apply(&tx(
            2,
            vec![
                Op::Splice(node(1), 0, 1, vec![]),
                Op::Splice(node(0), 0, 1, vec![node(2)]),
                Op::Remove(node(1)),
            ],
        ))
        .unwrap();
    assert!(
        session.invoke_command(window, request).is_none(),
        "menu unmounted while its command remains"
    );
    session.close(window).unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
