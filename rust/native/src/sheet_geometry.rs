//! Resolve application sheet insets without borrowing the styled toolkit's
//! separate custom title-bar or client-frame wrapper conventions.
use gpui::{Edges, Pixels, Size, px, size};
use gpuio_protocol::{sheet_insets::Insets, v1::OverlayKind};

pub(super) struct Frame {
    pub insets: Edges<Pixels>,
    pub panel_size: Size<Pixels>,
    pub available_width: Pixels,
}
fn axis(viewport: Pixels, leading: f64, trailing: f64) -> (Pixels, Pixels, Pixels) {
    let viewport = f64::from(viewport).max(0.);
    let reserved = leading + trailing;
    let available = (viewport - 1.).max(0.);
    let factor = if reserved > available {
        available / reserved
    } else {
        1.
    };
    let leading = leading * factor;
    let trailing = trailing * factor;
    (
        px(leading as f32),
        px(trailing as f32),
        px((viewport - leading - trailing).max(0.) as f32),
    )
}
pub(super) fn resolve(
    kind: OverlayKind,
    viewport: Size<Pixels>,
    extent: f64,
    insets: Option<Insets>,
) -> Option<Frame> {
    let insets = insets.unwrap_or_default();
    let (left, right, width) = axis(viewport.width, insets.left, insets.right);
    let (top, bottom, height) = axis(viewport.height, insets.top, insets.bottom);
    let panel_size = match kind {
        OverlayKind::SheetLeft | OverlayKind::SheetRight => {
            size(px(extent as f32).min(width), height)
        }
        OverlayKind::SheetTop | OverlayKind::SheetBottom => {
            size(width, px(extent as f32).min(height))
        }
        OverlayKind::Dialog | OverlayKind::AlertDialog | OverlayKind::Popover => return None,
    };
    Some(Frame {
        insets: Edges {
            top,
            right,
            bottom,
            left,
        },
        panel_size,
        available_width: width,
    })
}

// Taffy floors border-box dimensions at padding + border. Bound their combined
// contribution across all potentially overlapping highlight states so even a
// tiny sheet retains its promised geometry. Percentage padding uses the parent's
// inner width, matching Taffy's containing-block rule.
pub(super) fn constrain_box(
    base: &mut gpui::StyleRefinement,
    states: &mut [Option<gpui::StyleRefinement>],
    frame: &Frame,
    rem: Pixels,
) {
    let measure = |s: &gpui::StyleRefinement| {
        let p = |v: Option<gpui::DefiniteLength>| {
            f32::from(v.map_or(px(0.), |v| v.to_pixels(frame.available_width.into(), rem))).max(0.)
        };
        let b = |v: Option<gpui::AbsoluteLength>| {
            f32::from(v.map_or(px(0.), |v| v.to_pixels(rem))).max(0.)
        };
        [
            p(s.padding.left),
            p(s.padding.right),
            p(s.padding.top),
            p(s.padding.bottom),
            b(s.border_widths.left),
            b(s.border_widths.right),
            b(s.border_widths.top),
            b(s.border_widths.bottom),
        ]
    };
    let mut maxima = [0_f32; 8];
    for style in std::iter::once(&*base).chain(states.iter().flatten()) {
        for (maximum, value) in maxima.iter_mut().zip(measure(style)) {
            *maximum = maximum.max(value);
        }
    }
    let factor = |extent: Pixels, total: f32| {
        let room = (f32::from(extent) - 1.).max(0.);
        if total > room { room / total } else { 1. }
    };
    let x = factor(
        frame.panel_size.width,
        maxima[0] + maxima[1] + maxima[4] + maxima[5],
    );
    let y = factor(
        frame.panel_size.height,
        maxima[2] + maxima[3] + maxima[6] + maxima[7],
    );
    if x == 1. && y == 1. {
        return;
    }
    for style in std::iter::once(base).chain(states.iter_mut().flatten()) {
        let values = measure(style);
        for (index, slot) in [
            &mut style.padding.left,
            &mut style.padding.right,
            &mut style.padding.top,
            &mut style.padding.bottom,
        ]
        .into_iter()
        .enumerate()
        {
            let factor = if index < 2 { x } else { y };
            if slot.is_some() && factor < 1. {
                *slot = Some(px(values[index] * factor).into());
            }
        }
        for (index, slot) in [
            &mut style.border_widths.left,
            &mut style.border_widths.right,
            &mut style.border_widths.top,
            &mut style.border_widths.bottom,
        ]
        .into_iter()
        .enumerate()
        {
            let factor = if index < 2 { x } else { y };
            if slot.is_some() && factor < 1. {
                *slot = Some(px(values[index + 4] * factor).into());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn insets_compress_proportionally_only_when_needed_and_never_exceed_viewport() {
        assert_eq!(axis(px(100.), 10., 20.), (px(10.), px(20.), px(70.)));
        assert_eq!(axis(px(100.), 100., 200.), (px(33.), px(66.), px(1.)));
        assert_eq!(axis(px(0.5), 16384., 16384.), (px(0.), px(0.), px(0.5)));
        assert_eq!(axis(px(0.), 0., 0.), (px(0.), px(0.), px(0.)));
        assert_eq!(axis(px(1.), 0., 16384.), (px(0.), px(0.), px(1.)));
        for kind in [
            OverlayKind::SheetLeft,
            OverlayKind::SheetRight,
            OverlayKind::SheetTop,
            OverlayKind::SheetBottom,
        ] {
            let f = resolve(
                kind,
                size(px(100.), px(80.)),
                500.,
                Some(Insets {
                    top: 10.,
                    right: 20.,
                    bottom: 30.,
                    left: 40.,
                }),
            )
            .unwrap();
            assert_eq!(f.panel_size, size(px(40.), px(40.)));
        }
    }
}
