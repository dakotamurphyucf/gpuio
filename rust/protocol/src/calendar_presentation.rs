//! Fixed-size calendar geometry and month-pane presentation.
use binprot::macros::BinProtWrite;
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Appearance {
    pub months: i64,
    pub cell_height: f64,
    pub cell_gap: f64,
    pub month_gap: f64,
    pub padding: f64,
    pub cell_radius: f64,
    pub outline_width: f64,
    pub selected_background: Option<i64>,
    pub selected_foreground: Option<i64>,
    pub hover_background: Option<i64>,
    pub today_border: Option<i64>,
    pub focus_border: Option<i64>,
    pub muted_foreground: Option<i64>,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            months: 1,
            cell_height: 32.0,
            cell_gap: 4.0,
            month_gap: 16.0,
            padding: 8.0,
            cell_radius: 6.0,
            outline_width: 1.0,
            selected_background: None,
            selected_foreground: None,
            hover_background: None,
            today_border: None,
            focus_border: None,
            muted_foreground: None,
        }
    }
}
impl Appearance {
    pub fn is_valid(&self) -> bool {
        let bounded = |v: f64, lo: f64, hi: f64| v.is_finite() && (lo..=hi).contains(&v);
        (1..=12).contains(&self.months)
            && bounded(self.cell_height, 16., 128.)
            && [
                self.cell_gap,
                self.month_gap,
                self.padding,
                self.cell_radius,
            ]
            .into_iter()
            .all(|n| bounded(n, 0., 64.))
            && bounded(self.outline_width, 0., self.cell_height / 2.)
            && [
                self.selected_background,
                self.selected_foreground,
                self.hover_background,
                self.today_border,
                self.focus_border,
                self.muted_foreground,
            ]
            .into_iter()
            .all(|n| n.is_none_or(|n| (0..=0xffff_ffff).contains(&n)))
    }
}
