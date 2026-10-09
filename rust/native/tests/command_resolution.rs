use gpuio_native::tree::Tree;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
use std::sync::Arc;

fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn command(id: &str, key: Option<&str>, enabled: bool) -> CommandConfig {
    CommandConfig {
        id: id.into(),
        generation: 1,
        label: id.into(),
        enabled,
        checked: None,
        shortcuts: key
            .map(|key| Shortcut {
                key: key.into(),
                modifiers: vec![ShortcutModifier::Primary],
                priority: ShortcutPriority::NativeFirst,
                text_input: ShortcutTextInput::ModifiedOnly,
                during_composition: false,
            })
            .into_iter()
            .collect(),
        target: CommandTarget::Callback,
    }
}
fn apply(tree: &mut Tree, operations: Vec<Op>) {
    tree.apply(&Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: tree.revision(),
        revision: tree.revision() + 1,
        operations,
    })
    .unwrap();
}
fn fixture() -> Tree {
    let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
    let mut operations = vec![];
    for slot in 0..6 {
        let scope = [0, 1, 3].contains(&slot);
        operations.push(Op::Create(
            node(slot),
            if scope {
                Kind::CommandScope
            } else {
                Kind::Container
            },
            "".into(),
            scope.then(|| HandlerId::from_parts(slot, 1).unwrap()),
        ));
    }
    operations.extend([
        Op::SetCommands(
            node(0),
            vec![
                command("run", Some("k"), true),
                command("outer-only", Some("o"), true),
                command("fallback", Some("k"), true),
            ],
        ),
        Op::SetCommands(
            node(1),
            vec![
                command("run", None, false),
                command("inner-only", Some("i"), true),
            ],
        ),
        Op::SetCommands(node(3), vec![command("sibling-only", Some("s"), true)]),
        Op::Splice(node(0), 0, 0, vec![node(1), node(3), node(5)]),
        Op::Splice(node(1), 0, 0, vec![node(2)]),
        Op::Splice(node(3), 0, 0, vec![node(4)]),
        Op::SetRoot(Some(node(0))),
    ]);
    apply(&mut tree, operations);
    tree
}

fn names(tree: &Tree, at: i64) -> Vec<(NodeId, &str)> {
    tree.commands_from(node(at))
        .map(|(scope, config)| (scope, config.id.as_str()))
        .collect()
}

#[test]
fn nearest_ids_shadow_before_filters_and_siblings_do_not_leak() {
    let tree = fixture();
    assert_eq!(
        names(&tree, 2),
        [
            (node(1), "run"),
            (node(1), "inner-only"),
            (node(0), "outer-only"),
            (node(0), "fallback")
        ]
    );
    assert_eq!(
        names(&tree, 4),
        [
            (node(3), "sibling-only"),
            (node(0), "run"),
            (node(0), "outer-only"),
            (node(0), "fallback")
        ]
    );
    assert_eq!(
        names(&tree, 5),
        [
            (node(0), "run"),
            (node(0), "outer-only"),
            (node(0), "fallback")
        ]
    );
    assert!(names(&tree, 99).is_empty());
    let enabled: Vec<_> = tree
        .commands_from(node(2))
        .filter(|(_, config)| config.enabled)
        .map(|(_, config)| config.id.as_str())
        .collect();
    assert_eq!(enabled, ["inner-only", "outer-only", "fallback"]);
    for at in [0, 1, 2, 3, 4, 5] {
        for (scope, config) in tree.commands_from(node(at)) {
            let (named_scope, named_config) = tree.command(node(at), &config.id).unwrap();
            assert_eq!(scope, named_scope);
            assert!(
                Arc::ptr_eq(config, named_config),
                "lookup must borrow the current shared definition"
            );
        }
    }
}

#[test]
fn matching_preserves_inner_and_declaration_order_even_for_disabled_entries() {
    let mut tree = fixture();
    let first_k = |tree: &Tree| {
        tree.commands_from(node(2))
            .find(|(_, config)| config.shortcuts.iter().any(|shortcut| shortcut.key == "k"))
            .map(|(scope, config)| (scope, config.id.clone(), config.enabled))
    };
    // Empty inner run hides outer run; a different outer command remains eligible.
    assert_eq!(first_k(&tree), Some((node(0), "fallback".into(), true)));
    apply(
        &mut tree,
        vec![Op::SetCommands(
            node(1),
            vec![
                command("inner-first", Some("k"), false),
                command("run", Some("k"), true),
            ],
        )],
    );
    assert_eq!(first_k(&tree), Some((node(1), "inner-first".into(), false)));
    apply(
        &mut tree,
        vec![Op::SetCommands(
            node(1),
            vec![
                command("run", Some("k"), true),
                command("inner-first", Some("k"), false),
            ],
        )],
    );
    assert_eq!(first_k(&tree), Some((node(1), "run".into(), true)));
    apply(&mut tree, vec![Op::SetCommands(node(1), vec![])]);
    assert_eq!(first_k(&tree), Some((node(0), "run".into(), true)));
}

#[test]
fn moving_context_and_replacing_definitions_changes_lookup_without_retaining_old_payloads() {
    let mut tree = fixture();
    let old = Arc::downgrade(tree.command(node(2), "run").unwrap().1);
    let revision = tree.revision();
    let retained = tree.retained_bytes();
    for _ in 0..100 {
        assert_eq!(tree.commands_from(node(2)).count(), 4);
    }
    assert_eq!(tree.revision(), revision);
    assert_eq!(tree.retained_bytes(), retained);
    apply(
        &mut tree,
        vec![Op::SetCommands(
            node(1),
            vec![command("replacement", Some("r"), true)],
        )],
    );
    assert!(old.upgrade().is_none());
    apply(
        &mut tree,
        vec![
            Op::Splice(node(1), 0, 1, vec![]),
            Op::Splice(node(3), 1, 0, vec![node(2)]),
        ],
    );
    assert_eq!(names(&tree, 2), names(&tree, 4));
    apply(
        &mut tree,
        vec![Op::Splice(node(3), 1, 1, vec![]), Op::Remove(node(2))],
    );
    assert!(tree.commands_from(node(2)).next().is_none());
}
