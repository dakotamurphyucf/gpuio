use gpuio_native::tree::Tree;
use gpuio_protocol::{HandlerId, NodeId, WindowId, extension::*, v1::*};
fn window() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn config(sequence: Option<i64>) -> Config {
    Config {
        schema: Schema {
            name: "test.counter".into(),
            version: 1,
            fingerprint: "a".repeat(64),
        },
        generation: 1,
        label: "Count".into(),
        disabled: false,
        properties: Payload(vec![7]),
        command: sequence.map(|sequence| Command {
            sequence,
            payload: Payload(vec![8]),
        }),
    }
}
fn apply(tree: &mut Tree, ops: Vec<Op>) -> Result<gpuio_native::tree::Applied, ErrorCode> {
    tree.apply(&Transaction {
        window: window(),
        base: tree.revision(),
        revision: tree.revision() + 1,
        operations: ops,
    })
}
#[test]
fn command_history_survives_absent_command_and_failed_transactions() {
    let mut tree = Tree::new(window());
    apply(
        &mut tree,
        vec![
            Op::Create(
                node(),
                Kind::Extension,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetExtension(node(), config(Some(4))),
            Op::SetRoot(Some(node())),
        ],
    )
    .unwrap();
    apply(&mut tree, vec![Op::SetExtension(node(), config(None))]).unwrap();
    let revision = tree.revision();
    let mut changed = config(Some(4));
    changed.command.as_mut().unwrap().payload = Payload(vec![9]);
    for invalid in [config(Some(3)), changed] {
        assert_eq!(
            apply(&mut tree, vec![Op::SetExtension(node(), invalid)]),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(tree.revision(), revision);
    }
    apply(&mut tree, vec![Op::SetExtension(node(), config(Some(4)))]).unwrap();
    let mut reset = config(Some(1));
    reset.generation = 2;
    apply(&mut tree, vec![Op::SetExtension(node(), reset)]).unwrap();
    assert_eq!(
        tree.get(node())
            .unwrap()
            .extension_command
            .as_ref()
            .unwrap()
            .sequence,
        1
    );
    assert_eq!(
        apply(&mut tree, vec![Op::SetExtension(node(), config(None))]),
        Err(ErrorCode::InvalidTree)
    );
}
#[test]
fn a_transaction_cannot_silently_discard_an_earlier_extension_command() {
    let mut tree = Tree::new(window());
    let result = apply(
        &mut tree,
        vec![
            Op::Create(
                node(),
                Kind::Extension,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetExtension(node(), config(Some(1))),
            Op::SetExtension(node(), config(Some(2))),
            Op::SetRoot(Some(node())),
        ],
    );
    assert_eq!(result, Err(ErrorCode::InvalidTree));
    assert!(tree.is_empty());
    assert_eq!(tree.retained_bytes(), 0);
}
