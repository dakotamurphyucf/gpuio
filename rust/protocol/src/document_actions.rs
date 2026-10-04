//! Rich-document action configuration and immutable content snapshots.
use crate::document::Activation;
use binprot::macros::BinProtWrite;
pub const MAX_ACTIONS: usize = 16;
pub const MAX_TEXT: usize = 262_144;
pub const MAX_CELLS: usize = 4096;
pub fn valid_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id.as_bytes()[0].is_ascii_alphabetic()
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Action {
    pub id: String,
    pub label: String,
    pub enabled: bool,
}
impl Action {
    pub fn is_valid(&self) -> bool {
        valid_id(&self.id)
            && !self.label.is_empty()
            && self.label.len() <= 256
            && !self.label.contains('\0')
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub epoch: i64,
    pub observe: bool,
    pub copy_code: bool,
    pub copy_table: bool,
    pub code: Vec<Action>,
    pub table: Vec<Action>,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        let group = |actions: &[Action]| {
            actions.len() <= MAX_ACTIONS
                && actions.iter().all(Action::is_valid)
                && actions
                    .iter()
                    .map(|a| &a.id)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    == actions.len()
        };
        self.epoch > 0
            && group(&self.code)
            && group(&self.table)
            && (self.observe || (self.code.is_empty() && self.table.is_empty()))
    }
    pub fn enabled(&self) -> bool {
        self.observe
            || !self.copy_code
            || !self.copy_table
            || !self.code.is_empty()
            || !self.table.is_empty()
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + 256
            + self
                .code
                .iter()
                .chain(&self.table)
                .map(|a| std::mem::size_of::<Action>() + a.id.len() + a.label.len())
                .sum::<usize>()
    }
    pub fn allows(&self, id: &str, block: &Block) -> bool {
        self.observe
            && match block {
                Block::Code(..) => &self.code,
                Block::Table(..) => &self.table,
            }
            .iter()
            .any(|a| a.enabled && a.id == id)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct SourceRange {
    pub start_byte: i64,
    pub end_byte: i64,
}
impl SourceRange {
    pub fn is_valid(&self) -> bool {
        self.start_byte >= 0 && self.start_byte <= self.end_byte && self.end_byte <= 65536
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Block {
    Code(Option<String>, String),
    Table(Vec<String>, Vec<Vec<String>>, String),
}
impl Block {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Code(language, code) => {
                language.as_ref().is_none_or(|l| l.len() <= 4096) && code.len() <= 65536
            }
            Self::Table(headers, rows, markdown) => {
                if rows.len() > MAX_CELLS || headers.len() > MAX_CELLS || markdown.len() > MAX_TEXT
                {
                    return false;
                }
                let mut cells = headers.len();
                let mut bytes = markdown.len();
                for row in rows {
                    if row.len() > MAX_CELLS - cells {
                        return false;
                    }
                    cells += row.len();
                }
                for cell in headers.iter().chain(rows.iter().flatten()) {
                    if cell.len() > MAX_TEXT - bytes {
                        return false;
                    }
                    bytes += cell.len();
                }
                true
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Event {
    pub config_epoch: i64,
    pub action: String,
    pub source_revision: i64,
    pub source_generation: i64,
    pub source_range: Option<SourceRange>,
    pub block: Block,
    pub activation: Activation,
}
impl Event {
    pub fn payload_bytes(&self) -> usize {
        256 + self.action.len()
            + match &self.block {
                Block::Code(language, code) => {
                    language.as_ref().map_or(0, String::len) + code.len()
                }
                Block::Table(headers, rows, markdown) => {
                    markdown.len()
                        + rows.len() * std::mem::size_of::<Vec<String>>()
                        + headers
                            .iter()
                            .chain(rows.iter().flatten())
                            .map(|s| s.len() + std::mem::size_of::<String>())
                            .sum::<usize>()
                }
            }
    }
    pub fn is_valid(&self) -> bool {
        self.config_epoch > 0
            && valid_id(&self.action)
            && self.source_revision > 0
            && self.source_generation > 0
            && self.source_range.as_ref().is_none_or(SourceRange::is_valid)
            && self.block.is_valid()
            && self.activation.is_valid()
    }
}
