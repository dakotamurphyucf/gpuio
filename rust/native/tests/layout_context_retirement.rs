//! Regression for the pinned layout dependency's measurement-context ownership.
use std::rc::Rc;
use taffy::{Style, TaffyTree};

#[test]
fn removing_a_node_releases_only_its_context() {
    let mut tree = TaffyTree::new();
    let parent = Rc::new(String::from("parent measurement"));
    let child = Rc::new(String::from("child measurement"));
    let old_parent = Rc::downgrade(&parent);
    let live_child = Rc::downgrade(&child);
    let parent = tree
        .new_leaf_with_context(Style::default(), parent)
        .unwrap();
    let child = tree.new_leaf_with_context(Style::default(), child).unwrap();
    tree.add_child(parent, child).unwrap();
    tree.remove(parent).unwrap();
    assert!(old_parent.upgrade().is_none());
    assert!(live_child.upgrade().is_some());
    assert!(tree.parent(child).is_none());
    assert_eq!(
        tree.get_node_context(child).unwrap().as_str(),
        "child measurement"
    );
    tree.remove(child).unwrap();
    assert!(live_child.upgrade().is_none());
}

#[test]
fn clearing_layout_releases_measurements_before_reusing_slots() {
    let mut tree = TaffyTree::new();
    for size in [128, 32, 0, 128, 1, 0] {
        let mut retired = Vec::new();
        for row in 0..size {
            let text = Rc::new(format!("日本語 👨‍👩‍👧‍👦 {row}"));
            retired.push(Rc::downgrade(&text));
            tree.new_leaf_with_context(Style::default(), text).unwrap();
        }
        tree.clear();
        assert_eq!(tree.total_node_count(), 0);
        assert!(retired.iter().all(|weak| weak.upgrade().is_none()));
        // Keep the same tree alive: dropping the entire window is too late.
        tree.new_leaf(Style::default()).unwrap();
        tree.clear();
    }
}
