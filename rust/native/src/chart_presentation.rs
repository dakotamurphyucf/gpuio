//! Bounded native text layout around a prepared chart. Coordinates are logical pixels.
use crate::chart_geometry::{Label, LabelKind};
use gpuio_protocol::{
    chart_data::{Contents, Data},
    chart_view::Config,
};

pub(crate) const TEXT_HEIGHT: f64 = 18.;
pub(crate) const LEGEND_ROW: f64 = 24.;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Frame {
    pub width: f64,
    pub height: f64,
    pub plot: Rect,
    pub legend: Rect,
    pub legend_columns: usize,
    pub horizontal: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Align {
    Center,
    Right,
    Left,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placement {
    pub rect: Rect,
    pub align: Align,
}

/// Palette order matches the geometry's layer order, including Sankey nodes.
pub(crate) fn legend(data: &Data) -> Vec<&str> {
    match &data.contents {
        Contents::Cartesian(layers) => layers.iter().map(|l| l.series().name.as_str()).collect(),
        Contents::Pie(slices) => slices.iter().map(|s| s.label.as_str()).collect(),
        Contents::Radar(_, series) => series.iter().map(|s| s.name.as_str()).collect(),
        Contents::Candlestick(_) => vec!["Rise · hollow", "Fall · filled"],
        Contents::Sankey(nodes, _) => nodes.iter().map(|n| n.label.as_str()).collect(),
    }
}
/// A neutral backing for text painted over arbitrary series colors. Keep the
/// caller's resolved foreground; choose black/white backing from its luminance.
pub(crate) fn label_backing(color: u32) -> u32 {
    let r = (color >> 24) & 255;
    let g = (color >> 16) & 255;
    let b = (color >> 8) & 255;
    if 299 * r + 587 * g + 114 * b >= 128_000 {
        0x000000dd
    } else {
        0xffffffee
    }
}
/// Companion controls use resolved chart foreground/selection tokens. Neutral
/// surfaces remain opaque so hidden plot pixels cannot tint table readability.
#[derive(Clone, Copy)]
pub(crate) struct Controls {
    pub foreground: u32,
    pub background: u32,
    pub accent: u32,
    pub active_foreground: u32,
    pub muted: u32,
}
impl Controls {
    pub fn new(style: &gpuio_protocol::chart_style::Style) -> Self {
        let foreground = style.label_color as u32;
        let background = label_backing(foreground) | 255;
        let accent = style.selection_color as u32;
        let blend = |shift: u32| {
            (((foreground >> shift) & 255_u32) + ((background >> shift) & 255_u32)) / 2
        };
        let muted = (blend(24) << 24) | (blend(16) << 16) | (blend(8) << 8) | 255;
        Self {
            foreground,
            background,
            accent,
            active_foreground: label_backing(accent) | 255,
            muted,
        }
    }
}
impl Frame {
    /// Caller supplies positive finite dimensions validated by the paint layout.
    pub fn new(width: f64, height: f64, data: &Data, config: &Config) -> Self {
        let columns = ((width / 180.).floor() as usize).max(1);
        let rows = if config.legend {
            let count = match &data.contents {
                Contents::Cartesian(layers) => layers.len(),
                Contents::Pie(slices) => slices.len(),
                Contents::Radar(_, series) => series.len(),
                Contents::Candlestick(_) => 2,
                Contents::Sankey(nodes, _) => nodes.len(),
            };
            count.div_ceil(columns).min(3)
        } else {
            0
        };
        let legend_height = (rows as f64 * LEGEND_ROW).min(height * 0.3);
        let body_height = height - legend_height;
        let horizontal = matches!(data.contents, Contents::Cartesian(_))
            && config.options.cartesian.orientation.is_horizontal();
        let numeric = matches!(
            data.contents,
            Contents::Cartesian(_) | Contents::Candlestick(_)
        );
        let (left_axis, bottom_axis) = if horizontal {
            (config.options.axes.x, config.options.axes.y)
        } else {
            (config.options.axes.y, config.options.axes.x)
        };
        let radar_labels =
            matches!(data.contents, Contents::Radar(..)) && config.options.radar.labels;
        let left: f64 = if numeric && left_axis {
            76.
        } else if radar_labels {
            64.
        } else {
            0.
        };
        let right: f64 = if radar_labels {
            64.
        } else if numeric && bottom_axis {
            28.
        } else {
            0.
        };
        // Reserve a native data-control header instead of covering axis ticks
        // or making the first/last data points unclickable beneath its button.
        let top: f64 = 32.
            + if (numeric && left_axis) || radar_labels {
                12.
            } else {
                0.
            };
        let bottom: f64 = if numeric && bottom_axis {
            28.
        } else if radar_labels {
            24.
        } else {
            0.
        };
        // Bound gutters proportionally so even a tiny view has a positive plot.
        let left = left.min(width * 0.3);
        let right = right.min(width * 0.3);
        let top = top.min(body_height * 0.2);
        let bottom = bottom.min(body_height * 0.3);
        Self {
            width,
            height,
            plot: Rect {
                x: left,
                y: top,
                width: width - left - right,
                height: body_height - top - bottom,
            },
            legend: Rect {
                x: 0.,
                y: body_height,
                width,
                height: legend_height,
            },
            legend_columns: columns,
            horizontal,
        }
    }
    pub fn label(&self, label: &Label) -> Placement {
        let x = self.plot.x + label.position.x;
        let y = self.plot.y + label.position.y;
        let left_axis = matches!(label.kind, LabelKind::Y) && !self.horizontal
            || matches!(label.kind, LabelKind::X) && self.horizontal;
        let bottom_axis = matches!(label.kind, LabelKind::X | LabelKind::Y) && !left_axis;
        let (width, x, y, align) = if left_axis {
            (
                (self.plot.x - 8.).max(0.),
                0.,
                y - TEXT_HEIGHT / 2.,
                Align::Right,
            )
        } else if bottom_axis {
            (80., x - 40., y + 6., Align::Center)
        } else if matches!(label.kind, LabelKind::Series(_)) {
            (22., x - 11., y - TEXT_HEIGHT / 2., Align::Center)
        } else if matches!(label.kind, LabelKind::Flow) {
            if x > self.plot.x + self.plot.width / 2. {
                (140., x - 6. - 140., y - TEXT_HEIGHT / 2., Align::Right)
            } else {
                (140., x + 6., y - TEXT_HEIGHT / 2., Align::Left)
            }
        } else {
            (120., x - 60., y - TEXT_HEIGHT / 2., Align::Center)
        };
        let width = width.min(self.width);
        let height = TEXT_HEIGHT.min(self.legend.y);
        Placement {
            rect: Rect {
                x: x.clamp(0., self.width - width),
                y: y.clamp(0., self.legend.y - height),
                width,
                height,
            },
            align,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_geometry::Point;
    use gpuio_protocol::chart_options::Orientation;
    fn config() -> Config {
        Config {
            source: None,
            label: "Chart".into(),
            options: Default::default(),
            sampling: Default::default(),
            style: Default::default(),
            legend: true,
            disabled: false,
        }
    }
    #[test]
    fn axes_follow_orientation_and_tiny_views_stay_bounded() {
        let data = Data {
            version: 1,
            contents: Contents::Cartesian(vec![]),
        };
        let mut config = config();
        for horizontal in [false, true] {
            config.options.cartesian.orientation = if horizontal {
                Orientation::Horizontal
            } else {
                Orientation::Vertical
            };
            for (width, height) in [(800., 400.), (1., 1.)] {
                let frame = Frame::new(width, height, &data, &config);
                assert!(frame.plot.width > 0. && frame.plot.height > 0.);
                assert_eq!(frame.legend.height, 0.);
                for kind in [
                    LabelKind::X,
                    LabelKind::Y,
                    LabelKind::Radial,
                    LabelKind::Flow,
                    LabelKind::Series(0),
                ] {
                    let placement = frame.label(&Label {
                        position: Point {
                            x: frame.plot.width,
                            y: frame.plot.height,
                        },
                        text: "long".repeat(64),
                        kind,
                    });
                    assert!(placement.rect.x >= 0. && placement.rect.y >= 0.);
                    assert!(placement.rect.x + placement.rect.width <= width);
                    assert!(placement.rect.y + placement.rect.height <= height);
                }
                let y = frame.label(&Label {
                    position: Point { x: 0., y: 0. },
                    text: "0".into(),
                    kind: LabelKind::Y,
                });
                assert_eq!(
                    y.align,
                    if horizontal {
                        Align::Center
                    } else {
                        Align::Right
                    }
                );
            }
        }
    }
    #[test]
    fn label_backing_contrasts_with_light_and_dark_foregrounds() {
        assert_eq!(label_backing(0x94a3b8ff), 0x000000dd);
        assert_eq!(label_backing(0x102030ff), 0xffffffee);
    }
    #[test]
    fn dense_legend_has_three_visible_rows_and_retains_all_names() {
        let data = Data {
            version: 1,
            contents: Contents::Pie(
                (1..=256)
                    .map(|id| gpuio_protocol::chart_data::Slice {
                        id,
                        label: format!("Slice {id}"),
                        value: 1.,
                    })
                    .collect(),
            ),
        };
        let mut config = config();
        let frame = Frame::new(720., 400., &data, &config);
        assert_eq!(frame.legend_columns, 4);
        assert_eq!(frame.legend.height, 72.);
        assert_eq!(legend(&data).len(), 256);
        config.legend = false;
        assert_eq!(Frame::new(720., 400., &data, &config).legend.height, 0.);
    }
    #[test]
    fn controls_follow_resolved_foreground_and_selection_in_both_theme_directions() {
        for (foreground, background, selection, active) in [
            (0xeeeeeeff, 0x000000ff, 0xfafafaff, 0x000000ff),
            (0x102030ff, 0xffffffff, 0x102030ff, 0xffffffff),
        ] {
            let style = gpuio_protocol::chart_style::Style {
                label_color: foreground,
                selection_color: selection,
                ..Default::default()
            };
            let controls = Controls::new(&style);
            assert_eq!(controls.foreground, foreground as u32);
            assert_eq!(controls.background, background);
            assert_eq!(controls.accent, selection as u32);
            assert_eq!(controls.active_foreground, active);
            assert_eq!(controls.muted & 255, 255);
        }
    }
}
