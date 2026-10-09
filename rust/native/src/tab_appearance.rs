//! Checked tab presentation: native selection and ownership remain separate.
use gpuio_protocol::{
    tab_appearance::{Config, MAX_DECLARATIONS},
    v1::*,
};
pub fn validate(config: &Config) -> Result<(), ErrorCode> {
    if !config.valid_geometry_and_ids() {
        return Err(ErrorCode::Malformed);
    }
    let mut declarations = 0;
    for styles in std::iter::once(&config.tab_style)
        .chain(config.item_styles.iter().map(|(_, styles)| styles))
    {
        if styles.len() > MAX_DECLARATIONS {
            return Err(ErrorCode::LimitExceeded);
        }
        crate::tree::validate_style(styles)?;
        for style in styles {
            let fields = match style {
                Style::Fields(fields) | Style::State(1..=3 | 6..=7, fields) => fields,
                _ => return Err(ErrorCode::Malformed),
            };
            declarations += fields.len().max(1);
            if declarations > MAX_DECLARATIONS {
                return Err(ErrorCode::LimitExceeded);
            }
            if !fields.iter().all(|field| {
                matches!(
                    field,
                    Field::Width(_)
                        | Field::Height(_)
                        | Field::MinWidth(_)
                        | Field::MinHeight(_)
                        | Field::MaxWidth(_)
                        | Field::MaxHeight(_)
                        | Field::Grow(_)
                        | Field::Shrink(_)
                        | Field::Basis(_)
                        | Field::AlignItems(_)
                        | Field::JustifyContent(_)
                        | Field::RowGap(_)
                        | Field::ColumnGap(_)
                        | Field::PaddingTop(_)
                        | Field::PaddingRight(_)
                        | Field::PaddingBottom(_)
                        | Field::PaddingLeft(_)
                        | Field::MarginTop(_)
                        | Field::MarginRight(_)
                        | Field::MarginBottom(_)
                        | Field::MarginLeft(_)
                        | Field::Background(_)
                        | Field::Foreground(_)
                        | Field::Opacity(_)
                        | Field::BorderColor(_)
                        | Field::BorderTopWidth(_)
                        | Field::BorderRightWidth(_)
                        | Field::BorderBottomWidth(_)
                        | Field::BorderLeftWidth(_)
                        | Field::BorderStyle(_)
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
                        | Field::WhiteSpace(_)
                        | Field::TextOverflow(_)
                        | Field::LineClamp(_)
                        | Field::TextDecoration(_)
                        | Field::Cursor(_)
                )
            }) {
                return Err(ErrorCode::Malformed);
            }
        }
    }
    Ok(())
}
pub fn retained_bytes(config: &Config) -> usize {
    std::mem::size_of::<Config>()
        + config.item_styles.capacity() * std::mem::size_of::<(String, Vec<Style>)>()
        + config
            .item_styles
            .iter()
            .map(|(id, _)| id.capacity())
            .sum::<usize>()
        + std::iter::once(&config.tab_style)
            .chain(config.item_styles.iter().map(|(_, styles)| styles))
            .map(|styles| {
                styles.capacity() * std::mem::size_of::<Style>()
                    + styles
                        .iter()
                        .map(crate::style::retained_bytes)
                        .sum::<usize>()
            })
            .sum::<usize>()
}
