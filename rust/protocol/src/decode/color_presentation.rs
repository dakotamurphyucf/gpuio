use super::{DecodeError, Decoder};
use crate::color_presentation::{Panel, Panels, Presentation, Section};
impl Decoder<'_> {
    pub(super) fn color_presentation(&mut self) -> Result<Presentation, DecodeError> {
        let p = Presentation {
            sections: self.list(32, |d| {
                Ok(Section {
                    featured: d.boolean()?,
                    label: d.bounded_text(256)?,
                    count: d.int()?,
                })
            })?,
            swatch_size: self.float()?,
            featured_size: self.float()?,
            swatch_gap: self.float()?,
            section_gap: self.float()?,
            swatch_radius: self.float()?,
            outline_width: self.float()?,
            channel_height: self.float()?,
            control_gap: self.float()?,
            padding: self.float()?,
            selected_border: self.option(Self::int)?,
            hover_border: self.option(Self::int)?,
            panels: match self.tag()? {
                0 => Panels::All,
                1 => Panels::Tabs {
                    palette_label: self.bounded_text(256)?,
                    channels_label: self.bounded_text(256)?,
                    initial: match self.tag()? {
                        0 => Panel::Palette,
                        1 => Panel::Channels,
                        _ => return Err(DecodeError::Malformed),
                    },
                },
                _ => return Err(DecodeError::Malformed),
            },
        };
        if p.is_valid() {
            Ok(p)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
