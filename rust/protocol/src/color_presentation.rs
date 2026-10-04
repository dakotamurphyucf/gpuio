//! Bounded layout of the existing indexed palette and channel controls.
use binprot::macros::BinProtWrite;

#[derive(Clone, Copy, Debug, PartialEq, Eq, BinProtWrite)]
pub enum Panel {
    Palette,
    Channels,
}
#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub enum Panels {
    All,
    Tabs {
        palette_label: String,
        channels_label: String,
        initial: Panel,
    },
}
impl Panels {
    pub fn initial(&self) -> Panel {
        match self {
            Self::All => Panel::Palette,
            Self::Tabs { initial, .. } => *initial,
        }
    }
    pub fn is_tabbed(&self) -> bool {
        matches!(self, Self::Tabs { .. })
    }
    pub fn shows(&self, active: Panel, panel: Panel) -> bool {
        !self.is_tabbed() || active == panel
    }
    fn is_valid(&self) -> bool {
        match self {
            Self::All => true,
            Self::Tabs {
                palette_label,
                channels_label,
                ..
            } => [palette_label, channels_label]
                .into_iter()
                .all(|s| crate::color_input::valid_label(s, 256)),
        }
    }
    fn label_bytes(&self) -> usize {
        match self {
            Self::All => 0,
            Self::Tabs {
                palette_label,
                channels_label,
                ..
            } => palette_label.len() + channels_label.len(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Section {
    pub featured: bool,
    pub label: String,
    pub count: i64,
}

#[derive(Clone, Debug, PartialEq, BinProtWrite)]
pub struct Presentation {
    pub sections: Vec<Section>,
    pub swatch_size: f64,
    pub featured_size: f64,
    pub swatch_gap: f64,
    pub section_gap: f64,
    pub swatch_radius: f64,
    pub outline_width: f64,
    pub channel_height: f64,
    pub control_gap: f64,
    pub padding: f64,
    pub selected_border: Option<i64>,
    pub hover_border: Option<i64>,
    pub panels: Panels,
}
impl Default for Presentation {
    fn default() -> Self {
        Self {
            sections: vec![],
            swatch_size: 28.,
            featured_size: 36.,
            swatch_gap: 6.,
            section_gap: 10.,
            swatch_radius: 5.,
            outline_width: 2.,
            channel_height: 28.,
            control_gap: 10.,
            padding: 10.,
            selected_border: None,
            hover_border: None,
            panels: Panels::All,
        }
    }
}
impl Presentation {
    pub fn is_valid(&self) -> bool {
        let bounded = |n: f64, low: f64, high: f64| n.is_finite() && (low..=high).contains(&n);
        self.panels.is_valid()
            && self.sections.len() <= 32
            && self.sections.iter().enumerate().all(|(i, s)| {
                (!s.featured || i == 0)
                    && (1..=256).contains(&s.count)
                    && crate::color_input::valid_label(&s.label, 256)
            })
            && self.sections.iter().map(|s| s.count).sum::<i64>() <= 256
            && [self.swatch_size, self.featured_size, self.channel_height]
                .into_iter()
                .all(|n| bounded(n, 16., 128.))
            && [
                self.swatch_gap,
                self.section_gap,
                self.control_gap,
                self.padding,
            ]
            .into_iter()
            .all(|n| bounded(n, 0., 64.))
            && [self.swatch_radius, self.outline_width]
                .into_iter()
                .all(|n| bounded(n, 0., self.swatch_size.min(self.featured_size) / 2.))
            && [self.selected_border, self.hover_border]
                .into_iter()
                .all(|n| n.is_none_or(|n| (0..=0xffff_ffff).contains(&n)))
    }
    pub fn fits(&self, entries: usize) -> bool {
        self.is_valid()
            && (self.sections.is_empty()
                || self
                    .sections
                    .iter()
                    .map(|s| s.count as usize)
                    .sum::<usize>()
                    == entries)
    }
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + std::mem::size_of_val(self.sections.as_slice())
            + self.sections.iter().map(|s| s.label.len()).sum::<usize>()
            + self.panels.label_bytes()
    }
}
