//! Paint-only progress geometry. No semantic mutation, I/O or OCaml callbacks.
use crate::{
    progress_clock::{Driver, Sample, Value},
    progress_geometry as geometry,
};
use gpui::{App, Bounds, Corners, Pixels, Window, px};
use gpuio_protocol::{animation::Easing, progress_presentation::Shape};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Report {
    Skipped,
    /// A visible canvas can have zero fill at the beginning of a transition.
    /// It must still advance toward its nonzero target; root track paints separately.
    Visible {
        sample: Sample,
        paths: usize,
        points: usize,
    },
}

pub fn paint(
    driver: &Driver,
    bounds: Bounds<Pixels>,
    corners: Corners<Pixels>,
    inert: bool,
    window: &mut Window,
    cx: &mut App,
) -> Report {
    let color = window.text_style().color;
    let scale = window.scale_factor();
    let clip = bounds.intersect(&window.content_mask().bounds);
    let finite = [
        bounds.left(),
        bounds.top(),
        bounds.right(),
        bounds.bottom(),
        bounds.center().x,
        bounds.center().y,
        bounds.size.width,
        bounds.size.height,
    ]
    .into_iter()
    .all(|v| f32::from(v).is_finite() && (f32::from(v) * scale).is_finite());
    if !finite
        || !scale.is_finite()
        || scale <= 0.
        || clip.size.width <= px(0.)
        || clip.size.height <= px(0.)
        || window.element_opacity() <= 0.
        || color.a <= 0.
    {
        driver.suspend();
        return Report::Skipped;
    }
    let static_presentation = inert || cx.reduce_motion();
    let Some(sample) = driver.sample(static_presentation) else {
        return Report::Skipped;
    };
    let (start, end) = match (sample.shape, sample.value) {
        (_, Value::Determinate(fraction)) => (0., fraction),
        (
            Shape::Linear,
            Value::Indeterminate {
                static_presentation: true,
                ..
            },
        ) => (0.375, 0.625),
        (
            Shape::Circle,
            Value::Indeterminate {
                static_presentation: true,
                ..
            },
        ) => (0., 0.25),
        (Shape::Linear, Value::Indeterminate { phase, .. }) => {
            let start = f64::from(phase) * 1.25 - 0.25;
            (start, start + 0.25)
        }
        (Shape::Circle, Value::Indeterminate { phase, .. }) => {
            let phase = f64::from(phase);
            (
                Easing::EaseInOut.sample(((phase - 0.5) / 0.5).clamp(0., 1.)),
                Easing::EaseInOut.sample(phase),
            )
        }
    };
    let paths = match sample.shape {
        Shape::Linear => {
            geometry::linear(bounds, corners, start, end, scale).map(|fill| (None, fill))
        }
        Shape::Circle => geometry::ring(bounds, 0., 1., scale)
            .and_then(|track| geometry::ring(bounds, start, end, scale).map(|fill| (track, fill))),
    };
    let Ok((track, fill)) = paths else {
        driver.suspend();
        return Report::Skipped;
    };
    let mut paths = 0;
    let mut points = 0;
    window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
        for (geometry, color) in [(track, color.opacity(0.2)), (fill, color)] {
            if let Some(geometry) = geometry {
                points += geometry.points;
                paths += 1;
                window.paint_path(geometry.path, color);
            }
        }
    });
    driver.painted(true, static_presentation, window, cx);
    Report::Visible {
        sample,
        paths,
        points,
    }
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "progress_paint_test.rs"]
mod tests;
