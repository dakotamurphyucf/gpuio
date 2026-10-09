//! Bounded literal-search observations. Byte boundaries originate in the native
//! matcher; metadata alone cannot validate characters without the editor text.
use binprot::macros::BinProtWrite;

pub const MAX_QUERY_BYTES: usize = 2048;
pub fn valid_query(query: &str) -> bool {
    query.len() <= MAX_QUERY_BYTES && !query.contains('\0')
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Command {
    Read,
    Open(bool),
    Close,
    SetQuery(String, Case),
    Next,
    Previous,
    ReplaceCurrent(Stamp, String),
    ReplaceAll(Stamp, String),
    CloseAndFocus(i64),
    SetQueryText(String),
    SetCase(Case),
    ToggleCase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Case {
    Sensitive,
    AsciiInsensitive,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Mode {
    Closed,
    Find,
    Replace,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Stamp {
    pub editor_revision: i64,
    pub search_revision: i64,
}
impl Stamp {
    pub fn is_valid(&self) -> bool {
        self.editor_revision >= 0 && self.search_revision >= 0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Occurrence {
    pub index: i64,
    pub byte_start: i64,
    pub byte_end: i64,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Snapshot {
    pub stamp: Stamp,
    pub activation_revision: i64,
    pub mode: Mode,
    pub query: String,
    pub case: Case,
    pub text_bytes: i64,
    pub match_count: i64,
    pub current: Option<Occurrence>,
    pub can_replace: bool,
}
impl Snapshot {
    pub fn is_valid(&self) -> bool {
        self.stamp.is_valid()
            && valid_query(&self.query)
            && (0..=self.stamp.search_revision).contains(&self.activation_revision)
            && (0..=crate::v1::MAX_TEXT_BYTES as i64).contains(&self.text_bytes)
            && (0..=self.text_bytes).contains(&self.match_count)
            && (!self.can_replace || self.mode != Mode::Closed)
            && if self.query.is_empty() {
                self.match_count == 0
            } else {
                self.match_count * self.query.len() as i64 <= self.text_bytes
            }
            && match self.current {
                None => self.match_count == 0,
                Some(current) => {
                    current.index >= 0
                        && current.index < self.match_count
                        && current.byte_start >= 0
                        && current.byte_end > current.byte_start
                        && current.byte_end <= self.text_bytes
                        && current.byte_end - current.byte_start == self.query.len() as i64
                }
            }
    }
}
