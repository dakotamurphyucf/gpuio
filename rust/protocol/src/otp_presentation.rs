//! Bounded paint policy, independent of native code/selection/composition ownership.
use binprot::macros::BinProtWrite;
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Appearance {
    pub groups: i64,
    pub cell_width: Option<f64>,
    pub cell_gap: f64,
    pub group_gap: f64,
    pub radius: f64,
    pub border_width: f64,
    pub background: Option<i64>,
    pub border: Option<i64>,
    pub focus_border: Option<i64>,
    pub selection: Option<i64>,
    pub caret: Option<i64>,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            groups: 1,
            cell_width: None,
            cell_gap: 5.,
            group_gap: 20.,
            radius: 6.,
            border_width: 1.,
            background: None,
            border: None,
            focus_border: None,
            selection: None,
            caret: None,
        }
    }
}
impl Appearance {
    pub fn is_valid(&self) -> bool {
        let dimension = |n: f64| n.is_finite() && (0. ..=4096.).contains(&n);
        (1..=32).contains(&self.groups)
            && self.cell_width.is_none_or(|n| dimension(n) && n >= 1.)
            && [self.cell_gap, self.group_gap, self.radius]
                .into_iter()
                .all(dimension)
            && self.border_width.is_finite()
            && (0. ..=64.).contains(&self.border_width)
            && [
                self.background,
                self.border,
                self.focus_border,
                self.selection,
                self.caret,
            ]
            .into_iter()
            .all(|n| n.is_none_or(|n| (0..=0xffff_ffff).contains(&n)))
    }
    pub fn cells_per_group(&self, length: usize) -> usize {
        length
            .div_ceil((self.groups as usize).clamp(1, length.max(1)))
            .max(1)
    }
    pub fn cell_left(&self, index: usize, length: usize, width: f32) -> f32 {
        let group_gaps = index / self.cells_per_group(length);
        index as f32 * width
            + (index - group_gaps) as f32 * self.cell_gap as f32
            + group_gaps as f32 * self.group_gap as f32
    }
    pub fn width(&self, length: usize, cell_width: f32) -> f32 {
        self.cell_left(length.saturating_sub(1), length, cell_width) + cell_width
    }
}
