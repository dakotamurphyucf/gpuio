use crate::protocol::{Batch, Node, Op};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Default)]
pub struct Tree {
    pub revision: i64,
    pub root: Option<i64>,
    pub nodes: BTreeMap<i64, Node>,
}
impl Tree {
    pub fn stage(&self, batch: &Batch) -> Result<Self, String> {
        if batch.base != self.revision || batch.next != self.revision + 1 {
            return Err("revision mismatch".into());
        }
        let mut next = self.clone();
        for op in &batch.ops {
            match op {
                Op::Upsert(n) => {
                    if n.id <= 0 || !(0..=3).contains(&n.kind) {
                        return Err("invalid node".into());
                    }
                    let mut n = n.clone();
                    if let Some(old) = next.nodes.get(&n.id) {
                        if old.kind != n.kind {
                            return Err("cannot change node kind".into());
                        }
                        n.children = old.children.clone();
                    }
                    next.nodes.insert(n.id, n);
                }
                Op::Children(id, children) => {
                    next.nodes.get_mut(id).ok_or("unknown parent")?.children = children.clone()
                }
                Op::Remove(id) => {
                    next.nodes.remove(id).ok_or("unknown removal")?;
                }
                Op::Root(id) => next.root = Some(*id),
                Op::Edit(id, revision, _) => {
                    if next.nodes.get(id).map(|n| n.kind) != Some(3) || *revision < 0 {
                        return Err("invalid edit target".into());
                    }
                }
            }
        }
        if next.nodes.len() > 4096 {
            return Err("node limit exceeded".into());
        }
        fn visit(
            id: i64,
            depth: usize,
            tree: &Tree,
            seen: &mut BTreeSet<i64>,
        ) -> Result<(), String> {
            if depth > 64 || !seen.insert(id) {
                return Err("cycle, duplicate parent, or excessive depth".into());
            }
            let n = tree.nodes.get(&id).ok_or("missing node")?;
            if n.kind != 0 && !n.children.is_empty() {
                return Err("leaf has children".into());
            }
            for child in &n.children {
                visit(*child, depth + 1, tree, seen)?;
            }
            Ok(())
        }
        let mut seen = BTreeSet::new();
        visit(next.root.ok_or("missing root")?, 0, &next, &mut seen)?;
        if seen.len() != next.nodes.len() {
            return Err("orphan node".into());
        }
        next.revision = batch.next;
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(id: i64) -> Node {
        Node {
            id,
            kind: 0,
            text: String::new(),
            handler: None,
            children: vec![],
        }
    }
    #[test]
    fn invalid_transaction_preserves_original() {
        let tree = Tree::default();
        let b = Batch {
            base: 0,
            next: 1,
            ops: vec![Op::Upsert(node(1)), Op::Root(1), Op::Children(1, vec![1])],
        };
        assert!(tree.stage(&b).is_err());
        assert!(tree.nodes.is_empty());
        assert_eq!(tree.revision, 0);
    }
    #[test]
    fn valid_then_stale_revision() {
        let b = Batch {
            base: 0,
            next: 1,
            ops: vec![Op::Upsert(node(1)), Op::Root(1)],
        };
        let next = Tree::default().stage(&b).unwrap();
        assert!(next.stage(&b).is_err());
    }
}
