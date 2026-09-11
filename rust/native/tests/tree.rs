use gpuio_native::tree::Tree;
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};

fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn id(slot: i64, generation: i64) -> NodeId {
    NodeId::from_parts(slot, generation).unwrap()
}
fn tx(tree: &Tree, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: window(),
        base: tree.revision(),
        revision: tree.revision() + 1,
        operations,
    }
}
fn initial() -> Tree {
    let mut tree = Tree::new(window());
    tree.apply(&tx(
        &tree,
        vec![
            Op::Create(id(0, 1), Kind::Container, String::new(), None),
            Op::Create(
                id(1, 1),
                Kind::Button,
                "old".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::Splice(id(0, 1), 0, 0, vec![id(1, 1)]),
            Op::SetRoot(Some(id(0, 1))),
        ],
    ))
    .unwrap();
    tree
}

#[test]
fn invalid_last_operation_does_not_publish_earlier_changes() {
    let mut tree = initial();
    let result = tree.apply(&tx(
        &tree,
        vec![
            Op::SetText(id(1, 1), "must roll back".into()),
            Op::Create(id(2, 1), Kind::Text, "orphan".into(), None),
        ],
    ));
    assert_eq!(result, Err(ErrorCode::InvalidTree));
    assert_eq!(tree.revision(), 1);
    assert_eq!(tree.get(id(1, 1)).unwrap().text.as_ref(), "old");
    assert!(tree.get(id(2, 1)).is_none());
    tree.apply(&tx(
        &tree,
        vec![
            Op::Create(id(2, 1), Kind::Text, "valid".into(), None),
            Op::Splice(id(0, 1), 1, 0, vec![id(2, 1)]),
        ],
    ))
    .unwrap();
    assert_eq!(tree.len(), 3);
}

#[test]
fn removed_slot_reuse_rejects_old_node_and_handler_generation() {
    let mut tree = initial();
    let old_handler = HandlerId::from_parts(0, 1).unwrap();
    let new_handler = HandlerId::from_parts(0, 2).unwrap();
    assert!(tree.accepts_handler(id(1, 1), old_handler));
    tree.apply(&tx(
        &tree,
        vec![
            Op::Remove(id(1, 1)),
            Op::Create(
                id(1, 2),
                Kind::Button,
                "replacement".into(),
                Some(new_handler),
            ),
            Op::Splice(id(0, 1), 0, 1, vec![id(1, 2)]),
        ],
    ))
    .unwrap();
    assert!(!tree.accepts_handler(id(1, 1), old_handler));
    assert!(!tree.accepts_handler(id(1, 2), old_handler));
    assert!(tree.accepts_handler(id(1, 2), new_handler));
    assert_eq!(
        tree.apply(&tx(&tree, vec![Op::SetText(id(1, 1), "late".into())])),
        Err(ErrorCode::StaleHandle)
    );
    assert_eq!(tree.get(id(1, 2)).unwrap().text.as_ref(), "replacement");
}

#[test]
fn cycles_duplicate_parents_invalid_style_and_stale_revisions_are_atomic() {
    let mut tree = initial();
    for operations in [
        vec![Op::Splice(id(0, 1), 0, 0, vec![id(0, 1)])],
        vec![Op::Splice(id(0, 1), 0, 0, vec![id(1, 1)])],
        vec![Op::SetStyle(id(1, 1), vec![Style::Opacity(f64::NAN)])],
        vec![Op::Splice(id(0, 1), -1, 0, vec![])],
        vec![Op::Remove(id(1, 1))],
    ] {
        assert!(tree.apply(&tx(&tree, operations)).is_err());
        assert_eq!(tree.len(), 2);
        assert_eq!(tree.revision(), 1);
    }
    let mut stale = tx(&tree, vec![]);
    stale.base = 0;
    assert_eq!(tree.apply(&stale), Err(ErrorCode::InvalidRevision));
    stale = tx(&tree, vec![]);
    stale.window = WindowId::from_parts(0, 2).unwrap();
    assert_eq!(tree.apply(&stale), Err(ErrorCode::StaleHandle));
}

#[test]
fn small_edit_does_not_copy_or_validate_unrelated_history() {
    let mut tree = Tree::new(window());
    tree.apply(&tx(
        &tree,
        vec![
            Op::Create(id(0, 1), Kind::Container, String::new(), None),
            Op::SetRoot(Some(id(0, 1))),
        ],
    ))
    .unwrap();
    for start in (1..10_001).step_by(1000) {
        let nodes: Vec<_> = (start..start + 1000).map(|slot| id(slot, 1)).collect();
        let mut operations: Vec<_> = nodes
            .iter()
            .map(|id| Op::Create(*id, Kind::Text, "x".repeat(1024), None))
            .collect();
        operations.push(Op::Splice(id(0, 1), start - 1, 0, nodes));
        tree.apply(&tx(&tree, operations)).unwrap();
    }
    let unchanged_text = tree.get(id(9000, 1)).unwrap().text.clone();
    let children = tree.get(id(0, 1)).unwrap().children.clone();
    let started = std::time::Instant::now();
    let applied = tree
        .apply(&tx(&tree, vec![Op::SetText(id(5000, 1), "changed".into())]))
        .unwrap();
    assert_eq!(applied.touched_records, 1);
    assert_eq!(applied.validated_nodes, 0);
    assert_eq!(applied.dirty, vec![id(0, 1), id(5000, 1)]);
    assert!(std::sync::Arc::ptr_eq(
        &unchanged_text,
        &tree.get(id(9000, 1)).unwrap().text
    ));
    assert!(std::sync::Arc::ptr_eq(
        &children,
        &tree.get(id(0, 1)).unwrap().children
    ));
    eprintln!(
        "10001 nodes, 10 MiB text: one edit {:?}, {} touched, {} scanned",
        started.elapsed(),
        applied.touched_records,
        applied.validated_nodes
    );
}

#[test]
fn deterministic_edits_match_a_simple_value_model() {
    let mut tree = initial();
    let mut expected = String::from("old");
    let mut seed = 923u64;
    let mut revision = 1;
    for _ in 0..1000 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let next = seed.to_string();
        let valid = seed & 3 != 0;
        let target = if valid { id(1, 1) } else { id(1, 2) };
        let result = tree.apply(&tx(&tree, vec![Op::SetText(target, next.clone())]));
        if valid {
            expected = next;
            revision += 1;
            assert!(result.is_ok());
        } else {
            assert_eq!(result, Err(ErrorCode::StaleHandle));
        }
        assert_eq!(tree.revision(), revision);
        assert_eq!(tree.get(id(1, 1)).unwrap().text.as_ref(), expected);
    }
}

#[test]
fn payload_budget_is_atomic_and_releases_replaced_and_removed_data() {
    let mut tree = initial();
    let before = tree.retained_bytes();
    let batch = tx(&tree, vec![Op::SetText(id(1, 1), "larger".into())]);
    assert_eq!(
        tree.apply_with_budget(&batch, before),
        Err(ErrorCode::LimitExceeded)
    );
    assert_eq!(tree.revision(), 1);
    assert_eq!(tree.retained_bytes(), before);
    assert_eq!(tree.get(id(1, 1)).unwrap().text.as_ref(), "old");
    tree.apply_with_budget(&tx(&tree, vec![Op::SetText(id(1, 1), "x".into())]), before)
        .unwrap();
    assert_eq!(tree.retained_bytes(), before - 2);
    tree.apply(&tx(
        &tree,
        vec![Op::Splice(id(0, 1), 0, 1, vec![]), Op::Remove(id(1, 1))],
    ))
    .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
    assert_eq!(tree.len(), 1);
}

#[test]
fn randomized_structural_batches_match_ordered_reference_and_roll_back() {
    let mut tree = initial();
    let mut children = vec![id(1, 1)];
    let mut generations = vec![1_i64, 1];
    let mut values = std::collections::BTreeMap::from([(id(1, 1), String::from("old"))]);
    let mut free = Vec::new();
    let mut seed = 927_u64;
    for iteration in 0..500 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut next_children = children.clone();
        let mut next_values = values.clone();
        let mut next_free = free.clone();
        let mut next_generations = generations.clone();
        let mut ops = Vec::new();
        if seed & 1 == 0 || children.is_empty() {
            let slot = next_free.pop().unwrap_or_else(|| {
                next_generations.push(0);
                next_generations.len() - 1
            });
            next_generations[slot] += 1;
            let node = id(slot as i64, next_generations[slot]);
            let offset = seed as usize % (children.len() + 1);
            let text = iteration.to_string();
            ops.push(Op::Create(node, Kind::Text, text.clone(), None));
            ops.push(Op::Splice(id(0, 1), offset as i64, 0, vec![node]));
            next_children.insert(offset, node);
            next_values.insert(node, text);
        } else {
            let offset = seed as usize % children.len();
            let node = next_children.remove(offset);
            next_values.remove(&node);
            next_free.push(node.slot());
            ops.push(Op::Splice(id(0, 1), offset as i64, 1, vec![]));
            ops.push(Op::Remove(node));
        }
        let valid = seed & 8 != 0;
        if !valid {
            ops.push(Op::Splice(id(0, 1), 0, 0, vec![id(0, 1)]));
        }
        let revision = tree.revision();
        let bytes = tree.retained_bytes();
        let result = tree.apply(&tx(&tree, ops));
        if valid {
            result.unwrap();
            children = next_children;
            values = next_values;
            free = next_free;
            generations = next_generations;
        } else {
            assert_eq!(result, Err(ErrorCode::InvalidTree));
            assert_eq!(tree.revision(), revision);
            assert_eq!(tree.retained_bytes(), bytes);
        }
        assert_eq!(tree.get(id(0, 1)).unwrap().children.as_ref(), children);
        assert_eq!(tree.len(), values.len() + 1);
        for (node, text) in &values {
            assert_eq!(tree.get(*node).unwrap().text.as_ref(), text);
            assert_eq!(tree.get(*node).unwrap().parent, Some(id(0, 1)));
        }
    }
}
