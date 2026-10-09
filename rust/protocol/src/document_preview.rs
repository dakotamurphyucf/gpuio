//! Retained rich-document preview configuration and installed-picture observations.
use binprot::macros::BinProtWrite;
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub epoch: i64,
    pub max_lines: Option<i64>,
    pub observe: bool,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.epoch > 0 && self.max_lines.is_none_or(|n| (1..=4096).contains(&n))
    }
    pub fn enabled(&self) -> bool {
        self.observe || self.max_lines.is_some()
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum State {
    Pending,
    Collapsed,
    SourceView,
    Rich(bool),
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Event {
    pub config_epoch: i64,
    pub source_revision: i64,
    pub source_generation: i64,
    pub state: State,
}
impl Event {
    pub fn is_valid(&self) -> bool {
        self.config_epoch > 0 && self.source_revision >= 0 && self.source_generation >= 0
    }
}
