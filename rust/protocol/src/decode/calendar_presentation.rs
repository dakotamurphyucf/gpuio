use super::{DecodeError, Decoder};
impl Decoder<'_> {
    pub(super) fn calendar_appearance(
        &mut self,
    ) -> Result<crate::calendar_presentation::Appearance, DecodeError> {
        let appearance = crate::calendar_presentation::Appearance {
            months: self.int()?,
            cell_height: self.float()?,
            cell_gap: self.float()?,
            month_gap: self.float()?,
            padding: self.float()?,
            cell_radius: self.float()?,
            outline_width: self.float()?,
            selected_background: self.option(Self::int)?,
            selected_foreground: self.option(Self::int)?,
            hover_background: self.option(Self::int)?,
            today_border: self.option(Self::int)?,
            focus_border: self.option(Self::int)?,
            muted_foreground: self.option(Self::int)?,
        };
        if appearance.is_valid() {
            Ok(appearance)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
