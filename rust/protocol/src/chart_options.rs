//! Validated, source-independent plotting options, shared by all chart families.
use binprot::macros::BinProtWrite;

/// Upper bound for a standalone options frame, including 256 per-slice radii.
pub const MAX_OPTIONS_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum NumberFormat {
    Compact,
    Fixed(i64),
    Scientific(i64),
    Percent(i64),
}
impl NumberFormat {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Compact => true,
            Self::Fixed(n) | Self::Scientific(n) | Self::Percent(n) => (0..=6).contains(&n),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Axes {
    pub x: bool,
    pub y: bool,
    pub grid: bool,
    pub ticks: i64,
    pub x_format: NumberFormat,
    pub y_format: NumberFormat,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Curve {
    Linear,
    Natural,
    StepAfter,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Orientation {
    Vertical,
    Horizontal,
    VerticalReversed,
    HorizontalReversed,
}
impl Orientation {
    pub fn is_horizontal(self) -> bool {
        matches!(self, Self::Horizontal | Self::HorizontalReversed)
    }
    pub fn is_reversed(self) -> bool {
        matches!(self, Self::VerticalReversed | Self::HorizontalReversed)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub enum CategoryLayout {
    Auto,
    Point(f64),
    Band { inner: f64, outer: f64 },
}
impl CategoryLayout {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Auto => true,
            Self::Point(p) => between(p, 0., 1.),
            Self::Band { inner, outer } => {
                between(inner, 0., 1.) && inner < 1. && between(outer, 0., 1.)
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Stacking {
    Grouped,
    Stacked,
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Cartesian {
    pub curve: Curve,
    pub dots: bool,
    pub orientation: Orientation,
    pub bar_width: f64,
    pub category_layout: CategoryLayout,
    pub stacking: Stacking,
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub enum PieRadius {
    Fit,
    Pixels(f64),
}
impl PieRadius {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Fit => true,
            Self::Pixels(n) => between(n, 0., 32768.) && n > 0.,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct SliceRadii {
    pub slice: i64,
    pub inner: f64,
    pub outer: f64,
}
impl SliceRadii {
    pub fn is_valid(&self) -> bool {
        self.slice > 0
            && between(self.inner, 0., 32768.)
            && between(self.outer, 0., 32768.)
            && self.inner <= self.outer
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Pie {
    pub inner_radius: f64,
    pub pad_angle: f64,
    pub labels: bool,
    pub radius: PieRadius,
    pub slice_radii: Vec<SliceRadii>,
    pub label_placement: LabelPlacement,
    pub label_gap: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub enum RadarScale {
    PerAxis,
    DataMax,
    Maximum(f64),
}
impl RadarScale {
    pub fn is_valid(self) -> bool {
        match self {
            Self::PerAxis | Self::DataMax => true,
            Self::Maximum(n) => between(n, 0., 1e100) && n > 0.,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub enum RadarRadius {
    Fit,
    Pixels(f64),
}
impl RadarRadius {
    pub fn is_valid(self) -> bool {
        match self {
            Self::Fit => true,
            Self::Pixels(n) => between(n, 0., 32768.) && n > 0.,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Radar {
    pub levels: i64,
    pub dots: bool,
    pub labels: bool,
    pub scale: RadarScale,
    pub radius: RadarRadius,
    pub label_gap: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Candlestick {
    pub body_width: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Alignment {
    Left,
    Right,
    Center,
    Justify,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum FlowScale {
    Linear,
    Sqrt,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum LinkColor {
    Source,
    Target,
    Gradient,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum LabelPlacement {
    Inside,
    Outside,
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Sankey {
    pub node_width: f64,
    pub node_padding: f64,
    pub alignment: Alignment,
    pub scale: FlowScale,
    pub iterations: i64,
    pub labels: bool,
    pub node_corner_radius: f64,
    pub link_opacity: f64,
    pub min_link_width: f64,
    pub label_gap: f64,
    pub link_color: LinkColor,
    pub label_placement: LabelPlacement,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Options {
    pub version: i64,
    pub axes: Axes,
    pub cartesian: Cartesian,
    pub pie: Pie,
    pub radar: Radar,
    pub candlestick: Candlestick,
    pub sankey: Sankey,
}
fn between(n: f64, lo: f64, hi: f64) -> bool {
    n.is_finite() && (lo..=hi).contains(&n)
}
fn fraction(n: f64) -> bool {
    between(n, 0., 1.) && n > 0.
}
impl Options {
    pub fn heap_bytes(&self) -> usize {
        self.pie.slice_radii.capacity() * std::mem::size_of::<SliceRadii>()
    }
    pub fn is_valid(&self) -> bool {
        self.version == 9
            && (2..=12).contains(&self.axes.ticks)
            && self.axes.x_format.is_valid()
            && self.axes.y_format.is_valid()
            && fraction(self.cartesian.bar_width)
            && self.cartesian.category_layout.is_valid()
            && between(self.pie.inner_radius, 0., 0.95)
            && between(self.pie.pad_angle, 0., 0.2)
            && self.pie.radius.is_valid()
            && between(self.pie.label_gap, 0., 64.)
            && self.pie.slice_radii.len() <= 256
            && self.pie.slice_radii.iter().all(SliceRadii::is_valid)
            && self
                .pie
                .slice_radii
                .iter()
                .map(|r| r.slice)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == self.pie.slice_radii.len()
            && (1..=12).contains(&self.radar.levels)
            && self.radar.scale.is_valid()
            && self.radar.radius.is_valid()
            && between(self.radar.label_gap, 0., 64.)
            && fraction(self.candlestick.body_width)
            && between(self.sankey.node_width, 1., 64.)
            && between(self.sankey.node_padding, 0., 64.)
            && (0..=32).contains(&self.sankey.iterations)
            && between(self.sankey.node_corner_radius, 0., 32.)
            && between(self.sankey.link_opacity, 0., 1.)
            && between(self.sankey.min_link_width, 0., 64.)
            && between(self.sankey.label_gap, 0., 64.)
    }
}
impl Default for Options {
    fn default() -> Self {
        Self {
            version: 9,
            axes: Axes {
                x: true,
                y: true,
                grid: true,
                ticks: 5,
                x_format: NumberFormat::Compact,
                y_format: NumberFormat::Compact,
            },
            cartesian: Cartesian {
                curve: Curve::Linear,
                dots: false,
                orientation: Orientation::Vertical,
                bar_width: 0.8,
                category_layout: CategoryLayout::Auto,
                stacking: Stacking::Grouped,
            },
            pie: Pie {
                inner_radius: 0.,
                pad_angle: 0.,
                labels: true,
                radius: PieRadius::Fit,
                slice_radii: vec![],
                label_placement: LabelPlacement::Inside,
                label_gap: 15.,
            },
            radar: Radar {
                levels: 4,
                dots: true,
                labels: true,
                scale: RadarScale::PerAxis,
                radius: RadarRadius::Fit,
                label_gap: 0.,
            },
            candlestick: Candlestick { body_width: 0.7 },
            sankey: Sankey {
                node_width: 16.,
                node_padding: 12.,
                alignment: Alignment::Justify,
                scale: FlowScale::Linear,
                iterations: 6,
                labels: true,
                node_corner_radius: 1.,
                link_opacity: 0.5,
                min_link_width: 0.,
                label_gap: 6.,
                link_color: LinkColor::Source,
                label_placement: LabelPlacement::Inside,
            },
        }
    }
}
