use gpuio_native::session::{CommandInvocation, Session};
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn handler(slot: i64) -> HandlerId {
    HandlerId::from_parts(slot, 1).unwrap()
}
fn command(generation: i64, enabled: bool) -> CommandConfig {
    CommandConfig {
        id: "run".into(),
        generation,
        label: "Run".into(),
        enabled,
        checked: None,
        shortcuts: vec![],
        target: CommandTarget::Callback,
    }
}
#[test]
fn registry_shadowing_reference_changes_and_invocation_guards_are_atomic() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "commands", 400., 300.).unwrap();
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
                Op::Create(node(0), Kind::CommandScope, "".into(), Some(handler(0))),
                Op::SetCommands(node(0), vec![command(1, true)]),
                Op::Create(node(1), Kind::CommandButton, "".into(), None),
                Op::SetCommandRef(node(1), "run".into()),
                Op::Create(node(2), Kind::CommandScope, "".into(), Some(handler(2))),
                Op::SetCommands(node(2), vec![command(2, false)]),
                Op::Create(node(3), Kind::CommandButton, "".into(), None),
                Op::SetCommandRef(node(3), "run".into()),
                Op::Splice(node(0), 0, 0, vec![node(1), node(2)]),
                Op::Splice(node(2), 0, 0, vec![node(3)]),
                Op::SetRoot(Some(node(0))),
            ],
        ))
        .unwrap();
    let request = CommandInvocation {
        scope: node(0),
        handler: handler(0),
        revision: 1,
        command: "run",
        generation: 1,
        source: CommandSource::Button(node(1)),
    };
    assert!(session.invoke_command(window, request).is_some());
    for source in [CommandSource::Menu, CommandSource::Palette(node(1))] {
        assert!(
            session
                .invoke_command(window, CommandInvocation { source, ..request })
                .is_none()
        );
    }
    assert!(
        session
            .invoke_command(
                window,
                CommandInvocation {
                    source: CommandSource::Button(node(3)),
                    ..request
                }
            )
            .is_none()
    );
    assert!(
        session
            .invoke_command(
                window,
                CommandInvocation {
                    revision: 2,
                    ..request
                }
            )
            .is_none()
    );
    assert!(
        session
            .invoke_command(
                window,
                CommandInvocation {
                    generation: 2,
                    ..request
                }
            )
            .is_none()
    );
    let retained = session.retained_bytes();
    for operations in [
        vec![Op::SetCommandRef(node(1), "missing".into())],
        vec![Op::SetCommands(node(0), vec![])],
        vec![Op::SetCommands(
            node(0),
            vec![command(1, true), command(2, true)],
        )],
        vec![Op::SetCommands(node(0), vec![command(0, true)])],
    ] {
        assert_eq!(
            session.apply(&tx(1, operations)),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(session.retained_bytes(), retained);
        assert_eq!(session.tree(window).unwrap().revision(), 1);
    }
    session
        .apply(&tx(1, vec![Op::SetCommands(node(2), vec![])]))
        .unwrap();
    assert!(
        session
            .invoke_command(
                window,
                CommandInvocation {
                    source: CommandSource::Button(node(3)),
                    ..request
                }
            )
            .is_some()
    );
    session
        .apply(&tx(
            2,
            vec![Op::SetCommands(node(0), vec![command(3, false)])],
        ))
        .unwrap();
    assert!(session.invoke_command(window, request).is_none());
    session
        .apply(&tx(
            3,
            vec![Op::SetCommands(node(0), vec![command(4, true)])],
        ))
        .unwrap();
    assert!(session.invoke_command(window, request).is_none());
    assert!(
        session
            .invoke_command(
                window,
                CommandInvocation {
                    generation: 4,
                    ..request
                }
            )
            .is_some()
    );
    session
        .apply(&tx(
            4,
            vec![
                Op::Remove(node(3)),
                Op::Remove(node(2)),
                Op::Remove(node(1)),
                Op::Remove(node(0)),
                Op::SetRoot(None),
            ],
        ))
        .unwrap();
    assert_eq!(session.retained_bytes(), 0);
    assert!(session.invoke_command(window, request).is_none());
}
