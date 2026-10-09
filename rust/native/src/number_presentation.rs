//! Bounded numeric part styles; input ownership remains in number_input_view.
use gpuio_protocol::{number_presentation::Config, v1::*};

pub(crate) fn validate(config: &Config) -> Result<(), ErrorCode> {
    if !config.geometry_is_valid() {
        return Err(ErrorCode::Malformed);
    }
    crate::appearance::validate_parts([
        (config.frame_style.as_slice(), &[0, 1, 6][..]),
        (config.editor_style.as_slice(), &[0, 1, 6][..]),
        (config.decrement_style.as_slice(), &[0, 2, 3, 6][..]),
        (config.increment_style.as_slice(), &[0, 2, 3, 6][..]),
    ])?;
    // Cursor ownership belongs to the numeric editor/native buttons.
    if parts(config)
        .into_iter()
        .flat_map(|styles| styles.iter())
        .any(|style| match style {
            Style::Fields(fields) | Style::State(_, fields) => {
                fields.iter().any(|f| matches!(f, Field::Cursor(_)))
            }
            _ => false,
        })
    {
        return Err(ErrorCode::Malformed);
    }
    Ok(())
}
fn parts(config: &Config) -> [&[Style]; 4] {
    [
        &config.frame_style,
        &config.editor_style,
        &config.decrement_style,
        &config.increment_style,
    ]
}
pub(crate) fn retained_bytes(config: &Config) -> usize {
    std::mem::size_of::<Config>()
        + parts(config)
            .into_iter()
            .map(|styles| {
                std::mem::size_of_val(styles)
                    + styles
                        .iter()
                        .map(crate::style::retained_bytes)
                        .sum::<usize>()
            })
            .sum::<usize>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_parts_accept_disabled_but_reject_selected() {
        for part in 0..4 {
            for (state, expected) in [(6, Ok(())), (7, Err(ErrorCode::Malformed))] {
                let mut config = Config::default();
                let styles = match part {
                    0 => &mut config.frame_style,
                    1 => &mut config.editor_style,
                    2 => &mut config.decrement_style,
                    3 => &mut config.increment_style,
                    _ => unreachable!(),
                };
                styles.push(Style::State(
                    state,
                    vec![Field::Foreground(Color::Rgba(0x808080ff))],
                ));
                assert_eq!(validate(&config), expected, "part={part}, state={state}");
            }
        }
    }
}
