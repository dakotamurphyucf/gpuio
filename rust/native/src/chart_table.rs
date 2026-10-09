//! Original-data access, independent of plotting, reduction and mesh admission.
//! One bounded row is formatted at a time; never materialize a dataset-sized UI.
use crate::chart_cartesian::{Kind, Layers};
use gpuio_protocol::chart_data::{Contents, Data};

pub(crate) struct Row {
    pub name: String,
    pub value: String,
    pub detail: String,
}
pub(crate) fn count(data: &Data) -> usize {
    match &data.contents {
        Contents::Cartesian(_) | Contents::Categorical(..) => Layers::of(data)
            .unwrap()
            .iter()
            .map(|l| l.points.len())
            .sum(),
        Contents::Pie(values) => values.len(),
        Contents::Radar(axes, series) => axes.len() * series.len(),
        Contents::Candlestick(values) => values.len(),
        Contents::Sankey(nodes, edges) => nodes.len() + edges.len(),
    }
}
/// Input is immutable and validated by the resource store. Access is bounded by
/// the schema's series/axis/flow limits, not by the 100,000-point source length.
/// Display uses round-trippable numeric values, not rounded plot tick formatting.
pub(crate) fn row(data: &Data, mut index: usize) -> Option<Row> {
    Some(match &data.contents {
        Contents::Cartesian(_) | Contents::Categorical(..) => {
            let (layer, point, position) = Layers::of(data)?.iter().find_map(|layer| {
                let series = layer;
                if index < series.points.len() {
                    Some((layer, series.points.get(index)?, index))
                } else {
                    index -= series.points.len();
                    None
                }
            })?;
            let kind = match layer.kind {
                Kind::Line => "Line",
                Kind::Area => "Area",
                Kind::Bar => "Bar",
            };
            let baseline = if layer.kind == Kind::Bar && !data.bar_baselines.is_empty() {
                format!(
                    " · baseline: {}",
                    data.bar_baseline(layer.id, point.id).unwrap_or(0.)
                )
            } else {
                String::new()
            };
            Row {
                name: format!("{kind} · {}", layer.name),
                value: (match &data.contents {
                    Contents::Categorical(categories, _) => {
                        let c = categories.get(position)?;
                        format!(
                            "Category: {} ({}) · value: {}",
                            c.label,
                            c.id,
                            point.y.map_or_else(|| "Missing".into(), |v| v.to_string())
                        )
                    }
                    _ => format!(
                        "x: {} · y: {}",
                        point.x,
                        point.y.map_or_else(|| "Missing".into(), |v| v.to_string())
                    ),
                }) + &baseline,
                detail: format!(
                    "Series {} · datum {}{}",
                    layer.id,
                    point.id,
                    label(point.label)
                ),
            }
        }
        Contents::Pie(values) => {
            let slice = values.get(index)?;
            Row {
                name: slice.label.clone(),
                value: slice.value.to_string(),
                detail: format!("Slice {}", slice.id),
            }
        }
        Contents::Radar(axes, series) => {
            if axes.is_empty() {
                return None;
            }
            let series = series.get(index / axes.len())?;
            let axis = &axes[index % axes.len()];
            let value = series.values.iter().find(|(id, _)| *id == axis.id)?.1;
            Row {
                name: format!("{} · {}", series.name, axis.label),
                value: format!("{value} / {}", axis.maximum),
                detail: format!("Series {} · axis {}", series.id, axis.id),
            }
        }
        Contents::Candlestick(values) => {
            let candle = values.get(index)?;
            Row {
                name: if candle.label.is_empty() {
                    format!("Session at {}", candle.x)
                } else {
                    candle.label.clone()
                },
                value: format!(
                    "Open {} · High {} · Low {} · Close {}",
                    candle.open_, candle.high, candle.low, candle.close
                ),
                detail: format!("Datum {} · x: {}", candle.id, candle.x),
            }
        }
        Contents::Sankey(nodes, edges) => {
            if let Some(node) = nodes.get(index) {
                let incoming: f64 = edges
                    .iter()
                    .filter(|e| e.target == node.id)
                    .fold(0., |total, edge| total + edge.value);
                let outgoing: f64 = edges
                    .iter()
                    .filter(|e| e.source == node.id)
                    .fold(0., |total, edge| total + edge.value);
                Row {
                    name: node.label.clone(),
                    value: format!("Incoming {incoming} · Outgoing {outgoing}"),
                    detail: format!("Node {}", node.id),
                }
            } else {
                let edge = edges.get(index.checked_sub(nodes.len())?)?;
                let source = nodes.iter().find(|n| n.id == edge.source)?;
                let target = nodes.iter().find(|n| n.id == edge.target)?;
                Row {
                    name: format!("{} → {}", source.label, target.label),
                    value: edge.value.to_string(),
                    detail: format!(
                        "Edge {} · source {} · target {}",
                        edge.id, edge.source, edge.target
                    ),
                }
            }
        }
    })
}
fn label(value: &str) -> String {
    if value.is_empty() {
        String::new()
    } else {
        format!(" · {value}")
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Page {
    pub cursor: usize,
    pub start: usize,
    pub end: usize,
    pub size: usize,
}
impl Page {
    pub fn new(cursor: usize, count: usize, height: f64) -> Self {
        // Header, toolbar, footer and padding use 120px; rows are 24px. Tiny charts still
        // choose one original value; the enclosing chart clips undersized content.
        let size = (((height - 120.).max(0.) / 24.) as usize).clamp(1, 10);
        let cursor = cursor.min(count.saturating_sub(1));
        let start = cursor / size * size;
        Self {
            cursor,
            start,
            end: (start + size).min(count),
            size,
        }
    }
    pub fn navigate(self, key: &str, count: usize) -> Option<usize> {
        Some(match key {
            "up" | "left" => self.cursor.saturating_sub(1),
            "down" | "right" => self.cursor.saturating_add(1).min(count.saturating_sub(1)),
            "pageup" => self.cursor.saturating_sub(self.size),
            "pagedown" => self
                .cursor
                .saturating_add(self.size)
                .min(count.saturating_sub(1)),
            "home" => 0,
            "end" => count.saturating_sub(1),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::chart_data::*;
    #[test]
    fn pages_reach_every_original_including_gaps_without_materializing_all_rows() {
        let data = Data {
            version: 3,
            bar_baselines: vec![],
            bar_backgrounds: vec![],
            contents: Contents::Cartesian(vec![Layer::Line(Series {
                id: 9,
                name: "Originals".into(),
                points: (0..100_000)
                    .map(|i| Point {
                        id: 100_000 - i,
                        x: i as f64,
                        y: if i % 2 == 0 { None } else { Some(0.) },
                        label: String::new(),
                    })
                    .collect(),
            })]),
        };
        assert_eq!(count(&data), 100_000);
        let page = Page::new(usize::MAX, count(&data), 330.);
        assert_eq!(page.cursor, 99_999);
        assert!(page.end - page.start <= 10);
        assert_eq!(row(&data, 0).unwrap().value, "x: 0 · y: Missing");
        assert_eq!(row(&data, 99_999).unwrap().detail, "Series 9 · datum 1");
        assert!(row(&data, 100_000).is_none());
        for index in (0..100_000).step_by(page.size) {
            let page = Page::new(index, count(&data), 330.);
            for index in page.start..page.end {
                assert!(row(&data, index).is_some());
            }
        }
        assert_eq!(Page::new(0, 0, 0.).end, 0);
        assert_eq!(Page::new(0, 100_000, 1.).size, 1);
        assert_eq!(Page::new(0, 100_000, 32_768.).size, 10);
    }
    #[test]
    fn all_families_expose_original_values_and_flow_nodes_and_edges() {
        for fixture in include_str!("../../../test/fixtures/chart-v3-data.hex").lines() {
            let (_, hex) = fixture.split_once(' ').unwrap();
            let bytes = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect::<Vec<_>>();
            let data = gpuio_protocol::decode_chart_data(&bytes).unwrap();
            for index in 0..count(&data) {
                let row = row(&data, index).unwrap();
                assert!(!row.name.is_empty() && !row.value.is_empty() && !row.detail.is_empty());
            }
            assert!(row(&data, count(&data)).is_none());
            assert!(row(&data, usize::MAX).is_none());
            if let Contents::Sankey(nodes, edges) = &data.contents {
                assert_eq!(count(&data), nodes.len() + edges.len());
            }
        }
        let pie = Data {
            version: 3,
            bar_baselines: vec![],
            bar_backgrounds: vec![],
            contents: Contents::Pie(vec![Slice {
                id: 7,
                label: "Zero".into(),
                value: 0.,
            }]),
        };
        assert_eq!(row(&pie, 0).unwrap().value, "0");
        let radar = Data {
            version: 3,
            bar_baselines: vec![],
            bar_backgrounds: vec![],
            contents: Contents::Radar(
                vec![RadarAxis {
                    id: 42,
                    label: "Axis".into(),
                    maximum: 1.,
                }],
                vec![RadarSeries {
                    id: 9,
                    name: "Model".into(),
                    values: vec![(42, 0.125)],
                }],
            ),
        };
        assert_eq!(row(&radar, 0).unwrap().value, "0.125 / 1");
    }
    #[test]
    fn isolated_nodes_have_zero_totals_and_do_not_disappear() {
        let data = Data {
            version: 3,
            bar_baselines: vec![],
            bar_backgrounds: vec![],
            contents: Contents::Sankey(
                vec![Node {
                    id: 7,
                    label: "Isolated".into(),
                }],
                vec![],
            ),
        };
        assert_eq!(count(&data), 1);
        assert_eq!(row(&data, 0).unwrap().value, "Incoming 0 · Outgoing 0");
    }
}
