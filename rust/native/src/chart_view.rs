//! Mounted chart state; preparation and publication never invoke OCaml callbacks.
use super::*;
use crate::{
    chart_jobs as jobs, chart_paint as paint, chart_render_host as renderer, chart_store::Lease,
};
use gpuio_protocol::{
    HandlerId, ResourceId,
    chart_view::{Config, Error, Metrics, Observation},
};

pub(super) struct State {
    node: NodeId,
    window: WindowId,
    config: Arc<Config>,
    handler: Option<HandlerId>,
    revision: i64,
    lease: Option<Lease>,
    job: Option<renderer::Handle>,
    requested: Option<jobs::Request>,
    ready: Option<jobs::Ready>,
    failure: Option<Error>,
    reported_ready: Option<(i64, i64)>,
    reported_failure: Option<(i64, i64, Error)>,
    closed: bool,
    transport: std::sync::Weak<Transport>,
    session: std::rc::Weak<RefCell<Session>>,
}
impl State {
    fn emit(&self, revision: i64, generation: i64, observation: Observation, cx: &mut App) {
        let (Some(handler), Some(transport)) = (self.handler, self.transport.upgrade()) else {
            return;
        };
        if self.closed {
            return;
        }
        if !transport.input(Event::ChartEvent(
            self.window,
            self.node,
            handler,
            self.revision,
            self.config.source,
            revision,
            generation,
            observation,
        )) {
            let session = self.session.clone();
            let transport = self.transport.clone();
            let window = self.window;
            cx.defer(move |_| {
                if let (Some(session), Some(transport)) = (session.upgrade(), transport.upgrade())
                    && session.borrow_mut().overload(window)
                {
                    transport.fault(window);
                }
            });
        }
    }
    fn report(&mut self, revision: i64, generation: i64, error: Error, cx: &mut App) {
        let stamp = (revision, generation, error);
        if self.reported_failure != Some(stamp) {
            self.reported_failure = Some(stamp);
            self.emit(revision, generation, Observation::Failed(error), cx);
        }
    }
    fn suspend(&mut self) {
        self.job = None;
        self.requested = None;
        self.ready = None;
        self.lease = None;
        self.failure = None;
        self.reported_ready = None;
        self.reported_failure = None;
    }
    pub(super) fn close(&mut self) {
        self.closed = true;
        self.suspend();
    }
    fn configure(&mut self, node: &crate::tree::Node, revision: i64) {
        let config = node.chart.as_ref().expect("validated chart");
        if self.handler != node.handler || self.config != *config {
            self.reported_ready = None;
            self.reported_failure = None;
        }
        self.config = config.clone();
        self.handler = node.handler;
        self.revision = revision;
    }
    /// Polling the live lease also fences a completion delivered in the same UI
    /// turn as a release/reset; source notifications provide immediate idle cleanup.
    fn refresh_source(&mut self) -> Option<Arc<crate::chart_store::Snapshot>> {
        if self.closed {
            return None;
        }
        if self.lease.is_none() {
            self.lease = self.config.source.and_then(|source| {
                self.session
                    .upgrade()
                    .and_then(|session| session.borrow().chart(source).ok())
            });
        }
        let snapshot = self.lease.as_ref().and_then(Lease::snapshot);
        let changed = self.requested.as_ref().is_some_and(|old| {
            snapshot
                .as_ref()
                .is_none_or(|new| !Arc::ptr_eq(new, &old.snapshot))
        });
        if changed {
            self.job = None;
            self.requested = None;
            self.failure = None;
            self.reported_failure = None;
        }
        if self.ready.as_ref().is_some_and(|old| {
            snapshot
                .as_ref()
                .is_none_or(|new| new.generation() != old.snapshot.generation())
        }) {
            self.ready = None;
            self.reported_ready = None;
        }
        if snapshot.is_none() {
            self.lease = None;
        }
        snapshot
    }
    fn prepare(&mut self, layout: paint::Layout, window: &mut Window, cx: &mut App) {
        let Some(snapshot) = self.refresh_source() else {
            self.report(
                0,
                0,
                if self.config.source.is_none() {
                    Error::WrongApplication
                } else {
                    Error::UnavailableData
                },
                cx,
            );
            return;
        };
        let request = jobs::Request {
            observer: None,
            snapshot: snapshot.clone(),
            config: self.config.clone(),
            layout,
        };
        let changed = self.requested.as_ref().is_none_or(|old| {
            !Arc::ptr_eq(&old.snapshot, &snapshot)
                || old.layout != layout
                || old.config.options != self.config.options
                || old.config.sampling != self.config.sampling
                || old.config.style != self.config.style
        });
        if changed {
            self.reported_failure = None;
            self.failure = None;
            let result = match &self.job {
                Some(job) => job.update(request.clone(), cx),
                None => {
                    renderer::request(request.clone(), window, cx).map(|job| self.job = Some(job))
                }
            };
            self.requested = Some(request);
            if let Err(error) = result {
                self.failure = Some(job_error(error));
            }
        }
        if let Some(result) = self.job.as_ref().and_then(renderer::Handle::take_ready) {
            match result {
                Ok(ready) => {
                    self.ready = Some(ready);
                    self.reported_ready = None;
                }
                Err(error) => self.failure = Some(job_error(error)),
            }
        }
        if let Some(error) = self.failure {
            self.report(snapshot.revision(), snapshot.generation(), error, cx);
        }
        if let Some(ready) = &self.ready {
            let stamp = (ready.snapshot.revision(), ready.snapshot.generation());
            if self.reported_ready != Some(stamp) {
                let metrics = Metrics {
                    source_values: ready.plan.geometry().source_values as i64,
                    retained_values: ready.plan.geometry().rendered_values as i64,
                    mesh_vertices: ready.plan.vertices() as i64,
                    quads: ready.plan.quad_count() as i64,
                    bytes: ready.plan.retained_bytes() as i64,
                };
                self.reported_ready = Some(stamp);
                self.emit(stamp.0, stamp.1, Observation::Ready(metrics), cx);
            }
        }
    }
    fn paint(
        &mut self,
        bounds: Bounds<gpui::Pixels>,
        budget: &mut paint::FrameBudget,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.closed {
            return;
        }
        let layout = match paint::Layout::new(
            f64::from(f32::from(bounds.size.width)),
            f64::from(f32::from(bounds.size.height)),
            f64::from(window.scale_factor()),
        ) {
            Ok(layout) => layout,
            Err(_) => {
                self.report(0, 0, Error::RenderLimit, cx);
                return;
            }
        };
        self.prepare(layout, window, cx);
        if let Some(ready) = &self.ready {
            // During resize retain the old picture at its original logical size;
            // the enclosing element clips it while replacement work is pending.
            let prepared_bounds = Bounds::new(
                bounds.origin,
                gpui::size(
                    px(ready.plan.geometry().width as f32),
                    px(ready.plan.geometry().height as f32),
                ),
            );
            if let Err(error) = ready.plan.paint(prepared_bounds, budget, window) {
                self.report(
                    ready.snapshot.revision(),
                    ready.snapshot.generation(),
                    paint_error(error),
                    cx,
                );
            }
        }
    }
}
fn paint_error(error: paint::Error) -> Error {
    match error {
        paint::Error::InvalidInput | paint::Error::RenderLimit => Error::RenderLimit,
        paint::Error::Cancelled | paint::Error::NativeFailure => Error::NativeFailure,
    }
}
fn job_error(error: jobs::Error) -> Error {
    match error {
        jobs::Error::LimitExceeded => Error::RenderLimit,
        jobs::Error::Preparation(error) => paint_error(error),
        jobs::Error::Cancelled | jobs::Error::Closed => Error::NativeFailure,
    }
}
impl View {
    pub(super) fn sync_charts(&mut self, dirty: &[NodeId]) {
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.id) else {
            for state in self.charts.values() {
                state.borrow_mut().close();
            }
            self.charts.clear();
            return;
        };
        self.charts.retain(|id, state| {
            let keep = tree
                .get(*id)
                .and_then(|n| n.chart.as_ref())
                .is_some_and(|config| config.source == state.borrow().config.source);
            if !keep {
                state.borrow_mut().close();
            }
            keep
        });
        for id in dirty {
            let Some(node) = tree.get(*id).filter(|n| n.chart.is_some()) else {
                continue;
            };
            self.charts
                .entry(*id)
                .or_insert_with(|| {
                    Rc::new(RefCell::new(State {
                        node: *id,
                        window: self.id,
                        config: node.chart.clone().unwrap(),
                        handler: node.handler,
                        revision: tree.revision(),
                        lease: None,
                        job: None,
                        requested: None,
                        ready: None,
                        failure: None,
                        reported_ready: None,
                        reported_failure: None,
                        closed: false,
                        transport: Arc::downgrade(&self.transport),
                        session: Rc::downgrade(&self.session),
                    }))
                })
                .borrow_mut()
                .configure(node, tree.revision());
        }
        for (id, state) in &self.charts {
            if !self.focus.borrow().visible(*id) {
                state.borrow_mut().suspend();
            }
        }
    }
    pub(super) fn hide_unvisited_charts(&self) {
        for (id, state) in &self.charts {
            if !self.visited.contains(id) {
                state.borrow_mut().suspend();
            }
        }
    }
    pub(super) fn charts_changed(
        &self,
        source: Option<ResourceId>,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut changed = false;
        for (id, state) in &self.charts {
            let mut state = state.borrow_mut();
            if source.is_none() || state.config.source == source {
                state.refresh_source();
                self.invalidate_resource_row(*id);
                changed = true;
            }
        }
        if changed {
            cx.notify();
        }
        changed
    }
    pub(super) fn chart_element(
        &mut self,
        node: &crate::tree::Node,
        interaction: Interaction,
    ) -> gpui::AnyElement {
        self.visited.insert(node.id);
        let config = node.chart.as_ref().expect("validated chart");
        let identity = (node.id.generation() as u64) << 32 | node.id.slot() as u64;
        let element = div()
            .id(("gpuio-chart", identity))
            .role(gpui::Role::Group)
            .aria_label(config.label.clone());
        let (element, _) = apply_styles(element, &node.style, interaction, false);
        let Some(state) = self.charts.get(&node.id).cloned() else {
            return element.into_any_element();
        };
        let budget = self.chart_budget.clone();
        element
            .relative()
            .overflow_hidden()
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, cx| {
                        if bounds.size.width <= px(0.)
                            || bounds.size.height <= px(0.)
                            || !bounds.intersects(&window.content_mask().bounds)
                        {
                            state.borrow_mut().suspend();
                            return;
                        }
                        state
                            .borrow_mut()
                            .paint(bounds, &mut budget.borrow_mut(), window, cx);
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .into_any_element()
    }
}

#[cfg(feature = "native-canvas-tests")]
#[path = "chart_view_test.rs"]
pub(crate) mod test;
