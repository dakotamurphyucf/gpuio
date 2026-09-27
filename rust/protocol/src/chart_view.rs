use crate::{ResourceId, chart_options::Options, chart_sampling::Policy, chart_style::Style};
use binprot::macros::BinProtWrite;
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub source: Option<ResourceId>,
    pub label: String,
    pub options: Options,
    pub sampling: Policy,
    pub style: Style,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.label.len() <= 1024
            && self.label.bytes().any(|b| !matches!(b, 9..=13 | 32))
            && !self.label.bytes().any(|b| matches!(b, 0 | 10 | 13))
            && self.options.is_valid()
            && self.sampling.is_valid()
            && self.style.is_valid()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Error {
    WrongApplication,
    UnavailableData,
    RenderLimit,
    NativeFailure,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Metrics {
    pub source_values: i64,
    pub retained_values: i64,
    pub mesh_vertices: i64,
    pub quads: i64,
    pub bytes: i64,
}
impl Metrics {
    pub fn is_valid(&self) -> bool {
        (0..=100_000).contains(&self.source_values)
            && (0..=self.source_values).contains(&self.retained_values)
            && (0..=1_000_000).contains(&self.mesh_vertices)
            && (0..=300_000).contains(&self.quads)
            && (0..=67_108_864).contains(&self.bytes)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Observation {
    Ready(Metrics),
    Failed(Error),
}
impl Observation {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Ready(metrics) => metrics.is_valid(),
            Self::Failed(_) => true,
        }
    }
}
