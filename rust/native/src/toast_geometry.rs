//! Pure measured layered-stack geometry. Production rendering integration is separate.
use gpuio_protocol::NodeId;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layering {
    pub peek: f64,
    pub gap: f64,
    pub width_step: f64,
    pub visible: usize,
}
impl Default for Layering {
    fn default() -> Self {
        Self {
            peek: 14.,
            gap: 14.,
            width_step: 0.05,
            visible: 3,
        }
    }
}
impl Layering {
    pub fn is_valid(self) -> bool {
        [self.peek, self.gap]
            .into_iter()
            .all(|n| n.is_finite() && (0.0..=16384.).contains(&n))
            && self.width_step.is_finite()
            && (0.0..=0.1).contains(&self.width_step)
            && (1..=8).contains(&self.visible)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Card {
    pub id: NodeId,
    pub height: f64,
    pub ending: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Item {
    pub id: NodeId,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub visible: bool,
    pub interactive: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub height: f64,
    /// Oldest to newest: paint later entries in front. All mounted IDs remain
    /// represented, including nonpainted layers, without owning their resources.
    pub items: Vec<Item>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidParameters,
    InvalidMeasurements,
}

/// Local stack coordinates. The caller composes placement and scrolling before
/// retargeting motion, and clips to the usable native viewport.
pub fn layout(
    cards: &[Card],
    width: f64,
    layering: Layering,
    expanded: bool,
    bottom: bool,
) -> Result<Layout, Error> {
    if !layering.is_valid() {
        return Err(Error::InvalidParameters);
    }
    let mut ids = BTreeSet::new();
    if cards.len() > 32
        || !width.is_finite()
        || !(0.0..=1_000_000.).contains(&width)
        || cards.iter().any(|c| {
            !ids.insert(c.id) || !c.height.is_finite() || !(0.0..=1_000_000.).contains(&c.height)
        })
    {
        return Err(Error::InvalidMeasurements);
    }
    let height = if expanded {
        cards.iter().map(|c| c.height).sum::<f64>()
            + layering.gap * cards.len().saturating_sub(1) as f64
    } else {
        cards
            .iter()
            .rev()
            .take(layering.visible)
            .enumerate()
            .map(|(rank, c)| c.height + layering.peek * rank as f64)
            .fold(0., f64::max)
    };
    let mut newer = 0.;
    let mut items = cards
        .iter()
        .rev()
        .enumerate()
        .map(|(rank, c)| {
            let visible = expanded || rank < layering.visible;
            let inset = if expanded {
                0.
            } else {
                width * layering.width_step * rank.min(layering.visible - 1) as f64 / 2.
            };
            let offset = if expanded {
                newer
            } else {
                layering.peek * rank as f64
            };
            newer += c.height + layering.gap;
            Item {
                id: c.id,
                x: inset,
                y: if bottom {
                    height - c.height - offset
                } else {
                    offset
                },
                width: width - 2. * inset,
                height: c.height,
                visible,
                interactive: visible
                    && !c.ending
                    && (expanded || rank == 0)
                    && width > 0.
                    && c.height > 0.,
            }
        })
        .collect::<Vec<_>>();
    items.reverse();
    Ok(Layout { height, items })
}
