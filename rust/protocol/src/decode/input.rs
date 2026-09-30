use super::{DecodeError, Decoder};
use crate::{
    input::*,
    pointer::{PointerButton, PointerModifiers},
};
use std::io::Cursor;

impl Decoder<'_> {
    fn input_kind(&mut self) -> Result<Kind, DecodeError> {
        Ok(match self.tag()? {
            0 => Kind::Click,
            1 => Kind::AuxiliaryClick,
            2 => Kind::MouseDown,
            3 => Kind::MouseUp,
            4 => Kind::MouseMove,
            5 => Kind::MouseEnter,
            6 => Kind::MouseLeave,
            7 => Kind::MouseDownOutside,
            8 => Kind::KeyDown,
            9 => Kind::KeyUp,
            10 => Kind::Focus,
            11 => Kind::Blur,
            12 => Kind::Scroll,
            _ => return Err(DecodeError::Malformed),
        })
    }
    pub(super) fn input_config(&mut self) -> Result<Config, DecodeError> {
        let label = self.bounded_text(4096)?;
        let disabled = self.boolean()?;
        let focus = match self.tag()? {
            0 => Focus::None,
            1 => Focus::Click,
            2 => Focus::Tab,
            _ => return Err(DecodeError::Malformed),
        };
        let count = self.count(MAX_SUBSCRIPTIONS)?;
        let mut subscriptions = Vec::with_capacity(count);
        for _ in 0..count {
            subscriptions.push(Subscription {
                kind: self.input_kind()?,
                phase: match self.tag()? {
                    0 => Phase::Capture,
                    1 => Phase::Bubble,
                    _ => return Err(DecodeError::Malformed),
                },
                policy: match self.tag()? {
                    0 => Policy::Observe,
                    1 => Policy::StopPropagation,
                    2 => Policy::PreventDefault,
                    3 => Policy::PreventAndStop,
                    _ => return Err(DecodeError::Malformed),
                },
            });
        }
        let config = Config {
            label,
            disabled,
            focus,
            subscriptions,
        };
        if config.is_valid() {
            Ok(config)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    fn input_modifiers(&mut self) -> Result<PointerModifiers, DecodeError> {
        Ok(PointerModifiers {
            shift: self.boolean()?,
            control: self.boolean()?,
            alt: self.boolean()?,
            command: self.boolean()?,
            function: self.boolean()?,
        })
    }
    fn input_button(&mut self) -> Result<PointerButton, DecodeError> {
        Ok(match self.tag()? {
            0 => PointerButton::Left,
            1 => PointerButton::Right,
            2 => PointerButton::Middle,
            3 => PointerButton::Back,
            4 => PointerButton::Forward,
            _ => return Err(DecodeError::Malformed),
        })
    }
    fn input_position(&mut self) -> Result<Position, DecodeError> {
        Ok(Position {
            x: self.float()?,
            y: self.float()?,
        })
    }
    fn input_location(&mut self) -> Result<Location, DecodeError> {
        Ok(Location {
            window: self.input_position()?,
            local: self.input_position()?,
            modifiers: self.input_modifiers()?,
        })
    }
    fn input_mouse(&mut self) -> Result<Mouse, DecodeError> {
        Ok(Mouse {
            location: self.input_location()?,
            button: self.input_button()?,
            click_count: self.int()?,
        })
    }
    fn input_key(&mut self) -> Result<Key, DecodeError> {
        Ok(Key {
            key: self.bounded_text(256)?,
            character: self.option(|d| d.bounded_text(256))?,
            modifiers: self.input_modifiers()?,
        })
    }
    pub(super) fn input_event(&mut self) -> Result<Event, DecodeError> {
        let event = match self.input_kind()? {
            Kind::Click => Event::Click(self.input_mouse()?),
            Kind::AuxiliaryClick => Event::AuxiliaryClick(self.input_mouse()?),
            Kind::MouseDown => Event::MouseDown(self.input_mouse()?),
            Kind::MouseUp => Event::MouseUp(self.input_mouse()?),
            Kind::MouseMove => Event::MouseMove(Motion {
                location: self.input_location()?,
                pressed_button: self.option(|d| d.input_button())?,
            }),
            Kind::MouseEnter => Event::MouseEnter,
            Kind::MouseLeave => Event::MouseLeave,
            Kind::MouseDownOutside => Event::MouseDownOutside(self.input_mouse()?),
            Kind::KeyDown => Event::KeyDown(self.input_key()?, self.boolean()?),
            Kind::KeyUp => Event::KeyUp(self.input_key()?),
            Kind::Focus => Event::Focus,
            Kind::Blur => Event::Blur,
            Kind::Scroll => Event::Scroll(Scroll {
                location: self.input_location()?,
                delta: match self.tag()? {
                    0 => Delta::Pixels(self.input_position()?),
                    1 => Delta::Lines(self.input_position()?),
                    _ => return Err(DecodeError::Malformed),
                },
                phase: match self.tag()? {
                    0 => TouchPhase::Started,
                    1 => TouchPhase::Moved,
                    2 => TouchPhase::Ended,
                    3 => TouchPhase::Cancelled,
                    _ => return Err(DecodeError::Malformed),
                },
            }),
        };
        if event.is_valid() {
            Ok(event)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
pub fn decode_input_config(bytes: &[u8]) -> Result<Config, DecodeError> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let config = decoder.input_config()?;
    if decoder.remaining() == 0 {
        Ok(config)
    } else {
        Err(DecodeError::Malformed)
    }
}
pub fn decode_input_event(bytes: &[u8]) -> Result<Event, DecodeError> {
    if bytes.len() > MAX_EVENT_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let event = decoder.input_event()?;
    if decoder.remaining() == 0 {
        Ok(event)
    } else {
        Err(DecodeError::Malformed)
    }
}
