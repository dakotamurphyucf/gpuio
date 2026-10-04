use super::{DecodeError, Decoder};
impl Decoder<'_> {
    pub(super) fn slider_appearance(
        &mut self,
    ) -> Result<crate::slider_presentation::Appearance, DecodeError> {
        use crate::slider_presentation::{Appearance, Fill};
        let appearance = Appearance {
            fill: match self.tag()? {
                0 => Fill::Selected,
                1 => Fill::Remaining,
                _ => return Err(DecodeError::Malformed),
            },
            track_thickness: self.float()?,
            track_radius: self.float()?,
            thumb_size: self.float()?,
            target_size: self.float()?,
            ring_width: self.float()?,
            track_color: self.option(Self::int)?,
            fill_color: self.option(Self::int)?,
            thumb_color: self.option(Self::int)?,
            ring_color: self.option(Self::int)?,
        };
        if appearance.is_valid() {
            Ok(appearance)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
