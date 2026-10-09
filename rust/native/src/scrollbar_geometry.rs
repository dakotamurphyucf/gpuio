//! Viewport-local scrollbar geometry, independent of GPUI layout and clocks.
//! Offsets are positive distances from the start (negate GPUI scroll offsets at
//! the adapter boundary). The caller retains the actual native scroll handle.
use gpuio_protocol::scrollbar::{Axis as Selection, dimension};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}
impl Axis {
    fn index(self) -> usize {
        match self {
            Self::Horizontal => 0,
            Self::Vertical => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub origin: [f64; 2],
    pub size: [f64; 2],
}
impl Rect {
    pub fn contains(self, point: [f64; 2]) -> bool {
        (0..2).all(|i| {
            self.size[i] > 0.
                && point[i].is_finite()
                && point[i] >= self.origin[i]
                && point[i] < self.origin[i] + self.size[i]
        })
    }
}

/// Resolved, possibly animated dimensions for one frame. `envelope_width` is
/// the maximum track width across all states; it stays fixed during hover so
/// narrowing a painted track cannot repeatedly enter/leave its own hit target.
#[derive(Clone, Copy, Debug)]
pub struct Style {
    pub envelope_width: f64,
    pub track_width: f64,
    pub thumb_width: f64,
    pub inset: f64,
    pub radius: f64,
    pub min_length: f64,
}
impl Style {
    fn valid(self) -> bool {
        [
            self.envelope_width,
            self.track_width,
            self.thumb_width,
            self.inset,
            self.radius,
            self.min_length,
        ]
        .into_iter()
        .all(dimension)
            && self.track_width <= self.envelope_width
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    pub viewport: [f64; 2],
    pub content: [f64; 2],
    pub offset: [f64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bar {
    pub axis: Axis,
    pub envelope: Rect,
    pub track: Rect,
    pub thumb: Rect,
    pub radius: f64,
    pub max_offset: f64,
    pub offset: f64,
    start: f64,
    travel: f64,
}
impl Bar {
    /// A grip is a distance from the thumb's leading edge, not an old absolute
    /// position. Retain it during capture and apply it to freshly measured bars.
    pub fn grip(self, point: [f64; 2]) -> Option<f64> {
        self.thumb
            .contains(point)
            .then(|| point[self.axis.index()] - self.thumb.origin[self.axis.index()])
    }

    /// Project dragging onto current geometry. Zero travel has no meaningful
    /// pointer mapping; leave the scroll handle unchanged. Captured drags may
    /// leave the viewport but must still provide finite coordinates.
    pub fn drag_offset(self, position: f64, grip: f64) -> Option<f64> {
        if !position.is_finite()
            || !grip.is_finite()
            || self.travel <= 0.
            || self.thumb.size.into_iter().any(|x| x <= 0.)
        {
            return None;
        }
        let grip = grip.clamp(0., self.thumb.size[self.axis.index()]);
        let distance = (position - self.start - grip).clamp(0., self.travel);
        Some((distance / self.travel) * self.max_offset)
    }

    /// Track clicks center the thumb at the pointer; the renderer must first
    /// check eligibility/visibility and the painted track's clipped hit target.
    pub fn centered_offset(self, position: f64) -> Option<f64> {
        self.drag_offset(position, self.thumb.size[self.axis.index()] / 2.)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Geometry {
    pub horizontal: Option<Bar>,
    pub vertical: Option<Bar>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidGeometry;

/// Geometry is absent for axes without overflow or a usable edge envelope.
/// Corner reservation uses the other *eligible* axis only. This function never
/// changes content layout or invents overflow to make a bar appear.
pub fn measure(
    metrics: Metrics,
    selection: Selection,
    styles: [Style; 2],
) -> Result<Geometry, InvalidGeometry> {
    measure_reserved(metrics, selection, styles, [0.; 2])
}

/// Reserve edge space for a sibling range with a different viewport (tables
/// exclude headers and pinned columns). This shortens only the track, never
/// the semantic viewport or the scroll range. Insets are capped at half length.
pub fn measure_reserved(
    metrics: Metrics,
    selection: Selection,
    styles: [Style; 2],
    end_insets: [f64; 2],
) -> Result<Geometry, InvalidGeometry> {
    if !metrics
        .viewport
        .into_iter()
        .chain(metrics.content)
        .all(|x| x.is_finite() && (0. ..=f32::MAX as f64).contains(&x))
        || !metrics.offset.into_iter().all(f64::is_finite)
        || !styles.into_iter().all(Style::valid)
        || !end_insets.into_iter().all(dimension)
    {
        return Err(InvalidGeometry);
    }
    if metrics.viewport.into_iter().any(|x| x == 0.) {
        return Ok(Geometry::default());
    }
    let allowed = match selection {
        Selection::Horizontal => [true, false],
        Selection::Vertical => [false, true],
        Selection::Both => [true, true],
    };
    let eligible = [0, 1].map(|i| {
        allowed[i] && metrics.content[i] > metrics.viewport[i] && styles[i].envelope_width > 0.
    });
    // Keep both edge tracks usable in tiny viewports. If either requested width
    // covers the whole perpendicular axis, reserving it verbatim would erase
    // the other bar (and could make both disappear).
    let widths = [0, 1].map(|i| {
        let limit = metrics.viewport[1 - i] * if eligible[0] && eligible[1] { 0.5 } else { 1. };
        styles[i].envelope_width.min(limit)
    });
    let bars = [Axis::Horizontal, Axis::Vertical].map(|axis| {
        let i = axis.index();
        let cross = 1 - i;
        if !eligible[i] {
            return None;
        }
        let style = styles[i];
        let corner = (if eligible[cross] { widths[cross] } else { 0. })
            .max(end_insets[i].min(metrics.viewport[i] / 2.));
        let length = metrics.viewport[i] - corner;
        if length <= 0. {
            return None;
        }
        let rect = |start, length, width| {
            let mut origin = [0.; 2];
            let mut size = [0.; 2];
            origin[i] = start;
            origin[cross] = metrics.viewport[cross] - width;
            size[i] = length;
            size[cross] = width;
            Rect { origin, size }
        };
        let envelope = rect(0., length, widths[i]);
        let track = rect(0., length, style.track_width.min(envelope.size[cross]));
        let along_inset = style.inset.min(length / 2.);
        let cross_inset = style.inset.min(track.size[cross] / 2.);
        let available = (length - along_inset * 2.).max(0.);
        let thumb_length = (available * (metrics.viewport[i] / metrics.content[i]))
            .max(style.min_length.min(available))
            .min(available);
        let travel = (available - thumb_length).max(0.);
        let max_offset = metrics.content[i] - metrics.viewport[i];
        let offset = metrics.offset[i].clamp(0., max_offset);
        let thumb_width = style
            .thumb_width
            .min((track.size[cross] - cross_inset * 2.).max(0.));
        let mut thumb = rect(
            along_inset + (offset / max_offset) * travel,
            thumb_length,
            thumb_width,
        );
        thumb.origin[cross] -= cross_inset;
        Some(Bar {
            axis,
            envelope,
            track,
            thumb,
            radius: style.radius.min(thumb_length.min(thumb_width) / 2.),
            max_offset,
            offset,
            start: along_inset,
            travel,
        })
    });
    Ok(Geometry {
        horizontal: bars[0],
        vertical: bars[1],
    })
}

#[cfg(test)]
#[path = "scrollbar_geometry_test.rs"]
mod test;
