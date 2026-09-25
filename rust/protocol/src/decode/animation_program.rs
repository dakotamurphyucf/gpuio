use super::{DecodeError, Decoder};
use crate::{
    animation::{Easing, Repeat, Spring},
    animation_program::*,
};
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn animation_easing(&mut self) -> Result<Easing, DecodeError> {
        Ok(match self.tag()? {
            0 => Easing::Linear,
            1 => Easing::Ease,
            2 => Easing::EaseIn,
            3 => Easing::EaseOut,
            4 => Easing::EaseInOut,
            5 => Easing::CubicBezier(self.float()?, self.float()?, self.float()?, self.float()?),
            _ => return Err(DecodeError::Malformed),
        })
    }
    pub(super) fn animation_program_config(&mut self) -> Result<Config, DecodeError> {
        let generation = self.int()?;
        let initial = self.option(|d| d.animation_targets())?;
        let stages = self.list(MAX_STAGES, |d| {
            Ok(Stage {
                targets: d.animation_targets()?,
                timing: match d.tag()? {
                    0 => Timing::Tween(d.int()?, d.animation_easing()?),
                    1 => Timing::Spring(Spring {
                        stiffness: d.float()?,
                        damping: d.float()?,
                        mass: d.float()?,
                        epsilon: d.float()?,
                        max_duration_ms: d.int()?,
                    }),
                    _ => return Err(DecodeError::Malformed),
                },
                delay_ms: d.int()?,
            })
        })?;
        let delay_ms = self.int()?;
        let repeat = match self.tag()? {
            0 => Repeat::Once,
            1 => Repeat::Loop,
            2 => Repeat::Alternate,
            _ => return Err(DecodeError::Malformed),
        };
        let clock = match self.tag()? {
            0 => Clock::Independent,
            1 => Clock::Application,
            2 => Clock::Group(self.bounded_text(MAX_GROUP_BYTES)?),
            _ => return Err(DecodeError::Malformed),
        };
        let playback = match self.tag()? {
            0 => Playback::Running,
            1 => Playback::Paused,
            2 => Playback::Cancelled,
            _ => return Err(DecodeError::Malformed),
        };
        let restart = self.int()?;
        let config = Config {
            generation,
            program: Program {
                initial,
                stages,
                delay_ms,
                repeat,
                clock,
            },
            playback,
            restart,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
pub fn decode_animation_program(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.animation_program_config()?;
    if decoder.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(config)
}
