use super::{DecodeError, Decoder};
impl Decoder<'_> {
    pub(super) fn otp_appearance(
        &mut self,
    ) -> Result<crate::otp_presentation::Appearance, DecodeError> {
        let appearance = crate::otp_presentation::Appearance {
            groups: self.int()?,
            cell_width: self.option(Self::float)?,
            cell_gap: self.float()?,
            group_gap: self.float()?,
            radius: self.float()?,
            border_width: self.float()?,
            background: self.option(Self::int)?,
            border: self.option(Self::int)?,
            focus_border: self.option(Self::int)?,
            selection: self.option(Self::int)?,
            caret: self.option(Self::int)?,
        };
        if appearance.is_valid() {
            Ok(appearance)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
