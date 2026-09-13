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
