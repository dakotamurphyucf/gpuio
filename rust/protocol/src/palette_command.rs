//! Asynchronous commands against an exact observed palette subscription.
use crate::palette_state::Snapshot;
use binprot::macros::BinProtWrite;
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Command {
    ReadSnapshot,
    Focus,
    SetQuery(String),
    Highlight(Option<String>),
}
impl Command {
    pub fn is_valid(&self) -> bool {
        let (query, selected) = match self {
            Self::ReadSnapshot | Self::Focus => return true,
            Self::SetQuery(query) => (query.clone(), None),
            Self::Highlight(selected) => (String::new(), selected.clone()),
        };
        Snapshot {
            sequence: 1,
            query_revision: 1,
            query,
            composing: false,
            selected,
            matched_count: 1,
        }
        .is_valid()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Error {
    NotMounted,
    Closed,
    StalePalette,
    QueryChanged,
    Composing,
    Unavailable,
    InvalidQuery,
    Busy,
    NativeFailure,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Response {
    Applied(Snapshot),
    Failed(Error),
}
