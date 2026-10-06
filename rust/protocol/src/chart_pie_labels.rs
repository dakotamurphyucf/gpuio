//! Bounded ID-keyed pie captions and leader colors; chart style owns versioning.
use binprot::macros::BinProtWrite;
use std::{collections::BTreeSet, mem::size_of};

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Entry {
    pub slice: i64,
    pub text: Option<String>,
    pub line_color: Option<i64>,
}
pub fn is_valid(entries: &[Entry]) -> bool {
    entries.len() <= 256
        && entries.iter().all(|e| {
            e.slice > 0
                && e.text
                    .as_ref()
                    .is_none_or(|s| s.len() <= 256 && !s.bytes().any(|b| b < 32 || b == 127))
                && e.line_color.is_none_or(|c| (0..=0xffff_ffff).contains(&c))
        })
        && entries
            .iter()
            .map(|e| e.slice)
            .collect::<BTreeSet<_>>()
            .len()
            == entries.len()
        && entries
            .iter()
            .filter_map(|e| e.text.as_ref())
            .map(String::len)
            .sum::<usize>()
            <= 32768
}
pub fn heap_bytes(entries: &Vec<Entry>) -> usize {
    entries.capacity() * size_of::<Entry>()
        + entries
            .iter()
            .filter_map(|e| e.text.as_ref())
            .map(String::capacity)
            .sum::<usize>()
}
