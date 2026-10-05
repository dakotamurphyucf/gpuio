//! Measured, retained palette presentation. Navigation uses commands only;
//! headings and dividers have separate stable visual identities.
use gpui::{ListAlignment, ListOffset, ListState, px};
use gpuio_protocol::palette_layout::{Config, Entry};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Key {
    Command(String),
    Group(String),
    Separator(usize),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Row {
    Command { id: String, index: usize },
    Heading { id: String, label: String },
    Separator(usize),
}
impl Row {
    fn key(&self) -> Key {
        match self {
            Self::Command { id, .. } => Key::Command(id.clone()),
            Self::Heading { id, .. } => Key::Group(id.clone()),
            Self::Separator(index) => Key::Separator(*index),
        }
    }
}

pub(super) fn project(
    layout: Option<&Config>,
    declared: &[String],
    matched: &[String],
) -> Vec<Row> {
    let positions = matched
        .iter()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect::<BTreeMap<_, _>>();
    let command = |index: i64| {
        let id = declared.get(usize::try_from(index).ok()?)?;
        Some(Row::Command {
            id: id.clone(),
            index: *positions.get(id)?,
        })
    };
    let Some(layout) = layout else {
        return matched
            .iter()
            .enumerate()
            .map(|(index, id)| Row::Command {
                id: id.clone(),
                index,
            })
            .collect();
    };
    let mut rows = Vec::new();
    let mut separator = None;
    for (entry_index, entry) in layout.0.iter().enumerate() {
        let next = match entry {
            Entry::Command(index) => command(*index).into_iter().collect::<Vec<_>>(),
            Entry::Group(id, label, commands) => {
                let mut items = commands
                    .iter()
                    .filter_map(|i| command(*i))
                    .collect::<Vec<_>>();
                if !items.is_empty()
                    && let Some(label) = label
                {
                    items.insert(
                        0,
                        Row::Heading {
                            id: id.clone(),
                            label: label.clone(),
                        },
                    );
                }
                items
            }
            Entry::Separator => {
                if !rows.is_empty() && separator.is_none() {
                    separator = Some(entry_index);
                }
                continue;
            }
        };
        if !next.is_empty() {
            if let Some(index) = separator.take() {
                rows.push(Row::Separator(index));
            }
            rows.extend(next);
        }
    }
    rows
}

pub(super) struct State {
    rows: Arc<Vec<Row>>,
    pub(super) handle: ListState,
    estimate: f32,
}
impl Default for State {
    fn default() -> Self {
        Self {
            rows: Arc::new(vec![]),
            handle: ListState::new(0, ListAlignment::Top, px(128.)),
            estimate: 32.,
        }
    }
}
impl State {
    pub(super) fn rows(&self) -> Arc<Vec<Row>> {
        self.rows.clone()
    }
    pub(super) fn command_position(&self, command: &str) -> Option<usize> {
        self.rows
            .iter()
            .position(|row| matches!(row, Row::Command {id, ..} if id == command))
    }
    pub(super) fn replace(&mut self, next: Vec<Row>, estimate: f32) {
        let geometry_changed = self.estimate != estimate;
        if *self.rows == next {
            if geometry_changed {
                self.handle.remeasure_items(0..next.len());
                self.estimate = estimate;
            }
            return;
        }
        let offset = self.handle.logical_scroll_top();
        let positions = next
            .iter()
            .enumerate()
            .map(|(i, row)| (row.key(), i))
            .collect::<BTreeMap<_, _>>();
        let position = |i: usize| {
            self.rows
                .get(i)
                .and_then(|row| positions.get(&row.key()).copied())
        };
        let anchor = if let Some(item_ix) = position(offset.item_ix) {
            ListOffset {
                item_ix,
                offset_in_item: offset.offset_in_item,
            }
        } else {
            let successor = (offset.item_ix.saturating_add(1)..self.rows.len()).find_map(position);
            let predecessor = || {
                (0..offset.item_ix.min(self.rows.len()))
                    .rev()
                    .find_map(position)
            };
            ListOffset {
                item_ix: successor.or_else(predecessor).unwrap_or(0),
                offset_in_item: px(0.),
            }
        };
        let (old_len, new_len) = (self.rows.len(), next.len());
        let mut prefix = 0;
        while prefix < old_len.min(new_len) && self.rows[prefix].key() == next[prefix].key() {
            prefix += 1;
        }
        let mut suffix = 0;
        while suffix < old_len.min(new_len) - prefix
            && self.rows[old_len - suffix - 1].key() == next[new_len - suffix - 1].key()
        {
            suffix += 1;
        }
        let structural = prefix != old_len || prefix != new_len;
        if structural {
            self.handle
                .splice(prefix..old_len - suffix, new_len - prefix - suffix);
            self.handle.clone().with_uniform_item_height(px(estimate));
        }
        // Labels and registry-driven content may change height without new keys.
        self.handle.remeasure_items(0..new_len);
        self.rows = Arc::new(next);
        self.estimate = estimate;
        if structural {
            self.handle.scroll_to(anchor);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filtered_groups_and_separators_do_not_enter_command_navigation() {
        let declared = ["a", "b", "c"].map(String::from);
        let layout = Config(vec![
            Entry::Separator,
            Entry::Group("first".into(), Some("First".into()), vec![0]),
            Entry::Separator,
            Entry::Separator,
            Entry::Group("second".into(), Some("Second".into()), vec![1]),
            Entry::Separator,
            Entry::Command(2),
            Entry::Separator,
        ]);
        assert!(project(Some(&layout), &declared, &[]).is_empty());
        let rows = project(Some(&layout), &declared, &["b".into(), "c".into()]);
        assert_eq!(
            rows,
            vec![
                Row::Heading {
                    id: "second".into(),
                    label: "Second".into()
                },
                Row::Command {
                    id: "b".into(),
                    index: 0
                },
                Row::Separator(5),
                Row::Command {
                    id: "c".into(),
                    index: 1
                }
            ]
        );
    }
    #[test]
    fn keyed_anchor_survives_reorder_and_content_updates() {
        let row = |id: &str, index| Row::Command {
            id: id.into(),
            index,
        };
        let mut state = State::default();
        state.replace(vec![row("a", 0), row("b", 1), row("c", 2)], 32.);
        state.handle.scroll_to(ListOffset {
            item_ix: 1,
            offset_in_item: px(7.),
        });
        state.replace(vec![row("c", 0), row("a", 1), row("b", 2)], 32.);
        assert_eq!(state.handle.logical_scroll_top().item_ix, 2);
        assert_eq!(state.handle.logical_scroll_top().offset_in_item, px(7.));
        state.replace(vec![row("c", 0), row("a", 1)], 32.);
        assert_eq!(state.handle.logical_scroll_top().item_ix, 1);
        assert_eq!(state.handle.logical_scroll_top().offset_in_item, px(0.));
    }
}
