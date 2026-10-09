//! Ordered presentation referencing each flat registry command exactly once.
use crate::{
    palette_options,
    v1::{CommandConfig, PaletteConfig},
};
use binprot::macros::BinProtWrite;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Entry {
    Command(i64),
    Group(String, Option<String>, Vec<i64>),
    Separator,
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config(pub Vec<Entry>);
impl Config {
    pub fn text_bytes(&self) -> usize {
        self.0
            .iter()
            .map(|entry| match entry {
                Entry::Group(id, label, _) => id.len() + label.as_ref().map_or(0, String::len),
                Entry::Command(_) | Entry::Separator => 0,
            })
            .sum()
    }
    pub fn command_count(&self) -> usize {
        self.0
            .iter()
            .map(|entry| match entry {
                Entry::Command(_) => 1,
                Entry::Group(_, _, commands) => commands.len(),
                Entry::Separator => 0,
            })
            .sum()
    }
    pub fn is_valid(&self) -> bool {
        if self.0.len() > 1024 || self.command_count() > 1024 || self.text_bytes() > 262144 {
            return false;
        }
        let mut next = 0;
        let mut index = |value: i64| {
            let valid = value == next;
            next += 1;
            valid
        };
        let mut groups = BTreeSet::new();
        self.0.iter().all(|entry| match entry {
            Entry::Command(value) => index(*value),
            Entry::Group(id, label, commands) => {
                CommandConfig::valid_text(id, 256)
                    && groups.insert(id)
                    && label
                        .as_ref()
                        .is_none_or(|label| CommandConfig::valid_text(label, 4096))
                    && commands.iter().all(|value| index(*value))
            }
            Entry::Separator => true,
        })
    }
    pub fn fits(&self, palette: &PaletteConfig, options: Option<&palette_options::Config>) -> bool {
        self.is_valid()
            && self.command_count() == palette.commands.len()
            && self.text_bytes()
                + palette.label.len()
                + palette.placeholder.len()
                + palette.commands.iter().map(String::len).sum::<usize>()
                + options.map_or(0, palette_options::Config::text_bytes)
                <= 262144
    }
    pub fn retained_bytes(&self) -> usize {
        // Include the visual projection, key maps and measured-list row metadata.
        std::mem::size_of::<Self>()
            + self.text_bytes() * 3
            + self.0.len() * (std::mem::size_of::<Entry>() + 256)
            + self.command_count() * 256
    }
}
