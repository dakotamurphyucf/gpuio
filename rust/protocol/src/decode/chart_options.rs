use super::{DecodeError, Decoder};
use crate::chart_options::*;
use std::io::Cursor;

impl Decoder<'_> {
    fn chart_number_format(&mut self) -> Result<NumberFormat, DecodeError> {
        Ok(match self.tag()? {
            0 => NumberFormat::Compact,
            1 => NumberFormat::Fixed(self.int()?),
            2 => NumberFormat::Scientific(self.int()?),
            3 => NumberFormat::Percent(self.int()?),
            _ => return Err(DecodeError::Malformed),
        })
    }
    pub(super) fn chart_options(&mut self) -> Result<Options, DecodeError> {
        let options = Options {
            version: self.int()?,
            axes: Axes {
                x: self.boolean()?,
                y: self.boolean()?,
                grid: self.boolean()?,
                ticks: self.int()?,
                x_format: self.chart_number_format()?,
                y_format: self.chart_number_format()?,
            },
            cartesian: Cartesian {
                curve: match self.tag()? {
                    0 => Curve::Linear,
                    1 => Curve::Natural,
                    2 => Curve::StepAfter,
                    _ => return Err(DecodeError::Malformed),
                },
                dots: self.boolean()?,
                orientation: match self.tag()? {
                    0 => Orientation::Vertical,
                    1 => Orientation::Horizontal,
                    _ => return Err(DecodeError::Malformed),
                },
                bar_width: self.float()?,
            },
            pie: Pie {
                inner_radius: self.float()?,
                pad_angle: self.float()?,
                labels: self.boolean()?,
            },
            radar: Radar {
                levels: self.int()?,
                dots: self.boolean()?,
                labels: self.boolean()?,
            },
            candlestick: Candlestick {
                body_width: self.float()?,
            },
            sankey: Sankey {
                node_width: self.float()?,
                node_padding: self.float()?,
                alignment: match self.tag()? {
                    0 => Alignment::Left,
                    1 => Alignment::Right,
                    2 => Alignment::Center,
                    3 => Alignment::Justify,
                    _ => return Err(DecodeError::Malformed),
                },
                scale: match self.tag()? {
                    0 => FlowScale::Linear,
                    1 => FlowScale::Sqrt,
                    _ => return Err(DecodeError::Malformed),
                },
                iterations: self.int()?,
                labels: self.boolean()?,
            },
        };
        if options.is_valid() {
            Ok(options)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
pub fn decode_chart_options(bytes: &[u8]) -> Result<Options, DecodeError> {
    if bytes.len() > 256 {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(Cursor::new(bytes));
    let options = d.chart_options()?;
    if d.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(options)
}
