use binprot::macros::BinProtWrite;
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Style {
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
}
fn color(value: i64) -> bool {
    (0..=0xffff_ffff).contains(&value)
}
fn within(value: f64, min: f64, max: f64) -> bool {
    value.is_finite() && (min..=max).contains(&value)
}
impl Style {
    pub fn is_valid(&self) -> bool {
        (1..=32).contains(&self.palette.len())
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
    }
    pub fn color(&self, layer: usize) -> u32 {
        self.palette[layer % self.palette.len()] as u32
    }
}
impl Default for Style {
    fn default() -> Self {
        Self {
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
        }
    }
}
