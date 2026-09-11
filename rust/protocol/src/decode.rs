use crate::{HandlerId, NodeId, WindowId, v1::*};
use binprot::BinProtRead;
use std::io::{Cursor, Read};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    Malformed,
    LimitExceeded,
}

struct Decoder<'a>(Cursor<&'a [u8]>);

impl Decoder<'_> {
    fn remaining(&self) -> usize {
        self.0.get_ref().len() - self.0.position() as usize
    }

    fn int(&mut self) -> Result<i64, DecodeError> {
        i64::binprot_read(&mut self.0).map_err(|_| DecodeError::Malformed)
    }

    fn float(&mut self) -> Result<f64, DecodeError> {
        let value = f64::binprot_read(&mut self.0).map_err(|_| DecodeError::Malformed)?;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }

    fn tag(&mut self) -> Result<u8, DecodeError> {
        let mut byte = [0];
        self.0
            .read_exact(&mut byte)
            .map_err(|_| DecodeError::Malformed)?;
        Ok(byte[0])
    }

    fn count(&mut self, maximum: usize) -> Result<usize, DecodeError> {
        let value = binprot::Nat0::binprot_read(&mut self.0)
            .map_err(|_| DecodeError::Malformed)?
            .0;
        if value > maximum as u64 {
            Err(DecodeError::LimitExceeded)
        } else if value > self.remaining() as u64 {
            // Every supported list element occupies at least one byte. Bound
            // allocations by the actual input as well as the declared limit.
            Err(DecodeError::Malformed)
        } else {
            Ok(value as usize)
        }
    }

    fn text(&mut self) -> Result<String, DecodeError> {
        let count = self.count(MAX_TEXT_BYTES)?;
        let start = self.0.position() as usize;
        let value = std::str::from_utf8(&self.0.get_ref()[start..start + count])
            .map_err(|_| DecodeError::Malformed)?
            .to_owned();
        self.0.set_position((start + count) as u64);
        Ok(value)
    }

    fn window(&mut self) -> Result<WindowId, DecodeError> {
        WindowId::from_parts(self.int()?, self.int()?).ok_or(DecodeError::Malformed)
    }

    fn node(&mut self) -> Result<NodeId, DecodeError> {
        NodeId::from_parts(self.int()?, self.int()?).ok_or(DecodeError::Malformed)
    }

    fn option<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, DecodeError>,
    ) -> Result<Option<T>, DecodeError> {
        match self.tag()? {
            0 => Ok(None),
            1 => f(self).map(Some),
            _ => Err(DecodeError::Malformed),
        }
    }

    fn handler(&mut self) -> Result<Option<HandlerId>, DecodeError> {
        self.option(|d| HandlerId::from_parts(d.int()?, d.int()?).ok_or(DecodeError::Malformed))
    }

    fn length(&mut self) -> Result<Length, DecodeError> {
        match self.tag()? {
            0 => Ok(Length::Px(self.float()?)),
            1 => Ok(Length::Percent(self.float()?)),
            2 => Ok(Length::Auto),
            _ => Err(DecodeError::Malformed),
        }
    }

    fn color(&mut self) -> Result<Color, DecodeError> {
        match self.tag()? {
            0 => Ok(Color::Rgba(self.int()?)),
            1 => Ok(Color::Token(self.int()?)),
            _ => Err(DecodeError::Malformed),
        }
    }

    fn style(&mut self) -> Result<Style, DecodeError> {
        Ok(match self.tag()? {
            0 => Style::Width(self.length()?),
            1 => Style::Height(self.length()?),
            2 => Style::MinWidth(self.length()?),
            3 => Style::MinHeight(self.length()?),
            4 => Style::MaxWidth(self.length()?),
            5 => Style::MaxHeight(self.length()?),
            6 => Style::Padding(self.float()?),
            7 => Style::Gap(self.float()?),
            8 => Style::Grow(self.float()?),
            9 => Style::Shrink(self.float()?),
            10 => Style::Direction(self.int()?),
            11 => Style::Background(self.color()?),
            12 => Style::Foreground(self.color()?),
            13 => Style::FontSize(self.float()?),
            14 => Style::Radius(self.float()?),
            15 => Style::Opacity(self.float()?),
            16 => Style::HoverBackground(self.color()?),
            17 => Style::PressedBackground(self.color()?),
            18 => Style::FocusBackground(self.color()?),
            _ => return Err(DecodeError::Malformed),
        })
    }

    fn list<T>(
        &mut self,
        max: usize,
        mut f: impl FnMut(&mut Self) -> Result<T, DecodeError>,
    ) -> Result<Vec<T>, DecodeError> {
        let count = self.count(max)?;
        (0..count).map(|_| f(self)).collect()
    }

    fn op(&mut self) -> Result<Op, DecodeError> {
        Ok(match self.tag()? {
            0 => {
                let id = self.node()?;
                let kind = match self.tag()? {
                    0 => Kind::Container,
                    1 => Kind::Text,
                    2 => Kind::Button,
                    _ => return Err(DecodeError::Malformed),
                };
                Op::Create(id, kind, self.text()?, self.handler()?)
            }
            1 => Op::Remove(self.node()?),
            2 => Op::SetText(self.node()?, self.text()?),
            3 => Op::SetStyle(self.node()?, self.list(MAX_STYLE_FIELDS, Self::style)?),
            4 => Op::Bind(self.node()?, self.handler()?),
            5 => Op::Splice(
                self.node()?,
                self.int()?,
                self.int()?,
                self.list(MAX_NODES, Self::node)?,
            ),
            6 => Op::SetRoot(self.option(Self::node)?),
            _ => return Err(DecodeError::Malformed),
        })
    }
}

/// Decode one message, rejecting trailing data, non-finite numbers and oversized
/// containers before allocating their declared capacity. UTF-8 text is required.
pub fn decode(bytes: &[u8]) -> Result<Message, DecodeError> {
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let d = &mut Decoder(Cursor::new(bytes));
    let value = match d.tag()? {
        0 => Message::Hello(d.int()?, d.int()?),
        1 => Message::Open(d.int()?, d.window()?, d.text()?, d.float()?, d.float()?),
        2 => Message::Close(d.int()?, d.window()?),
        3 => Message::Apply(Transaction {
            window: d.window()?,
            base: d.int()?,
            revision: d.int()?,
            operations: d.list(MAX_OPERATIONS, Decoder::op)?,
        }),
        4 => Message::RequestFrame(d.int()?, d.window()?),
        5 => Message::Shutdown,
        _ => return Err(DecodeError::Malformed),
    };
    if d.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(value)
}
