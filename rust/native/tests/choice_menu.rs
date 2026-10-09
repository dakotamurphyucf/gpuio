use gpuio_native::tree::Tree;
use gpuio_protocol::{NodeId, decode, v1::*};

fn fixtures() -> Vec<Transaction> {
    include_str!("../../../test/fixtures/tab-menu-transactions.hex")
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
fn public_menu_replay_preserves_tab_identity_and_disposes_all_nodes() {
    let fixtures = fixtures();
    assert_eq!(fixtures.len(), 7);
    let mut tree = Tree::new(fixtures[0].window);
    tree.apply(&fixtures[0]).unwrap();
    let tab = fixtures[0]
        .operations
        .iter()
        .find_map(|op| match op {
            Op::Create(id, Kind::TabBar, _, _) => Some(*id),
            _ => None,
        })
        .unwrap();
    let mut menu = None;
    for tx in &fixtures[1..6] {
        tree.apply(tx).unwrap();
        assert!(tree.get(tab).is_some());
        if tx.revision == 2 {
            menu = tx.operations.iter().find_map(|op| match op {
                Op::SetChoiceMenu(id, true) => Some(*id),
                _ => None,
            });
        }
        if (2..=5).contains(&tx.revision) {
            let menu = tree.get(menu.unwrap()).unwrap();
            assert!(menu.choice_menu);
            let tabs = tree.get(tab).unwrap().choice.as_ref().unwrap();
            let choices = menu.choice.as_ref().unwrap();
            assert_eq!(choices.items, tabs.items);
            assert_eq!(choices.selected, tabs.selected);
            assert_eq!(choices.disabled, tabs.disabled);
        }
    }
    assert!(tree.get(menu.unwrap()).is_none());
    tree.apply(&fixtures[6]).unwrap();
    assert_eq!(tree.retained_bytes(), 0);
}

#[test]
fn menu_metadata_rejects_non_select_owners_and_stale_generations_atomically() {
    let fixtures = fixtures();
    let mut tree = Tree::new(fixtures[0].window);
    tree.apply(&fixtures[0]).unwrap();
    let root = NodeId::from_parts(0, 1).unwrap();
    let stale = NodeId::from_parts(0, 2).unwrap();
    let before = tree.retained_bytes();
    for id in [root, stale] {
        let tx = Transaction {
            window: fixtures[0].window,
            base: 1,
            revision: 2,
            operations: vec![Op::SetChoiceMenu(id, true)],
        };
        assert!(tree.apply(&tx).is_err());
        assert_eq!(tree.revision(), 1);
        assert_eq!(tree.retained_bytes(), before);
        assert!(!tree.get(root).unwrap().choice_menu);
    }
}
