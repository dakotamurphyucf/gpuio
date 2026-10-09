//! Horizontal viewport geometry, separate from buffered render ranges.
use gpui::{Bounds, Pixels, SharedString, px};

/// A column band with positive horizontal intersection with the table viewport.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisibleColumn {
    pub column: SharedString,
    pub pinned: bool,
    /// Horizontal extent only; does not imply every row or header is visible.
    pub fully_visible: bool,
}

/// Latest completed native layout. Entries are in native display order and do
/// not include row-header gutters, group headers, fillers or rendering overscan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ColumnViewport {
    pub columns: Vec<VisibleColumn>,
}

pub(super) fn measure<'a>(
    columns: impl Iterator<Item = (&'a SharedString, Pixels)>,
    pinned_count: usize,
    pinned: Bounds<Pixels>,
    scrolling: Bounds<Pixels>,
    offset: Pixels,
    visible: Bounds<Pixels>,
) -> ColumnViewport {
    let mut result = ColumnViewport::default();
    if visible.size.width <= px(0.) || visible.size.height <= px(0.) {
        return result;
    }
    let mut pinned_left = pinned.left();
    let mut scrolling_left = scrolling.left() + offset;
    for (index, (column, width)) in columns.enumerate() {
        let is_pinned = index < pinned_count;
        let (left, pane) = if is_pinned {
            (&mut pinned_left, pinned)
        } else {
            (&mut scrolling_left, scrolling)
        };
        let right = *left + width;
        let clip_left = pane.left().max(visible.left());
        let clip_right = pane.right().min(visible.right());
        if clip_right > clip_left && right > clip_left && *left < clip_right {
            result.columns.push(VisibleColumn {
                column: column.clone(),
                pinned: is_pinned,
                fully_visible: *left >= clip_left && right <= clip_right,
            });
        }
        *left = right;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, size};

    fn bounds(left: f32, width: f32) -> Bounds<Pixels> {
        Bounds::new(point(px(left), px(0.)), size(px(width), px(30.)))
    }

    #[test]
    fn clipping_does_not_include_overscan_or_columns_hidden_under_pins() {
        let ids: Vec<SharedString> = ["pin", "a", "b", "c"].map(Into::into).into();
        let observe = |offset, clip| {
            measure(
                ids.iter().map(|id| (id, px(100.))),
                1,
                bounds(20., 100.),
                bounds(120., 150.),
                px(offset),
                clip,
            )
            .columns
            .into_iter()
            .map(|c| (c.column.to_string(), c.pinned, c.fully_visible))
            .collect::<Vec<_>>()
        };
        assert_eq!(
            observe(0., bounds(0., 270.)),
            vec![
                ("pin".into(), true, true),
                ("a".into(), false, true),
                ("b".into(), false, false)
            ]
        );
        assert_eq!(
            observe(-100., bounds(0., 270.)),
            vec![
                ("pin".into(), true, true),
                ("b".into(), false, true),
                ("c".into(), false, false)
            ]
        );
        assert_eq!(
            observe(-150., bounds(80., 170.)),
            vec![
                ("pin".into(), true, false),
                ("b".into(), false, false),
                ("c".into(), false, false)
            ]
        );
        assert!(observe(0., bounds(0., 0.)).is_empty());
        assert!(observe(0., bounds(400., 100.)).is_empty());
    }
}
