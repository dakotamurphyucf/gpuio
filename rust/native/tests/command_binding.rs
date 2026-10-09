use gpuio_native::{session::Session, tree::Tree};
use gpuio_protocol::{HandlerId, NodeId, WindowId, command_binding::*, v1::*};
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn handler(slot: i64, generation: i64) -> HandlerId {
    HandlerId::from_parts(slot, generation).unwrap()
}
fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn config() -> Config {
    Config {
        context: Context::Here,
        targets: vec![Target::Command("run".into())],
    }
}
fn tx(base: i64, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: window(),
        base,
        revision: base + 1,
        operations,
    }
}
#[test]
fn query_admission_and_config_retirement_are_atomic_and_reserved() {
    let mut tree = Tree::new(window());
    let mut ops = vec![Op::Create(node(0), Kind::Container, "".into(), None)];
    for i in 1..=65 {
        ops.push(Op::Create(
            node(i),
            Kind::Container,
            "".into(),
            (i <= 64).then(|| handler(i, 1)),
        ));
        if i <= 64 {
            ops.push(Op::SetCommandBinding(node(i), Some(config())));
        }
    }
    ops.extend([
        Op::Splice(node(0), 0, 0, (1..=65).map(node).collect()),
        Op::SetRoot(Some(node(0))),
    ]);
    tree.apply(&tx(0, ops)).unwrap();
    assert_eq!(tree.binding_owners().count(), 64);
    assert!(tree.retained_bytes() >= 64 * 2 * (MAX_OBSERVATION_BYTES + 256));
    let bytes = tree.retained_bytes();
    let old =
        std::sync::Arc::downgrade(tree.get(node(1)).unwrap().command_binding.as_ref().unwrap());
    let extra = vec![
        Op::Bind(node(65), Some(handler(65, 1))),
        Op::SetCommandBinding(node(65), Some(config())),
    ];
    assert_eq!(
        tree.apply(&tx(1, extra.clone())),
        Err(ErrorCode::LimitExceeded)
    );
    assert_eq!(tree.revision(), 1);
    assert_eq!(tree.retained_bytes(), bytes);
    assert!(tree.get(node(65)).unwrap().command_binding.is_none());
    let changed = Config {
        context: Context::Focused,
        ..config()
    };
    assert_eq!(
        tree.apply(&tx(1, vec![Op::SetCommandBinding(node(1), Some(changed))])),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(
        tree.apply(&tx(
            1,
            vec![
                Op::Bind(node(1), Some(handler(1, 2))),
                Op::SetCommandBinding(
                    node(1),
                    Some(Config {
                        context: Context::Editor(WindowId::from_parts(1, 1).unwrap(), node(2)),
                        ..config()
                    })
                )
            ]
        )),
        Err(ErrorCode::InvalidTree)
    );
    let mut replace = vec![
        Op::Bind(node(1), None),
        Op::SetCommandBinding(node(1), None),
    ];
    replace.extend(extra);
    tree.apply(&tx(1, replace)).unwrap();
    assert_eq!(tree.binding_owners().count(), 64);
    assert!(old.upgrade().is_none());
}
#[test]
fn query_events_validate_owner_handler_context_and_retirement() {
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window(), "queries", 320., 240.).unwrap();
    session
        .apply(&tx(
            0,
            vec![
                Op::Create(node(0), Kind::Container, "".into(), Some(handler(0, 1))),
                Op::SetCommandBinding(node(0), Some(config())),
                Op::SetRoot(Some(node(0))),
            ],
        ))
        .unwrap();
    let observation = Observation {
        epoch: 1,
        state: State::Ready(vec![Entry::MissingCommand]),
    };
    assert!(
        session
            .command_binding_observed(window(), node(0), handler(0, 1), observation.clone())
            .is_some()
    );
    assert!(
        session
            .command_binding_observed(window(), node(0), handler(0, 2), observation.clone())
            .is_none()
    );
    assert!(
        session
            .command_binding_observed(
                window(),
                node(0),
                handler(0, 1),
                Observation {
                    epoch: 1,
                    state: State::ContextGone
                }
            )
            .is_none()
    );
    assert!(
        session.press(window(), node(0), handler(0, 1), 1).is_none(),
        "query handler is not a click handler"
    );
    session
        .apply(&tx(
            1,
            vec![
                Op::Bind(node(0), Some(handler(0, 2))),
                Op::SetCommandBinding(
                    node(0),
                    Some(Config {
                        context: Context::Focused,
                        ..config()
                    }),
                ),
            ],
        ))
        .unwrap();
    assert!(
        session
            .command_binding_observed(window(), node(0), handler(0, 1), observation.clone())
            .is_none()
    );
    assert!(
        session
            .command_binding_observed(window(), node(0), handler(0, 2), observation.clone())
            .is_some()
    );
    session.close(window()).unwrap();
    assert!(
        session
            .command_binding_observed(window(), node(0), handler(0, 2), observation)
            .is_none()
    );
}

#[test]
fn sampling_work_counts_hidden_declarations_and_ancestors_before_shadowing() {
    let mut tree = Tree::new(window());
    let command = |id: &str| CommandConfig {
        id: id.into(),
        label: id.into(),
        generation: 1,
        enabled: true,
        checked: None,
        shortcuts: vec![],
        target: CommandTarget::Callback,
    };
    tree.apply(&tx(
        0,
        vec![
            Op::Create(node(0), Kind::CommandScope, "".into(), Some(handler(0, 1))),
            Op::SetCommands(node(0), vec![command("run"), command("other")]),
            Op::Create(node(1), Kind::CommandScope, "".into(), Some(handler(1, 1))),
            Op::SetCommands(node(1), vec![command("run")]),
            Op::Splice(node(0), 0, 0, vec![node(1)]),
            Op::SetRoot(Some(node(0))),
        ],
    ))
    .unwrap();
    let mut work = 4;
    assert!(tree.bounded_commands_from(node(1), &mut work).is_none());
    assert_eq!(work, 0);
    let mut work = 5;
    let bounded = tree.bounded_commands_from(node(1), &mut work).unwrap();
    assert_eq!(bounded, tree.commands_from(node(1)).collect::<Vec<_>>());
    assert_eq!(
        bounded
            .iter()
            .map(|(scope, command)| (*scope, command.id.as_str()))
            .collect::<Vec<_>>(),
        vec![(node(1), "run"), (node(0), "other")]
    );
    assert_eq!(work, 0);
    assert_eq!(tree.revision(), 1);
}
