//! Measured-track model and layout observations. Payload and admission validation
//! are independent of native measurement and presentation.
use crate::carousel;
use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Loop {
    Finite,
    Jump,
    Continuous,
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Stops {
    pub canonical: Vec<i64>,
    pub looping: Loop,
}
impl Stops {
    pub fn is_valid(&self) -> bool {
        if self.canonical.len() > carousel::MAX_ITEMS {
            return false;
        }
        let mut previous = 0;
        for (index, value) in self.canonical.iter().copied().enumerate() {
            if value != previous && value != index as i64 {
                return false;
            }
            previous = value;
        }
        match self.looping {
            Loop::Finite => true,
            Loop::Jump => self.canonical.len() > 1,
            Loop::Continuous => self.canonical.len() > 1 && self.canonical.iter().any(|v| *v != 0),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Layout {
    pub lineage: i64,
    pub epoch: i64,
    pub stops: Option<Stops>,
}
impl Layout {
    pub fn is_valid(&self) -> bool {
        self.lineage >= 0 && self.epoch >= 0 && self.stops.as_ref().is_none_or(Stops::is_valid)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Config {
    pub carousel: carousel::Config,
    pub lineage: i64,
}
impl Config {
    pub fn is_valid(&self) -> bool {
        self.carousel.is_valid() && self.lineage >= 0 && self.lineage <= self.carousel.revision
    }
    pub fn can_replace(&self, previous: &Self) -> bool {
        self.is_valid()
            && self.carousel.can_replace(&previous.carousel)
            && if self.lineage == previous.lineage {
                self.carousel.ids == previous.carousel.ids
                    && self.carousel.axis == previous.carousel.axis
                    && self.carousel.looping == previous.carousel.looping
            } else {
                self.lineage > previous.lineage
                    && self.carousel.revision > previous.carousel.revision
            }
    }
    pub fn retained_bytes(&self) -> usize {
        8 + self.carousel.retained_bytes()
    }
    /// Admission checks only. The mounted presenter additionally fences geometry
    /// epochs and measured automatic successors before sending a proposal.
    pub fn accepts_request(&self, request: &Request) -> bool {
        if !self.is_valid() || !request.is_valid() {
            return false;
        }
        let enabled = !self.carousel.disabled && !self.carousel.ids.is_empty();
        match request {
            Request::Layout(layout) => self.accepts_layout(layout),
            Request::Previous | Request::Next | Request::First | Request::Last => enabled,
            Request::Select(id) => enabled && self.carousel.ids.contains(id),
            Request::AutoNext(proposal) => {
                enabled
                    && self.carousel.auto_advance_ms.is_some()
                    && proposal.revision == self.carousel.revision
                    && self
                        .carousel
                        .selected
                        .and_then(|i| self.carousel.ids.get(i as usize))
                        == Some(&proposal.from)
                    && self.carousel.ids.contains(&proposal.target)
            }
        }
    }
    pub fn accepts_layout(&self, layout: &Layout) -> bool {
        self.is_valid()
            && layout.is_valid()
            && self.lineage == layout.lineage
            && layout.stops.as_ref().is_none_or(|stops| {
                stops.canonical.len() == self.carousel.ids.len()
                    && (stops.looping == Loop::Finite || self.carousel.looping)
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub struct Proposal {
    pub revision: i64,
    pub geometry_epoch: i64,
    pub from: String,
    pub target: String,
}
impl Proposal {
    pub fn is_valid(&self) -> bool {
        let valid_id = |id: &str| !id.is_empty() && id.len() <= 256 && !id.contains('\0');
        self.revision >= 0
            && self.geometry_epoch >= 0
            && valid_id(&self.from)
            && valid_id(&self.target)
            && self.from != self.target
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Request {
    Previous,
    Next,
    First,
    Last,
    Select(String),
    Layout(Layout),
    AutoNext(Proposal),
}
impl Request {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Previous | Self::Next | Self::First | Self::Last => true,
            Self::Select(id) => !id.is_empty() && id.len() <= 256 && !id.contains('\0'),
            Self::Layout(layout) => layout.is_valid(),
            Self::AutoNext(proposal) => proposal.is_valid(),
        }
    }
}

/// Optional presentation policy. Absence means immediate; model revision is
/// independent of motion style. Progress is bounded to the segment being traveled.
#[derive(Clone, Copy, Debug, PartialEq, BinProtWrite)]
pub struct Motion {
    pub duration_ms: i64,
    pub easing: crate::animation::Easing,
}
impl Motion {
    pub fn is_valid(self) -> bool {
        (1..=10_000).contains(&self.duration_ms) && self.easing.is_valid()
    }
}
