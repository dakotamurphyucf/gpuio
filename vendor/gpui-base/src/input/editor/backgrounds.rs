//! Immutable background layers for a prepared read-only document revision.
use crate::input::EditorState;
use gpui::{Context, Hsla, Pixels, px};
use std::{ops::Range, rc::Rc};

#[derive(Clone, Debug)]
pub struct RangeBackground {
    /// Half-open UTF-8 bytes in the currently installed editor buffer.
    pub bytes: Range<usize>,
    pub color: Hsla,
    pub radius: Pixels,
}

/// Frozen ordered layers. Implementations may retain a prepared-result lease;
/// the editor and in-flight frame keep this owner alive while reading its data.
/// `ranges` must return the same immutable slice for this owner's lifetime and
/// must not run matching, I/O, or application callbacks.
pub trait RangeBackgrounds: 'static {
    fn ranges(&self) -> &[RangeBackground];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RangeBackgroundError {
    Editable,
    Limit,
    InvalidRange,
    InvalidRadius,
}

impl EditorState {
    /// Replace prepared washes without changing syntax styles or selection.
    /// Text replacement clears the owner. Invalid input also clears old washes.
    /// Only read-only buffers accept this revision-bound decoration API.
    pub fn set_range_backgrounds(
        &mut self,
        backgrounds: Option<Rc<dyn RangeBackgrounds>>,
        cx: &mut Context<Self>,
    ) -> Result<(), RangeBackgroundError> {
        let same = match (&self.extras.range_backgrounds, &backgrounds) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if same && (self.readonly || backgrounds.is_none()) {
            return Ok(());
        }
        let valid = backgrounds.as_ref().map_or(Ok(()), |source| {
            if !self.readonly {
                return Err(RangeBackgroundError::Editable);
            }
            let ranges = source.ranges();
            if ranges.len() > 32768 {
                return Err(RangeBackgroundError::Limit);
            }
            for wash in ranges {
                if wash.bytes.start >= wash.bytes.end
                    || wash.bytes.end > self.text.len()
                    || !self.text.is_char_boundary(wash.bytes.start)
                    || !self.text.is_char_boundary(wash.bytes.end)
                {
                    return Err(RangeBackgroundError::InvalidRange);
                }
                if !wash.radius.as_f32().is_finite()
                    || wash.radius < px(0.)
                    || wash.radius > px(64.)
                {
                    return Err(RangeBackgroundError::InvalidRadius);
                }
            }
            Ok(())
        });
        let backgrounds = valid.is_ok().then_some(backgrounds).flatten();
        let same = match (&self.extras.range_backgrounds, &backgrounds) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if !same {
            self.extras.range_backgrounds = backgrounds;
            cx.notify();
        }
        valid
    }
}
