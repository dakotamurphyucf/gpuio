use gpuio_native::{
    session::{CommandInvocation, Session},
    tree::Tree,
};
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    button::{Config, Content, Focus, Policy},
    checkable::TabOrder,
    v1::*,
};
fn id(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn handler() -> HandlerId {
    HandlerId::from_parts(0, 1).unwrap()
}
fn tx(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: window(),
        base,
        revision: base + 1,
        operations,
    }
}
fn config(loading: bool, content: Content) -> Config {
    Config {
        policy: Policy {
            loading,
            focus: Focus::default(),
        },
        content,
    }
}
#[test]
fn rich_content_revalidates_descendants_and_resets_atomically() {
    let mut tree = Tree::new(window());
    tree.apply(&tx(
        0,
        vec![
            Op::Create(id(0), Kind::Button, "Run".into(), Some(handler())),
            Op::SetButtonPresentation(id(0), Some(config(false, Content::Rich))),
            Op::Create(id(1), Kind::Container, "".into(), None),
            Op::Create(id(2), Kind::Text, "Rich".into(), None),
            Op::Splice(id(1), 0, 0, vec![id(2)]),
            Op::Splice(id(0), 0, 0, vec![id(1)]),
            Op::SetRoot(Some(id(0))),
        ],
    ))
    .unwrap();
    let retained = tree.retained_bytes();
    for invalid in [
        Op::Bind(id(2), Some(handler())),
        Op::SetStyle(id(2), vec![Style::State(2, vec![Field::UserSelect(true)])]),
        Op::SetText(id(0), " \t".into()),
        Op::SetButtonPresentation(id(0), None),
        Op::SetButtonPresentation(id(1), Some(Config::default())),
        Op::SetButtonPresentation(
            id(0),
            Some(Config {
                policy: Policy {
                    loading: true,
                    focus: Focus::Focusable(TabOrder {
                        tab_stop: false,
                        index: i64::MAX,
                    }),
                },
                content: Content::Rich,
            }),
        ),
    ] {
        assert!(
            tree.apply(&tx(1, vec![Op::SetText(id(2), "Rollback".into()), invalid]))
                .is_err()
        );
        assert_eq!(tree.revision(), 1);
        assert_eq!(tree.get(id(2)).unwrap().text.as_ref(), "Rich");
        assert_eq!(tree.retained_bytes(), retained);
        assert_eq!(tree.get(id(0)).unwrap().button_activation_revision, 0);
    }
    tree.apply(&tx(
        1,
        vec![
            Op::SetButtonPresentation(id(0), None),
            Op::Splice(id(0), 0, 1, vec![]),
            Op::Splice(id(1), 0, 1, vec![]),
            Op::Remove(id(2)),
            Op::Remove(id(1)),
        ],
    ))
    .unwrap();
    assert_eq!(tree.get(id(0)).unwrap().handler, Some(handler()));
    assert!(tree.get(id(0)).unwrap().button_presentation.is_none());
    tree.apply(&tx(2, vec![Op::SetRoot(None), Op::Remove(id(0))]))
        .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
}
#[test]
fn busy_command_owner_fences_old_activation_without_disabling_other_owners() {
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window(), "commands", 400., 300.).unwrap();
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(id(0), Kind::CommandScope, "".into(), Some(handler())),
                Op::SetCommands(
                    id(0),
                    vec![
                        CommandConfig {
                            id: "other".into(),
                            generation: 2,
                            label: "Other".into(),
                            enabled: true,
                            checked: None,
                            shortcuts: vec![],
                            target: CommandTarget::Callback,
                        },
                        CommandConfig {
                            id: "run".into(),
                            generation: 1,
                            label: "Run".into(),
                            enabled: true,
                            checked: None,
                            shortcuts: vec![],
                            target: CommandTarget::Callback,
                        },
                    ],
                ),
                Op::Create(id(1), Kind::CommandButton, "".into(), None),
                Op::SetCommandRef(id(1), "run".into()),
                Op::Create(id(2), Kind::CommandButton, "".into(), None),
                Op::SetCommandRef(id(2), "run".into()),
                Op::Splice(id(0), 0, 0, vec![id(1), id(2)]),
                Op::SetRoot(Some(id(0))),
            ],
        ))
        .unwrap();
    let request = CommandInvocation {
        scope: id(0),
        handler: handler(),
        revision: 1,
        command: "run",
        generation: 1,
        source: CommandSource::Button(id(1)),
    };
    assert!(session.invoke_command(window(), request).is_some());
    session
        .apply(&tx(
            1,
            vec![Op::SetButtonPresentation(
                id(1),
                Some(config(true, Content::IconSlots)),
            )],
        ))
        .unwrap();
    assert!(session.invoke_command(window(), request).is_none());
    assert!(
        session
            .invoke_command(
                window(),
                CommandInvocation {
                    source: CommandSource::Button(id(2)),
                    ..request
                }
            )
            .is_some()
    );
    assert!(
        session
            .invoke_command(
                window(),
                CommandInvocation {
                    revision: 2,
                    ..request
                }
            )
            .is_none()
    );
    session
        .apply(&tx(2, vec![Op::SetButtonPresentation(id(1), None)]))
        .unwrap();
    assert!(session.invoke_command(window(), request).is_none());
    assert!(
        session
            .invoke_command(
                window(),
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
                window(),
                CommandInvocation {
                    revision: 3,
                    ..request
                }
            )
            .is_some()
    );
    assert!(
        session
            .invoke_command(
                window(),
                CommandInvocation {
                    source: CommandSource::Button(id(2)),
                    ..request
                }
            )
            .is_some()
    );
    session
        .apply(&tx(3, vec![Op::SetCommandRef(id(1), "other".into())]))
        .unwrap();
    session
        .apply(&tx(4, vec![Op::SetCommandRef(id(1), "run".into())]))
        .unwrap();
    assert!(
        session
            .invoke_command(
                window(),
                CommandInvocation {
                    revision: 3,
                    ..request
                }
            )
            .is_none()
    );
    assert!(
        session
            .invoke_command(
                window(),
                CommandInvocation {
                    revision: 5,
                    ..request
                }
            )
            .is_some()
    );
    assert!(
        session
            .invoke_command(
                window(),
                CommandInvocation {
                    source: CommandSource::Button(id(2)),
                    ..request
                }
            )
            .is_some()
    );
    session.close(window()).unwrap();
    assert!(session.invoke_command(window(), request).is_none());
    assert_eq!(session.retained_bytes(), 0);
}

#[test]
fn rich_progress_content_cannot_smuggle_an_interactive_center() {
    let mut tree = Tree::new(window());
    tree.apply(&tx(
        0,
        vec![
            Op::Create(id(0), Kind::Button, "Upload".into(), Some(handler())),
            Op::SetButtonPresentation(id(0), Some(config(true, Content::Rich))),
            Op::Create(id(1), Kind::Progress, "".into(), None),
            Op::SetProgressPresentation(
                id(1),
                gpuio_protocol::progress_presentation::Config {
                    progress: ProgressConfig {
                        label: "Uploading".into(),
                        fraction: Some(0.5),
                    },
                    shape: gpuio_protocol::progress_presentation::Shape::Circle,
                    transition: gpuio_protocol::progress_presentation::Transition::Immediate,
                },
            ),
            Op::Create(id(2), Kind::Text, "50%".into(), None),
            Op::Splice(id(1), 0, 0, vec![id(2)]),
            Op::Splice(id(0), 0, 0, vec![id(1)]),
            Op::SetRoot(Some(id(0))),
        ],
    ))
    .unwrap();
    let retained = tree.retained_bytes();
    assert!(
        tree.apply(&tx(
            1,
            vec![
                Op::Create(id(3), Kind::Button, "Nested".into(), Some(handler())),
                Op::Splice(id(1), 0, 1, vec![id(3)]),
                Op::Remove(id(2)),
            ]
        ))
        .is_err()
    );
    assert_eq!(tree.retained_bytes(), retained);
    assert_eq!(tree.get(id(1)).unwrap().children.as_ref(), &[id(2)]);
}
