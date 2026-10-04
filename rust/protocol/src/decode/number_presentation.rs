use super::*;
impl Decoder<'_> {
    pub(super) fn number_presentation(
        &mut self,
    ) -> Result<crate::number_presentation::Config, DecodeError> {
        let config = crate::number_presentation::Config {
            gap: self.float()?,
            button_width: self.float()?,
            button_min_height: self.float()?,
            stacked_button_min_height: self.float()?,
            editor_padding: self.float()?,
            border_width: self.option(Self::float)?,
            frame_style: self.list(16, Self::style)?,
            editor_style: self.list(16, Self::style)?,
            decrement_style: self.list(16, Self::style)?,
            increment_style: self.list(16, Self::style)?,
        };
        if config.geometry_is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
