//! Bounded native command-palette search policies; registry commands stay separate.
use crate::v1::{CommandConfig, PaletteConfig};
use binprot::macros::BinProtWrite;
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, BinProtWrite)]
pub enum Presentation {
    #[default]
    Modal,
    Embedded,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, BinProtWrite)]
pub enum Search {
    #[default]
    AllTerms,
    Substring,
    Unfiltered,
    External,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, BinProtWrite)]
pub enum Escape {
    #[default]
    Dismiss,
    ClearQueryFirst,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Keywords {
    pub command: String,
    pub words: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub search: Search,
    pub searchable: bool,
    pub escape: Escape,
    pub keywords: Vec<Keywords>,
    pub presentation: Presentation,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            search: Search::AllTerms,
            searchable: true,
            escape: Escape::Dismiss,
            keywords: vec![],
            presentation: Presentation::Modal,
        }
    }
}
impl Config {
    pub fn text_bytes(&self) -> usize {
        self.keywords
            .iter()
            .map(|k| k.command.len() + k.words.iter().map(String::len).sum::<usize>())
            .sum()
    }
    pub fn is_valid(&self) -> bool {
        let mut seen = BTreeSet::new();
        self.keywords.len() <= 1024
            && self.keywords.iter().all(|k| {
                CommandConfig::valid_text(&k.command, 256)
                    && seen.insert(&k.command)
                    && k.words.len() <= 64
                    && k.words
                        .iter()
                        .all(|word| CommandConfig::valid_text(word, 4096))
            })
            && self.text_bytes() <= 262144
    }
    pub fn fits(&self, palette: &PaletteConfig) -> bool {
        self.is_valid()
            && self
                .keywords
                .iter()
                .all(|entry| palette.permits(&entry.command))
            && self.text_bytes()
                + palette.label.len()
                + palette.placeholder.len()
                + palette.commands.iter().map(String::len).sum::<usize>()
                <= 262144
    }
    pub fn retained_bytes(&self) -> usize {
        // Reserve both wire data and the native lowercase search index. Unicode
        // lowercase can expand UTF-8; four text copies bound original+folded data.
        std::mem::size_of::<Self>()
            + if self.search == Search::External {
                crate::palette_results::RESERVATION_BYTES
            } else {
                0
            }
            + self.text_bytes() * 4
            + self
                .keywords
                .iter()
                .map(|k| 256 + k.words.len() * 2 * std::mem::size_of::<String>())
                .sum::<usize>()
    }
}
impl Search {
    /// Inputs have already been lowercased by the native projection.
    pub fn matches(self, label: &str, id: &str, keywords: &[String], query: &str) -> bool {
        match self {
            Self::AllTerms => query.split_whitespace().all(|term| {
                label.contains(term)
                    || id.contains(term)
                    || keywords.iter().any(|k| k.contains(term))
            }),
            Self::Substring => {
                // The pinned CommandState trims before CommandItem::matches.
                // Keep the query entity unchanged; this affects matching only.
                let query = query.trim();
                label.contains(query) || keywords.iter().any(|k| k.contains(query))
            }
            Self::Unfiltered => true,
            Self::External => false,
        }
    }
}
