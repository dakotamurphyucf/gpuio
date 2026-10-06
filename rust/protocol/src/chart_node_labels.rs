//! Bounded, preformatted Sankey label overrides. Parent chart style owns versioning.
use binprot::macros::BinProtWrite;
use std::{collections::BTreeSet, mem::size_of};

pub const MAX_NODES: usize = 128;
pub const MAX_LINES: usize = 4;
pub const MAX_LINE_BYTES: usize = 256;
pub const MAX_TEXT_BYTES: usize = 32 * 1024;
pub const MAX_BYTES: usize = 48 * 1024;

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Line {
    pub text: String,
    pub color: Option<i64>,
    pub font_size: Option<f64>,
}
impl Line {
    pub fn is_valid(&self) -> bool {
        self.text.len() <= MAX_LINE_BYTES
            && !self.text.bytes().any(|b| b < 32 || b == 127)
            && self.color.is_none_or(|n| (0..=0xffff_ffff).contains(&n))
            && self
                .font_size
                .is_none_or(|n| n.is_finite() && (8. ..=32.).contains(&n))
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Node {
    pub node: i64,
    pub lines: Vec<Line>,
}
pub fn is_valid(nodes: &[Node]) -> bool {
    nodes.len() <= MAX_NODES
        && nodes
            .iter()
            .all(|n| n.node > 0 && n.lines.len() <= MAX_LINES && n.lines.iter().all(Line::is_valid))
        && nodes.iter().map(|n| n.node).collect::<BTreeSet<_>>().len() == nodes.len()
        && nodes
            .iter()
            .flat_map(|n| &n.lines)
            .map(|line| line.text.len())
            .sum::<usize>()
            <= MAX_TEXT_BYTES
}

pub fn heap_bytes(nodes: &Vec<Node>) -> usize {
    nodes.capacity() * size_of::<Node>()
        + nodes
            .iter()
            .map(|n| {
                n.lines.capacity() * size_of::<Line>()
                    + n.lines
                        .iter()
                        .map(|line| line.text.capacity())
                        .sum::<usize>()
            })
            .sum::<usize>()
}
