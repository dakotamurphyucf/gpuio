use super::{DecodeError, Decoder};
use crate::loading::{Config, Kind};
impl Decoder<'_> {
    pub(super) fn loading_config(&mut self) -> Result<Config, DecodeError> {
        let config = Config {
            kind: match self.tag()? {
                0 => Kind::Skeleton,
                1 => Kind::Shimmer,
                2 => Kind::Spinner,
                _ => return Err(DecodeError::Malformed),
            },
            label: self.bounded_text(4096)?,
            animated: self.boolean()?,
            period_ms: self.int()?,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
