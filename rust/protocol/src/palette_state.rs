//! Bounded asynchronous snapshots of native palette interaction state.
use binprot::macros::BinProtWrite;
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Snapshot {
    pub sequence: i64,
    pub query_revision: i64,
    pub query: String,
    pub composing: bool,
    pub selected: Option<String>,
    pub matched_count: i64,
    pub loading: bool,
}
impl Snapshot {
    pub fn is_valid(&self) -> bool {
        self.sequence > 0
            && self.query_revision > 0
            && self.query_revision <= self.sequence
            && self.query.len() <= 4096
            && !self.query.contains(['\0', '\r', '\n'])
            && (0..=1024).contains(&self.matched_count)
            && self.selected.as_ref().is_none_or(|id| {
                self.matched_count > 0
                    && id.len() <= 256
                    && !id
                        .trim_matches(|c: char| c.is_ascii_whitespace())
                        .is_empty()
                    && !id.contains('\0')
            })
    }
}
