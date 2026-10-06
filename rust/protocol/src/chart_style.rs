use binprot::macros::BinProtWrite;
use std::{collections::BTreeSet, mem::size_of};

pub const MAX_COLOR_DOMAIN: usize = 1024;
pub const MAX_STYLE_BYTES: usize = 192 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, BinProtWrite)]
pub enum Key {
    Series(i64),
    Slice(i64),
    Node(i64),
    Rising,
    Falling,
}
impl Key {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Series(id) | Self::Slice(id) | Self::Node(id) => id > 0,
            Self::Rising | Self::Falling => true,
        }
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Ordinal {
    pub domain: Vec<Key>,
    pub range: Vec<i64>,
    pub unknown: Option<i64>,
}
impl Ordinal {
    pub fn is_valid(&self) -> bool {
        self.domain.len() <= MAX_COLOR_DOMAIN
            && self.domain.iter().all(|k| k.is_valid())
            && self.domain.iter().collect::<BTreeSet<_>>().len() == self.domain.len()
            && (1..=32).contains(&self.range.len())
            && self.range.iter().all(|c| color(*c))
            && self.unknown.is_none_or(color)
    }
    pub fn heap_bytes(&self) -> usize {
        self.domain.capacity() * size_of::<Key>() + self.range.capacity() * size_of::<i64>()
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Style {
    pub version: i64,
    pub palette: Vec<i64>,
    pub axis_color: i64,
    pub grid_color: i64,
    pub label_color: i64,
    pub selection_color: i64,
    pub gradient_end: Option<i64>,
    pub stroke_width: f64,
    pub point_radius: f64,
    pub bar_radius: f64,
    pub area_opacity: f64,
    pub ordinal: Option<Ordinal>,
    pub inspection: Box<crate::chart_inspection::Inspection>,
    pub node_labels: Vec<crate::chart_node_labels::Node>,
    pub pie_labels: Vec<crate::chart_pie_labels::Entry>,
    pub pie_label_line_color: Option<i64>,
    pub x_axis: Box<crate::chart_axis::Axis>,
    pub y_axis: Box<crate::chart_axis::Axis>,
    pub grid: Box<crate::chart_grid::Grid>,
}
fn color(value: i64) -> bool {
    (0..=0xffff_ffff).contains(&value)
}
fn within(value: f64, min: f64, max: f64) -> bool {
    value.is_finite() && (min..=max).contains(&value)
}
impl Style {
    pub fn is_valid(&self) -> bool {
        self.version == -4
            && (1..=32).contains(&self.palette.len())
            && self.palette.iter().all(|c| color(*c))
            && [
                self.axis_color,
                self.grid_color,
                self.label_color,
                self.selection_color,
            ]
            .into_iter()
            .all(color)
            && self.gradient_end.is_none_or(color)
            && within(self.stroke_width, 0.5, 8.)
            && within(self.point_radius, 1., 12.)
            && within(self.bar_radius, 0., 32.)
            && within(self.area_opacity, 0., 1.)
            && self.ordinal.as_ref().is_none_or(Ordinal::is_valid)
            && self.inspection.is_valid()
            && crate::chart_node_labels::is_valid(&self.node_labels)
            && crate::chart_pie_labels::is_valid(&self.pie_labels)
            && self.pie_label_line_color.is_none_or(color)
            && self.x_axis.is_valid()
            && self.y_axis.is_valid()
            && self.grid.is_valid()
    }
    pub fn heap_bytes(&self) -> usize {
        size_of::<crate::chart_inspection::Inspection>()
            + self.palette.capacity() * size_of::<i64>()
            + self.ordinal.as_ref().map_or(0, Ordinal::heap_bytes)
            + crate::chart_node_labels::heap_bytes(&self.node_labels)
            + crate::chart_pie_labels::heap_bytes(&self.pie_labels)
            + 2 * size_of::<crate::chart_axis::Axis>()
            + size_of::<crate::chart_grid::Grid>()
            + self.x_axis.heap_bytes()
            + self.y_axis.heap_bytes()
            + self.grid.heap_bytes()
    }
    pub fn color(&self, layer: usize) -> u32 {
        self.palette[layer % self.palette.len()] as u32
    }
}
impl Default for Style {
    fn default() -> Self {
        Self {
            version: -4,
            palette: vec![
                0x818cf8ff, 0x2dd4bfff, 0xfbbf24ff, 0xf472b6ff, 0x38bdf8ff, 0xfb923cff,
            ],
            axis_color: 0x64748bff,
            grid_color: 0x64748b40,
            label_color: 0x94a3b8ff,
            selection_color: 0xf8fafcff,
            gradient_end: None,
            stroke_width: 2.,
            point_radius: 3.5,
            bar_radius: 3.,
            area_opacity: 0.2,
            ordinal: None,
            inspection: Default::default(),
            node_labels: vec![],
            pie_labels: vec![],
            pie_label_line_color: None,
            x_axis: Default::default(),
            y_axis: Default::default(),
            grid: Default::default(),
        }
    }
}
