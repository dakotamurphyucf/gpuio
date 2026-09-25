//! Window-owned canvas leases and disposable native presentations. Native state
//! survives presentation eviction; jobs and paint callbacks never enter OCaml.
use super::*;
use crate::{
    canvas_content::{Content, Status},
    canvas_host, canvas_jobs,
    canvas_paint::{self, FrameBudget, Placement},
    canvas_plan::Quality,
    canvas_state,
    canvas_store::{Lease, Snapshot},
};
use gpuio_protocol::{
    HandlerId, ResourceId,
    canvas::Point,
    canvas_scene::Drawing,
    canvas_view::{Config, Error, Observation},
};

pub(super) struct State {
    node: NodeId,
    window: WindowId,
    config: Arc<Config>,
    handler: Option<HandlerId>,
    revision: i64,
    lease: Option<Lease>,
    native: Option<canvas_state::State>,
    job: Option<canvas_host::Handle>,
    requested: Option<(Arc<Snapshot>, Quality)>,
    ready: Option<canvas_jobs::Ready>,
    content: Option<Content>,
    failure: Option<Error>,
    reported: Option<(i64, i64, Error)>,
    reported_preparation: Option<(i64, i64, Error)>,
    closed: bool,
    transport: std::sync::Weak<Transport>,
    session: std::rc::Weak<RefCell<Session>>,
}
impl State {
    fn emit(&mut self, observations: Vec<Observation>, cx: &mut App) {
        let (revision, generation) = self.native.as_ref().map_or((0, 0), |native| {
            (native.snapshot().revision, native.snapshot().generation)
        });
        self.emit_at(observations, revision, generation, cx);
    }
    fn emit_at(
        &mut self,
        observations: Vec<Observation>,
        revision: i64,
        generation: i64,
        cx: &mut App,
    ) {
        if self.closed {
            return;
        }
        let Some(handler) = self.handler else {
            return;
        };
        let Some(transport) = self.transport.upgrade() else {
            return;
        };
        for observation in observations {
            if !transport.input(Event::CanvasEvent(
                self.window,
                self.node,
                handler,
                self.revision,
                self.config.source,
                revision,
                generation,
                observation,
            )) {
                // Paint holds an immutable retained-tree borrow. Defer any
                // overload mutation until that borrow has ended.
                let session = self.session.clone();
                let transport = self.transport.clone();
                let window = self.window;
                cx.defer(move |_| {
                    if let (Some(session), Some(transport)) =
                        (session.upgrade(), transport.upgrade())
                        && session.borrow_mut().overload(window)
                    {
                        transport.fault(window);
                    }
                });
                break;
            }
        }
    }
    fn report(&mut self, error: Error, cx: &mut App) {
        let (revision, generation) = self.native.as_ref().map_or((0, 0), |native| {
            (native.snapshot().revision, native.snapshot().generation)
        });
        self.report_at(error, revision, generation, false, cx);
    }
    fn report_at(
        &mut self,
        error: Error,
        revision: i64,
        generation: i64,
        preparation: bool,
        cx: &mut App,
    ) {
        let stamp = (revision, generation, error);
        // A failed replacement and an older frame's content error can coexist.
        // Keep their reports separate so alternating failures never replay.
        let reported = if preparation {
            &mut self.reported_preparation
        } else {
            &mut self.reported
        };
        if *reported != Some(stamp) {
            *reported = Some(stamp);
            self.emit_at(vec![Observation::Failed(error)], revision, generation, cx);
        }
    }
    fn suspend(&mut self) {
        if let Some(native) = &mut self.native {
            native.set_input_enabled(false);
        }
        self.job = None;
        self.requested = None;
        self.ready = None;
        self.content = None;
        self.failure = None;
    }
    pub(super) fn close(&mut self) {
        self.closed = true;
        self.suspend();
        self.native = None;
        self.lease = None;
    }
    fn configure(&mut self, node: &crate::tree::Node, revision: i64, cx: &mut App) {
        if self.handler != node.handler {
            self.reported = None;
            self.reported_preparation = None;
        }
        self.handler = node.handler;
        self.revision = revision;
        self.config = node.canvas.clone().expect("validated canvas");
        if let Some(native) = &mut self.native {
            match native.configure((*self.config).clone()) {
                Ok(events) => self.emit(events, cx),
                Err(error) => self.report(error, cx),
            }
        }
    }
    fn prepare(&mut self, window: &mut Window, cx: &mut App) {
        if self.closed {
            return;
        }
        let Some(lease) = &self.lease else {
            self.report(
                if self.config.source.is_none() {
                    Error::WrongApplication
                } else {
                    Error::UnavailableScene
                },
                cx,
            );
            return;
        };
        let snapshot = lease.snapshot();
        let Some(native) = &self.native else {
            return;
        };
        let zoom = if snapshot.generation != native.snapshot().generation {
            self.config.initial_viewport.zoom
        } else {
            native.viewport().zoom
        };
        let quality = match Quality::new(zoom, f64::from(window.scale_factor())) {
            Ok(quality) => quality,
            Err(_) => {
                self.report_at(
                    Error::RenderLimit,
                    snapshot.revision,
                    snapshot.generation,
                    true,
                    cx,
                );
                return;
            }
        };
        let changed = self.requested.as_ref().is_none_or(|(old, old_quality)| {
            !Arc::ptr_eq(old, &snapshot) || *old_quality != quality
        });
        if changed {
            self.failure = None;
            self.reported = None;
            self.reported_preparation = None;
            self.requested = Some((snapshot.clone(), quality));
            let request = canvas_jobs::Request {
                observer: None,
                snapshot,
                quality,
            };
            let result = match &self.job {
                Some(job) => job.update(request, cx),
                None => canvas_host::request(request, window, cx).map(|job| self.job = Some(job)),
            };
            if let Err(error) = result {
                self.failure = Some(job_error(error));
            }
        }
        if let Some(result) = self.job.as_ref().and_then(canvas_host::Handle::take_ready) {
            match result {
                Ok(ready) => {
                    let native = self.native.as_mut().expect("acquired canvas state");
                    match native.publish(ready.snapshot.clone()) {
                        Ok(events) => {
                            match &mut self.content {
                                Some(content) => content.publish(ready.snapshot.clone()),
                                None => {
                                    self.content = Some(Content::new(ready.snapshot.clone(), cx))
                                }
                            }
                            self.ready = Some(ready);
                            self.emit(events, cx);
                        }
                        Err(error) => self.failure = Some(error),
                    }
                }
                Err(error) => self.failure = Some(job_error(error)),
            }
        }
        if let Some(error) = self.failure {
            let (snapshot, _) = self.requested.as_ref().expect("requested preparation");
            // A failed replacement has not become displayed state. Attribute
            // its failure to the requested publication so Eio can accept it.
            self.report_at(error, snapshot.revision, snapshot.generation, true, cx);
        }
    }
    fn paint(
        &mut self,
        bounds: Bounds<gpui::Pixels>,
        budget: &mut FrameBudget,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.prepare(window, cx);
        if self.closed {
            return;
        }
        let (Some(ready), Some(native), Some(content)) =
            (&self.ready, &self.native, &mut self.content)
        else {
            return;
        };
        content.begin_frame();
        let result = paint(ready, native, content, bounds, budget, window, cx);
        content.end_frame();
        match result {
            Ok(true) => window.refresh(), // bounded deferred shaping; images wake independently
            Ok(false) => (),
            Err(error) => self.report(error, cx),
        }
    }
}
fn job_error(error: canvas_jobs::Error) -> Error {
    match error {
        canvas_jobs::Error::LimitExceeded
        | canvas_jobs::Error::Preparation(
            crate::canvas_plan::Error::LimitExceeded | crate::canvas_plan::Error::InvalidScale,
        ) => Error::RenderLimit,
        canvas_jobs::Error::Preparation(_)
        | canvas_jobs::Error::Closed
        | canvas_jobs::Error::Cancelled => Error::NativeFailure,
    }
}
fn paint(
    ready: &canvas_jobs::Ready,
    native: &canvas_state::State,
    content: &mut Content,
    bounds: Bounds<gpui::Pixels>,
    budget: &mut FrameBudget,
    window: &mut Window,
    cx: &mut App,
) -> Result<bool, Error> {
    let mut deferred = false;
    for (item, meshes) in ready.snapshot.scene.items.iter().zip(&ready.plan.shapes) {
        let placement = Placement {
            origin: meshes
                .as_ref()
                .map_or(Point { x: 0., y: 0. }, |meshes| meshes.origin),
            transform: native.transform(item),
            viewport: native.viewport(),
            bounds,
            clips: &item.clips,
        };
        if let Drawing::Shape(_, paint) = item.drawing {
            let meshes = meshes.as_ref().expect("prepared shape");
            if let (Some(mesh), Some(color)) = (&meshes.fill, paint.fill) {
                canvas_paint::paint(mesh.mesh(), &placement, color as u32, budget, window)
                    .map_err(|_| Error::RenderLimit)?;
            }
            if let (Some(mesh), Some(stroke)) = (&meshes.stroke, paint.stroke) {
                canvas_paint::paint(mesh.mesh(), &placement, stroke.color as u32, budget, window)
                    .map_err(|_| Error::RenderLimit)?;
            }
        } else {
            deferred |= matches!(
                content.paint(&item.drawing, &placement, budget, window, cx)?,
                Status::Deferred
            );
        }
    }
    Ok(deferred)
}
impl View {
    pub(super) fn sync_canvases(&mut self, dirty: &[NodeId], cx: &mut Context<Self>) {
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.id) else {
            for state in self.canvases.values() {
                state.borrow_mut().close();
            }
            self.canvases.clear();
            return;
        };
        self.canvases.retain(|id, state| {
            let keep = tree
                .get(*id)
                .and_then(|n| n.canvas.as_ref())
                .is_some_and(|config| config.source == state.borrow().config.source);
            if !keep {
                state.borrow_mut().close();
            }
            keep
        });
        for id in dirty {
            let Some(node) = tree.get(*id).filter(|node| node.canvas.is_some()) else {
                continue;
            };
            if !self.canvases.contains_key(id) {
                let config = node.canvas.clone().unwrap();
                let lease = config.source.and_then(|id| session.canvas(id).ok());
                let mut state = State {
                    node: *id,
                    window: self.id,
                    config: config.clone(),
                    handler: node.handler,
                    revision: tree.revision(),
                    lease,
                    native: None,
                    job: None,
                    requested: None,
                    ready: None,
                    content: None,
                    failure: None,
                    reported: None,
                    reported_preparation: None,
                    closed: false,
                    transport: Arc::downgrade(&self.transport),
                    session: Rc::downgrade(&self.session),
                };
                if let Some(lease) = &state.lease {
                    match canvas_state::State::new((*config).clone(), lease.snapshot()) {
                        Ok((native, events)) => {
                            state.native = Some(native);
                            state.emit(events, cx);
                        }
                        Err(error) => state.report(error, cx),
                    }
                }
                self.canvases.insert(*id, Rc::new(RefCell::new(state)));
            } else {
                self.canvases[id]
                    .borrow_mut()
                    .configure(node, tree.revision(), cx);
            }
        }
        for (id, state) in &self.canvases {
            if !self.focus.borrow().visible(*id) {
                state.borrow_mut().suspend();
            } else if (!self.focus.borrow().allows(*id) || !pointer_enabled(tree, *id))
                && let Some(native) = &mut state.borrow_mut().native
            {
                native.set_input_enabled(false);
            }
        }
    }
    pub(super) fn hide_unvisited_canvases(&self) {
        for (id, state) in &self.canvases {
            if !self.visited.contains(id) {
                state.borrow_mut().suspend();
            }
        }
    }
    pub(super) fn canvas_changed(&self, source: ResourceId, cx: &mut Context<Self>) {
        for (id, state) in &self.canvases {
            let mut state = state.borrow_mut();
            if state.config.source == Some(source) {
                if let Some(native) = &mut state.native {
                    native.cancel();
                }
                self.invalidate_resource_row(*id);
            }
        }
        cx.notify();
    }
    pub(super) fn canvas_element(
        &mut self,
        node: &crate::tree::Node,
        interaction: Interaction,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        self.visited.insert(node.id);
        let identity = (node.id.generation() as u64) << 32 | node.id.slot() as u64;
        let config = node.canvas.as_ref().expect("validated canvas");
        let element = div()
            .id(("gpuio-canvas", identity))
            .role(gpui::Role::Group)
            .aria_label(config.label.clone());
        let (element, _) = apply_styles(element, &node.style, interaction, config.disabled);
        let Some(state) = self.canvases.get(&node.id).cloned() else {
            return element.into_any_element();
        };
        let budget = self.canvas_budget.clone();
        element
            .relative()
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
#[path = "canvas_view_test.rs"]
pub(crate) mod test;
