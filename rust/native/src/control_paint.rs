//! Decorative checkable artwork. Focus, actions and values belong to its parent.
use crate::{
    control_appearance,
    control_geometry::{Geometry, Kind, Mark, Rect},
};
use gpui::{
    AnyElement, Bounds, Corners, Edges, Pixels, StyleRefinement, Window, canvas, div, point,
    prelude::*, px, size,
};
use gpuio_protocol::control_appearance::Config;

pub(crate) fn element(
    kind: Kind,
    config: &Config,
    checked: bool,
    mixed: bool,
    disabled: bool,
) -> AnyElement {
    let Some(geometry) = Geometry::new(kind, config, checked, mixed) else {
        // Admission rejects this; never send invalid coordinates to the renderer.
        return div().into_any_element();
    };
    let indicator =
        control_appearance::refinement(&config.indicator_style, checked, mixed, disabled);
    let mark = control_appearance::refinement(&config.mark_style, checked, mixed, disabled);
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| paint(geometry, &indicator, &mark, bounds, window),
    )
    .w(px(geometry.width as f32))
    .h(px(geometry.height as f32))
    .flex_shrink_0()
    .into_any_element()
}

fn paint(
    geometry: Geometry,
    indicator: &StyleRefinement,
    mark: &StyleRefinement,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    let finite = [
        bounds.origin.x,
        bounds.origin.y,
        bounds.size.width,
        bounds.size.height,
    ]
    .into_iter()
    .all(|value| f32::from(value).is_finite());
    let clip = bounds.intersect(&window.content_mask().bounds);
    if !finite || clip.size.width <= px(0.) || clip.size.height <= px(0.) {
        return;
    }
    let opacity = indicator.opacity.unwrap_or(1.);
    let foreground = indicator.text.color.unwrap_or(window.text_style().color);
    let mark_color = mark
        .text
        .color
        .unwrap_or(foreground)
        .opacity(opacity * mark.opacity.unwrap_or(1.));
    let length = |value: Option<gpui::AbsoluteLength>, fallback| {
        value.map_or(px(fallback), |value| value.to_pixels(window.rem_size()))
    };
    let radius = geometry.radius as f32;
    let mut outline = gpui::outline(
        bounds,
        indicator
            .border_color
            .unwrap_or(foreground)
            .opacity(opacity),
        Default::default(),
    );
    outline.background = indicator
        .background
        .as_ref()
        .and_then(gpui::Fill::color)
        .unwrap_or_else(|| gpui::rgba(0).into())
        .opacity(opacity);
    outline.border_widths = Edges {
        top: length(indicator.border_widths.top, 1.),
        right: length(indicator.border_widths.right, 1.),
        bottom: length(indicator.border_widths.bottom, 1.),
        left: length(indicator.border_widths.left, 1.),
    };
    outline.corner_radii = Corners {
        top_left: length(indicator.corner_radii.top_left, radius),
        top_right: length(indicator.corner_radii.top_right, radius),
        bottom_left: length(indicator.corner_radii.bottom_left, radius),
        bottom_right: length(indicator.corner_radii.bottom_right, radius),
    }
    .clamp_radii_for_quad_size(bounds.size);
    let translate = |r: Rect| {
        Bounds::new(
            bounds.origin + point(px(r.x as f32), px(r.y as f32)),
            size(px(r.width as f32), px(r.height as f32)),
        )
    };
    window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
        window.paint_quad(outline);
        match geometry.mark {
            Mark::None => (),
            Mark::Check { points, stroke } => {
                let mut path = gpui::PathBuilder::stroke(px(stroke as f32));
                for (index, [x, y]) in points.into_iter().enumerate() {
                    let point = bounds.origin + point(px(x as f32), px(y as f32));
                    if index == 0 {
                        path.move_to(point);
                    } else {
                        path.line_to(point);
                    }
                }
                if let Ok(path) = path.build() {
                    window.paint_path(path, mark_color);
                }
            }
            Mark::Dash(rect) => {
                // GPUI rounds quad edges independently. A subpixel-high dash
                // can otherwise snap to zero height on a 1x display. Keep its
                // center and ensure at least one physical pixel, like a stroke.
                let mut dash = translate(rect);
                let height = dash.size.height.max(px(1. / window.scale_factor()));
                dash.origin.y -= (height - dash.size.height) / 2.;
                dash.size.height = height;
                window.paint_quad(gpui::fill(dash, mark_color));
            }
            Mark::Dot(rect) | Mark::Thumb(rect) => {
                let mut quad = gpui::fill(translate(rect), mark_color);
                quad.corner_radii = px((rect.height / 2.) as f32).into();
                window.paint_quad(quad);
            }
        }
    });
}

#[cfg(all(test, feature = "native-image-tests"))]
mod tests {
    use super::*;
    use gpui::{Context, Render, TestAppContext};
    use gpuio_protocol::v1::{Color, Field, Fill, Style};

    struct Fixture {
        config: Config,
        kind: Kind,
        opacity: f32,
        clipped: bool,
        visible: bool,
        mixed: bool,
    }
    impl Render for Fixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let mut root = div().w(px(500.)).h(px(200.)).opacity(self.opacity);
            if self.clipped {
                root = root.w(px(4.)).h(px(4.)).overflow_hidden();
            }
            if self.visible {
                root = root.child(element(self.kind, &self.config, true, self.mixed, false));
            }
            root
        }
    }

    #[test]
    fn submitted_quads_scale_apply_nested_opacity_and_retire_without_wakes() {
        for kind in [Kind::Radio, Kind::Switch] {
            for size in [8., 18., 128.] {
                let mut app = TestAppContext::single();
                let (view, cx) = app.add_window_view(|_, _| Fixture {
                    kind,
                    opacity: 0.5,
                    clipped: false,
                    visible: true,
                    mixed: false,
                    config: Config {
                        size,
                        switch_width: size * 2.,
                        indicator_style: vec![Style::Fields(vec![
                            Field::Background(Fill::Solid(Color::Rgba(0xff0000ff))),
                            Field::Opacity(0.5),
                            Field::TopLeftRadius(1000.),
                        ])],
                        mark_style: vec![Style::Fields(vec![
                            Field::Foreground(Color::Rgba(0x0000ffff)),
                            Field::Opacity(0.5),
                        ])],
                        ..Config::default()
                    },
                });
                cx.update(|window, cx| window.draw(cx).clear(cx));
                cx.update(|window, cx| {
                    let quads = window.painted_quads();
                    assert_eq!(quads.len(), 2);
                    let scale = window.scale_factor();
                    let background: gpui::Background = gpui::rgba(0xff0000ff).into();
                    let mark: gpui::Background = gpui::rgba(0x0000ffff).into();
                    assert_eq!(quads[0].background, background.opacity(0.25));
                    assert_eq!(quads[1].background, mark.opacity(0.125));
                    assert_eq!(quads[0].bounds.size.height.0, size as f32 * scale);
                    assert_eq!(
                        quads[0].bounds.size.width.0,
                        size as f32 * scale * if kind == Kind::Switch { 2. } else { 1. }
                    );
                    assert_eq!(quads[0].corner_radii.top_left.0, size as f32 * scale / 2.);
                    assert_eq!(window.simulate_next_frame(cx), 0);
                });
                view.update(cx, |view, cx| {
                    view.clipped = true;
                    cx.notify();
                });
                cx.update(|window, cx| {
                    window.draw(cx).clear(cx);
                    assert!(!window.painted_quads().is_empty());
                    for quad in window.painted_quads() {
                        assert!(
                            quad.content_mask.bounds.size.width.0 <= 4. * window.scale_factor()
                        );
                        assert!(
                            quad.content_mask.bounds.size.height.0 <= 4. * window.scale_factor()
                        );
                    }
                });
                view.update(cx, |view, cx| {
                    view.visible = false;
                    cx.notify();
                });
                cx.update(|window, cx| {
                    window.draw(cx).clear(cx);
                    assert!(window.painted_quads().is_empty());
                    assert_eq!(window.simulate_next_frame(cx), 0);
                });
            }
        }
    }

    #[test]
    fn minimum_mixed_checkbox_keeps_a_visible_dash_at_each_display_scale() {
        for scale in [1., 1.25, 1.5, 2., 3.] {
            let mut app = TestAppContext::single();
            let (_, cx) = app.add_window_view(|_, _| Fixture {
                config: Config {
                    size: 8.,
                    indicator_style: vec![Style::Fields(vec![Field::Background(Fill::Solid(
                        Color::Rgba(0xff0000ff),
                    ))])],
                    mark_style: vec![Style::Fields(vec![Field::Foreground(Color::Rgba(
                        0x00ff00ff,
                    ))])],
                    ..Config::default()
                },
                kind: Kind::Checkbox,
                opacity: 1.,
                clipped: false,
                visible: true,
                mixed: true,
            });
            cx.update(|window, cx| {
                window.set_scale_factor(scale);
                window.draw(cx).clear(cx);
                let quads = window.painted_quads();
                assert_eq!(quads.len(), 2, "outline and dash at scale {scale}");
                let dash = quads.last().expect("mixed checkbox dash");
                assert!(dash.bounds.size.width.0 >= 1.);
                assert!(
                    dash.bounds.size.height.0 >= 1.,
                    "invisible dash at scale {scale}"
                );
            });
        }
    }
}
