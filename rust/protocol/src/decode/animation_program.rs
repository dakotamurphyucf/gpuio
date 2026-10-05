use super::{DecodeError, Decoder};
use crate::{
    animation::{Easing, LinearStops, MAX_LINEAR_STOPS, Repeat, Spring, StepPosition},
    animation_program::*,
};
use std::io::Cursor;

impl Decoder<'_> {
    pub(super) fn animation_repeat(&mut self) -> Result<Repeat, DecodeError> {
        use crate::animation::{Direction, IterationCount};
        let direction = |decoder: &mut Self| {
            Ok(match decoder.tag()? {
                0 => Direction::Normal,
                1 => Direction::Reverse,
                2 => Direction::Alternate,
                3 => Direction::AlternateReverse,
                _ => return Err(DecodeError::Malformed),
            })
        };
        Ok(match self.tag()? {
            0 => Repeat::Once,
            1 => Repeat::Loop,
            2 => Repeat::Alternate,
            3 => {
                let count = IterationCount {
                    high: self.int()?,
                    low: self.int()?,
                };
                if !count.is_valid() {
                    return Err(DecodeError::Malformed);
                }
                Repeat::Finite(count, direction(self)?)
            }
            4 => Repeat::Infinite(direction(self)?),
            _ => return Err(DecodeError::Malformed),
        })
    }

    pub(super) fn animation_easing(&mut self) -> Result<Easing, DecodeError> {
        Ok(match self.tag()? {
            0 => Easing::Linear,
            1 => Easing::Ease,
            2 => Easing::EaseIn,
            3 => Easing::EaseOut,
            4 => Easing::EaseInOut,
            5 => Easing::CubicBezier(self.float()?, self.float()?, self.float()?, self.float()?),
            6 => Easing::EaseInOutCubic,
            7 => Easing::Steps(
                self.int()?,
                match self.tag()? {
                    0 => StepPosition::JumpStart,
                    1 => StepPosition::JumpEnd,
                    2 => StepPosition::JumpNone,
                    3 => StepPosition::JumpBoth,
                    _ => return Err(DecodeError::Malformed),
                },
            ),
            8 => Easing::LinearStops(
                LinearStops::new(self.list(MAX_LINEAR_STOPS, |d| Ok((d.float()?, d.float()?)))?)
                    .ok_or(DecodeError::Malformed)?,
            ),
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
        let repeat = self.animation_repeat()?;
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
