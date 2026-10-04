use gpuio_native::tree::Tree;
use gpuio_protocol::{NodeId, decode, v1::*};

fn fixtures() -> Vec<Transaction> {
    include_str!("../../../test/fixtures/tab-frame-transactions.hex")
        .lines()
        .map(|hex| {
            let bytes: Vec<_> = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect();
            let Message::Apply(tx) = decode(&bytes).unwrap() else {
                panic!("expected transaction")
            };
            tx
        })
        .collect()
}

#[test]
fn public_frame_replay_retains_controls_and_releases_on_disposal() {
    let fixtures = fixtures();
    assert_eq!(fixtures.len(), 6);
    let mut tree = Tree::new(fixtures[0].window);
    for tx in &fixtures {
        tree.apply(tx).unwrap();
    }
    assert_eq!(tree.retained_bytes(), 0);
    // Reorder/reveal updates reuse every placement; only prefix removal disposes a control.
    assert!(
        !fixtures[1]
            .operations
            .iter()
            .any(|op| matches!(op, Op::Create(..) | Op::Remove(..)))
    );
}

#[test]
fn trailing_admission_revalidates_late_slot_edits_and_rolls_back() {
    let fixture = fixtures().remove(0);
    let mut tree = Tree::new(fixture.window);
    tree.apply(&fixture).unwrap();
    let tab = fixture
        .operations
        .iter()
        .find_map(|op| match op {
            Op::SetTabTrailing(id, true) => Some(*id),
            _ => None,
        })
        .unwrap();
    let trailing = *tree.get(tab).unwrap().children.last().unwrap();
    let button = tree.get(trailing).unwrap().children[0];
    let unused = NodeId::from_parts(1000, 1).unwrap();
    let before = tree.retained_bytes();
    for operations in [
        vec![Op::SetTabTrailing(button, true)],
        vec![Op::SetTabTrailing(tab, false)],
        vec![Op::SetStyle(trailing, vec![Style::Fields(vec![])])],
        vec![Op::SetText(trailing, "unexpected wrapper text".into())],
        vec![
            Op::Create(unused, Kind::Text, "extra".into(), None),
            Op::Splice(trailing, 1, 0, vec![unused]),
        ],
        vec![Op::Splice(tab, 0, 1, vec![])],
    ] {
        assert!(
            tree.apply(&Transaction {
                window: fixture.window,
                base: 1,
                revision: 2,
                operations,
            })
            .is_err()
        );
        assert_eq!(tree.revision(), 1);
        assert_eq!(tree.retained_bytes(), before);
        assert!(tree.get(unused).is_none());
    }
}
