use super::{DecodeError, Decoder};
use crate::{chart_appearance::*, chart_options::Curve};
use std::io::Cursor;

impl Decoder<'_> {
    fn chart_brush(&mut self) -> Result<Brush, DecodeError> {
        Ok(match self.tag()? {
            0 => Brush::Solid(self.int()?),
            1 => Brush::Linear {
                oklab: self.boolean()?,
                angle: self.float()?,
                from: self.int()?,
                start: self.float()?,
                to: self.int()?,
                stop: self.float()?,
            },
            2 => Brush::PatternSlash(self.int()?, self.float()?, self.float()?),
            3 => Brush::Checkerboard(self.int()?, self.float()?),
            _ => return Err(DecodeError::Malformed),
        })
    }
    fn chart_mark_bar(&mut self) -> Result<Bar, DecodeError> {
        Ok(Bar {
            fill: self.option(|d| {
                Ok(match d.tag()? {
                    0 => BarFill::Background(d.chart_brush()?),
                    1 => BarFill::BaseToTip(d.int()?, d.int()?),
                    2 => BarFill::Domain(d.int()?, d.int()?),
                    3 => BarFill::Values(d.float()?, d.int()?, d.float()?, d.int()?),
                    _ => return Err(DecodeError::Malformed),
                })
            })?,
            corners: self.option(|d| {
                Ok(Corners {
                    top_left: d.float()?,
                    top_right: d.float()?,
                    bottom_right: d.float()?,
                    bottom_left: d.float()?,
                })
            })?,
        })
    }
    fn chart_mark_marker(&mut self) -> Result<Marker, DecodeError> {
        Ok(Marker {
            visible: self.option(|d| d.boolean())?,
            radius: self.option(|d| d.float())?,
            fill: self.option(|d| d.int())?,
            stroke: self.option(|d| d.int())?,
            stroke_width: self.option(|d| d.float())?,
        })
    }
    fn chart_mark_path(&mut self) -> Result<Path, DecodeError> {
        Ok(Path {
            stroke: self.option(|d| {
                Ok(Stroke {
                    visible: d.boolean()?,
                    width: d.option(|d| d.float())?,
                    brush: d.chart_brush()?,
                })
            })?,
            fill: self.option(|d| d.chart_brush())?,
            curve: self.option(|d| {
                Ok(match d.tag()? {
                    0 => Curve::Linear,
                    1 => Curve::Natural,
                    2 => Curve::StepAfter,
                    _ => return Err(DecodeError::Malformed),
                })
            })?,
        })
    }
    pub(super) fn chart_appearance(&mut self) -> Result<Appearance, DecodeError> {
        let value = Appearance {
            series: self.list(MAX_SERIES, |d| {
                Ok(Series {
                    series: d.int()?,
                    path: d.option(|d| d.chart_mark_path())?,
                    marker: d.option(|d| d.chart_mark_marker())?,
                    bar: d.option(|d| d.chart_mark_bar())?,
                    legend: d.option(|d| d.int())?,
                    area_baseline: d.option(|d| d.float())?,
                })
            })?,
            data: self.list(MAX_DATA, |d| {
                Ok(Datum {
                    series: d.int()?,
                    datum: d.int()?,
                    marker: d.option(|d| d.chart_mark_marker())?,
                    bar: d.option(|d| d.chart_mark_bar())?,
                })
            })?,
            aggregates: match self.tag()? {
                0 => Aggregates::InheritSeries,
                1 => Aggregates::Uniform,
                _ => return Err(DecodeError::Malformed),
            },
        };
        if value.is_valid() {
            Ok(value)
        } else {
            Err(DecodeError::Malformed)
        }
    }
}

pub fn decode_chart_appearance(bytes: &[u8]) -> Result<Appearance, DecodeError> {
    if bytes.len() > MAX_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut d = Decoder(Cursor::new(bytes));
    let value = d.chart_appearance()?;
    if d.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    Ok(value)
}
