//! Admission and value-state refinement for checkable indicator parts.
use gpuio_protocol::{
    control_appearance::{Config, MAX_DECLARATIONS},
    v1::*,
};

/// Validate before retaining or refining. Interaction/layout fields are forbidden.
pub fn validate(config: &Config) -> Result<(), ErrorCode> {
    if !config.valid_geometry() {
        return Err(ErrorCode::Malformed);
    }
    let mut count = 0;
    for (styles, mark) in [(&config.indicator_style, false), (&config.mark_style, true)] {
        if styles.len() > MAX_DECLARATIONS {
            return Err(ErrorCode::LimitExceeded);
        }
        crate::tree::validate_style(styles)?;
        for style in styles {
            let fields = match style {
                Style::Fields(fields) => fields,
                Style::State(4..=6, fields) => fields,
                _ => return Err(ErrorCode::Malformed),
            };
            count += fields.len();
            if count > MAX_DECLARATIONS {
                return Err(ErrorCode::LimitExceeded);
            }
            for field in fields {
                let common = matches!(field, Field::Foreground(_) | Field::Opacity(_));
                let outline = matches!(
                    field,
                    Field::Background(_)
                        | Field::BorderColor(_)
                        | Field::BorderTopWidth(_)
                        | Field::BorderRightWidth(_)
                        | Field::BorderBottomWidth(_)
                        | Field::BorderLeftWidth(_)
                        | Field::TopLeftRadius(_)
                        | Field::TopRightRadius(_)
                        | Field::BottomLeftRadius(_)
                        | Field::BottomRightRadius(_)
                );
                if !(common || (!mark && outline)) {
                    return Err(ErrorCode::Malformed);
                }
            }
        }
    }
    Ok(())
}

/// Allocated payload for a validated config; allowed fields own no nested heaps.
pub fn retained_bytes(config: &Config) -> usize {
    std::mem::size_of::<Config>()
        + [&config.indicator_style, &config.mark_style]
            .into_iter()
            .map(|styles| {
                styles.capacity() * std::mem::size_of::<Style>()
                    + styles
                        .iter()
                        .map(|style| match style {
                            Style::Fields(fields) | Style::State(_, fields) => {
                                fields.capacity() * std::mem::size_of::<Field>()
                            }
                            _ => 0,
                        })
                        .sum::<usize>()
            })
            .sum::<usize>()
}

/// Caller supplies native value/disabled state, never a stale application callback.
/// Mixed takes precedence over checked; disabled refines either value last.
pub fn refinement(
    styles: &[Style],
    checked: bool,
    mixed: bool,
    disabled: bool,
) -> gpui::StyleRefinement {
    use gpui::Refineable;
    let mut result = crate::appearance::refinement(styles, 0);
    let state = if mixed {
        Some(5)
    } else if checked {
        Some(4)
    } else {
        None
    };
    if let Some(state) = state {
        result.refine(&crate::appearance::refinement(styles, state));
    }
    if disabled {
        result.refine(&crate::appearance::refinement(styles, 6));
    }
    result
}

/// Shared immutable legacy appearance; no per-frame allocation.
pub fn default() -> &'static Config {
    static DEFAULT: std::sync::OnceLock<Config> = std::sync::OnceLock::new();
    DEFAULT.get_or_init(Config::default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_reject_layout_interaction_wrong_states_and_invalid_values() {
        let mut config = Config::default();
        validate(&config).unwrap();
        for field in [
            Field::Width(Length::Px(20.)),
            Field::Inert(true),
            Field::AccessibleName("Wrong".into()),
            Field::FontSize(14.),
            Field::Opacity(f64::NAN),
            Field::BorderTopWidth(-1.),
            Field::Foreground(Color::Token(7)),
        ] {
            config.indicator_style = vec![Style::Fields(vec![field])];
            assert!(validate(&config).is_err());
        }
        for state in [0, 1, 2, 3, 7, 99] {
            config.indicator_style = vec![Style::State(state, vec![Field::Opacity(0.5)])];
            assert!(validate(&config).is_err());
        }
        config.indicator_style.clear();
        config.mark_style = vec![Style::Fields(vec![Field::Background(Fill::Solid(
            Color::Rgba(0xff),
        ))])];
        assert!(validate(&config).is_err());
        config.mark_style = vec![Style::Fields(vec![Field::Opacity(1.); 128])];
        validate(&config).unwrap();
        config.indicator_style = vec![Style::Fields(vec![Field::Opacity(1.)])];
        assert_eq!(validate(&config), Err(ErrorCode::LimitExceeded));
        config.indicator_style.clear();
        config.mark_style = vec![Style::Fields(vec![]); 129];
        assert_eq!(validate(&config), Err(ErrorCode::LimitExceeded));
    }

    #[test]
    fn value_and_disabled_precedence_do_not_depend_on_declaration_order() {
        let colors = [0x102030ff, 0x405060ff, 0x708090ff, 0xa0b0c0ff];
        let styles = vec![
            Style::State(6, vec![Field::Foreground(Color::Rgba(colors[3]))]),
            Style::State(5, vec![Field::Foreground(Color::Rgba(colors[2]))]),
            Style::State(4, vec![Field::Foreground(Color::Rgba(colors[1]))]),
            Style::Fields(vec![Field::Foreground(Color::Rgba(colors[0]))]),
        ];
        let config = Config {
            indicator_style: styles.clone(),
            ..Config::default()
        };
        validate(&config).unwrap();
        for (checked, mixed, disabled, expected) in [
            (false, false, false, 0),
            (true, false, false, 1),
            (false, true, false, 2),
            (true, true, false, 2),
            (true, false, true, 3),
            (false, true, true, 3),
        ] {
            assert_eq!(
                refinement(&styles, checked, mixed, disabled).text.color,
                Some(gpui::rgba(colors[expected] as u32).into())
            );
        }
        assert!(retained_bytes(&config) > std::mem::size_of::<Config>());
    }
}
