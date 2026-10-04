//! Checked custom-header addresses; mounting/ownership is a separate admission step.
use crate::table::{MAX_COLUMNS, MAX_HEADER_LEVELS, Schema, valid_id};
use binprot::macros::BinProtWrite;

pub const MAX_HEADERS: usize = MAX_COLUMNS * (MAX_HEADER_LEVELS + 1);
pub const MAX_TARGET_BYTES: usize = 20_000;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, BinProtWrite)]
pub enum Target {
    Column(String),
    Group { level: i64, columns: Vec<String> },
}
impl Target {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Column(id) => valid_id(id),
            Self::Group { level, columns } => {
                (0..MAX_HEADER_LEVELS as i64).contains(level)
                    && !columns.is_empty()
                    && columns.len() <= MAX_COLUMNS
                    && columns.iter().all(|id| valid_id(id))
                    && columns.windows(2).all(|pair| pair[0] < pair[1])
            }
        }
    }

    /// Exact membership against an already admitted schema. Labels and current
    /// member display order are intentionally not part of the address.
    pub fn matches(&self, schema: &Schema) -> bool {
        if !self.is_valid() {
            return false;
        }
        match self {
            Self::Column(id) => schema.columns.iter().any(|column| column.id == *id),
            Self::Group { level, columns } => {
                schema.headers.get(*level as usize).is_some_and(|groups| {
                    groups.iter().any(|group| {
                        group.columns.len() == columns.len()
                            && group
                                .columns
                                .iter()
                                .all(|id| columns.binary_search(id).is_ok())
                    })
                })
            }
        }
    }

    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Column(id) => id.len(),
                Self::Group { columns, .. } => {
                    columns.len() * std::mem::size_of::<String>()
                        + columns.iter().map(String::len).sum::<usize>()
                }
            }
    }
}
