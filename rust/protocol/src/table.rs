//! Bounded retained table descriptions and keyed asynchronous intents.
use binprot::macros::BinProtWrite;
use std::collections::BTreeSet;

pub const MAX_COLUMNS: usize = 64;
pub const MAX_HEADER_LEVELS: usize = 4;
pub const MAX_SCHEMA_TEXT_BYTES: usize = 262_144;
pub const MAX_ACTIVE_CELLS: usize = 16_384;
pub const MAX_COPY_BYTES: usize = 65_536;

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
