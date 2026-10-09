//! Native tab target presentation, independent of selection and callback identity.
use crate::v1::Style;
use binprot::macros::BinProtWrite;
use std::collections::BTreeSet;

pub const MAX_DECLARATIONS: usize = 256;
pub const MAX_ITEMS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Variant {
    Tab,
    Outline,
    Pill,
    Segmented,
    Underline,
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub variant: Variant,
    pub height: f64,
    pub gap: f64,
    pub padding: f64,
    pub tab_style: Vec<Style>,
    pub item_styles: Vec<(String, Vec<Style>)>,
}
impl Config {
    pub fn valid_geometry_and_ids(&self) -> bool {
        let bound = |v: f64, min, max| v.is_finite() && (min..=max).contains(&v);
        let mut ids = BTreeSet::new();
        bound(self.height, 16., 256.)
            && bound(self.gap, 0., 128.)
            && bound(self.padding, 0., 128.)
            && self.item_styles.len() <= MAX_ITEMS
            && self.item_styles.iter().all(|(id, _)| {
                !id.is_empty() && id.len() <= 256 && !id.contains('\0') && ids.insert(id)
            })
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            variant: Variant::Tab,
            height: 32.,
            gap: 4.,
            padding: 12.,
            tab_style: vec![],
            item_styles: vec![],
        }
    }
}
