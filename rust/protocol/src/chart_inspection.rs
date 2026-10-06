//! Bounded native chart inspection appearance. No callbacks or data accessors.
use binprot::macros::BinProtWrite;
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Placement {
    Corner,
    Anchor,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Axis {
    Off,
    Vertical,
    Horizontal,
    Both,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Pattern {
    Dashed,
    Solid,
}
fn within(n: f64, a: f64, b: f64) -> bool {
    n.is_finite() && (a..=b).contains(&n)
}
fn color(n: i64) -> bool {
    (0..=0xffff_ffff).contains(&n)
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Card {
    pub visible: bool,
    pub title: bool,
    pub values: bool,
    pub placement: Placement,
    pub width: f64,
    pub gap: f64,
    pub padding: f64,
    pub radius: f64,
    pub font_size: f64,
    pub line_height: f64,
    pub border_width: f64,
    pub text_color: Option<i64>,
    pub background: Option<i64>,
    pub border_color: Option<i64>,
}
impl Card {
    pub fn is_valid(self) -> bool {
        within(self.width, 96.0, 480.0)
            && within(self.gap, 0.0, 64.0)
            && within(self.padding, 0.0, 24.0)
            && within(self.radius, 0.0, 24.0)
            && within(self.font_size, 8.0, 32.0)
            && within(self.line_height, 8.0, 48.0)
            && within(self.border_width, 0.0, 8.0)
            && self.text_color.is_none_or(color)
            && self.background.is_none_or(color)
            && self.border_color.is_none_or(color)
            && self.line_height >= self.font_size
    }
}
impl Default for Card {
    fn default() -> Self {
        Self {
            visible: true,
            title: true,
            values: true,
            placement: Placement::Corner,
            width: 280.,
            gap: 8.,
            padding: 8.,
            radius: 6.,
            font_size: 12.,
            line_height: 17.,
            border_width: 0.,
            text_color: None,
            background: None,
            border_color: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Crosshair {
    pub axis: Axis,
    pub pattern: Pattern,
    pub thickness: f64,
    pub color: Option<i64>,
}
impl Crosshair {
    pub fn is_valid(self) -> bool {
        within(self.thickness, 0.5, 64.0) && self.color.is_none_or(color)
    }
}
impl Default for Crosshair {
    fn default() -> Self {
        Self {
            axis: Axis::Off,
            pattern: Pattern::Dashed,
            thickness: 1.,
            color: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Marker {
    pub visible: bool,
    pub status: bool,
    pub size: f64,
    pub stroke_width: f64,
    pub fill: Option<i64>,
    pub stroke: Option<i64>,
}
impl Marker {
    pub fn is_valid(self) -> bool {
        within(self.size, 2.0, 48.0)
            && within(self.stroke_width, 0.0, 8.0)
            && self.fill.is_none_or(color)
            && self.stroke.is_none_or(color)
            && self.stroke_width <= self.size / 2.
    }
}
impl Default for Marker {
    fn default() -> Self {
        Self {
            visible: true,
            status: true,
            size: 16.,
            stroke_width: 0.,
            fill: None,
            stroke: None,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, BinProtWrite)]
pub struct Inspection {
    pub card: Card,
    pub crosshair: Crosshair,
    pub marker: Marker,
}
impl Inspection {
    pub fn is_valid(self) -> bool {
        self.card.is_valid() && self.crosshair.is_valid() && self.marker.is_valid()
    }
}
