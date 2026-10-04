use super::{DecodeError, Decoder};
impl Decoder<'_> {
    pub(super) fn tab_motion(&mut self) -> Result<crate::tab_motion::Config, DecodeError> {
        let config = crate::tab_motion::Config {
            spring: crate::animation::Spring {
                stiffness: self.float()?,
                damping: self.float()?,
                mass: self.float()?,
                epsilon: self.float()?,
                max_duration_ms: self.int()?,
            },
            color_duration_ms: self.int()?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
