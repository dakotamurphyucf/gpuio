//! Axis captions stay bounded by the view body; fonts never run OCaml callbacks.
use super::*;
use crate::chart_geometry::axis_presentation::{self, AxisLabel};
use gpuio_protocol::chart_axis::LabelAlign;

pub(super) fn gutters(config: &Config, transposed: bool) -> (f64, f64, f64, f64) {
    let (mut left, mut right, mut top, mut bottom) = (0_f64, 0_f64, 32_f64, 0_f64);
    for (axis, visible, horizontal) in [
        (&config.style.x_axis, config.options.axes.x, !transposed),
        (&config.style.y_axis, config.options.axes.y, transposed),
    ] {
        if !visible || !axis.labels {
            continue;
        }
        let before = axis_presentation::before(axis, horizontal);
        let gap = axis_presentation::gap(axis, horizontal);
        let font = axis
            .ticks
            .iter()
            .flatten()
            .filter_map(|t| t.font_size)
            .fold(axis.font_size, f64::max);
        if horizontal {
            let extent = (font + 4.).max(TEXT_HEIGHT) + gap + 4.;
            if before {
                top = top.max(32. + extent);
            } else {
                bottom = bottom.max(extent);
            }
            // Endpoint captions need some space on the orthogonal axis too.
            right = right.max(28.);
        } else {
            let extent = axis.label_width.unwrap_or(68.) + gap;
            if before {
                left = left.max(extent);
            } else {
                right = right.max(extent);
            }
            top = top.max(32. + (font + 4.).max(TEXT_HEIGHT) / 2. + 3.);
            if axis.as_ref() != &gpuio_protocol::chart_axis::Axis::default() {
                // A custom horizontal axis may sit above or inside the plot,
                // leaving no bottom gutter for this vertical endpoint caption.
                bottom = bottom.max((font + 4.).max(TEXT_HEIGHT) / 2. + 3.);
            }
        }
    }
    (left, right, top, bottom)
}

pub(super) fn label(frame: &Frame, x: f64, y: f64, axis: AxisLabel) -> Placement {
    let align = match axis.align {
        LabelAlign::Left => Align::Left,
        LabelAlign::Center => Align::Center,
        LabelAlign::Right => Align::Right,
        LabelAlign::Auto if axis.horizontal => Align::Center,
        LabelAlign::Auto if axis.before => Align::Right,
        LabelAlign::Auto => Align::Left,
    };
    let anchor = if axis.horizontal {
        x
    } else if axis.before {
        x - axis.gap
    } else {
        x + axis.gap
    };
    let available = match align {
        Align::Left => frame.width - anchor,
        Align::Right => anchor,
        Align::Center => frame.width,
    }
    .max(0.);
    let width = axis
        .width
        .unwrap_or(if axis.horizontal { 80. } else { 68. })
        .min(available)
        .min(frame.width);
    let height = (axis.font_size + 4.).max(TEXT_HEIGHT);
    let top = if axis.horizontal {
        if axis.before {
            y - axis.gap - height
        } else {
            y + axis.gap
        }
    } else {
        y - height / 2.
    };
    let left = match align {
        Align::Left => anchor,
        Align::Center => anchor - width / 2.,
        Align::Right => anchor - width,
    };
    // Intersect vertically instead of moving a label across its axis or into
    // the data-control header when the view is too small for the requested gap.
    let header = 32_f64.min(frame.plot.y);
    let clipped_top = top.clamp(header, frame.legend.y);
    let clipped_bottom = (top + height).clamp(header, frame.legend.y);
    Placement {
        rect: Rect {
            x: left.clamp(0., frame.width - width),
            y: clipped_top,
            width,
            height: (clipped_bottom - clipped_top).max(0.),
        },
        align,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame() -> Frame {
        Frame {
            width: 800.,
            height: 400.,
            plot: Rect {
                x: 100.,
                y: 80.,
                width: 600.,
                height: 240.,
            },
            legend: Rect {
                x: 0.,
                y: 360.,
                width: 800.,
                height: 40.,
            },
            legend_columns: 4,
            horizontal: false,
        }
    }
    fn style(horizontal: bool, before: bool) -> AxisLabel {
        AxisLabel {
            horizontal,
            before,
            gap: 6.,
            width: Some(60.),
            font_size: 20.,
            color: None,
            align: LabelAlign::Auto,
        }
    }
    #[test]
    fn side_and_alignment_control_actual_caption_rectangles() {
        let f = frame();
        for (horizontal, before, x, y, align) in [
            (true, true, 170., 70., Align::Center),
            (true, false, 170., 106., Align::Center),
            (false, true, 134., 88., Align::Right),
            (false, false, 206., 88., Align::Left),
        ] {
            let p = label(&f, 200., 100., style(horizontal, before));
            assert_eq!(
                p,
                Placement {
                    rect: Rect {
                        x,
                        y,
                        width: 60.,
                        height: 24.
                    },
                    align
                }
            );
        }
        let mut a = style(true, false);
        a.align = LabelAlign::Right;
        assert_eq!(label(&f, 200., 100., a).rect.x, 140.);
        a.align = LabelAlign::Left;
        assert_eq!(label(&f, 200., 100., a).rect.x, 200.);
    }
    #[test]
    fn captions_cannot_cover_the_header_or_legend() {
        let f = frame();
        assert_eq!(label(&f, 0., 10., style(true, true)).rect.height, 0.);
        assert_eq!(label(&f, 800., 360., style(true, false)).rect.height, 0.);
        let p = label(&f, 8., 40., style(false, true));
        assert_eq!(p.rect.width, 2.);
        assert_eq!(p.rect.y, 32.);
        assert_eq!(p.rect.height, 20.);
    }
    #[test]
    fn default_gutters_are_preserved_and_custom_sides_remain_bounded() {
        let mut config = Config {
            version: -2,
            source: None,
            label: "Axes".into(),
            options: Default::default(),
            sampling: Default::default(),
            style: Default::default(),
            radar_labels: vec![],
            inspection_content: vec![],
            legend: true,
            disabled: false,
        };
        let source = Data {
            version: 2,
            bar_backgrounds: vec![],
            contents: Contents::Cartesian(vec![]),
        };
        assert_eq!(gutters(&config, false), (76., 28., 44., 28.));
        assert_eq!(gutters(&config, true), (76., 28., 44., 28.));
        config.style.x_axis.position = Some(0.);
        config.style.x_axis.font_size = 32.;
        config.style.y_axis.position = Some(1.);
        config.style.y_axis.label_width = Some(120.);
        assert_eq!(gutters(&config, false), (0., 128., 78., 12.));
        for (width, height) in [(800., 400.), (4., 2.), (0.1, 0.1)] {
            let frame = Frame::new(width, height, &source, &config);
            assert!(frame.plot.width > 0. && frame.plot.height > 0.);
            assert!(frame.plot.x >= 0. && frame.plot.y >= 0.);
            assert!(frame.plot.x + frame.plot.width <= width);
            assert!(frame.plot.y + frame.plot.height <= height);
        }
        config.style.x_axis.labels = false;
        config.style.y_axis.labels = false;
        assert_eq!(gutters(&config, false), (0., 0., 32., 0.));
    }

    #[test]
    fn custom_vertical_endpoints_have_room_above_the_legend() {
        let mut config = Config {
            version: -2,
            source: None,
            label: "Endpoints".into(),
            options: Default::default(),
            sampling: Default::default(),
            style: Default::default(),
            radar_labels: vec![],
            inspection_content: vec![],
            legend: true,
            disabled: false,
        };
        config.style.x_axis.position = Some(0.5);
        config.style.y_axis.position = Some(0.5);
        config.style.y_axis.font_size = 24.;
        let source = Data {
            version: 2,
            bar_backgrounds: vec![],
            contents: Contents::Cartesian(vec![]),
        };
        let frame = Frame::new(600., 400., &source, &config);
        let mut axis = style(false, true);
        axis.font_size = 24.;
        let p = label(
            &frame,
            frame.plot.x + frame.plot.width / 2.,
            frame.plot.y + frame.plot.height,
            axis,
        );
        assert_eq!(
            p.rect.height, 28.,
            "bottom endpoint caption must retain its full line"
        );
        assert!(p.rect.y + p.rect.height <= frame.legend.y);
    }
}
