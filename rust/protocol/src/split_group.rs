//! Flat panel descriptions. Native geometry and gestures never cross as handles.
use crate::split::Axis;
use binprot::macros::BinProtWrite;
use std::collections::BTreeSet;
pub const MAX_PANELS: usize = 64;
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 256 && !id.contains('\0')
}
pub fn valid_size(v: f64) -> bool {
    v.is_finite() && (0.0..=16384.).contains(&v)
}
fn label(v: &str, max: usize) -> bool {
    !v.trim_ascii().is_empty() && v.len() <= max && !v.contains('\0')
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Panel {
    pub id: String,
    pub label: String,
    pub initial_size: Option<f64>,
    pub minimum_size: f64,
    pub maximum_size: f64,
    pub visible: bool,
}
impl Panel {
    pub fn is_valid(&self) -> bool {
        valid_id(&self.id)
            && label(&self.label, 1024)
            && valid_size(self.minimum_size)
            && valid_size(self.maximum_size)
            && self.minimum_size <= self.maximum_size
            && self
                .initial_size
                .is_none_or(|v| valid_size(v) && v >= self.minimum_size && v <= self.maximum_size)
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct ResizeRequest {
    pub id: String,
    pub size: f64,
    pub serial: i64,
}
impl ResizeRequest {
    pub fn is_valid(&self) -> bool {
        valid_id(&self.id) && valid_size(self.size) && self.serial > 0
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub label: String,
    pub axis: Axis,
    pub keyboard_step: f64,
    pub reset_generation: i64,
    pub resize: Option<ResizeRequest>,
    pub panels: Vec<Panel>,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        let mut ids = BTreeSet::new();
        label(&self.label, 4096)
            && valid_size(self.keyboard_step)
            && self.keyboard_step > 0.
            && self.reset_generation >= 0
            && self.resize.as_ref().is_none_or(ResizeRequest::is_valid)
            && self.panels.len() <= MAX_PANELS
            && self
                .panels
                .iter()
                .all(|p| p.is_valid() && ids.insert(&p.id))
    }
}
#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Source {
    Pointer,
    Keyboard,
    Accessibility,
    Request(i64),
}
impl Source {
    pub fn is_valid(&self) -> bool {
        !matches!(self,Self::Request(serial) if *serial<=0)
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Snapshot {
    pub source: Source,
    pub sizes: Vec<(String, f64)>,
}
impl Snapshot {
    pub fn is_valid(&self) -> bool {
        let mut ids = BTreeSet::new();
        self.source.is_valid()
            && self.sizes.len() <= MAX_PANELS
            && self
                .sizes
                .iter()
                .all(|(id, size)| valid_id(id) && valid_size(*size) && ids.insert(id))
    }
    pub fn valid_for(&self, config: &Config) -> bool {
        self.is_valid()
            && match self.source {
                Source::Request(serial) => {
                    config.resize.as_ref().is_some_and(|r| r.serial == serial)
                }
                _ => true,
            }
            && self.sizes.len() == config.panels.len()
            && self
                .sizes
                .iter()
                .zip(&config.panels)
                .all(|((id, size), p)| {
                    id == &p.id && *size >= p.minimum_size && *size <= p.maximum_size
                })
    }
}
