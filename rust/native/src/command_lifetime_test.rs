use super::*;
use crate::session::Session;
use gpuio_protocol::WindowId;

#[test]
fn routes_share_entries_and_release_obsolete_generations() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let scope = NodeId::from_parts(0, 1).unwrap();
    let button = NodeId::from_parts(1, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session
        .open(1, window, "command lifetimes", 400., 300.)
        .unwrap();
    let command = |generation| CommandConfig {
        id: "run".into(),
        generation,
        label: "Run".repeat(1000),
        enabled: true,
        checked: None,
        shortcuts: vec![],
        target: CommandTarget::Callback,
    };
    let transaction = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    session
        .apply(&transaction(
            0,
            vec![
                Op::Create(scope, Kind::CommandScope, "".into(), Some(handler)),
                Op::SetCommands(scope, vec![command(1)]),
                Op::Create(button, Kind::CommandButton, "".into(), None),
                Op::SetCommandRef(button, "run".into()),
                Op::Splice(scope, 0, 0, vec![button]),
                Op::SetRoot(Some(scope)),
            ],
        ))
        .unwrap();
    let mut routes: Vec<_> = (0..1024)
        .map(|_| {
            let tree = session.tree(window).unwrap();
            let (scope, config) = tree.command(button, "run").unwrap();
            Route::new(tree, scope, config, CommandSource::Button(button))
        })
        .collect();
    let old = Arc::downgrade(&routes[0].config);
    assert!(routes.iter().all(|route| {
        Arc::ptr_eq(
            &route.config,
            session
                .tree(window)
                .unwrap()
                .command(button, "run")
                .unwrap()
                .1,
        )
    }));
    assert!(
        session
            .command_target(window, routes[0].request())
            .is_some()
    );
    session
        .apply(&transaction(
            1,
            vec![Op::SetCommands(scope, vec![command(2)])],
        ))
        .unwrap();
    assert!(
        session
            .command_target(window, routes[0].request())
            .is_none()
    );
    assert_eq!(old.upgrade().unwrap().generation, 1);
    let current = {
        let tree = session.tree(window).unwrap();
        let (scope, config) = tree.command(button, "run").unwrap();
        Route::new(tree, scope, config, CommandSource::Button(button))
    };
    assert!(session.command_target(window, current.request()).is_some());
    let last = routes.pop().unwrap();
    drop(routes);
    assert!(
        old.upgrade().is_some(),
        "last stale frame still owns its snapshot"
    );
    drop(last);
    assert!(
        old.upgrade().is_none(),
        "no cache retains an obsolete generation"
    );
    let live = Arc::downgrade(&current.config);
    session
        .apply(&transaction(
            2,
            vec![Op::SetRoot(None), Op::Remove(button), Op::Remove(scope)],
        ))
        .unwrap();
    assert!(session.command_target(window, current.request()).is_none());
    drop(current);
    assert!(
        live.upgrade().is_none(),
        "last disposed route releases the entry"
    );
}
