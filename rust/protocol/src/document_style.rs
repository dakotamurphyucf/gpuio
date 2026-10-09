//! Checked internal reader presentation; no callbacks, resources or interaction owners.
use crate::v1::{Color, Field, Style};
use binprot::macros::BinProtWrite;
pub const MAX_DECLARATIONS: usize = 512;
pub const MAX_CONFIG_BYTES: usize = 256 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, BinProtWrite)]
pub enum Part {
    Foreground,
    MutedForeground,
    Link,
    Selection,
    CodeBackground,
    Border,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct HeadingSizes {
    pub h1: f64,
    pub h2: f64,
    pub h3: f64,
    pub h4: f64,
    pub h5: f64,
    pub h6: f64,
}
impl HeadingSizes {
    pub fn values(&self) -> [f64; 6] {
        [self.h1, self.h2, self.h3, self.h4, self.h5, self.h6]
    }
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Underline {
    pub color: Option<Color>,
    pub thickness: f64,
    pub wavy: bool,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Strikethrough {
    pub color: Option<Color>,
    pub thickness: f64,
}
#[derive(Clone, Debug, Default, PartialEq, BinProtWrite)]
pub struct InlineCode {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub font_weight: Option<i64>,
    pub italic: Option<bool>,
    pub underline: Option<Underline>,
    pub strikethrough: Option<Strikethrough>,
    pub fade_out: Option<f64>,
}
#[derive(Clone, Debug, Default, PartialEq, BinProtWrite)]
pub struct Config {
    pub colors: Vec<(Part, Color)>,
    pub paragraph_gap_rem: Option<f64>,
    pub heading_base_font_size: Option<f64>,
    pub heading_sizes: Option<HeadingSizes>,
    pub inline_code: InlineCode,
    pub code_block: Vec<Style>,
    pub table: Vec<Style>,
    pub table_head: Vec<Style>,
    pub table_cell: Vec<Style>,
}
fn bounded(value: f64, min: f64, max: f64) -> bool {
    value.is_finite() && (min..=max).contains(&value)
}
fn color(value: &Color) -> bool {
    matches!(value,Color::Rgba(n) if (0..=u32::MAX as i64).contains(n))
}
impl Config {
    pub fn styles(&self) -> [&[Style]; 4] {
        [
            &self.code_block,
            &self.table,
            &self.table_head,
            &self.table_cell,
        ]
    }
    pub fn is_valid(&self) -> bool {
        let code = &self.inline_code;
        let mut seen = std::collections::BTreeSet::new();
        let valid_values = self.colors.len() <= 6
            && self
                .colors
                .iter()
                .all(|(part, c)| seen.insert(*part) && color(c))
            && self.paragraph_gap_rem.is_none_or(|n| bounded(n, 0., 64.))
            && self
                .heading_base_font_size
                .is_none_or(|n| bounded(n, 1., 512.))
            && self
                .heading_sizes
                .as_ref()
                .is_none_or(|s| s.values().into_iter().all(|n| bounded(n, 1., 512.)))
            && code.foreground.as_ref().is_none_or(color)
            && code.background.as_ref().is_none_or(color)
            && code.font_weight.is_none_or(|n| (1..=1000).contains(&n))
            && code.fade_out.is_none_or(|n| bounded(n, 0., 1.))
            && code.underline.as_ref().is_none_or(|u| {
                u.color.as_ref().is_none_or(color) && bounded(u.thickness, 0., 32.)
            })
            && code.strikethrough.as_ref().is_none_or(|u| {
                u.color.as_ref().is_none_or(color) && bounded(u.thickness, 0., 32.)
            });
        if !valid_values {
            return false;
        }
        let mut count = 0;
        for styles in self.styles() {
            if styles.len() > crate::v1::MAX_STYLE_FIELDS {
                return false;
            }
            for style in styles {
                let Style::Fields(fields) = style else {
                    return false;
                };
                count += fields.len();
                if count > MAX_DECLARATIONS || !fields.iter().all(allowed_field) {
                    return false;
                }
            }
        }
        true
    }
}
pub fn allowed_field(field: &Field) -> bool {
    matches!(
        field,
        Field::Background(_)
            | Field::Foreground(_)
            | Field::Opacity(_)
            | Field::BorderColor(_)
            | Field::BorderStyle(_)
            | Field::BorderTopWidth(_)
            | Field::BorderRightWidth(_)
            | Field::BorderBottomWidth(_)
            | Field::BorderLeftWidth(_)
            | Field::TopLeftRadius(_)
            | Field::TopRightRadius(_)
            | Field::BottomLeftRadius(_)
            | Field::BottomRightRadius(_)
            | Field::Shadows(_)
            | Field::FontSize(_)
            | Field::FontFamily(_)
            | Field::FontWeight(_)
            | Field::TextAlign(_)
            | Field::LineHeight(_)
            | Field::TextDecoration(_)
            | Field::Width(_)
            | Field::MinWidth(_)
            | Field::MaxWidth(_)
            | Field::PaddingTop(_)
            | Field::PaddingRight(_)
            | Field::PaddingBottom(_)
            | Field::PaddingLeft(_)
            | Field::MarginTop(_)
            | Field::MarginRight(_)
            | Field::MarginBottom(_)
            | Field::MarginLeft(_)
    )
}
