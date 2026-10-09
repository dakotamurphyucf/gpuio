//! Already-decoded spinner artwork; no I/O, parsing, resampling or OCaml callback.
use crate::spinner_clock::Driver;
use gpui::{
    App, Bounds, Pixels, RenderImage, TransformationMatrix, Window, point, px, radians, size,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Report {
    Skipped,
    Icon { turns: f32 },
    Fallback { turns: f32, mask_failed: bool },
}

/// The caller must reserve [image] through image_host::image_mask first. This
/// function paints inside the supplied bounds and returns any mask-paint failure
/// for the native adapter to report; asset-loading/failure selects fallback.
/// [inert] requests recognizable static artwork while retaining the native owner.
pub fn paint(
    driver: &Driver,
    bounds: Bounds<Pixels>,
    image: Option<&Arc<RenderImage>>,
    inert: bool,
    window: &mut Window,
    cx: &mut App,
) -> Report {
    let color = window.text_style().color;
    let clip = bounds.intersect(&window.content_mask().bounds);
    let finite = [
        bounds.origin.x,
        bounds.origin.y,
        bounds.size.width,
        bounds.size.height,
        bounds.right(),
        bounds.bottom(),
        bounds.center().x,
        bounds.center().y,
    ]
    .into_iter()
    .all(|value| f32::from(value).is_finite());
    if !finite
        || clip.size.width <= px(0.)
        || clip.size.height <= px(0.)
        || window.element_opacity() <= 0.
        || color.a <= 0.
    {
        driver.suspend();
        return Report::Skipped;
    }
    let static_presentation = inert || cx.reduce_motion();
    let Some(turns) = driver.sample(static_presentation) else {
        return Report::Skipped;
    };
    let diameter = bounds.size.width.min(bounds.size.height);
    let report = window.with_content_mask(Some(gpui::ContentMask { bounds: clip }), |window| {
        let mut mask_failed = false;
        if let Some(image) = image {
            // SVG decode always has one nonempty frame; be defensive at this
            // low-level entry point so malformed input falls back without panic.
            let dimensions = (image.frame_count() > 0).then(|| image.size(0));
            if let Some(dimensions) =
                dimensions.filter(|size| u32::from(size.width) > 0 && u32::from(size.height) > 0)
            {
                let width = u32::from(dimensions.width) as f32;
                let height = u32::from(dimensions.height) as f32;
                let scale = diameter / width.max(height);
                let image_size = size(scale * width, scale * height);
                let image_bounds = Bounds::new(
                    bounds.center() - point(image_size.width / 2., image_size.height / 2.),
                    image_size,
                );
                let transform = TransformationMatrix::unit()
                    .translate(bounds.center().scale(window.scale_factor()))
                    .rotate(radians(turns * std::f32::consts::TAU))
                    .translate(bounds.center().scale(-window.scale_factor()));
                if window
                    .paint_image_mask(image_bounds, image.clone(), 0, transform, color)
                    .is_ok()
                {
                    return Report::Icon { turns };
                }
            }
            mask_failed = true;
        }
        let radius = diameter * 0.38;
        let mut strokes = 0;
        for index in 0..12 {
            let angle = index as f32 * std::f32::consts::TAU / 12. - std::f32::consts::FRAC_PI_2;
            let vector = point(radius * angle.cos(), radius * angle.sin());
            let mut path = gpui::PathBuilder::stroke(diameter * 0.09);
            path.move_to(bounds.center() + vector * 0.55);
            path.line_to(bounds.center() + vector);
            if let Ok(path) = path.build() {
                let distance = (index as f32 / 12. - turns).rem_euclid(1.);
                window.paint_path(path, color.opacity(0.2 + 0.8 * distance));
                strokes += 1;
            }
        }
        if strokes == 0 {
            Report::Skipped
        } else {
            Report::Fallback { turns, mask_failed }
        }
    });
    driver.painted(
        !matches!(report, Report::Skipped),
        static_presentation,
        window,
        cx,
    );
    report
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "spinner_paint_test.rs"]
mod tests;
