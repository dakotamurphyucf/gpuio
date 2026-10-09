use super::{DecodeError, Decoder};
use crate::{chart_axis::*, chart_grid::Grid};

impl Decoder<'_> {
    fn chart_tick_position(&mut self) -> Result<TickPosition, DecodeError> {
        Ok(match self.tag()? {
            0 => TickPosition::Value(self.float()?),
            1 => TickPosition::Category(self.int()?),
            2 => TickPosition::Fraction(self.float()?),
            _ => return Err(DecodeError::Malformed),
        })
    }
    fn chart_label_align(&mut self) -> Result<LabelAlign, DecodeError> {
        Ok(match self.tag()? {
            0 => LabelAlign::Auto,
            1 => LabelAlign::Left,
            2 => LabelAlign::Center,
            3 => LabelAlign::Right,
            _ => return Err(DecodeError::Malformed),
        })
    }
    pub(super) fn chart_axis(&mut self) -> Result<Axis, DecodeError> {
        let axis = Axis {
            line: self.boolean()?,
            labels: self.boolean()?,
            position: self.option(|d| d.float())?,
            ticks: self.option(|d| {
                d.list(MAX_TICKS, |d| {
                    Ok(Tick {
                        position: d.chart_tick_position()?,
                        text: d.bounded_text(256)?,
                        color: d.option(|d| d.int())?,
                        font_size: d.option(|d| d.float())?,
                        align: d.chart_label_align()?,
                    })
                })
            })?,
            tick_count: self.option(|d| d.int())?,
            label_side: match self.tag()? {
                0 => LabelSide::Auto,
                1 => LabelSide::Before,
                2 => LabelSide::After,
                _ => return Err(DecodeError::Malformed),
            },
            label_align: self.chart_label_align()?,
            label_gap: self.option(|d| d.float())?,
            label_width: self.option(|d| d.float())?,
            font_size: self.float()?,
            line_width: self.float()?,
            line_color: self.option(|d| d.int())?,
            label_color: self.option(|d| d.int())?,
        };
        if axis.is_valid() {
            Ok(axis)
        } else {
            Err(DecodeError::Malformed)
        }
    }
    pub(super) fn chart_grid(&mut self) -> Result<Grid, DecodeError> {
        let grid = Grid {
            x: self.option(|d| d.list(MAX_TICKS, |d| d.chart_tick_position()))?,
            y: self.option(|d| d.list(MAX_TICKS, |d| d.chart_tick_position()))?,
            dashes: self.list(16, |d| d.float())?,
            width: self.float()?,
            color: self.option(|d| d.int())?,
        };
        if grid.is_valid() {
            Ok(grid)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}
