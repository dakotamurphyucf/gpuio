//! Immutable, bounded row projection for the picker popup's variable-height list.
//!
//! Build when the accepted catalog or native query changes, not during row layout.
//! The projection retains the complete catalog; filtering never mutates committed
//! selection. Row keys use separate group/item namespaces. A renderer supplies
//! rich content and measures only its requested rows; no OCaml render callback is
//! involved. Cursor changes do not rebuild or copy the projection.
use gpuio_protocol::choice_picker::{
    Collection, Config, Group, Item, MAX_QUERY_BYTES, Search, Selection,
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidConfig,
    InvalidQuery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key<'a> {
    Group(&'a str),
    Item(&'a str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row<'a> {
    Header(&'a Group),
    Item(&'a Item),
}

impl<'a> Row<'a> {
    pub fn key(self) -> Key<'a> {
        match self {
            Self::Header(group) => Key::Group(&group.id),
            Self::Item(item) => Key::Item(&item.id),
        }
    }
}

#[derive(Clone, Copy)]
enum Location {
    Header(usize),
    Flat(usize),
    GroupItem(usize, usize),
}

/// Logical membership after filtering, including disabled options. The index is
/// zero-based within its group (or the flat collection), independent of which
/// rows GPUI currently mounts. Header rows have no option membership.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Membership<'a> {
    pub index: usize,
    pub count: usize,
    pub group: Option<&'a Group>,
}

pub struct Projection {
    config: Arc<Config>,
    rows: Vec<Location>,
    item_rows: BTreeMap<String, usize>,
    group_rows: BTreeMap<String, usize>,
    group_item_counts: Vec<usize>,
    enabled_rows: Vec<usize>,
    selected_rows: Vec<bool>,
}

impl Projection {
    pub fn new(config: Arc<Config>, query: &str) -> Result<Self, Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        if query.len() > MAX_QUERY_BYTES || query.contains(['\0', '\n', '\r']) {
            return Err(Error::InvalidQuery);
        }
        let filter = config.search == Search::Substring && !query.is_empty();
        let folded = if filter {
            query.to_lowercase()
        } else {
            String::new()
        };
        let matches = |item: &Item| !filter || item.label.to_lowercase().contains(&folded);
        let mut rows = Vec::new();
        match &config.options {
            Collection::Flat(items) => {
                rows.extend(
                    items
                        .iter()
                        .enumerate()
                        .filter(|(_, item)| matches(item))
                        .map(|(i, _)| Location::Flat(i)),
                );
            }
            Collection::Grouped(groups) => {
                for (g, group) in groups.iter().enumerate() {
                    let header = rows.len();
                    rows.push(Location::Header(g));
                    rows.extend(
                        group
                            .items
                            .iter()
                            .enumerate()
                            .filter(|(_, item)| matches(item))
                            .map(|(i, _)| Location::GroupItem(g, i)),
                    );
                    if filter && rows.len() == header + 1 {
                        rows.pop();
                    }
                }
            }
        }
        let selected_rows = vec![false; rows.len()];
        let mut group_item_counts = match &config.options {
            Collection::Flat(_) => vec![],
            Collection::Grouped(groups) => vec![0; groups.len()],
        };
        for location in &rows {
            if let Location::GroupItem(group, _) = location {
                group_item_counts[*group] += 1;
            }
        }
        let mut projection = Self {
            config,
            rows,
            item_rows: BTreeMap::new(),
            group_rows: BTreeMap::new(),
            group_item_counts,
            enabled_rows: Vec::new(),
            selected_rows,
        };
        for index in 0..projection.len() {
            if let Some(Row::Header(group)) = projection.row(index) {
                let id = group.id.clone();
                projection.group_rows.insert(id, index);
            }
            if let Some(Row::Item(item)) = projection.row(index) {
                let enabled = !projection.config.disabled && !item.disabled;
                let id = item.id.clone();
                projection.item_rows.insert(id, index);
                if enabled {
                    projection.enabled_rows.push(index);
                }
            }
        }
        let mut select = |id: &str| {
            if let Some(&index) = projection.item_rows.get(id) {
                projection.selected_rows[index] = true;
            }
        };
        match &projection.config.selected {
            Selection::Single(id) => {
                if let Some(id) = id {
                    select(id);
                }
            }
            Selection::Multiple(ids) => {
                for id in ids {
                    select(id);
                }
            }
        }
        Ok(projection)
    }

    pub fn config(&self) -> &Arc<Config> {
        &self.config
    }
    pub fn len(&self) -> usize {
        self.rows.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    /// Use this for the empty-content slot: an empty group header is not a choice.
    pub fn item_count(&self) -> usize {
        self.item_rows.len()
    }

    pub fn row(&self, index: usize) -> Option<Row<'_>> {
        match (*self.rows.get(index)?, &self.config.options) {
            (Location::Flat(i), Collection::Flat(items)) => items.get(i).map(Row::Item),
            (Location::Header(g), Collection::Grouped(groups)) => groups.get(g).map(Row::Header),
            (Location::GroupItem(g, i), Collection::Grouped(groups)) => {
                groups.get(g)?.items.get(i).map(Row::Item)
            }
            _ => unreachable!("private row locations belong to their retained catalog"),
        }
    }

    pub fn membership(&self, index: usize) -> Option<Membership<'_>> {
        match (*self.rows.get(index)?, &self.config.options) {
            (Location::Flat(_), Collection::Flat(_)) => Some(Membership {
                index,
                count: self.item_count(),
                group: None,
            }),
            (Location::GroupItem(g, _), Collection::Grouped(groups)) => {
                let group = &groups[g];
                Some(Membership {
                    index: index - self.group_rows[&group.id] - 1,
                    count: self.group_item_counts[g],
                    group: Some(group),
                })
            }
            (Location::Header(_), _) => None,
            _ => unreachable!("private row locations belong to their retained catalog"),
        }
    }

    pub fn item_row(&self, id: &str) -> Option<usize> {
        self.item_rows.get(id).copied()
    }

    pub fn position(&self, key: Key<'_>) -> Option<usize> {
        match key {
            Key::Group(id) => self.group_rows.get(id).copied(),
            Key::Item(id) => self.item_rows.get(id).copied(),
        }
    }

    pub fn is_enabled(&self, index: usize) -> bool {
        !self.config.disabled && matches!(self.row(index), Some(Row::Item(item)) if !item.disabled)
    }

    pub fn is_selected(&self, index: usize) -> bool {
        self.selected_rows.get(index).copied().unwrap_or(false)
    }

    fn preferred(&self) -> Option<usize> {
        let enabled = |id: &str| self.item_row(id).filter(|&row| self.is_enabled(row));
        let selected = match &self.config.selected {
            Selection::Single(id) => id.as_deref().and_then(enabled),
            Selection::Multiple(ids) => ids.iter().find_map(|id| enabled(id)),
        };
        selected.or_else(|| self.enabled_rows.first().copied())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Navigation {
    First,
    Last,
    Next,
    Previous,
}

#[derive(Default)]
pub struct Cursor {
    active: Option<String>,
}

impl Cursor {
    pub fn active_id(&self) -> Option<&str> {
        self.active.as_deref()
    }
    pub fn active_row(&self, projection: &Projection) -> Option<usize> {
        self.active_id()
            .and_then(|id| projection.item_row(id))
            .filter(|&row| projection.is_enabled(row))
    }
    fn set_row(&mut self, projection: &Projection, row: Option<usize>) -> bool {
        let id = row.and_then(|row| match projection.row(row) {
            Some(Row::Item(item)) if projection.is_enabled(row) => Some(item.id.as_str()),
            Some(Row::Header(_)) | Some(Row::Item(_)) | None => None,
        });
        if self.active.as_deref() == id {
            return false;
        }
        self.active = id.map(str::to_owned);
        true
    }
    /// Preserve stable identity on filter/reorder; opening may explicitly reset
    /// to the first enabled selected ID (selection order), then the first item.
    pub fn reconcile(&mut self, projection: &Projection, reset: bool) -> bool {
        let row = if reset {
            None
        } else {
            self.active_row(projection)
        };
        self.set_row(projection, row.or_else(|| projection.preferred()))
    }
    /// Pointer hover targets enabled choices only; headers/disabled/stale indices
    /// leave the current highlight unchanged.
    pub fn highlight(&mut self, projection: &Projection, row: usize) -> bool {
        if !projection.is_enabled(row) {
            return false;
        }
        self.set_row(projection, Some(row))
    }
    pub fn navigate(&mut self, projection: &Projection, direction: Navigation) -> bool {
        let enabled = &projection.enabled_rows;
        if enabled.is_empty() {
            return self.set_row(projection, None);
        }
        let current = self
            .active_row(projection)
            .and_then(|row| enabled.binary_search(&row).ok());
        let next = match direction {
            Navigation::First => 0,
            Navigation::Last => enabled.len() - 1,
            Navigation::Next => current.map_or(0, |i| (i + 1) % enabled.len()),
            Navigation::Previous => current.map_or(enabled.len() - 1, |i| {
                (i + enabled.len() - 1) % enabled.len()
            }),
        };
        self.set_row(projection, Some(enabled[next]))
    }
}

#[cfg(test)]
#[path = "choice_picker_rows_test.rs"]
mod tests;
