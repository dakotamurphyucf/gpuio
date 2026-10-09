//! Bounded retained table descriptions and keyed asynchronous intents.
use binprot::macros::BinProtWrite;
use std::collections::BTreeSet;

pub const MAX_COLUMNS: usize = 64;
pub const MAX_HEADER_LEVELS: usize = 4;
pub const MAX_SCHEMA_TEXT_BYTES: usize = 262_144;
pub const MAX_ACTIVE_CELLS: usize = 16_384;
pub const MAX_COPY_BYTES: usize = 65_536;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Boundary {
    Stop,
    Wrap,
}

/// Optional table behavior extension; the original Config and Column wire
/// layouts remain unchanged. Eligibility gates column headers, never cells.
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Behavior {
    pub row_header: bool,
    pub boundary: Boundary,
    pub selectable_headers: Option<Vec<String>>,
}
impl Default for Behavior {
    fn default() -> Self {
        Self {
            row_header: true,
            boundary: Boundary::Wrap,
            selectable_headers: None,
        }
    }
}
impl Behavior {
    pub fn is_valid(&self) -> bool {
        self.selectable_headers.as_ref().is_none_or(|ids| {
            ids.len() <= MAX_COLUMNS
                && ids.iter().all(|id| valid_id(id))
                && ids.iter().collect::<BTreeSet<_>>().len() == ids.len()
        })
    }
    pub fn valid_schema(&self, config: &Config) -> bool {
        self.selectable_headers
            .as_ref()
            .is_none_or(|ids| ids.iter().all(|id| config.has_column(id)))
    }
    pub fn header_selectable(&self, id: &str) -> bool {
        self.selectable_headers
            .as_ref()
            .is_none_or(|ids| ids.iter().any(|v| v == id))
    }
    pub fn allows_selection(&self, selection: &Selection) -> bool {
        match selection {
            Selection::Column(id) => self.header_selectable(id),
            _ => true,
        }
    }
    pub fn allows_request(&self, request: &Request) -> bool {
        match request {
            Request::Select(s) | Request::Context(s) | Request::Copy(s) => self.allows_selection(s),
            _ => true,
        }
    }
    pub fn allows_target(&self, target: &Target) -> bool {
        match target {
            Target::SetSelection(s) => self.allows_selection(s),
            _ => true,
        }
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.selectable_headers.as_ref().map_or(0, |ids| {
                ids.iter()
                    .map(|id| std::mem::size_of::<String>() + id.len())
                    .sum::<usize>()
            })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, BinProtWrite)]
pub enum Part {
    HeaderBackground,
    HeaderForeground,
    StripeBackground,
    HoverBackground,
    SelectedBackground,
    SelectedBorder,
    RowBorder,
    ColumnBorder,
    SortHoverBackground,
    SortPressedBackground,
    SortForeground,
    DragBorder,
    ContextBorder,
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Padding {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}
impl Padding {
    pub fn is_valid(&self) -> bool {
        [self.top, self.right, self.bottom, self.left]
            .into_iter()
            .all(|n| n.is_finite() && (0. ..=4096.).contains(&n))
    }
}
#[derive(Clone, Debug, Default, PartialEq, BinProtWrite)]
pub struct Appearance {
    pub striped: bool,
    pub colors: Vec<(Part, i64)>,
    pub padding: Option<Padding>,
    pub column_padding: Vec<(String, Padding)>,
}
impl Appearance {
    pub fn is_valid(&self) -> bool {
        self.colors.len() <= 13
            && self
                .colors
                .iter()
                .map(|(p, _)| p)
                .collect::<BTreeSet<_>>()
                .len()
                == self.colors.len()
            && self
                .colors
                .iter()
                .all(|(_, c)| (0..=0xffff_ffff).contains(c))
            && self.padding.is_none_or(|p| p.is_valid())
            && self.column_padding.len() <= MAX_COLUMNS
            && self
                .column_padding
                .iter()
                .map(|(id, _)| id)
                .collect::<BTreeSet<_>>()
                .len()
                == self.column_padding.len()
            && self
                .column_padding
                .iter()
                .all(|(id, p)| valid_id(id) && p.is_valid())
    }
    pub fn valid_schema(&self, config: &Config) -> bool {
        self.column_padding
            .iter()
            .all(|(id, _)| config.has_column(id))
    }
    pub fn color(&self, part: Part) -> Option<i64> {
        self.colors
            .iter()
            .find_map(|(p, c)| (*p == part).then_some(*c))
    }
    pub fn column_padding(&self, id: &str) -> Option<Padding> {
        self.column_padding
            .iter()
            .find_map(|(key, p)| (key == id).then_some(*p))
            .or(self.padding)
    }
    pub fn geometry_equal(a: Option<&Self>, b: Option<&Self>) -> bool {
        a.and_then(|v| v.padding) == b.and_then(|v| v.padding)
            && a.map_or(&[][..], |v| v.column_padding.as_slice())
                == b.map_or(&[][..], |v| v.column_padding.as_slice())
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.colors.len() * std::mem::size_of::<(Part, i64)>()
            + self
                .column_padding
                .iter()
                .map(|(id, _)| std::mem::size_of::<(String, Padding)>() + id.len())
                .sum::<usize>()
    }
}

pub fn valid_text(text: &str, empty: bool, limit: usize) -> bool {
    (empty || !text.is_empty()) && text.len() <= limit && !text.contains('\0')
}
pub fn valid_id(id: &str) -> bool {
    valid_text(id, false, 256)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Pin {
    Unpinned,
    Left,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Alignment {
    Left,
    Center,
    Right,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Direction {
    Ascending,
    Descending,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Column {
    pub id: String,
    pub label: String,
    pub width: f64,
    pub min_width: f64,
    pub max_width: f64,
    pub pin: Pin,
    pub alignment: Alignment,
    pub resizable: bool,
    pub movable: bool,
    pub sortable: bool,
}
impl Column {
    pub fn is_valid(&self) -> bool {
        valid_id(&self.id)
            && valid_text(&self.label, false, 4096)
            && self.width.is_finite()
            && self.min_width.is_finite()
            && self.max_width.is_finite()
            && 20. <= self.min_width
            && self.min_width <= self.width
            && self.width <= self.max_width
            && self.max_width <= 16384.
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Group {
    pub label: String,
    pub columns: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Schema {
    pub columns: Vec<Column>,
    pub headers: Vec<Vec<Group>>,
}
impl Schema {
    pub fn is_valid(&self) -> bool {
        if self.columns.len() > MAX_COLUMNS
            || self.headers.len() > MAX_HEADER_LEVELS
            || self.columns.iter().any(|column| !column.is_valid())
            || (self.columns.is_empty() && !self.headers.is_empty())
        {
            return false;
        }
        let ids: Vec<_> = self
            .columns
            .iter()
            .map(|column| column.id.as_str())
            .collect();
        if ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
            return false;
        }
        let pinned = self
            .columns
            .iter()
            .take_while(|column| column.pin == Pin::Left)
            .count();
        if self.columns[pinned..]
            .iter()
            .any(|column| column.pin != Pin::Unpinned)
        {
            return false;
        }
        let mut bytes: usize = self
            .columns
            .iter()
            .map(|column| column.id.len() + column.label.len())
            .sum();
        let mut upper = BTreeSet::new();
        for groups in &self.headers {
            if groups.is_empty() || groups.len() > MAX_COLUMNS {
                return false;
            }
            let mut start = 0;
            let mut boundaries = BTreeSet::new();
            for group in groups {
                if !valid_text(&group.label, false, 4096)
                    || group.columns.is_empty()
                    || group.columns.len() > MAX_COLUMNS
                {
                    return false;
                }
                let stop = start + group.columns.len();
                if stop > ids.len()
                    || (start < pinned && stop > pinned)
                    || group
                        .columns
                        .iter()
                        .map(String::as_str)
                        .ne(ids[start..stop].iter().copied())
                {
                    return false;
                }
                bytes += group.label.len() + group.columns.iter().map(String::len).sum::<usize>();
                if bytes > MAX_SCHEMA_TEXT_BYTES {
                    return false;
                }
                boundaries.insert(stop);
                start = stop;
            }
            if start != ids.len() || !upper.is_subset(&boundaries) {
                return false;
            }
            upper = boundaries;
        }
        bytes <= MAX_SCHEMA_TEXT_BYTES
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Sort {
    pub column: String,
    pub direction: Direction,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum SelectionMode {
    Rows,
    Cells,
    RowsAndCells,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub schema_revision: i64,
    pub query_generation: i64,
    pub schema: Schema,
    pub sort: Option<Sort>,
    pub row_height: f64,
    pub overscan: f64,
    pub max_active_rows: i64,
    pub max_active_cells: i64,
    pub selection_mode: SelectionMode,
    pub column_selection: bool,
    pub disabled: bool,
    pub scrollbar: bool,
    pub label: String,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.schema_revision > 0
            && self.query_generation >= 0
            && self.schema.is_valid()
            && self.sort.as_ref().is_none_or(|sort| {
                self.schema
                    .columns
                    .iter()
                    .any(|column| column.id == sort.column && column.sortable)
            })
            && self.row_height.is_finite()
            && (20. ..=4096.).contains(&self.row_height)
            && self.overscan.is_finite()
            && (0. ..=1_000_000.).contains(&self.overscan)
            && (1..=crate::list::MAX_ACTIVE_ROWS as i64).contains(&self.max_active_rows)
            && (1..=MAX_ACTIVE_CELLS as i64).contains(&self.max_active_cells)
            && self.max_active_rows * self.schema.columns.len() as i64 <= self.max_active_cells
            && valid_text(&self.label, false, 1024)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Cell {
    pub column: String,
    pub copy_text: String,
}
impl Cell {
    pub fn is_valid(&self) -> bool {
        valid_id(&self.column) && valid_text(&self.copy_text, true, MAX_COPY_BYTES)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Selection {
    Empty,
    Row(i64),
    Column(String),
    Cell(i64, String),
}
impl Selection {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Empty => true,
            Self::Row(row) => *row > 0,
            Self::Column(column) => valid_id(column),
            Self::Cell(row, column) => *row > 0 && valid_id(column),
        }
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Target {
    SetSelection(Selection),
    Reveal(i64, Option<String>),
    ScrollTo(i64, f64),
    ScrollToColumn(String),
    ScrollToEnd,
    ResetColumns,
}
impl Target {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::SetSelection(selection) => selection.is_valid(),
            Self::Reveal(row, column) => {
                *row > 0 && column.as_ref().is_none_or(|column| valid_id(column))
            }
            Self::ScrollTo(row, offset) => {
                *row > 0 && offset.is_finite() && (0. ..=4096.).contains(offset)
            }
            Self::ScrollToColumn(column) => valid_id(column),
            Self::ScrollToEnd | Self::ResetColumns => true,
        }
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Command {
    pub serial: i64,
    pub query_generation: i64,
    pub target: Target,
}
impl Command {
    pub fn is_valid(&self) -> bool {
        self.serial > 0 && self.query_generation >= 0 && self.target.is_valid()
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Request {
    Select(Selection),
    Activate(i64, Option<String>),
    Context(Selection),
    Resize(Vec<(String, f64)>),
    Move(String, Option<String>),
    Sort(String, Option<Direction>),
    Copy(Selection),
}
impl Request {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Select(selection) | Self::Context(selection) | Self::Copy(selection) => {
                selection.is_valid()
            }
            Self::Activate(row, column) => {
                *row > 0 && column.as_ref().is_none_or(|column| valid_id(column))
            }
            Self::Resize(widths) => {
                !widths.is_empty()
                    && widths.len() <= MAX_COLUMNS
                    && widths
                        .iter()
                        .map(|(column, _)| column)
                        .collect::<BTreeSet<_>>()
                        .len()
                        == widths.len()
                    && widths.iter().all(|(column, width)| {
                        valid_id(column) && width.is_finite() && (20. ..=16384.).contains(width)
                    })
            }
            Self::Move(column, before) => {
                valid_id(column)
                    && before
                        .as_ref()
                        .is_none_or(|target| valid_id(target) && target != column)
            }
            Self::Sort(column, _) => valid_id(column),
        }
    }
}

impl Schema {
    /// Retained admission units include containers and repeated group strings.
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.columns.len() * std::mem::size_of::<Column>()
            + self
                .columns
                .iter()
                .map(|c| c.id.len() + c.label.len())
                .sum::<usize>()
            + self.headers.len() * std::mem::size_of::<Vec<Group>>()
            + self
                .headers
                .iter()
                .flatten()
                .map(|g| {
                    std::mem::size_of::<Group>()
                        + g.label.len()
                        + g.columns.len() * std::mem::size_of::<String>()
                        + g.columns.iter().map(String::len).sum::<usize>()
                })
                .sum::<usize>()
    }

    /// Reorder keyed groups together with their members, preserving their sets.
    /// A move that splits a group or pinned prefix has no valid result.
    pub fn moved(&self, column: &str, before: Option<&str>) -> Option<Self> {
        let from = self
            .columns
            .iter()
            .position(|c| c.id == column && c.movable)?;
        if before == Some(column)
            || before.is_some_and(|id| !self.columns.iter().any(|c| c.id == id))
        {
            return None;
        }
        let mut next = self.clone();
        let moved = next.columns.remove(from);
        let target = before
            .and_then(|id| next.columns.iter().position(|c| c.id == id))
            .unwrap_or(next.columns.len());
        next.columns.insert(target, moved);
        let positions = next
            .columns
            .iter()
            .enumerate()
            .map(|(i, c)| (c.id.as_str(), i))
            .collect::<std::collections::BTreeMap<_, _>>();
        for level in &mut next.headers {
            for group in level.iter_mut() {
                group
                    .columns
                    .sort_by_key(|id| positions.get(id.as_str()).copied());
            }
            level.sort_by_key(|group| {
                group
                    .columns
                    .first()
                    .and_then(|id| positions.get(id.as_str()))
                    .copied()
            });
        }
        next.is_valid().then_some(next)
    }
}
impl Config {
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.schema.retained_bytes()
            + self.label.len()
            + self.sort.as_ref().map_or(0, |sort| sort.column.len())
    }
    pub fn list_config(&self) -> crate::list::Config {
        crate::list::Config {
            estimated_height: self.row_height,
            overscan: self.overscan,
            max_active: self.max_active_rows,
            scroll_policy: crate::list::ScrollPolicy::KeepPosition,
            scrollbar: self.scrollbar,
            managed: true,
        }
    }
    pub fn has_column(&self, id: &str) -> bool {
        self.schema.columns.iter().any(|column| column.id == id)
    }
    pub fn allows_selection(
        &self,
        selection: &Selection,
        row_exists: impl Fn(i64) -> bool,
    ) -> bool {
        match selection {
            Selection::Empty => true,
            Selection::Row(row) => self.selection_mode != SelectionMode::Cells && row_exists(*row),
            Selection::Column(column) => self.column_selection && self.has_column(column),
            Selection::Cell(row, column) => {
                self.selection_mode != SelectionMode::Rows
                    && row_exists(*row)
                    && self.has_column(column)
            }
        }
    }
    pub fn allows_target(&self, target: &Target, row_exists: impl Fn(i64) -> bool) -> bool {
        match target {
            Target::SetSelection(selection) => self.allows_selection(selection, row_exists),
            Target::Reveal(row, column) => {
                row_exists(*row) && column.as_ref().is_none_or(|id| self.has_column(id))
            }
            Target::ScrollTo(row, offset) => row_exists(*row) && *offset < self.row_height,
            Target::ScrollToColumn(column) => self.has_column(column),
            Target::ScrollToEnd | Target::ResetColumns => true,
        }
    }
    pub fn allows_request(&self, request: &Request, row_exists: impl Fn(i64) -> bool) -> bool {
        if self.disabled || !request.is_valid() {
            return false;
        }
        match request {
            Request::Select(selection) | Request::Context(selection) => {
                self.allows_selection(selection, row_exists)
            }
            Request::Copy(selection) => {
                *selection != Selection::Empty && self.allows_selection(selection, row_exists)
            }
            Request::Activate(row, column) => {
                row_exists(*row) && column.as_ref().is_none_or(|id| self.has_column(id))
            }
            Request::Resize(widths) => widths.iter().all(|(id, width)| {
                self.schema.columns.iter().any(|c| {
                    c.id == *id
                        && (c.min_width..=c.max_width).contains(width)
                        && (c.resizable || c.width == *width)
                })
            }),
            Request::Move(column, before) => self.schema.moved(column, before.as_deref()).is_some(),
            Request::Sort(column, _) => self
                .schema
                .columns
                .iter()
                .any(|c| c.id == *column && c.sortable),
        }
    }
}
impl Cell {
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.column.len() + self.copy_text.len()
    }
}
impl Request {
    /// Conservative encoded size contribution, including per-item framing.
    pub fn payload_bytes(&self) -> usize {
        match self {
            Self::Select(s) | Self::Context(s) | Self::Copy(s) => match s {
                Selection::Column(c) | Selection::Cell(_, c) => c.len() + 16,
                Selection::Empty | Selection::Row(_) => 16,
            },
            Self::Activate(_, c) => c.as_ref().map_or(0, String::len) + 32,
            Self::Resize(widths) => widths.iter().map(|(c, _)| c.len() + 32).sum(),
            Self::Move(c, before) => c.len() + before.as_ref().map_or(0, String::len) + 32,
            Self::Sort(c, _) => c.len() + 16,
        }
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Input {
    pub schema_revision: i64,
    pub query_generation: i64,
    pub request: Request,
}

/// Stable column bands in display order, independent of row-data availability.
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct ColumnViewport {
    pub schema_revision: i64,
    pub query_generation: i64,
    pub columns: Vec<(String, Pin, bool)>,
}
impl ColumnViewport {
    pub fn is_valid(&self) -> bool {
        let mut ids = BTreeSet::new();
        let mut scrolling = false;
        self.schema_revision > 0
            && self.query_generation >= 0
            && self.columns.len() <= MAX_COLUMNS
            && self.columns.iter().all(|(id, pin, _)| {
                let pin_order = !scrolling || *pin == Pin::Unpinned;
                scrolling |= *pin == Pin::Unpinned;
                valid_id(id) && ids.insert(id) && pin_order
            })
    }
    pub fn matches_schema(&self, config: &Config) -> bool {
        self.is_valid()
            && self.schema_revision == config.schema_revision
            && self.query_generation == config.query_generation
            && self.columns.iter().all(|(id, pin, _)| {
                config
                    .schema
                    .columns
                    .iter()
                    .any(|c| c.id == *id && c.pin == *pin)
            })
    }
    pub fn payload_bytes(&self) -> usize {
        self.columns.iter().map(|(id, _, _)| id.len() + 11).sum()
    }
}
impl Input {
    pub fn is_valid(&self) -> bool {
        self.schema_revision > 0 && self.query_generation >= 0 && self.request.is_valid()
    }
}
