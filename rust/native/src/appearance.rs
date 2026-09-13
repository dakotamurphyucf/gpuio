//! Validated choice-part presentation. Geometry and semantics remain explicit;
//! styles reuse the existing color/font/state machinery without layout overrides.
use gpuio_protocol::v1::*;

pub fn validate(appearance: &ChoiceAppearance) -> Result<(), ErrorCode> {
    let dimension = |v: f64| v.is_finite() && v > 0. && v <= 1_000_000.;
    if !dimension(appearance.popup_width)
        || !dimension(appearance.row_height)
        || !(1..=64).contains(&appearance.max_visible_rows)
        || appearance.empty_label.is_empty()
        || appearance.empty_label.len() > 1024
        || appearance.empty_label.contains('\0')
    {
        return Err(ErrorCode::Malformed);
    }
    let mut count = 0;
    for (styles, states) in [
        (&appearance.popup_style, &[0, 2][..]),
        (&appearance.option_style, &[0, 1, 2, 3, 6, 7][..]),
        (&appearance.empty_style, &[0][..]),
    ] {
        crate::tree::validate_style(styles)?;
        for style in styles {
            let (state, fields) = match style {
                Style::Fields(fields) => (0, fields),
                Style::State(state, fields) => (*state, fields),
                _ => return Err(ErrorCode::Malformed),
            };
            if !states.contains(&state) {
                return Err(ErrorCode::Malformed);
            }
            count += fields.len();
            if count > 128 {
                return Err(ErrorCode::LimitExceeded);
            }
            if !fields.iter().all(|field| {
                matches!(
                    field,
                    Field::Background(_)
                        | Field::Foreground(_)
                        | Field::Opacity(_)
                        | Field::BorderColor(_)
                        | Field::Shadows(_)
                        | Field::TopLeftRadius(_)
                        | Field::TopRightRadius(_)
                        | Field::BottomLeftRadius(_)
                        | Field::BottomRightRadius(_)
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

pub fn retained_bytes(appearance: &ChoiceAppearance) -> usize {
    std::mem::size_of::<ChoiceAppearance>()
        + appearance.empty_label.len()
        + [
            &appearance.popup_style,
            &appearance.option_style,
            &appearance.empty_style,
        ]
        .iter()
        .map(|styles| {
            std::mem::size_of_val(styles.as_slice())
                + styles
                    .iter()
                    .map(crate::style::retained_bytes)
                    .sum::<usize>()
        })
        .sum::<usize>()
}

pub fn refinement(styles: &[Style], state: i64) -> gpui::StyleRefinement {
    let mut result = gpui::StyleRefinement::default();
    for style in styles {
        match style {
            Style::Fields(fields) if state == 0 => crate::style::refine(&mut result, fields),
            Style::State(found, fields) if *found == state => {
                crate::style::refine(&mut result, fields)
            }
            _ => (),
        }
    }
    result
}

pub fn default() -> std::sync::Arc<ChoiceAppearance> {
    static DEFAULT: std::sync::OnceLock<std::sync::Arc<ChoiceAppearance>> =
        std::sync::OnceLock::new();
    DEFAULT
        .get_or_init(|| std::sync::Arc::new(ChoiceAppearance::default()))
        .clone()
}
