//! Choice-picker configuration, presentation and event values.
//! Op72 admits presentation; Event69 carries events. The public View is pending.
pub use crate::v1::ChoiceItem as Item;
use binprot::macros::BinProtWrite;
use std::collections::BTreeSet;
pub const MAX_ITEMS: usize = 4096;
pub const MAX_GROUPS: usize = 256;
pub const MAX_TEXT_BYTES: usize = 262_144;
// Catalog + repeated selected IDs + prefixes/record overhead, conservatively bounded.
pub const MAX_CONFIG_BYTES: usize = 786_432;
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Group {
    pub id: String,
    pub label: String,
    pub items: Vec<Item>,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Collection {
    Flat(Vec<Item>),
    Grouped(Vec<Group>),
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Selection {
    Single(Option<String>),
    Multiple(Vec<String>),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Search {
    None,
    Substring,
    Application,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum OpenState {
    Managed(bool),
    Controlled(bool),
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub label: String,
    pub options: Collection,
    pub selected: Selection,
    pub disabled: bool,
    pub search: Search,
    pub clearable: bool,
    pub open_state: OpenState,
    pub placeholder: String,
    pub search_placeholder: String,
}
fn text(value: &str, empty: bool, limit: usize) -> bool {
    (empty || !value.is_empty()) && value.len() <= limit && !value.contains('\0')
}
#[derive(Default)]
struct Catalog<'a> {
    ids: BTreeSet<&'a str>,
    count: usize,
    bytes: usize,
}
impl<'a> Catalog<'a> {
    fn items(&mut self, items: &'a [Item]) -> bool {
        if items.len() > MAX_ITEMS - self.count {
            return false;
        }
        for item in items {
            if !text(&item.id, false, 256) || !text(&item.label, false, 4096) {
                return false;
            }
            self.bytes += item.id.len() + item.label.len();
            if self.bytes > MAX_TEXT_BYTES || !self.ids.insert(&item.id) {
                return false;
            }
            self.count += 1;
        }
        true
    }
}
impl Config {
    pub fn is_valid(&self) -> bool {
        if !text(&self.label, false, 1024)
            || !text(&self.placeholder, true, 1024)
            || !text(&self.search_placeholder, true, 1024)
        {
            return false;
        }
        let mut catalog = Catalog::default();
        match &self.options {
            Collection::Flat(items) => {
                if !catalog.items(items) {
                    return false;
                }
            }
            Collection::Grouped(groups) => {
                if groups.len() > MAX_GROUPS {
                    return false;
                }
                let mut group_ids = BTreeSet::new();
                for group in groups {
                    if !text(&group.id, false, 256)
                        || !text(&group.label, false, 1024)
                        || !group_ids.insert(group.id.as_str())
                    {
                        return false;
                    }
                    catalog.bytes += group.id.len() + group.label.len();
                    if catalog.bytes > MAX_TEXT_BYTES || !catalog.items(&group.items) {
                        return false;
                    }
                }
            }
        }
        match &self.selected {
            Selection::Single(id) => id
                .as_ref()
                .is_none_or(|id| catalog.ids.contains(id.as_str())),
            Selection::Multiple(ids) => {
                if ids.len() > MAX_ITEMS {
                    return false;
                }
                let mut seen = BTreeSet::new();
                ids.iter()
                    .all(|id| catalog.ids.contains(id.as_str()) && seen.insert(id.as_str()))
            }
        }
    }
}

pub const MAX_QUERY_BYTES: usize = 262_144;
pub const MAX_EVENT_BYTES: usize = MAX_QUERY_BYTES + 512;
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Request {
    Select(String),
    Toggle(String),
    Clear,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum OpenReason {
    Trigger,
    Keyboard,
    Escape,
    OutsidePointer,
    FocusLeft,
    Selection,
}
impl OpenReason {
    pub fn allows(self, open: bool) -> bool {
        match self {
            Self::Trigger => true,
            Self::Keyboard => open,
            Self::Escape | Self::OutsidePointer | Self::FocusLeft | Self::Selection => !open,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum VisibilityReason {
    Interaction(OpenReason),
    Application,
    Unavailable,
}
impl VisibilityReason {
    pub fn allows(self, open: bool) -> bool {
        match self {
            Self::Interaction(reason) => reason.allows(open),
            Self::Application => true,
            Self::Unavailable => !open,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Visibility {
    Snapshot(bool),
    Changed(bool, VisibilityReason),
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Event {
    SelectionRequested(Request, Option<Query>),
    OpenRequested(bool, OpenReason),
    Visibility(Visibility),
    QueryChanged(Query),
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Query {
    pub node: crate::NodeId,
    pub snapshot: crate::v1::EditorSnapshot,
}
impl Query {
    pub fn is_valid(&self) -> bool {
        let snapshot = &self.snapshot;
        let boundary = |offset: i64| {
            usize::try_from(offset).is_ok_and(|offset| snapshot.text.is_char_boundary(offset))
        };
        let range =
            |range: &crate::v1::EditorSelection| boundary(range.anchor) && boundary(range.head);
        snapshot.revision >= 0
            && text(&snapshot.text, true, MAX_QUERY_BYTES)
            && !snapshot.text.contains(['\n', '\r'])
            && range(&snapshot.selection)
            && snapshot
                .composition
                .as_ref()
                .is_none_or(|r| r.anchor <= r.head && range(r))
    }
}
impl Event {
    /// Dynamic bytes in the event, in addition to the bridge's fixed envelope bound.
    pub fn payload_bytes(&self) -> usize {
        match self {
            Self::SelectionRequested(request, query) => {
                let id = match request {
                    Request::Select(id) | Request::Toggle(id) => id.len(),
                    Request::Clear => 0,
                };
                id + query.as_ref().map_or(0, |q| q.snapshot.text.len())
            }
            Self::QueryChanged(query) => query.snapshot.text.len(),
            Self::OpenRequested(..) | Self::Visibility(..) => 0,
        }
    }

    pub fn is_valid(&self) -> bool {
        match self {
            Self::SelectionRequested(request, query) => {
                let request_valid = match request {
                    Request::Select(id) | Request::Toggle(id) => text(id, false, 256),
                    Request::Clear => true,
                };
                request_valid
                    && query
                        .as_ref()
                        .is_none_or(|q| q.is_valid() && q.snapshot.composition.is_none())
            }
            Self::Visibility(Visibility::Snapshot(_)) => true,
            Self::OpenRequested(open, reason) => reason.allows(*open),
            Self::Visibility(Visibility::Changed(open, reason)) => reason.allows(*open),
            Self::QueryChanged(query) => query.is_valid(),
        }
    }
}

pub const MAX_SLOTS: usize = MAX_ITEMS + MAX_GROUPS + 4;
pub const MAX_PRESENTATION_BYTES: usize = 1_048_576;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Checkmark {
    Native,
    Custom,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Slot {
    Trigger,
    Query,
    Empty,
    Footer,
    Group(String),
    Option(String, Checkmark),
}

pub fn valid_slots(config: &Config, slots: &[Slot]) -> bool {
    if slots.len() > MAX_SLOTS {
        return false;
    }
    let mut group_ids = BTreeSet::new();
    let mut item_ids = BTreeSet::new();
    match &config.options {
        Collection::Flat(items) => item_ids.extend(items.iter().map(|item| item.id.as_str())),
        Collection::Grouped(groups) => {
            for group in groups {
                group_ids.insert(group.id.as_str());
                item_ids.extend(group.items.iter().map(|item| item.id.as_str()));
            }
        }
    }
    let mut fixed = [false; 4];
    let mut groups = BTreeSet::new();
    let mut items = BTreeSet::new();
    for slot in slots {
        let fixed_slot = match slot {
            Slot::Trigger => Some(0),
            Slot::Query => Some(1),
            Slot::Empty => Some(2),
            Slot::Footer => Some(3),
            Slot::Group(id) => {
                if !group_ids.contains(id.as_str()) || !groups.insert(id.as_str()) {
                    return false;
                }
                None
            }
            Slot::Option(id, _) => {
                if !item_ids.contains(id.as_str()) || !items.insert(id.as_str()) {
                    return false;
                }
                None
            }
        };
        if let Some(index) = fixed_slot {
            if fixed[index] {
                return false;
            }
            fixed[index] = true;
        }
    }
    fixed[1] == (config.search != Search::None)
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Presentation {
    pub config: Config,
    pub popup_width: f64,
    pub max_height: f64,
    pub estimated_row_height: f64,
    pub overscan: f64,
    pub empty_label: String,
    pub popup_style: Vec<crate::v1::Style>,
    pub option_style: Vec<crate::v1::Style>,
    pub header_style: Vec<crate::v1::Style>,
    pub empty_style: Vec<crate::v1::Style>,
    pub slots: Vec<Slot>,
}
impl Presentation {
    /// Domain/geometry/slot shape. Native admission must validate style fields,
    /// child kinds/passivity, editor identity/configuration and retained quotas.
    pub fn has_valid_shape(&self) -> bool {
        let dimension = |v: f64| v.is_finite() && v > 0. && v <= 1_000_000.;
        self.config.is_valid()
            && dimension(self.popup_width)
            && dimension(self.max_height)
            && dimension(self.estimated_row_height)
            && self.estimated_row_height >= 1.
            && self.overscan.is_finite()
            && (0.0..=4096.0).contains(&self.overscan)
            && text(&self.empty_label, false, 1024)
            && valid_slots(&self.config, &self.slots)
    }
}
