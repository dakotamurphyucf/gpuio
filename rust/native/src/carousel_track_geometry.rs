//! Measured, axis-independent track geometry in logical pixels.
//!
//! This is the geometry foundation for the measured-track adapter. Existing
//! page carousels do not use it. Inputs are unscrolled coordinates relative to
//! the track viewport, with any loop runway already subtracted by the adapter.
//! Layout, item ownership, transport and animation remain separate concerns.
use crate::carousel_gesture::Step;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Item {
    pub start: f32,
    pub extent: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidViewport,
    InvalidScrollLimit,
    TooManyItems,
    InvalidItem,
    OverlappingOrUnorderedItems,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Loop {
    /// A finite track still wraps selection, but a boundary wrap must settle
    /// immediately. One item would otherwise need two simultaneously painted
    /// copies, which cannot share a native input/IME owner safely.
    Jump,
    Continuous {
        /// Distance between equivalent positions in neighboring cycles.
        cycle: f32,
        /// Leading spacer plus the gap before the first item.
        origin: f32,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Geometry {
    viewport: f32,
    items: Vec<Item>,
    snaps: Vec<f32>,
    looping: Option<Loop>,
}

impl Geometry {
    /// The adapter supplies GPUI's measured scroll limit, including track
    /// padding. The geometric span supplies a lower bound if it is larger.
    /// At most 128 nonoverlapping items; zero-width items are valid. A zero-size
    /// viewport is not ready for input and must be handled by the adapter.
    pub fn new(
        viewport: f32,
        scroll_limit: f32,
        items: Vec<Item>,
        looping: bool,
    ) -> Result<Self, Error> {
        if !viewport.is_finite() || viewport <= 0. {
            return Err(Error::InvalidViewport);
        }
        if !scroll_limit.is_finite() || scroll_limit < 0. {
            return Err(Error::InvalidScrollLimit);
        }
        if items.len() > gpuio_protocol::carousel::MAX_ITEMS {
            return Err(Error::TooManyItems);
        }
        for item in &items {
            if !item.start.is_finite()
                || !item.extent.is_finite()
                || item.extent < 0.
                || !(item.start + item.extent).is_finite()
            {
                return Err(Error::InvalidItem);
            }
        }
        if items
            .windows(2)
            .any(|pair| pair[1].start < pair[0].start + pair[0].extent)
        {
            return Err(Error::OverlappingOrUnorderedItems);
        }
        let span = items
            .first()
            .zip(items.last())
            .map_or(0., |(first, last)| last.start + last.extent - first.start);
        if !span.is_finite() {
            return Err(Error::InvalidItem);
        }
        let gap = items
            .get(1)
            .map_or(0., |second| second.start - items[0].start - items[0].extent);
        let cycle = span + gap;
        let origin = cycle + gap;
        if !cycle.is_finite() || !origin.is_finite() {
            return Err(Error::InvalidItem);
        }
        let looping = (looping && items.len() > 1 && cycle > 0. && cycle >= viewport).then(|| {
            let largest = items.iter().map(|item| item.extent).fold(0., f32::max);
            if f64::from(cycle) - f64::from(largest) >= f64::from(viewport) {
                Loop::Continuous { cycle, origin }
            } else {
                Loop::Jump
            }
        });
        let max_scroll = scroll_limit.max((span - viewport).max(0.));
        let snaps: Vec<_> = items
            .iter()
            .map(|item| match looping {
                Some(Loop::Continuous { origin, .. }) => -(item.start - items[0].start) - origin,
                None | Some(Loop::Jump) => (-item.start).clamp(-max_scroll, 0.),
            })
            .collect();
        if snaps.iter().any(|offset| !offset.is_finite()) {
            return Err(Error::InvalidItem);
        }
        Ok(Self {
            viewport,
            items,
            snaps,
            looping,
        })
    }

    /// Clamp finite tracks or rebase a continuous track into one canonical cycle.
    /// Sampling in that cycle keeps neighboring item translations sufficient.
    pub fn bounded_offset(&self, offset: f64) -> Option<f32> {
        if !offset.is_finite() {
            return None;
        }
        let value = match self.looping {
            Some(Loop::Continuous { cycle, origin }) => {
                -f64::from(origin) - (-offset - f64::from(origin)).rem_euclid(f64::from(cycle))
            }
            None | Some(Loop::Jump) => {
                let minimum = self.snaps.iter().copied().fold(0., f32::min);
                offset.clamp(f64::from(minimum), 0.)
            }
        } as f32;
        value.is_finite().then_some(value)
    }
    /// Match the pinned track's deliberate boundary wrap even when the short
    /// runway cannot display a continuous wrap. Other releases choose nearest.
    pub fn release_target(&self, start: usize, delta: f64, offset: f32) -> Option<usize> {
        if !delta.is_finite() || !offset.is_finite() || start >= self.items.len() {
            return None;
        }
        if self.looping.is_some() && delta.abs() >= (f64::from(self.viewport) * 0.25).max(1.) {
            if start == 0 && delta > 0. {
                return self.items.len().checked_sub(1);
            }
            if start + 1 == self.items.len() && delta < 0. {
                return Some(0);
            }
        }
        self.nearest(offset)
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }
    pub fn snaps(&self) -> &[f32] {
        &self.snaps
    }
    /// First index at each item's measured stop, bounded by the item count.
    /// A layout observation can carry these indices rather than pixel geometry.
    pub fn canonical_indices(&self) -> Vec<usize> {
        let mut canonical = 0;
        self.snaps
            .iter()
            .enumerate()
            .map(|(index, snap)| {
                if index != 0 && *snap != self.snaps[index - 1] {
                    canonical = index;
                }
                canonical
            })
            .collect()
    }

    /// Compact bridge metadata changes only when measured stops or loop mode
    /// change, never on each animation offset.
    pub fn stops(&self) -> gpuio_protocol::carousel_track::Stops {
        use gpuio_protocol::carousel_track::{Loop as Mode, Stops};
        Stops {
            canonical: self
                .canonical_indices()
                .into_iter()
                .map(|index| index as i64)
                .collect(),
            looping: match self.looping {
                None => Mode::Finite,
                Some(Loop::Jump) => Mode::Jump,
                Some(Loop::Continuous { .. }) => Mode::Continuous,
            },
        }
    }

    pub fn looping(&self) -> Option<Loop> {
        self.looping
    }
    pub fn snap(&self, index: usize) -> Option<f32> {
        self.snaps.get(index).copied()
    }

    /// Skip positions that cannot produce movement. Explicit selection may still
    /// address any logical item, including a trailing duplicate.
    pub fn step(&self, current: usize, direction: Step) -> Option<usize> {
        let origin = self.snap(current)?;
        let count = self.snaps.len();
        for distance in 1..count {
            let candidate = match direction {
                Step::Next => current.checked_add(distance)?,
                Step::Previous => current + count - distance,
            };
            if self.looping.is_none()
                && match direction {
                    Step::Next => candidate >= count,
                    Step::Previous => distance > current,
                }
            {
                break;
            }
            let candidate = candidate % count;
            if self.snaps[candidate] != origin {
                return Some(candidate);
            }
        }
        None
    }

    /// A directional boundary move lands in an adjacent cycle. The adapter
    /// silently rebases to snap(target) after motion settles, without selection.
    pub fn step_target(&self, current: usize, direction: Step) -> Option<(usize, f32)> {
        let target = self.step(current, direction)?;
        let mut offset = self.snaps[target];
        if let Some(Loop::Continuous { cycle, .. }) = self.looping {
            match direction {
                Step::Next if target < current => offset -= cycle,
                Step::Previous if target > current => offset += cycle,
                _ => (),
            }
        }
        offset.is_finite().then_some((target, offset))
    }

    /// First logical item wins equal-distance/duplicate ties.
    pub fn nearest(&self, offset: f32) -> Option<usize> {
        if !offset.is_finite() {
            return None;
        }
        self.snaps
            .iter()
            .enumerate()
            .min_by(|(a, x), (b, y)| {
                let distance = |snap: f32| {
                    let delta = (snap as f64 - offset as f64).abs();
                    match self.looping {
                        Some(Loop::Continuous { cycle, .. }) => {
                            let cycle = f64::from(cycle);
                            let delta = delta % cycle;
                            delta.min(cycle - delta)
                        }
                        None | Some(Loop::Jump) => delta,
                    }
                };
                distance(**x).total_cmp(&distance(**y)).then(a.cmp(b))
            })
            .map(|(index, _)| index)
    }

    /// Translate one retained item to its nearest loop copy. This never creates
    /// another native owner. A nonlooping track returns zero translation.
    pub fn item_translation(&self, index: usize, offset: f32) -> Option<f32> {
        let item = self.items.get(index)?;
        if !offset.is_finite() {
            return None;
        }
        let Some(Loop::Continuous { cycle, origin }) = self.looping else {
            return Some(0.);
        };
        let viewport_center = f64::from(self.viewport) / 2. - f64::from(offset);
        let item_center = f64::from(origin) + f64::from(item.start) + f64::from(item.extent) / 2.;
        let cycles = ((viewport_center - item_center) / f64::from(cycle))
            .round()
            .clamp(-1., 1.);
        Some((f64::from(cycle) * cycles) as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn item(start: f32, extent: f32) -> Item {
        Item { start, extent }
    }

    #[test]
    fn trailing_duplicates_use_one_navigation_stop_but_keep_explicit_selection() {
        let g = Geometry::new(
            100.,
            50.,
            vec![item(0., 50.), item(50., 50.), item(100., 50.)],
            false,
        )
        .unwrap();
        assert_eq!(g.snaps(), [0., -50., -50.]);
        assert_eq!(g.nearest(-50.), Some(1));
        assert_eq!(g.canonical_indices(), [0, 1, 1]);
        assert_eq!(
            g.stops(),
            gpuio_protocol::carousel_track::Stops {
                canonical: vec![0, 1, 1],
                looping: gpuio_protocol::carousel_track::Loop::Finite,
            }
        );
        assert_eq!(g.step(2, Step::Previous), Some(0));
        assert_eq!(g.step(1, Step::Next), None);
        assert_eq!(g.snap(2), Some(-50.));
    }

    #[test]
    fn measured_gap_inset_and_wrap_direction_follow_the_pinned_track() {
        let g = Geometry::new(100., 148., vec![item(16., 100.), item(132., 100.)], true).unwrap();
        assert_eq!(
            g.looping(),
            Some(Loop::Continuous {
                cycle: 232.,
                origin: 248.
            })
        );
        assert_eq!(g.snaps(), [-248., -364.]);
        assert_eq!(g.step_target(1, Step::Next), Some((0, -480.)));
        assert_eq!(g.step_target(0, Step::Previous), Some((1, -132.)));
        assert_eq!(g.nearest(-480.), Some(0));
        assert_eq!(g.item_translation(0, -480.), Some(232.));
    }

    #[test]
    fn unequal_extents_and_short_loop_fallback_do_not_invent_scroll() {
        let items = vec![item(0., 40.), item(48., 120.), item(176., 64.)];
        let g = Geometry::new(100., 140., items.clone(), false).unwrap();
        assert_eq!(g.snaps(), [0., -48., -140.]);
        assert_eq!(g.nearest(-100.), Some(2));
        let short = Geometry::new(300., 0., items, true).unwrap();
        assert_eq!(short.looping(), None);
        assert_eq!(short.snaps(), [0., 0., 0.]);
        assert_eq!(short.step(1, Step::Next), None);
        assert_eq!(short.step(1, Step::Previous), None);
    }

    #[test]
    fn wide_item_loops_keep_wrapping_without_duplicate_native_presenters() {
        let g = Geometry::new(180., 20., vec![item(0., 150.), item(150., 50.)], true).unwrap();
        assert_eq!(g.looping(), Some(Loop::Jump));
        assert_eq!(g.snaps(), [0., -20.]);
        assert_eq!(g.step_target(1, Step::Next), Some((0, 0.)));
        assert_eq!(g.step_target(0, Step::Previous), Some((1, -20.)));
        assert_eq!(g.item_translation(0, -10.), Some(0.));
        assert_eq!(g.nearest(-18.), Some(1));
    }

    #[test]
    fn continuous_translation_matches_periodic_coverage_without_cloning_items() {
        for a in [20., 50., 100.] {
            for b in [20., 50., 100.] {
                for c in [20., 50., 100.] {
                    for gap in [0., 8.] {
                        for viewport in [20., 60., 140.] {
                            let items =
                                vec![item(0., a), item(a + gap, b), item(a + b + 2. * gap, c)];
                            let g = Geometry::new(viewport, 0., items, true).unwrap();
                            assert!(g.stops().is_valid());
                            let Some(Loop::Continuous { cycle, origin }) = g.looping() else {
                                continue;
                            };
                            for phase in 0..=32 {
                                let offset = -origin - cycle * phase as f32 / 32.;
                                for pixel in 0..viewport as usize {
                                    let x = pixel as f32 + 0.5;
                                    let contains = |item: &Item, shift: f32| {
                                        let start = item.start + origin + shift + offset;
                                        start <= x && x < start + item.extent
                                    };
                                    let actual =
                                        g.items().iter().enumerate().any(|(index, item)| {
                                            contains(
                                                item,
                                                g.item_translation(index, offset).unwrap(),
                                            )
                                        });
                                    let periodic = g.items().iter().any(|item| {
                                        (-1..=2).any(|copy| contains(item, copy as f32 * cycle))
                                    });
                                    assert_eq!(
                                        actual, periodic,
                                        "sizes {a}/{b}/{c}, gap {gap}, viewport {viewport}, offset {offset}, x {x}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn geometry_is_bounded_and_rejects_invalid_layout_without_panicking() {
        for viewport in [0., -1., f32::NAN, f32::INFINITY] {
            assert_eq!(
                Geometry::new(viewport, 0., vec![], false),
                Err(Error::InvalidViewport)
            );
        }
        assert_eq!(
            Geometry::new(10., 0., vec![item(0., 0.); 129], false),
            Err(Error::TooManyItems)
        );
        for bad in [item(0., -1.), item(f32::NAN, 1.), item(f32::MAX, f32::MAX)] {
            assert_eq!(
                Geometry::new(10., 0., vec![bad], false),
                Err(Error::InvalidItem)
            );
        }
        assert_eq!(
            Geometry::new(10., 0., vec![item(0., 10.), item(5., 10.)], false),
            Err(Error::OverlappingOrUnorderedItems)
        );
        let empty = Geometry::new(10., 0., vec![], true).unwrap();
        assert_eq!(empty.step(0, Step::Next), None);
        assert_eq!(empty.nearest(0.), None);
        let zero = Geometry::new(10., 0., vec![item(0., 0.); 128], true).unwrap();
        assert_eq!(zero.looping(), None);
        assert_eq!(zero.nearest(0.), Some(0));
        assert_eq!(zero.nearest(f32::NAN), None);
        assert_eq!(zero.item_translation(128, 0.), None);
    }
}
