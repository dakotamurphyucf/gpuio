//! Bounded numeric part styles; input ownership remains in number_input_view.
use gpuio_protocol::{number_presentation::Config, v1::*};

pub(crate) fn validate(config: &Config) -> Result<(), ErrorCode> {
    if !config.geometry_is_valid() {
        return Err(ErrorCode::Malformed);
    }
    crate::appearance::validate_parts([
        (config.frame_style.as_slice(), &[0, 1, 7][..]),
        (config.editor_style.as_slice(), &[0, 1, 7][..]),
        (config.decrement_style.as_slice(), &[0, 2, 3, 7][..]),
        (config.increment_style.as_slice(), &[0, 2, 3, 7][..]),
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
