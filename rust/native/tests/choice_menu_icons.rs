use gpuio_native::tree::Tree;
use gpuio_protocol::{NodeId, decode, v1::*};
fn fixtures() -> Vec<Transaction> {
    include_str!("../../../test/fixtures/tab-menu-icons-transactions.hex")
        .lines()
        .map(|hex| {
            let bytes: Vec<_> = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect();
            let Message::Apply(tx) = decode(&bytes).unwrap() else {
                panic!("transaction")
            };
            tx
        })
        .collect()
}
#[test]
fn public_menu_icons_replay_without_remounting_reordered_or_replaced_icons() {
    let fixtures = fixtures();
    assert_eq!(fixtures.len(), 8);
    let mut tree = Tree::new(fixtures[0].window);
    let icons: Vec<_> = fixtures[0]
        .operations
        .iter()
        .filter_map(|op| match op {
            Op::Create(id, Kind::Icon, _, _) => Some(*id),
            _ => None,
        })
        .collect();
    assert_eq!(icons.len(), 2);
    for tx in &fixtures[..3] {
        tree.apply(tx).unwrap();
        for id in &icons {
            assert!(tree.get(*id).is_some());
        }
    }
    for tx in &fixtures[3..] {
        tree.apply(tx).unwrap();
    }
    assert_eq!(tree.retained_bytes(), 0);
}
#[test]
fn menu_icon_admission_revalidates_descendants_and_resets_atomically() {
    let fixture = fixtures().remove(0);
    let menu = fixture
        .operations
        .iter()
        .find_map(|op| match op {
            Op::SetChoiceMenu(id, true) => Some(*id),
            _ => None,
        })
        .unwrap();
    let mut tree = Tree::new(fixture.window);
    tree.apply(&fixture).unwrap();
    let slot = tree.get(menu).unwrap().children[0];
    let icon = tree.get(slot).unwrap().children[0];
    let mut named = tree
        .get(icon)
        .unwrap()
        .image
        .as_ref()
        .unwrap()
        .as_ref()
        .clone();
    named.label = Some("not decorative".into());
    let extra = NodeId::from_parts(1000, 1).unwrap();
    let retained = tree.retained_bytes();
    for ops in [
        vec![Op::SetChoiceMenu(menu, false)],
        vec![Op::Splice(menu, 0, 1, vec![])],
        vec![Op::SetStyle(
            slot,
            vec![Style::Fields(vec![Field::Grow(1.)])],
        )],
        vec![Op::SetText(slot, "not structural".into())],
        vec![Op::SetImage(icon, named)],
        vec![Op::SetStyle(
            icon,
            vec![Style::Fields(vec![Field::UserSelect(true)])],
        )],
        vec![
            Op::Create(extra, Kind::Text, "extra".into(), None),
            Op::Splice(icon, 0, 0, vec![extra]),
        ],
        vec![
            Op::Create(extra, Kind::Text, "extra".into(), None),
            Op::Splice(slot, 0, 1, vec![extra]),
        ],
    ] {
        assert!(
            tree.apply(&Transaction {
                window: fixture.window,
                base: 1,
                revision: 2,
                operations: ops
            })
            .is_err()
        );
        assert_eq!(tree.revision(), 1);
        assert_eq!(tree.retained_bytes(), retained);
        assert!(tree.get(extra).is_none());
    }
}
