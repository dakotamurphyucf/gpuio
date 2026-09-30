//! Oklab highlight mixing adapted from gpui-kit84f57fd component/theme/color.rs.
//! Copyright 2024-2026 Longbridge. Apache-2.0; see docs/catalog/sources/gpui-kit-LICENSE.
use gpui::{Hsla, Rgba};

fn linear(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn srgb(value: f64) -> f32 {
    let value = if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1. / 2.4) - 0.055
    };
    value.clamp(0., 1.) as f32
}

fn lab(color: Hsla) -> [f64; 3] {
    let rgb = color.to_rgb();
    let [r, g, b] = [rgb.r, rgb.g, rgb.b].map(|c| linear(c as f64));
    let l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b).cbrt();
    let m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b).cbrt();
    let s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b).cbrt();
    [
        0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s,
        1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s,
        0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s,
    ]
}

/// Source's factor .2 means 20% text, 80% theme target, with premultiplied alpha.
pub(super) fn highlight(text: Hsla, target: Hsla) -> Hsla {
    let a = text.a as f64 * 0.2;
    let b = target.a as f64 * 0.8;
    let alpha = a + b;
    if alpha == 0. {
        return Hsla::transparent_black();
    }
    let first = lab(text);
    let second = lab(target);
    let [lightness, red_green, yellow_blue] =
        std::array::from_fn(|i| (first[i] * a + second[i] * b) / alpha);
    let l = (lightness + 0.3963377774 * red_green + 0.2158037573 * yellow_blue).powi(3);
    let m = (lightness - 0.1055613458 * red_green - 0.0638541728 * yellow_blue).powi(3);
    let s = (lightness - 0.0894841775 * red_green - 1.2914855480 * yellow_blue).powi(3);
    Rgba {
        r: srgb(4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s),
        g: srgb(-1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s),
        b: srgb(-0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s),
        a: alpha as f32,
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mix_is_perceptual_and_alpha_premultiplied() {
        // Neutral Oklab L=.8 -> linear RGB .8^3 -> sRGB approximately .743206.
        let color = highlight(Hsla::black(), Hsla::white()).to_rgb();
        for c in [color.r, color.g, color.b] {
            assert!((c - 0.743206).abs() < 0.0001);
        }
        assert_eq!(color.a, 1.);
        let color = highlight(Hsla::black().opacity(0.), Hsla::white()).to_rgb();
        assert!((color.r - 1.).abs() < 0.0001);
        assert!((color.a - 0.8).abs() < 0.0001);
        assert!(highlight(Hsla::transparent_black(), Hsla::transparent_black()).is_transparent());
        let color = highlight(Hsla::white(), Hsla::white().opacity(0.)).to_rgb();
        assert!((color.r - 1.).abs() < 0.0001);
        assert!((color.a - 0.2).abs() < 0.0001);
    }
}
