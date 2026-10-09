//! Actions live only as long as their installed menu owner. AppKit may retain an
//! old item after replacement, so tags are never reused or shared between owners.
use std::collections::BTreeMap;

pub(super) struct MenuActions<T> {
    next_tag: Option<i64>,
    bar: BTreeMap<i64, T>,
    dock: BTreeMap<i64, T>,
}

impl<T> Default for MenuActions<T> {
    fn default() -> Self {
        Self {
            next_tag: Some(0),
            bar: BTreeMap::new(),
            dock: BTreeMap::new(),
        }
    }
}

impl<T> MenuActions<T> {
    pub(super) fn replace_bar(&mut self) -> MenuActionBuilder<'_, T> {
        self.bar.clear();
        MenuActionBuilder {
            next_tag: &mut self.next_tag,
            actions: &mut self.bar,
        }
    }

    pub(super) fn replace_dock(&mut self) -> MenuActionBuilder<'_, T> {
        self.dock.clear();
        MenuActionBuilder {
            next_tag: &mut self.next_tag,
            actions: &mut self.dock,
        }
    }

    pub(super) fn get(&self, tag: i64) -> Option<&T> {
        self.bar.get(&tag).or_else(|| self.dock.get(&tag))
    }
}

/// The platform lock covers the whole replacement. A failed tag allocation
/// leaves the item disabled; it must never alias an older command on overflow.
pub(super) struct MenuActionBuilder<'a, T> {
    next_tag: &'a mut Option<i64>,
    actions: &'a mut BTreeMap<i64, T>,
}

impl<T> MenuActionBuilder<'_, T> {
    pub(super) fn insert(&mut self, action: T) -> Option<i64> {
        let tag = (*self.next_tag)?;
        *self.next_tag = tag.checked_add(1);
        self.actions.insert(tag, action);
        Some(tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn replacements_release_payloads_and_never_alias_stale_items() {
        let mut registry = MenuActions::default();
        let dock = Arc::new(());
        let dock_weak = Arc::downgrade(&dock);
        let dock_tag = registry.replace_dock().insert(dock).unwrap();
        let mut previous = None;
        for _ in 0..1000 {
            let owner = Arc::new(());
            let weak = Arc::downgrade(&owner);
            let tag = registry.replace_bar().insert(owner).unwrap();
            if let Some((old_tag, old_weak)) = previous {
                assert!(registry.get(old_tag).is_none());
                assert_eq!(std::sync::Weak::strong_count(&old_weak), 0);
                assert!(tag > old_tag);
            }
            assert_eq!(weak.strong_count(), 1);
            assert_eq!(dock_weak.strong_count(), 1);
            assert!(registry.get(dock_tag).is_some());
            previous = Some((tag, weak));
        }
        registry.replace_bar();
        assert_eq!(previous.unwrap().1.strong_count(), 0);
        registry.replace_dock();
        assert_eq!(dock_weak.strong_count(), 0);
    }

    #[test]
    fn dock_replacement_preserves_bar_and_retires_all_nested_actions() {
        let mut registry = MenuActions::default();
        let bar = registry.replace_bar().insert("bar").unwrap();
        let (dock, nested) = {
            let mut builder = registry.replace_dock();
            (
                builder.insert("dock").unwrap(),
                builder.insert("nested").unwrap(),
            )
        };
        let replacement = registry.replace_dock().insert("new").unwrap();
        assert_eq!(registry.get(bar), Some(&"bar"));
        assert_eq!(registry.get(replacement), Some(&"new"));
        assert!(registry.get(dock).is_none());
        assert!(registry.get(nested).is_none());
        assert!(registry.get(-1).is_none());
    }

    #[test]
    fn exhausted_tags_fail_closed_without_wrapping_or_retaining_payload() {
        let mut registry = MenuActions {
            next_tag: Some(i64::MAX),
            ..Default::default()
        };
        assert_eq!(registry.replace_bar().insert(Arc::new(())), Some(i64::MAX));
        let owner = Arc::new(());
        let weak = Arc::downgrade(&owner);
        assert!(registry.replace_dock().insert(owner).is_none());
        assert_eq!(weak.strong_count(), 0);
        assert!(registry.get(i64::MAX).is_some());
        registry.replace_bar();
        assert!(registry.get(i64::MAX).is_none());
        assert!(registry.replace_bar().insert(Arc::new(())).is_none());
    }
}
