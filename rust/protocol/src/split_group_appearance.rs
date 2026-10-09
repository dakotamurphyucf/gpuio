//! Flat-group handle paint, independent of native hit geometry and ownership.
use crate::v1::Style;
use binprot::macros::BinProtWrite;
use std::collections::BTreeSet;

pub const MAX_DECLARATIONS: usize = 256;
pub const MAX_ITEMS: usize = 64;

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Config {
    pub thickness: f64,
    pub hit_extent: f64,
    pub handle_style: Vec<Style>,
    pub item_styles: Vec<(String, Vec<Style>)>,
}
impl Config {
    pub fn valid_geometry_and_ids(&self) -> bool {
        let bound = |v: f64, min, max| v.is_finite() && (min..=max).contains(&v);
        let mut ids = BTreeSet::new();
        bound(self.thickness, 1., 16.)
            && bound(self.hit_extent, 8., 32.)
            && self.thickness <= self.hit_extent
            && self.item_styles.len() <= MAX_ITEMS
            && self.item_styles.iter().all(|(id, _)| {
                !id.is_empty() && id.len() <= 256 && !id.contains('\0') && ids.insert(id)
            })
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            thickness: 1.,
            hit_extent: 8.,
            handle_style: vec![],
            item_styles: vec![],
        }
    }
}
