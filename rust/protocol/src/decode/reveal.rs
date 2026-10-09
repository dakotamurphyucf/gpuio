use super::{DecodeError, Decoder};
impl Decoder<'_> {
    pub(super) fn reveal_config(&mut self) -> Result<crate::reveal::Config, DecodeError> {
        let config = crate::reveal::Config {
            expanded: self.boolean()?,
            retain: self.boolean()?,
            spring: crate::animation::Spring {
                stiffness: self.float()?,
                damping: self.float()?,
                mass: self.float()?,
                epsilon: self.float()?,
                max_duration_ms: self.int()?,
            },
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
