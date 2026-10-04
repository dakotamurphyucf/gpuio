//! Checked split handle paint; hit geometry and behavior remain native.
use gpuio_protocol::{
    split_group_appearance::{Config, MAX_DECLARATIONS},
    v1::*,
};
pub fn validate(config: &Config) -> Result<(), ErrorCode> {
    if !config.valid_geometry_and_ids() {
        return Err(ErrorCode::Malformed);
    }
    let mut declarations = 0;
    for styles in std::iter::once(&config.handle_style)
        .chain(config.item_styles.iter().map(|(_, styles)| styles))
    {
        if styles.len() > MAX_DECLARATIONS {
            return Err(ErrorCode::LimitExceeded);
        }
        crate::tree::validate_style(styles)?;
        for style in styles {
            let fields = match style {
                Style::Fields(fields) | Style::State(1..=3 | 6, fields) => fields,
                _ => return Err(ErrorCode::Malformed),
            };
            declarations += fields.len().max(1);
            if declarations > MAX_DECLARATIONS {
                return Err(ErrorCode::LimitExceeded);
            }
            if !fields.iter().all(|field| {
                matches!(
                    field,
                    Field::Background(_)
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
        + std::iter::once(&config.handle_style)
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

/// Per-ID declarations override shared declarations in each state. Interaction
/// state wins over base paint, and disabled always wins over active feedback.
pub fn refinement(
    config: &Config,
    id: &str,
    hovered: bool,
    focused: bool,
    pressed: bool,
    disabled: bool,
) -> gpui::StyleRefinement {
    use gpui::Refineable;
    let item = config
        .item_styles
        .iter()
        .find(|(key, _)| key == id)
        .map_or(&[][..], |(_, styles)| styles.as_slice());
    let mut paint = gpui::StyleRefinement::default();
    for (state, active) in [
        (0, true),
        (2, hovered),
        (1, focused),
        (3, pressed),
        (6, disabled),
    ] {
        if active {
            paint.refine(&crate::appearance::refinement(&config.handle_style, state));
            paint.refine(&crate::appearance::refinement(item, state));
        }
    }
    paint
}
