//! Bounded, owned regex preparation data. Compilation belongs to the native
//! adapter; constructing Source does not claim syntax or resource acceptance.
use binprot::macros::BinProtWrite;

pub const MAX_SOURCE_BYTES: usize = 2048;
pub const MAX_DIAGNOSTIC_BYTES: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Matching {
    WholeValue,
    Substring,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Source {
    pub pattern: String,
    pub matching: Matching,
    pub case_sensitive: bool,
}
impl Source {
    pub fn is_valid(&self) -> bool {
        self.pattern.len() <= MAX_SOURCE_BYTES && !self.pattern.contains('\0')
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Rule {
    pub regex: Source,
    pub allow_empty: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Error {
    InvalidSource,
    InvalidRegex(String),
    TooComplex,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Preparation {
    Checked,
    Failed(Error),
}
