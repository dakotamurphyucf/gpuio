use gpuio_native::{text_shimmer_clock::RESERVED_BYTES, tree::Tree};
use gpuio_protocol::{
    NodeId, WindowId,
    text_content::{Content, Span},
    text_shimmer::{Config, Direction, MAX_TEXT_BYTES, Repeat, Spread},
    v1::*,
};

fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn config() -> Config {
    Config {
        duration_ms: 1000,
        spread: Spread::Relative(0.3),
        direction: Direction::LeftToRight,
        repeat: Repeat::Loop,
        animated: true,
        highlight: None,
    }
}
fn tx(tree: &Tree, operations: Vec<Op>) -> Transaction {
    Transaction {
        window: WindowId::from_parts(0, 1).unwrap(),
        base: tree.revision(),
        revision: tree.revision() + 1,
        operations,
    }
}
fn initial(kind: Kind) -> Tree {
    let mut tree = Tree::new(WindowId::from_parts(0, 1).unwrap());
    tree.apply(&tx(
        &tree,
        vec![
            Op::Create(node(), kind, "Aé世界".into(), None),
            Op::SetRoot(Some(node())),
        ],
    ))
    .unwrap();
    tree
}

#[test]
fn set_update_clear_preserve_source_spans_identity_and_account_for_owners() {
    let mut tree = initial(Kind::Text);
    tree.apply(&tx(
        &tree,
        vec![Op::SetStyledText(
            node(),
            Content {
                text: "Aé世界".into(),
                spans: vec![Span {
                    start_byte: 1,
                    end_byte: 3,
                    foreground: 0xaabbccff,
                }],
            },
        )],
    ))
    .unwrap();
    let original = tree.get(node()).unwrap().clone();
    let baseline = tree.retained_bytes();
    let ops = vec![Op::SetTextShimmer(node(), Some(config()))];
    assert_eq!(
        tree.apply_with_budget(&tx(&tree, ops.clone()), baseline + RESERVED_BYTES - 1),
        Err(ErrorCode::LimitExceeded)
    );
    assert_eq!(tree.get(node()).unwrap(), &original);
    tree.apply_with_budget(&tx(&tree, ops), baseline + RESERVED_BYTES)
        .unwrap();
    assert_eq!(tree.retained_bytes(), baseline + RESERVED_BYTES);
    assert!(std::sync::Arc::ptr_eq(
        &original.text,
        &tree.get(node()).unwrap().text
    ));
    assert!(std::sync::Arc::ptr_eq(
        &original.text_spans,
        &tree.get(node()).unwrap().text_spans
    ));
    let paused = Config {
        animated: false,
        ..config()
    };
    tree.apply(&tx(&tree, vec![Op::SetTextShimmer(node(), Some(paused))]))
        .unwrap();
    assert_eq!(
        tree.get(node()).unwrap().text_shimmer.as_deref(),
        Some(&paused)
    );
    assert_eq!(tree.retained_bytes(), baseline + RESERVED_BYTES);
    tree.apply(&tx(&tree, vec![Op::SetTextShimmer(node(), None)]))
        .unwrap();
    assert_eq!(tree.get(node()).unwrap(), &original);
    assert_eq!(tree.retained_bytes(), baseline);
}

#[test]
fn malformed_or_oversized_updates_roll_back_the_entire_transaction() {
    let mut tree = initial(Kind::Text);
    tree.apply(&tx(&tree, vec![Op::SetTextShimmer(node(), Some(config()))]))
        .unwrap();
    let original = tree.get(node()).unwrap().clone();
    let revision = tree.revision();
    let baseline = tree.retained_bytes();
    let oversized = "x".repeat(MAX_TEXT_BYTES + 1);
    for (ops, error) in [
        (
            vec![
                Op::SetText(node(), "discard".into()),
                Op::SetTextShimmer(
                    node(),
                    Some(Config {
                        duration_ms: 0,
                        ..config()
                    }),
                ),
            ],
            ErrorCode::InvalidTree,
        ),
        (
            vec![Op::SetText(node(), oversized.clone())],
            ErrorCode::LimitExceeded,
        ),
        (
            vec![Op::SetStyledText(
                node(),
                Content {
                    text: oversized.clone(),
                    spans: vec![],
                },
            )],
            ErrorCode::LimitExceeded,
        ),
    ] {
        assert_eq!(tree.apply(&tx(&tree, ops)), Err(error));
        assert_eq!(tree.get(node()).unwrap(), &original);
        assert_eq!(tree.retained_bytes(), baseline);
        assert_eq!(tree.revision(), revision);
    }
    // Final-state admission: grow then clear and clear then grow both succeed.
    for reverse in [false, true] {
        tree.apply(&tx(
            &tree,
            vec![
                Op::SetText(node(), "x".repeat(MAX_TEXT_BYTES)),
                Op::SetTextShimmer(node(), Some(config())),
            ],
        ))
        .unwrap();
        let mut ops = vec![
            Op::SetText(node(), oversized.clone()),
            Op::SetTextShimmer(node(), None),
        ];
        if reverse {
            ops.reverse();
        }
        tree.apply(&tx(&tree, ops)).unwrap();
        assert_eq!(tree.get(node()).unwrap().text.len(), MAX_TEXT_BYTES + 1);
        assert!(tree.get(node()).unwrap().text_shimmer.is_none());
        let revision = tree.revision();
        assert_eq!(
            tree.apply(&tx(&tree, vec![Op::SetTextShimmer(node(), Some(config()))])),
            Err(ErrorCode::LimitExceeded)
        );
        assert_eq!(tree.revision(), revision);
    }
    // The source may also shrink after the enable op within one atomic batch.
    tree.apply(&tx(
        &tree,
        vec![
            Op::SetTextShimmer(node(), Some(config())),
            Op::SetText(node(), "".into()),
        ],
    ))
    .unwrap();
    assert!(tree.get(node()).unwrap().text_shimmer.is_some());
}

#[test]
fn source_changes_keep_the_effect_and_removed_generations_cannot_reach_a_new_owner() {
    let mut tree = initial(Kind::Text);
    tree.apply(&tx(&tree, vec![Op::SetTextShimmer(node(), Some(config()))]))
        .unwrap();
    tree.apply(&tx(&tree, vec![Op::SetText(node(), "👩‍💻".into())]))
        .unwrap();
    assert_eq!(
        tree.get(node()).unwrap().text_shimmer.as_deref(),
        Some(&config())
    );
    let next = NodeId::from_parts(0, 2).unwrap();
    tree.apply(&tx(
        &tree,
        vec![
            Op::Remove(node()),
            Op::Create(next, Kind::Text, "New".into(), None),
            Op::SetRoot(Some(next)),
        ],
    ))
    .unwrap();
    let baseline = tree.retained_bytes();
    for config in [Some(config()), None] {
        assert_eq!(
            tree.apply(&tx(&tree, vec![Op::SetTextShimmer(node(), config)])),
            Err(ErrorCode::StaleHandle)
        );
    }
    assert!(tree.get(next).unwrap().text_shimmer.is_none());
    assert_eq!(tree.retained_bytes(), baseline);
    tree.apply(&tx(&tree, vec![Op::SetRoot(None), Op::Remove(next)]))
        .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
    for kind in [Kind::Container, Kind::Button] {
        let mut tree = initial(kind);
        let original = tree.get(node()).unwrap().clone();
        for config in [Some(config()), None] {
            assert_eq!(
                tree.apply(&tx(&tree, vec![Op::SetTextShimmer(node(), config)])),
                Err(ErrorCode::InvalidTree)
            );
            assert_eq!(tree.get(node()).unwrap(), &original);
        }
    }
}

#[test]
fn session_counts_independent_windows_and_releases_each_declaration_on_close() {
    use gpuio_native::session::Session;
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    let windows = [
        WindowId::from_parts(0, 1).unwrap(),
        WindowId::from_parts(1, 1).unwrap(),
    ];
    let mut before_effects = 0;
    for (i, window) in windows.iter().copied().enumerate() {
        session
            .open(i as i64 + 1, window, "Shimmer", 100., 100.)
            .unwrap();
        session
            .apply(&Transaction {
                window,
                base: 0,
                revision: 1,
                operations: vec![
                    Op::Create(node(), Kind::Text, "Working".into(), None),
                    Op::SetRoot(Some(node())),
                ],
            })
            .unwrap();
        before_effects += session.tree(window).unwrap().retained_bytes();
        session
            .apply(&Transaction {
                window,
                base: 1,
                revision: 2,
                operations: vec![Op::SetTextShimmer(node(), Some(config()))],
            })
            .unwrap();
    }
    assert_eq!(
        session.retained_bytes(),
        before_effects + 2 * RESERVED_BYTES
    );
    session
        .apply(&Transaction {
            window: windows[0],
            base: 2,
            revision: 3,
            operations: vec![Op::SetTextShimmer(node(), None)],
        })
        .unwrap();
    assert_eq!(session.retained_bytes(), before_effects + RESERVED_BYTES);
    assert!(
        session
            .tree(windows[1])
            .unwrap()
            .get(node())
            .unwrap()
            .text_shimmer
            .is_some()
    );
    let remaining = session.tree(windows[1]).unwrap().retained_bytes();
    session.close(windows[0]).unwrap();
    assert_eq!(session.retained_bytes(), remaining);
    session.close(windows[1]).unwrap();
    assert_eq!(session.retained_bytes(), 0);
}
