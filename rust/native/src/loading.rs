//! Bounded native paint: one placeholder or twelve radial spinner strokes.
#[cfg(feature = "native-tests")]
use gpui::Pixels;
use gpui::{Animation, AnimationExt, Bounds, canvas, div, point, prelude::*, px, size};
use gpuio_protocol::loading::{Config, Kind};
use std::time::Duration;

#[cfg(feature = "native-tests")]
#[derive(Clone, Copy, Default)]
pub(super) struct Paint {
    pub bounds: Bounds<Pixels>,
    pub color: gpui::Hsla,
    pub phase: f32,
    pub corners: gpui::Corners<Pixels>,
    pub count: u64,
}
#[cfg(feature = "native-tests")]
pub(super) type Probe = std::rc::Rc<std::cell::Cell<Paint>>;

fn artwork(
    kind: Kind,
    phase: f32,
    corners: super::image_corners::Shared,
    #[cfg(feature = "native-tests")] probe: Probe,
) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let color = window.text_style().color;
            #[cfg(feature = "native-tests")]
            probe.set(Paint {
                bounds,
                color,
                phase,
                corners: corners.get(),
                count: probe.get().count + 1,
            });
            if bounds.size.width <= px(0.) || bounds.size.height <= px(0.) {
                return;
            }
            match kind {
                Kind::Skeleton => {
                    let opacity = 0.5 + 0.25 * (phase * std::f32::consts::TAU).cos();
                    let mut quad = gpui::fill(bounds, color.opacity(opacity));
                    quad.corner_radii = corners.get();
                    window.paint_quad(quad);
                }
                Kind::Shimmer => {
                    let mut base = gpui::fill(bounds, color.opacity(0.2));
                    base.corner_radii = corners.get();
                    window.paint_quad(base);
                    let half = bounds.size.width * 0.25;
                    let center = bounds.left() - half + (bounds.size.width + half * 2.) * phase;
                    for (x, from, to) in [(center - half, 0., 0.6), (center, 0.6, 0.)] {
                        let clip = bounds.intersect(&Bounds::new(
                            point(x, bounds.top()),
                            size(half, bounds.size.height),
                        ));
                        window.with_content_mask(
                            Some(gpui::ContentMask { bounds: clip }),
                            |window| {
                                let mut quad = gpui::fill(
                                    bounds,
                                    gpui::linear_gradient(
                                        90.,
                                        gpui::linear_color_stop(
                                            color.opacity(from),
                                            (x - bounds.left()) / bounds.size.width,
                                        ),
                                        gpui::linear_color_stop(
                                            color.opacity(to),
                                            (x + half - bounds.left()) / bounds.size.width,
                                        ),
                                    ),
                                );
                                quad.corner_radii = corners.get();
                                window.paint_quad(quad);
                            },
                        );
                    }
                }

                Kind::Spinner => {
                    let diameter = bounds.size.width.min(bounds.size.height);
                    let radius = diameter * 0.38;
                    for index in 0..12 {
                        let angle = index as f32 * std::f32::consts::TAU / 12.
                            - std::f32::consts::FRAC_PI_2;
                        let vector = point(radius * angle.cos(), radius * angle.sin());
                        let mut path = gpui::PathBuilder::stroke(diameter * 0.09);
                        path.move_to(bounds.center() + vector * 0.55);
                        path.line_to(bounds.center() + vector);
                        if let Ok(path) = path.build() {
                            let distance = (index as f32 / 12. - phase).rem_euclid(1.);
                            window.paint_path(path, color.opacity(0.2 + 0.8 * distance));
                        }
                    }
                }
            }
        },
    )
    .size_full()
}

pub(super) fn indicator(
    config: &Config,
    identity: u64,
    reduced: bool,
    corners: super::image_corners::Shared,
    #[cfg(feature = "native-tests")] probe: Probe,
) -> gpui::AnyElement {
    let element = div().absolute().size_full();
    let kind = config.kind;
    if !config.animated || reduced {
        // Center the static highlight; keep skeleton and spinner fully recognizable.
        let phase = if kind == Kind::Shimmer { 0.5 } else { 0. };
        element
            .child(artwork(
                kind,
                phase,
                corners,
                #[cfg(feature = "native-tests")]
                probe,
            ))
            .into_any_element()
    } else {
        element
            .with_animation(
                ("loading-cycle", identity),
                Animation::new(Duration::from_millis(config.period_ms as u64)).repeat(),
                move |element, phase| {
                    element.child(artwork(
                        kind,
                        phase,
                        corners.clone(),
                        #[cfg(feature = "native-tests")]
                        probe.clone(),
                    ))
                },
            )
            .into_any_element()
    }
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "loading_lifecycle_test.rs"]
mod lifecycle_test;
