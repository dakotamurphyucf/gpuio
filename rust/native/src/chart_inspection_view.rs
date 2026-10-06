//! Native-only inspection elements. Positions use prepared geometry; no source
//! upload, application effect or asynchronous round trip is needed for hover.
use crate::{
    chart_details::Details, chart_geometry::Point, chart_jobs::Ready, chart_presentation::Frame,
};
use gpui::StatefulInteractiveElement;
use gpui::{AnyElement, InteractiveElement, IntoElement, ParentElement, Styled, div, px};
use gpuio_protocol::chart_inspection::{Axis, Card, Pattern, Placement};

#[derive(Debug)]
struct CardBox {
    left: f64,
    top: Option<f64>,
    bottom: Option<f64>,
    width: f64,
    max_height: f64,
}
fn card_box(frame: Frame, anchor: Point, card: Card) -> CardBox {
    let inset = 8_f64.min(frame.width / 2.);
    let top_edge = 40_f64.min(frame.height);
    let bottom_edge = (frame.legend.y - 8.).max(top_edge).min(frame.height);
    let width = card.width.min((frame.width - 2. * inset).max(0.));
    if card.placement == Placement::Corner {
        return CardBox {
            left: (frame.width - inset - width).max(0.),
            top: Some(top_edge),
            bottom: None,
            width,
            max_height: (bottom_edge - top_edge).max(0.),
        };
    }
    let x = (frame.plot.x + anchor.x).clamp(inset, frame.width - inset);
    let y = (frame.plot.y + anchor.y).clamp(top_edge, bottom_edge);
    let left = if x + card.gap + width <= frame.width - inset {
        x + card.gap
    } else {
        (x - card.gap - width).max(inset)
    };
    let below = (bottom_edge - y - card.gap).max(0.);
    let above = (y - card.gap - top_edge).max(0.);
    if below >= above {
        CardBox {
            left,
            top: Some((y + card.gap).min(bottom_edge)),
            bottom: None,
            width,
            max_height: below,
        }
    } else {
        CardBox {
            left,
            top: None,
            bottom: Some((frame.height - y + card.gap).min(frame.height - top_edge)),
            width,
            max_height: above,
        }
    }
}

pub(super) fn overlay(ready: &Ready, frame: Frame, details: Details, selected: bool) -> AnyElement {
    let style = &ready.config.style;
    let inspection = *style.inspection;
    let selection = style.selection_color as u32;
    // The owning chart already clips the entire overlay.
    let mut result = div().absolute().top_0().left_0().size_full();
    let cross = inspection.crosshair;
    let anchor = Point {
        x: details.anchor.x.clamp(0., frame.plot.width),
        y: details.anchor.y.clamp(0., frame.plot.height),
    };
    if cross.axis != Axis::Off {
        let color = gpui::rgba(cross.color.map_or(selection, |c| c as u32));
        let mut lines = div()
            .absolute()
            .left(px(frame.plot.x as f32))
            .top(px(frame.plot.y as f32))
            .w(px(frame.plot.width as f32))
            .h(px(frame.plot.height as f32))
            .overflow_hidden();
        let thickness = px(cross.thickness as f32);
        if matches!(cross.axis, Axis::Vertical | Axis::Both) {
            let line = div()
                .absolute()
                .left(px((anchor.x - cross.thickness / 2.)
                    .clamp(0., (frame.plot.width - cross.thickness).max(0.))
                    as f32))
                .top_0()
                .h_full();
            lines = lines.child(if cross.pattern == Pattern::Dashed {
                line.w(thickness)
                    .border_l(thickness)
                    .border_dashed()
                    .border_color(color)
            } else {
                line.w(thickness).bg(color)
            });
        }
        if matches!(cross.axis, Axis::Horizontal | Axis::Both) {
            let line = div()
                .absolute()
                .top(px((anchor.y - cross.thickness / 2.)
                    .clamp(0., (frame.plot.height - cross.thickness).max(0.))
                    as f32))
                .left_0()
                .w_full();
            lines = lines.child(if cross.pattern == Pattern::Dashed {
                line.h(thickness)
                    .border_t(thickness)
                    .border_dashed()
                    .border_color(color)
            } else {
                line.h(thickness).bg(color)
            });
        }
        result = result.child(lines);
    }
    let marker = inspection.marker;
    if marker.visible {
        let fill = marker.fill.map_or(selection, |c| c as u32);
        let stroke = marker.stroke.map_or(selection, |c| c as u32);
        let mut element = div()
            .absolute()
            .left(px((frame.plot.x + anchor.x - marker.size / 2.) as f32))
            .top(px((frame.plot.y + anchor.y - marker.size / 2.) as f32))
            .size(px(marker.size as f32))
            .rounded_full()
            .bg(gpui::rgba(fill))
            .border(px(marker.stroke_width as f32))
            .border_color(gpui::rgba(stroke))
            .text_color(gpui::rgba(crate::chart_presentation::label_backing(fill)))
            .text_size(px(marker.size.min(12.) as f32))
            .line_height(px(marker.size as f32))
            .text_center();
        if marker.status {
            element = element.child(if selected { "✓" } else { "○" });
        }
        result = result.child(element);
    }
    let config = inspection.card;
    if config.visible {
        let box_ = card_box(frame, anchor, config);
        let color = config
            .text_color
            .map_or(style.label_color as u32, |c| c as u32);
        let background = config
            .background
            .map_or(crate::chart_presentation::label_backing(color), |c| {
                c as u32
            });
        let mut card = div()
            .id("gpuio-chart-details")
            .role(gpui::Role::Group)
            .aria_label(format!("{}: {}", details.title, details.text))
            .absolute()
            .left(px(box_.left as f32))
            .w(px(box_.width as f32))
            .max_h(px(box_.max_height as f32))
            .overflow_hidden()
            .p(px(config.padding as f32))
            .rounded(px(config.radius as f32))
            .bg(gpui::rgba(background))
            .text_color(gpui::rgba(color))
            .border(px(config.border_width as f32))
            .border_color(gpui::rgba(config.border_color.map_or(color, |c| c as u32)))
            .text_size(px(config.font_size as f32))
            .line_height(px(config.line_height as f32));
        if let Some(top) = box_.top {
            card = card.top(px(top as f32));
        }
        if let Some(bottom) = box_.bottom {
            card = card.bottom(px(bottom as f32));
        }
        if config.title {
            card = card.child(
                div()
                    .id("gpuio-chart-detail-title")
                    .role(gpui::Role::Label)
                    .aria_label(details.title.clone())
                    .text_ellipsis()
                    .child(details.title),
            );
        }
        if config.values {
            card = card.child(
                div()
                    .id("gpuio-chart-detail-values")
                    .role(gpui::Role::Label)
                    .aria_label(details.text.clone())
                    .child(details.text),
            );
        }
        result = result.child(card);
    }
    result.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chart_presentation::Rect;
    fn frame() -> Frame {
        Frame {
            width: 400.,
            height: 300.,
            plot: Rect {
                x: 40.,
                y: 50.,
                width: 320.,
                height: 180.,
            },
            legend: Rect {
                x: 0.,
                y: 240.,
                width: 400.,
                height: 24.,
            },
            legend_columns: 2,
            horizontal: false,
        }
    }
    #[test]
    fn anchored_card_flips_at_plot_edges_and_reserves_real_content_space() {
        let card = Card {
            placement: Placement::Anchor,
            ..Default::default()
        };
        let first = card_box(frame(), Point { x: 0., y: 0. }, card);
        assert_eq!(
            (
                first.left,
                first.top,
                first.bottom,
                first.width,
                first.max_height
            ),
            (48., Some(58.), None, 280., 174.)
        );
        let last = card_box(frame(), Point { x: 320., y: 180. }, card);
        assert_eq!(
            (
                last.left,
                last.top,
                last.bottom,
                last.width,
                last.max_height
            ),
            (72., None, Some(78.), 280., 182.)
        );
        let corner = card_box(frame(), Point { x: 320., y: 180. }, Card::default());
        assert_eq!(
            (
                corner.left,
                corner.top,
                corner.bottom,
                corner.width,
                corner.max_height
            ),
            (112., Some(40.), None, 280., 192.)
        );
    }
    #[test]
    fn all_anchor_positions_stay_inside_small_or_large_chart_bounds() {
        for width in [1., 16., 80., 400., 2048.] {
            for height in [1., 30., 80., 300., 2048.] {
                let mut frame = frame();
                frame.width = width;
                frame.height = height;
                frame.plot = Rect {
                    x: 0.,
                    y: 0.,
                    width,
                    height,
                };
                frame.legend.y = height;
                for placement in [Placement::Corner, Placement::Anchor] {
                    for x in [-100., 0., width / 2., width, width + 100.] {
                        for y in [-100., 0., height / 2., height, height + 100.] {
                            let b = card_box(
                                frame,
                                Point { x, y },
                                Card {
                                    placement,
                                    gap: 64.,
                                    width: 480.,
                                    ..Default::default()
                                },
                            );
                            assert!(b.left >= 0. && b.left + b.width <= width);
                            assert!(b.max_height >= 0. && b.max_height <= height);
                            if let Some(top) = b.top {
                                assert!(top >= 0. && top + b.max_height <= height);
                            }
                            if let Some(bottom) = b.bottom {
                                assert!(bottom >= 0. && bottom + b.max_height <= height);
                            }
                            assert_ne!(b.top.is_some(), b.bottom.is_some());
                        }
                    }
                }
            }
        }
    }
}
