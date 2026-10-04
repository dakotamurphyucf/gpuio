//! Scrollbar input against an existing GPUI handle. No copied offset, event
//! callback or OCaml state. The widget supplies current geometry/policy and owns
//! OS pointer capture and focus; this controller balances native list drag hooks.
use crate::scrollbar_geometry::{self as geometry, Axis, Bar, Geometry, Metrics, Style};
use gpui::{Bounds, Pixels, point, px};
use gpui_base::ScrollbarHandle;
use gpuio_protocol::scrollbar::Axis as Selection;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Backward,
    Forward,
}
impl Direction {
    fn sign(self) -> f64 {
        match self {
            Self::Backward => -1.,
            Self::Forward => 1.,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Adjustment {
    Line { direction: Direction, height: f64 },
    Page(Direction),
    First,
    Last,
    Set(f64),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Policy {
    pub enabled: bool,
    pub pointer: bool,
}
#[derive(Clone, Copy)]
struct Capture {
    axis: Axis,
    grip: f64,
}

/// One viewport's retained input owner. Geometry refinements may change while
/// captured; the grip remains a distance from the newly measured thumb start.
pub struct State {
    handle: Rc<dyn ScrollbarHandle>,
    corner_peer: Option<Rc<dyn ScrollbarHandle>>,
    selection: Selection,
    styles: [Style; 2],
    viewport: Option<Bounds<Pixels>>,
    policy: Policy,
    capture: Option<Capture>,
    closed: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Measurement {
    pub viewport: Bounds<Pixels>,
    pub bars: Geometry,
}
fn index(axis: Axis) -> usize {
    match axis {
        Axis::Horizontal => 0,
        Axis::Vertical => 1,
    }
}
fn bar(geometry: Geometry, axis: Axis) -> Option<Bar> {
    match axis {
        Axis::Horizontal => geometry.horizontal,
        Axis::Vertical => geometry.vertical,
    }
}
fn measure(
    handle: &dyn ScrollbarHandle,
    selection: Selection,
    styles: [Style; 2],
    viewport: Option<Bounds<Pixels>>,
    corner_peer: Option<&dyn ScrollbarHandle>,
) -> Result<Measurement, geometry::InvalidGeometry> {
    let viewport = viewport.unwrap_or_else(|| handle.viewport_bounds());
    if ![
        viewport.origin.x,
        viewport.origin.y,
        viewport.right(),
        viewport.bottom(),
    ]
    .into_iter()
    .all(|v| f32::from(v).is_finite())
    {
        return Err(geometry::InvalidGeometry);
    }
    let content = handle.content_size();
    let offset = handle.offset();
    let metrics = Metrics {
        viewport: [viewport.size.width, viewport.size.height].map(|v| f64::from(f32::from(v))),
        content: [content.width, content.height].map(|v| f64::from(f32::from(v))),
        offset: [offset.x, offset.y].map(|v| -f64::from(f32::from(v))),
    };
    let mut end_insets = [0.; 2];
    if let Some(peer) = corner_peer {
        let axis = match selection {
            Selection::Horizontal => Some(0),
            Selection::Vertical => Some(1),
            Selection::Both => None,
        };
        if let Some(i) = axis {
            let bounds = peer.viewport_bounds();
            let content = peer.content_size();
            let visible = [bounds.size.width, bounds.size.height];
            let extent = [content.width, content.height];
            if visible
                .into_iter()
                .all(|v| v > px(0.) && f32::from(v).is_finite())
                && f32::from(extent[1 - i]).is_finite()
                && extent[1 - i] > visible[1 - i]
            {
                end_insets[i] = styles[1 - i].envelope_width;
            }
        }
    }
    Ok(Measurement {
        viewport,
        bars: if corner_peer.is_some() {
            geometry::measure_reserved(metrics, selection, styles, end_insets)?
        } else {
            geometry::measure(metrics, selection, styles)?
        },
    })
}
impl State {
    pub fn new(
        handle: Rc<dyn ScrollbarHandle>,
        selection: Selection,
        styles: [Style; 2],
    ) -> Result<Self, geometry::InvalidGeometry> {
        measure(handle.as_ref(), selection, styles, None, None)?;
        Ok(Self {
            handle,
            corner_peer: None,
            selection,
            styles,
            viewport: None,
            policy: Policy {
                enabled: true,
                pointer: true,
            },
            capture: None,
            closed: false,
        })
    }
    /// A native sibling axis, sharing presentation but owning a different
    /// viewport. Only its live overflow determines corner reservation.
    pub fn set_corner_peer(&mut self, peer: Option<Rc<dyn ScrollbarHandle>>) {
        self.corner_peer = peer;
    }
    /// Invalid refinements are atomic. The supplied layout viewport is optional;
    /// None uses the current native handle bounds on every input event.
    pub fn reconcile(
        &mut self,
        selection: Selection,
        styles: [Style; 2],
        viewport: Option<Bounds<Pixels>>,
        policy: Policy,
    ) -> Result<(), geometry::InvalidGeometry> {
        let measured = measure(
            self.handle.as_ref(),
            selection,
            styles,
            viewport,
            self.corner_peer.as_deref(),
        )?;
        self.selection = selection;
        self.styles = styles;
        self.viewport = viewport;
        self.policy = policy;
        if !policy.enabled
            || !policy.pointer
            || self.capture.is_some_and(|c| {
                bar(measured.bars, c.axis)
                    .is_none_or(|b| b.thumb.size.into_iter().any(|size| size <= 0.))
            })
        {
            self.cancel();
        }
        Ok(())
    }
    pub fn measurement(&self) -> Result<Measurement, geometry::InvalidGeometry> {
        self.preview(self.styles)
    }
    /// Measure alternate presentation without changing capture or live input.
    pub fn preview(&self, styles: [Style; 2]) -> Result<Measurement, geometry::InvalidGeometry> {
        measure(
            self.handle.as_ref(),
            self.selection,
            styles,
            self.viewport,
            self.corner_peer.as_deref(),
        )
    }
    pub fn is_dragging(&self) -> bool {
        self.capture.is_some()
    }
    pub fn captured_axis(&self) -> Option<Axis> {
        self.capture.map(|c| c.axis)
    }
    fn allowed(&self, pointer: bool) -> bool {
        !self.closed && self.policy.enabled && (!pointer || self.policy.pointer)
    }
    fn local(measured: Measurement, position: [f64; 2]) -> [f64; 2] {
        [
            position[0] - f64::from(f32::from(measured.viewport.origin.x)),
            position[1] - f64::from(f32::from(measured.viewport.origin.y)),
        ]
    }
    /// Only call after the renderer verifies actual visible thumb and parent
    /// clip. Geometry enforces the thumb bounds; opacity/occlusion is not here.
    pub fn begin_drag(&mut self, axis: Axis, position: [f64; 2]) -> bool {
        if !self.allowed(true) {
            return false;
        }
        let Ok(measured) = self.measurement() else {
            self.cancel();
            return false;
        };
        let Some(bar) = bar(measured.bars, axis) else {
            return false;
        };
        let Some(grip) = bar.grip(Self::local(measured, position)) else {
            return false;
        };
        self.cancel();
        self.handle.start_drag();
        self.capture = Some(Capture { axis, grip });
        true
    }
    fn set(&self, axis: Axis, value: f64, maximum: f64) -> bool {
        if !value.is_finite() {
            return false;
        }
        let old = self.handle.offset();
        let offset = px(-value.clamp(0., maximum) as f32);
        let next = match axis {
            Axis::Horizontal => point(offset, old.y),
            Axis::Vertical => point(old.x, offset),
        };
        if old == next {
            return false;
        }
        self.handle.set_offset(next);
        true
    }
    pub fn drag_to(&mut self, position: [f64; 2]) -> bool {
        if !self.allowed(true) {
            self.cancel();
            return false;
        }
        if !position.into_iter().all(f64::is_finite) {
            return false;
        }
        let Some(capture) = self.capture else {
            return false;
        };
        let Ok(measured) = self.measurement() else {
            self.cancel();
            return false;
        };
        let Some(bar) = bar(measured.bars, capture.axis) else {
            self.cancel();
            return false;
        };
        if bar.thumb.size.into_iter().any(|size| size <= 0.) {
            self.cancel();
            return false;
        }
        let position = Self::local(measured, position);
        let Some(offset) = bar.drag_offset(position[index(capture.axis)], capture.grip) else {
            return false;
        };
        self.set(capture.axis, offset, bar.max_offset)
    }
    pub fn track_click(&mut self, axis: Axis, position: [f64; 2]) -> bool {
        if !self.allowed(true) {
            return false;
        }
        let Ok(measured) = self.measurement() else {
            return false;
        };
        let Some(bar) = bar(measured.bars, axis) else {
            return false;
        };
        let local = Self::local(measured, position);
        if !bar.track.contains(local) {
            return false;
        }
        let Some(offset) = bar.centered_offset(local[index(axis)]) else {
            return false;
        };
        self.cancel();
        self.set(axis, offset, bar.max_offset)
    }
    /// Shared keyboard/AX adjustment. The widget validates actual focus, key
    /// modifiers, live owner and native action payload before calling this.
    pub fn adjust(&mut self, axis: Axis, adjustment: Adjustment) -> bool {
        if !self.allowed(false) {
            return false;
        }
        let Ok(measured) = self.measurement() else {
            return false;
        };
        let Some(bar) = bar(measured.bars, axis) else {
            return false;
        };
        if bar.thumb.size.into_iter().any(|size| size <= 0.) {
            return false;
        }
        let target = match adjustment {
            Adjustment::Line { direction, height } if height.is_finite() && height > 0. => {
                bar.offset + direction.sign() * height.min(bar.max_offset)
            }
            Adjustment::Line { .. } => return false,
            Adjustment::Page(direction) => {
                bar.offset
                    + direction.sign()
                        * f64::from(f32::from(match axis {
                            Axis::Horizontal => measured.viewport.size.width,
                            Axis::Vertical => measured.viewport.size.height,
                        }))
            }
            Adjustment::First => 0.,
            Adjustment::Last => bar.max_offset,
            Adjustment::Set(value) if value.is_finite() => value,
            Adjustment::Set(_) => return false,
        };
        self.cancel();
        self.set(axis, target, bar.max_offset)
    }
    /// End and cancellation both retain the actual current scroll position.
    pub fn cancel(&mut self) -> bool {
        if self.capture.take().is_none() {
            return false;
        }
        self.handle.end_drag();
        true
    }
    pub fn close(&mut self) {
        self.cancel();
        self.closed = true;
    }
}
impl Drop for State {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
#[path = "scrollbar_input_test.rs"]
mod test;
