use super::{DecodeError, Decoder};
use crate::chart_data::*;
use std::io::Cursor;

struct Budget {
    text: usize,
    points: usize,
}
impl Decoder<'_> {
    fn chart_text(&mut self, max: usize, budget: &mut Budget) -> Result<String, DecodeError> {
        let text = self.bounded_text(max.min(budget.text))?;
        budget.text -= text.len();
        Ok(text)
    }
    fn chart_series(&mut self, budget: &mut Budget) -> Result<Series, DecodeError> {
        let id = self.int()?;
        let name = self.chart_text(128, budget)?;
        let count = self.count(budget.points)?;
        budget.points -= count;
        let mut points = Vec::with_capacity(count);
        for _ in 0..count {
            points.push(Point {
                id: self.int()?,
                x: self.float()?,
                y: self.option(|d| d.float())?,
                label: self.chart_text(256, budget)?,
            });
        }
        Ok(Series { id, name, points })
    }
    fn chart_contents(&mut self, budget: &mut Budget) -> Result<Contents, DecodeError> {
        Ok(match self.tag()? {
            0 => Contents::Cartesian(self.list(MAX_SERIES, |d| {
                let tag = d.tag()?;
                if tag > 2 {
                    return Err(DecodeError::Malformed);
                }
                let series = d.chart_series(budget)?;
                Ok(match tag {
                    0 => Layer::Line(series),
                    1 => Layer::Area(series),
                    2 => Layer::Bar(series),
                    _ => unreachable!(),
                })
            })?),
            1 => Contents::Pie(self.list(256, |d| {
                Ok(Slice {
                    id: d.int()?,
                    label: d.chart_text(256, budget)?,
                    value: d.float()?,
                })
            })?),
            2 => Contents::Radar(
                self.list(64, |d| {
                    Ok(RadarAxis {
                        id: d.int()?,
                        label: d.chart_text(256, budget)?,
                        maximum: d.float()?,
                    })
                })?,
                self.list(MAX_SERIES, |d| {
                    Ok(RadarSeries {
                        id: d.int()?,
                        name: d.chart_text(128, budget)?,
                        values: d.list(64, |d| Ok((d.int()?, d.float()?)))?,
                    })
                })?,
            ),
            3 => Contents::Candlestick(self.list(MAX_POINTS, |d| {
                Ok(Candle {
                    id: d.int()?,
                    x: d.float()?,
                    label: d.chart_text(256, budget)?,
                    open_: d.float()?,
                    high: d.float()?,
                    low: d.float()?,
                    close: d.float()?,
                })
            })?),
            4 => Contents::Sankey(
                self.list(256, |d| {
                    Ok(Node {
                        id: d.int()?,
                        label: d.chart_text(256, budget)?,
                    })
                })?,
                self.list(2048, |d| {
                    Ok(Edge {
                        id: d.int()?,
                        source: d.int()?,
                        target: d.int()?,
                        value: d.float()?,
                    })
                })?,
            ),
            5 => Contents::Categorical(
                self.list(MAX_POINTS, |d| {
                    Ok(Category {
                        id: d.int()?,
                        label: d.chart_text(256, budget)?,
                    })
                })?,
                self.list(MAX_SERIES, |d| {
                    let tag = d.tag()?;
                    if tag > 2 {
                        return Err(DecodeError::Malformed);
                    }
                    let id = d.int()?;
                    let name = d.chart_text(128, budget)?;
                    let count = d.count(budget.points)?;
                    budget.points -= count;
                    let mut points = Vec::with_capacity(count);
                    for _ in 0..count {
                        points.push(CategoricalPoint {
                            id: d.int()?,
                            category: d.int()?,
                            value: d.option(|d| d.float())?,
                            label: d.chart_text(256, budget)?,
                        });
                    }
                    let series = CategoricalSeries { id, name, points };
                    Ok(match tag {
                        0 => CategoricalLayer::Line(series),
                        1 => CategoricalLayer::Area(series),
                        2 => CategoricalLayer::Bar(series),
                        _ => unreachable!(),
                    })
                })?,
            ),
            _ => return Err(DecodeError::Malformed),
        })
    }
}
pub fn decode_chart_data(bytes: &[u8]) -> Result<Data, DecodeError> {
    if bytes.len() > MAX_BYTES {
        return Err(DecodeError::LimitExceeded);
    }
    let mut decoder = Decoder(Cursor::new(bytes));
    let version = decoder.int()?;
    if version != 3 {
        return Err(DecodeError::Malformed);
    }
    let contents = decoder.chart_contents(&mut Budget {
        text: MAX_TEXT_BYTES,
        points: MAX_POINTS,
    })?;
    let bar_backgrounds = decoder.list(MAX_POINTS, |d| {
        Ok(BarBackground {
            series: d.int()?,
            datum: d.int()?,
            brush: d.chart_brush()?,
        })
    })?;
    let bar_baselines = decoder.list(MAX_POINTS, |d| {
        Ok(BarBaseline {
            series: d.int()?,
            datum: d.int()?,
            baseline: d.float()?,
        })
    })?;
    if decoder.remaining() != 0 {
        return Err(DecodeError::Malformed);
    }
    let data = Data {
        version,
        contents,
        bar_backgrounds,
        bar_baselines,
    };
    data.validate().map_err(|error| match error {
        ValidationError::InvalidData => DecodeError::Malformed,
        ValidationError::LimitExceeded => DecodeError::LimitExceeded,
    })?;
    Ok(data)
}
