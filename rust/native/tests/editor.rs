use gpuio_native::{mailbox::Mailbox, tree::Tree};
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};
fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn node(slot: i64) -> NodeId {
    NodeId::from_parts(slot, 1).unwrap()
}
fn config() -> EditorConfig {
    EditorConfig {
        label: "Prompt".into(),
        placeholder: "".into(),
        read_only: false,
        disabled: false,
        submit_on_enter: true,
        auto_focus: false,
        min_rows: 1,
        max_rows: 8,
    }
}
fn create() -> Transaction {
    Transaction {
        window: window(),
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(
                node(0),
                Kind::Textarea,
                "initial".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetEditor(node(0), config()),
            Op::SetRoot(Some(node(0))),
        ],
    }
}
fn observation(revision: i64, kind: EditorEventKind, len: usize) -> Event {
    Event::EditorEvent(
        window(),
        node(0),
        HandlerId::from_parts(0, 1).unwrap(),
        1,
        kind,
        EditorSnapshot {
            revision,
            text: "x".repeat(len),
            selection: EditorSelection { anchor: 0, head: 0 },
            composition: None,
            focused: true,
        },
    )
}

#[test]
fn password_privacy_is_single_line_only_and_rejected_batches_roll_back() {
    for kind in [Kind::Input, Kind::Textarea, Kind::Text] {
        let mut tree = Tree::new(window());
        let mut tx = create();
        tx.operations[0] = Op::Create(
            node(0),
            kind,
            "seed".into(),
            (kind != Kind::Text).then(|| HandlerId::from_parts(0, 1).unwrap()),
        );
        if kind == Kind::Text {
            tx.operations.remove(1);
        } else if kind == Kind::Input {
            tx.operations[1] = Op::SetEditor(
                node(0),
                EditorConfig {
                    max_rows: 1,
                    ..config()
                },
            );
        }
        tree.apply(&tx).unwrap();
        assert_eq!(
            tree.get(node(0)).unwrap().editor_privacy,
            EditorPrivacy::Plain
        );
        let retained = tree.retained_bytes();
        for privacy in [
            EditorPrivacy::PasswordHidden,
            EditorPrivacy::PasswordRevealed,
            EditorPrivacy::Plain,
        ] {
            let base = tree.revision();
            let update = Transaction {
                window: window(),
                base,
                revision: base + 1,
                operations: vec![Op::SetEditorPrivacy(node(0), privacy)],
            };
            if kind == Kind::Input {
                tree.apply(&update).unwrap();
                assert_eq!(tree.get(node(0)).unwrap().editor_privacy, privacy);
                let mut rejected = Transaction {
                    base: tree.revision(),
                    revision: tree.revision() + 1,
                    ..update
                };
                rejected.operations = vec![
                    Op::SetEditorPrivacy(node(0), EditorPrivacy::PasswordHidden),
                    Op::SetText(node(0), "forbidden overwrite".into()),
                ];
                assert_eq!(tree.apply(&rejected), Err(ErrorCode::InvalidTree));
                assert_eq!(tree.revision(), base + 1);
                assert_eq!(tree.get(node(0)).unwrap().editor_privacy, privacy);
            } else {
                assert_eq!(tree.apply(&update), Err(ErrorCode::InvalidTree));
                assert_eq!(tree.revision(), base);
                assert_eq!(
                    tree.get(node(0)).unwrap().editor_privacy,
                    EditorPrivacy::Plain
                );
            }
            assert_eq!(tree.get(node(0)).unwrap().text.as_ref(), "seed");
            assert_eq!(tree.retained_bytes(), retained);
        }
    }
}
#[test]
fn tree_requires_valid_config_and_explicit_commands_and_accounts_for_editors() {
    let mut tree = Tree::new(window());
    let mut invalid = create();
    invalid.operations.remove(1);
    assert_eq!(tree.apply(&invalid), Err(ErrorCode::InvalidTree));
    assert!(tree.is_empty());
    assert_eq!(tree.retained_bytes(), 0);
    let tx = create();
    assert_eq!(
        tree.apply_with_budget(&tx, EDITOR_RESERVED_BYTES),
        Err(ErrorCode::LimitExceeded)
    );
    tree.apply(&tx).unwrap();
    assert!(tree.retained_bytes() >= EDITOR_RESERVED_BYTES);
    assert_eq!(
        tree.apply(&Transaction {
            window: window(),
            base: 1,
            revision: 2,
            operations: vec![Op::SetText(node(0), "overwrite".into())]
        }),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(tree.get(node(0)).unwrap().text.as_ref(), "initial");
    assert_eq!(tree.revision(), 1);
    let mut cfg = config();
    cfg.max_rows = 0;
    assert_eq!(
        tree.apply(&Transaction {
            window: window(),
            base: 1,
            revision: 2,
            operations: vec![Op::SetEditor(node(0), cfg)]
        }),
        Err(ErrorCode::InvalidTree)
    );
    tree.apply(&Transaction {
        window: window(),
        base: 1,
        revision: 2,
        operations: vec![Op::Remove(node(0)), Op::SetRoot(None)],
    })
    .unwrap();
    assert_eq!(tree.retained_bytes(), 0);
}
#[test]
fn snapshot_coalescing_preserves_submission_barriers_and_drain_byte_limit() {
    let mut mailbox = Mailbox::default();
    mailbox
        .input(observation(1, EditorEventKind::Changed, MAX_TEXT_BYTES))
        .unwrap();
    mailbox
        .input(observation(2, EditorEventKind::Changed, MAX_TEXT_BYTES))
        .unwrap();
    mailbox
        .input(observation(2, EditorEventKind::Submitted, MAX_TEXT_BYTES))
        .unwrap();
    mailbox
        .input(observation(3, EditorEventKind::Changed, MAX_TEXT_BYTES))
        .unwrap();
    mailbox
        .input(observation(3, EditorEventKind::Submitted, MAX_TEXT_BYTES))
        .unwrap();
    let first = mailbox.drain(256);
    assert_eq!(
        first,
        vec![
            observation(2, EditorEventKind::Changed, MAX_TEXT_BYTES),
            observation(2, EditorEventKind::Submitted, MAX_TEXT_BYTES),
            observation(3, EditorEventKind::Changed, MAX_TEXT_BYTES)
        ]
    );
    let mut bytes = Vec::new();
    binprot::BinProtWrite::binprot_write(&first, &mut bytes).unwrap();
    assert!(bytes.len() <= MAX_MESSAGE_BYTES);
    assert_eq!(
        mailbox.drain(256),
        vec![observation(3, EditorEventKind::Submitted, MAX_TEXT_BYTES)]
    );
    for revision in 0..15 {
        mailbox
            .input(observation(
                revision,
                EditorEventKind::Submitted,
                MAX_TEXT_BYTES,
            ))
            .unwrap();
    }
    assert!(
        mailbox
            .input(observation(16, EditorEventKind::Submitted, MAX_TEXT_BYTES))
            .is_err()
    );
    while mailbox.has_output() {
        assert!(!mailbox.drain(256).is_empty());
    }
    mailbox
        .input(observation(17, EditorEventKind::Changed, MAX_TEXT_BYTES))
        .unwrap();
}

#[test]
fn escape_policy_is_editor_only_and_failed_batch_preserves_prior_policy_and_resources() {
    let mut tree = Tree::new(window());
    tree.apply(&create()).unwrap();
    let bytes = tree.retained_bytes();
    let tx = |base, operations| Transaction {
        window: window(),
        base,
        revision: base + 1,
        operations,
    };
    tree.apply(&tx(1, vec![Op::SetEditorClearOnEscape(node(0), true)]))
        .unwrap();
    let original = tree.get(node(0)).unwrap().clone();
    assert!(original.editor_clear_on_escape);
    assert_eq!(original.text.as_ref(), "initial");
    assert_eq!(tree.retained_bytes(), bytes);
    assert_eq!(
        tree.apply(&tx(
            2,
            vec![
                Op::SetEditorClearOnEscape(node(0), false),
                Op::Create(node(1), Kind::Text, "label".into(), None),
                Op::SetEditorClearOnEscape(node(1), false),
            ]
        )),
        Err(ErrorCode::InvalidTree)
    );
    assert_eq!(tree.revision(), 2);
    assert_eq!(tree.get(node(0)).unwrap(), &original);
    assert_eq!(tree.retained_bytes(), bytes);
    assert!(tree.get(node(1)).is_none());
    tree.apply(&tx(2, vec![Op::SetEditorClearOnEscape(node(0), false)]))
        .unwrap();
    assert!(!tree.get(node(0)).unwrap().editor_clear_on_escape);
}
