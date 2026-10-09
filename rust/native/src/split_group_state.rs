//! Retained stable-ID preferences and one captured gesture. The host owns capture,
//! visibility, event routing and widget lifetimes; this model owns no GPUI handle.
use crate::split_group_geometry::{self as geometry, Layout};
use gpuio_protocol::split_group::{Config, ResizeRequest, Snapshot, Source};
use std::sync::Arc;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidConfig,
    DecreasingGeneration,
    InvalidGeometry,
}
impl From<geometry::Error> for Error {
    fn from(_: geometry::Error) -> Self {
        Self::InvalidGeometry
    }
}
struct Drag {
    after: String,
    baseline: Vec<f64>,
    preview: Vec<f64>,
}
pub struct Measured {
    pub layout: Layout,
    pub observation: Option<Snapshot>,
    pub cancelled_drag: bool,
}
pub struct State {
    config: Arc<Config>,
    preferences: Vec<Option<f64>>,
    extent: Option<f64>,
    drag: Option<Drag>,
    pending: Option<ResizeRequest>,
    last_serial: i64,
}
impl State {
    pub fn new(config: Arc<Config>) -> Result<Self, Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        let preferences = config.panels.iter().map(|p| p.initial_size).collect();
        let pending = config
            .resize
            .as_ref()
            .filter(|r| config.panels.iter().any(|p| p.id == r.id))
            .cloned();
        let last_serial = config.resize.as_ref().map_or(0, |r| r.serial);
        Ok(Self {
            config,
            preferences,
            extent: None,
            drag: None,
            pending,
            last_serial,
        })
    }
    /// Immutable accepted metadata; renderers clone the Arc without copying labels.
    pub fn config(&self) -> &Arc<Config> {
        &self.config
    }
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }
    pub fn pending_serial(&self) -> Option<i64> {
        self.pending.as_ref().map(|r| r.serial)
    }
    /// Atomic accepted policy update. Returns whether native capture must be
    /// released. Labels and seed-only updates do not disturb a running drag.
    pub fn reconcile(&mut self, config: Arc<Config>) -> Result<bool, Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        if config.reset_generation < self.config.reset_generation {
            return Err(Error::DecreasingGeneration);
        }
        let reset = config.axis != self.config.axis
            || config.reset_generation != self.config.reset_generation;
        let structural = self.config.panels.len() != config.panels.len()
            || self.config.panels.iter().zip(&config.panels).any(|(a, b)| {
                a.id != b.id
                    || a.visible != b.visible
                    || a.minimum_size != b.minimum_size
                    || a.maximum_size != b.maximum_size
            });
        let new_request = config
            .resize
            .as_ref()
            .is_some_and(|r| r.serial > self.last_serial);
        let cancelled = (reset || structural || new_request) && self.cancel_drag();
        let preferences = config
            .panels
            .iter()
            .map(|p| {
                let old = (!reset)
                    .then(|| self.config.panels.iter().position(|old| old.id == p.id))
                    .flatten();
                old.map_or(p.initial_size, |i| self.preferences[i])
                    .map(|size| size.clamp(p.minimum_size, p.maximum_size))
            })
            .collect();
        if config.resize.is_none() {
            self.pending = None;
        }
        if new_request {
            let request = config.resize.as_ref().unwrap();
            self.last_serial = request.serial;
            self.pending = Some(request.clone());
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|r| !config.panels.iter().any(|p| p.id == r.id))
        {
            self.pending = None;
        }
        if reset || structural {
            self.extent = None;
        }
        self.config = config;
        self.preferences = preferences;
        Ok(cancelled)
    }
    pub fn cancel_drag(&mut self) -> bool {
        self.drag.take().is_some()
    }
    fn current_sizes(&self) -> Vec<f64> {
        self.config
            .panels
            .iter()
            .zip(&self.preferences)
            .map(|(p, size)| size.unwrap_or(p.minimum_size))
            .collect()
    }
    fn store_visible(&mut self, sizes: &[f64]) {
        for (i, p) in self.config.panels.iter().enumerate() {
            if p.visible {
                self.preferences[i] = Some(sizes[i]);
            }
        }
    }
    fn snapshot(&self, source: Source) -> Snapshot {
        Snapshot {
            source,
            sizes: self
                .config
                .panels
                .iter()
                .zip(self.current_sizes())
                .map(|(p, size)| (p.id.clone(), size))
                .collect(),
        }
    }
    /// No usable measurement preserves preferences and cancels any gesture.
    /// A returned observation is a completed programmatic request; the host must
    /// deliver it asynchronously with current owner/reset/collection fences.
    pub fn measure(&mut self, extent: f64) -> Result<Option<Measured>, Error> {
        if !extent.is_finite() || extent < 0. {
            return Err(Error::InvalidGeometry);
        }
        if extent == 0. {
            self.cancel_drag();
            self.extent = None;
            return Ok(None);
        }
        let cancelled_drag =
            self.extent.is_some_and(|previous| previous != extent) && self.cancel_drag();
        let mut layout = geometry::fit(&self.config.panels, &self.preferences, extent)?;
        self.extent = Some(extent);
        self.store_visible(&layout.sizes);
        if let Some(drag) = &self.drag {
            layout.sizes = drag.preview.clone();
        }
        let ready = self
            .pending
            .as_ref()
            .is_some_and(|r| self.config.panels.iter().any(|p| p.id == r.id && p.visible));
        let observation = if ready {
            let request = self.pending.take().unwrap();
            self.cancel_drag();
            layout.sizes = geometry::resize_panel(
                &self.config.panels,
                &layout.sizes,
                &request.id,
                request.size,
            )?;
            self.store_visible(&layout.sizes);
            Some(self.snapshot(Source::Request(request.serial)))
        } else {
            None
        };
        Ok(Some(Measured {
            layout,
            observation,
            cancelled_drag,
        }))
    }
    pub fn begin_drag(&mut self, after: &str) -> Result<(), Error> {
        if self.extent.is_none() {
            return Err(Error::InvalidGeometry);
        }
        let baseline = self.current_sizes();
        geometry::boundary_range(&self.config.panels, &baseline, after)?;
        self.drag = Some(Drag {
            after: after.into(),
            preview: baseline.clone(),
            baseline,
        });
        Ok(())
    }
    pub fn drag_to(&mut self, delta: f64) -> Result<(), Error> {
        let drag = self.drag.as_mut().ok_or(Error::InvalidGeometry)?;
        drag.preview =
            geometry::move_boundary(&self.config.panels, &drag.baseline, &drag.after, delta)?;
        Ok(())
    }
    pub fn finish_drag(&mut self) -> Option<Snapshot> {
        let drag = self.drag.take()?;
        if drag.preview == drag.baseline {
            return None;
        }
        self.store_visible(&drag.preview);
        Some(self.snapshot(Source::Pointer))
    }
    pub fn adjust_boundary(
        &mut self,
        after: &str,
        delta: f64,
        source: Source,
    ) -> Result<Option<Snapshot>, Error> {
        if self.extent.is_none() || !matches!(source, Source::Keyboard | Source::Accessibility) {
            return Err(Error::InvalidGeometry);
        }
        let current = self.current_sizes();
        let next = geometry::move_boundary(&self.config.panels, &current, after, delta)?;
        self.cancel_drag();
        if current == next {
            return Ok(None);
        }
        self.store_visible(&next);
        Ok(Some(self.snapshot(source)))
    }
    pub fn boundary_range(&self, after: &str) -> Result<(f64, f64, f64), Error> {
        if self.extent.is_none() {
            return Err(Error::InvalidGeometry);
        }
        Ok(geometry::boundary_range(
            &self.config.panels,
            &self
                .drag
                .as_ref()
                .map_or_else(|| self.current_sizes(), |drag| drag.preview.clone()),
            after,
        )?)
    }
}
