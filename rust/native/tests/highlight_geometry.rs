//! Logical ranges may be visually disconnected. Fixtures have explicit shaping
//! output, independent of installed fonts; native tests cover real CoreText/GPU.
use gpui::{FontId, GlyphId, LineLayout, ShapedGlyph, ShapedRun, point, px};
use gpui_base::text::ReorderedTextGeometry;

fn line(indices: &[usize], len: usize) -> LineLayout {
    LineLayout {
        len,
        width: px(indices.len() as f32 * 10.),
        runs: vec![ShapedRun {
            font_id: FontId(0),
            glyphs: indices
                .iter()
                .enumerate()
                .map(|(i, index)| ShapedGlyph {
                    id: GlyphId(i as u32),
                    index: *index,
                    position: point(px(i as f32 * 10.), px(0.)),
                    is_emoji: false,
                })
                .collect(),
        }],
        ..Default::default()
    }
}
fn spans(
    geometry: &ReorderedTextGeometry,
    range: std::ops::Range<usize>,
    clip: std::ops::Range<f32>,
) -> Vec<(f32, f32)> {
    let mut out = vec![];
    geometry.spans(range, px(clip.start)..px(clip.end), |x| {
        out.push((x.start.as_f32(), x.end.as_f32()))
    });
    out
}

#[test]
fn rtl_endpoints_and_visual_clipping() {
    let geometry = ReorderedTextGeometry::new(&line(&[4, 2, 0], 6)).unwrap();
    assert_eq!(spans(&geometry, 2..4, 0.0..30.0), vec![(10., 20.)]);
    assert_eq!(spans(&geometry, 0..2, 0.0..30.0), vec![(20., 30.)]);
    assert_eq!(spans(&geometry, 0..6, 0.0..30.0), vec![(0., 30.)]);
    assert_eq!(spans(&geometry, 0..6, 8.0..21.0), vec![(8., 21.)]);
    assert!(spans(&geometry, 2..4, 20.0..30.0).is_empty());
    assert!(spans(&geometry, 2..2, 0.0..30.0).is_empty());
}

#[test]
fn mixed_direction_range_keeps_unselected_visual_gap() {
    let geometry = ReorderedTextGeometry::new(&line(&[0, 1, 6, 4, 2, 8, 9], 10)).unwrap();
    assert_eq!(
        spans(&geometry, 0..4, 0.0..70.0),
        vec![(0., 20.), (40., 50.)]
    );
    assert_eq!(spans(&geometry, 2..8, 0.0..70.0), vec![(20., 50.)]);
}

#[test]
fn interior_cluster_ranges_and_monotonic_fast_path() {
    // A shaped glyph can cover more than one Unicode scalar, e.g. a ligature.
    let geometry = ReorderedTextGeometry::new(&line(&[6, 2, 0], 8)).unwrap();
    assert_eq!(spans(&geometry, 3..4, 0.0..30.0), vec![(10., 20.)]);
    assert!(ReorderedTextGeometry::new(&line(&[0, 2, 6], 8)).is_none());
    assert!(ReorderedTextGeometry::new(&line(&[], 0)).is_none());
}
