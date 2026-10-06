//! Mounted chart state; preparation and publication never invoke OCaml callbacks.
use super::*;
use crate::{
    chart_jobs as jobs, chart_paint as paint, chart_presentation as presentation,
    chart_render_host as renderer, chart_store::Lease,
};
use gpuio_protocol::{
    HandlerId, ResourceId,
    chart_view::{Config, Error, Metrics, Observation},
};

#[path = "chart_input.rs"]
mod input;

pub(super) struct State {
    input: input::Input,
    node: NodeId,
    window: WindowId,
    config: Arc<Config>,
    handler: Option<HandlerId>,
    revision: i64,
    lease: Option<Lease>,
    job: Option<renderer::Handle>,
    requested: Option<jobs::Request>,
    ready: Option<jobs::Ready>,
    requested_frame: Option<presentation::Frame>,
    ready_frame: Option<presentation::Frame>,
    legend_scroll: gpui::ScrollHandle,
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
    fn suspend(&mut self, window: &mut Window) {
        self.cancel_input(window);
        self.input.clear_selection();
        self.input.data_cursor = None;
        self.input.token = Rc::new(());
        self.job = None;
        self.requested = None;
        self.requested_frame = None;
        self.ready = None;
        self.ready_frame = None;
        self.legend_scroll.set_offset(Default::default());
        self.lease = None;
        self.failure = None;
        self.reported_ready = None;
        self.reported_failure = None;
    }
    pub(super) fn close(&mut self, window: &mut Window) {
        self.closed = true;
        self.suspend(window);
    }
    fn configure(&mut self, node: &crate::tree::Node, revision: i64, window: &mut Window) {
        let config = node.chart.as_ref().expect("validated chart");
        if self.handler != node.handler || self.config != *config {
            self.cancel_input(window);
            self.input.token = Rc::new(());
            self.reported_ready = None;
            self.reported_failure = None;
        }
        self.config = config.clone();
        self.handler = node.handler;
        self.revision = revision;
    }
    /// Polling the live lease also fences a completion delivered in the same UI
    /// turn as a release/reset; source notifications provide immediate idle cleanup.
    fn refresh_source(&mut self, window: &mut Window) -> Option<Arc<crate::chart_store::Snapshot>> {
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
            if snapshot.as_ref().is_none_or(|new| {
                self.requested
                    .as_ref()
                    .is_some_and(|old| new.generation() != old.snapshot.generation())
            }) {
                self.input.data_cursor = None;
            }
            self.cancel_input(window);
            self.input.token = Rc::new(());
            self.job = None;
            self.requested = None;
            self.requested_frame = None;
            self.failure = None;
            self.reported_failure = None;
        }
        if self.ready.as_ref().is_some_and(|old| {
            snapshot
                .as_ref()
                .is_none_or(|new| new.generation() != old.snapshot.generation())
        }) {
            self.input.clear_selection();
            self.input.data_cursor = None;
            self.ready = None;
            self.ready_frame = None;
            self.legend_scroll.set_offset(Default::default());
            self.reported_ready = None;
        }
        if let (Some(cursor), Some(source)) = (self.input.data_cursor, &snapshot) {
            self.input.data_cursor =
                Some(cursor.min(crate::chart_table::count(source.data()).saturating_sub(1)));
        }
        if snapshot.is_none() {
            self.lease = None;
        }
        snapshot
    }
    fn prepare(&mut self, total: paint::Layout, window: &mut Window, cx: &mut App) {
        let Some(snapshot) = self.refresh_source(window) else {
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
        let (width, height, scale) = total.dimensions();
        let frame = presentation::Frame::new(width, height, snapshot.data(), &self.config);
        let layout = paint::Layout::new(frame.plot.width, frame.plot.height, scale)
            .expect("positive bounded chart plot");
        let request = jobs::Request {
            observer: None,
            snapshot: snapshot.clone(),
            config: self.config.clone(),
            layout,
            text: (matches!(
                snapshot.data().contents,
                gpuio_protocol::chart_data::Contents::Sankey(..)
            ) && self.config.options.sankey.labels
                && self.config.options.sankey.label_placement
                    == gpuio_protocol::chart_options::LabelPlacement::Outside)
                .then(|| crate::chart_label_metrics::Context {
                    system: cx.text_system().clone(),
                    style: crate::chart_label_metrics::LabelStyle::new(
                        window.text_style().font(),
                        f32::from(window.rem_size()) * 0.25,
                    ),
                }),
        };
        let changed = self.requested.as_ref().is_none_or(|old| {
            old.text != request.text
                || !Arc::ptr_eq(&old.snapshot, &snapshot)
                || old.layout != layout
                || old.config.options != self.config.options
                || old.config.sampling != self.config.sampling
                || old.config.style != self.config.style
                || old.config.legend != self.config.legend
                || self.requested_frame != Some(frame)
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
            self.requested_frame = Some(frame);
            if let Err(error) = result {
                self.failure = Some(job_error(error));
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
    fn poll_ready(&mut self, window: &mut Window) {
        if let Some(result) = self.job.as_ref().and_then(renderer::Handle::take_ready) {
            match result {
                Ok(ready) => {
                    self.cancel_input(window);
                    self.input.token = Rc::new(());
                    self.input.selected_index = self
                        .input
                        .selected
                        .and_then(|selected| ready.plan.selection_index(selected));
                    self.input.selected = self.input.selected_index.and_then(|index| {
                        crate::chart_selection::resolve(
                            ready.snapshot.data(),
                            &ready.config.sampling,
                            ready.plan.geometry().marks[index].source,
                        )
                    });
                    self.ready = Some(ready);
                    self.ready_frame = self.requested_frame;
                    self.reported_ready = None;
                }
                Err(error) => self.failure = Some(job_error(error)),
            }
        }
    }
    fn text_element(&self, identity: u64) -> Option<gpui::AnyElement> {
        if self.input.data_cursor.is_some() {
            return None;
        }
        let ready = self.ready.as_ref()?;
        let frame = self.ready_frame?;
        let style = &ready.config.style;
        let mut text = div()
            .absolute()
            .top_0()
            .left_0()
            .w(px(frame.width as f32))
            .h(px(frame.height as f32))
            .text_size(px(11.))
            .line_height(px(presentation::TEXT_HEIGHT as f32))
            .text_color(gpui::rgba(style.label_color as u32));
        if let Some(label_style) = &ready.label_style {
            text = text.font(label_style.font.clone());
        }
        let series_names = presentation::legend(ready.snapshot.data());
        for (index, label) in ready.plan.geometry().labels.iter().enumerate() {
            let placement = frame.label(label);
            let r = placement.rect;
            if r.height <= 0. || r.width <= 0. {
                continue;
            }
            let (font_size, foreground) = match label.kind {
                crate::chart_geometry::LabelKind::FlowLine {
                    font_size, color, ..
                } => (font_size, color.unwrap_or(style.label_color as u32)),
                _ => (11., style.label_color as u32),
            };
            let backed = matches!(
                label.kind,
                crate::chart_geometry::LabelKind::Flow { .. }
                    | crate::chart_geometry::LabelKind::FlowLine { .. }
                    | crate::chart_geometry::LabelKind::Series(_)
            ) || (matches!(label.kind, crate::chart_geometry::LabelKind::Radial)
                && matches!(
                    ready.snapshot.data().contents,
                    gpuio_protocol::chart_data::Contents::Pie(_)
                ));
            // Each prepared caption owns one line; wrapping would hide later
            // words behind its fixed-height clipping rectangle.
            let mut content = div().min_w_0().truncate().child(label.text.clone());
            if backed {
                content = if let Some(label_style) = &ready.label_style {
                    content.px(px(label_style.padding))
                } else {
                    content.px_1()
                };
                content = content
                    .rounded_sm()
                    .bg(gpui::rgba(presentation::label_backing(foreground)));
            }
            let element = div()
                .id(("gpuio-chart-label", index as u64))
                .role(gpui::Role::Label)
                .absolute()
                .left(px(r.x as f32))
                .top(px(r.y as f32))
                .w(px(r.width as f32))
                .h(px(r.height as f32))
                .text_size(px(font_size as f32))
                .line_height(px((font_size + 4.).max(presentation::TEXT_HEIGHT) as f32))
                .text_color(gpui::rgba(foreground))
                .truncate()
                .aria_label(match label.kind {
                    crate::chart_geometry::LabelKind::Series(series) => {
                        format!("Series {} · {}", series + 1, series_names[series])
                    }
                    _ => label.text.clone(),
                })
                .flex()
                .child(content);
            let element = match placement.align {
                presentation::Align::Left => element.justify_start(),
                presentation::Align::Right => element.justify_end(),
                presentation::Align::Center => element.justify_center(),
            };
            text = text.child(element);
        }
        if frame.legend.height > 0. {
            let names = presentation::legend(ready.snapshot.data());
            let width = frame.width / frame.legend_columns as f64;
            let mut legend = div()
                .id(("gpuio-chart-legend", identity))
                .absolute()
                .left_0()
                .top(px(frame.legend.y as f32))
                .w(px(frame.width as f32))
                .h(px(frame.legend.height as f32))
                .overflow_y_scroll()
                .track_scroll(&self.legend_scroll)
                .role(gpui::Role::Group)
                .aria_label("Chart legend");
            for (row_number, row) in names.chunks(frame.legend_columns).enumerate() {
                let row_index = row_number * frame.legend_columns;
                let mut items = div()
                    .flex()
                    .h(px(presentation::LEGEND_ROW as f32))
                    .flex_shrink_0();
                for (column, name) in row.iter().enumerate() {
                    let index = row_index + column;
                    items = items.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .w(px(width as f32))
                            .h_full()
                            .px_1()
                            .overflow_hidden()
                            .child(
                                div()
                                    .size(px(8.))
                                    .flex_shrink_0()
                                    .rounded_sm()
                                    .bg(gpui::rgba(ready.plan.series_color(index))),
                            )
                            .child(
                                div()
                                    .id(("gpuio-chart-legend-name", index as u64))
                                    .role(gpui::Role::Label)
                                    .min_w_0()
                                    .text_ellipsis()
                                    .aria_label((*name).to_owned())
                                    .child(format!("{} · {}", index + 1, name)),
                            ),
                    );
                }
                legend = legend.child(items);
            }
            text = text.child(legend);
        }
        Some(text.into_any_element())
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
        if self.input.data_cursor.is_some() {
            return;
        }
        if let (Some(ready), Some(frame)) = (&self.ready, self.ready_frame) {
            // During resize retain the old picture at its original logical size;
            // the enclosing element clips it while replacement work is pending.
            let prepared_bounds = Bounds::new(
                bounds.origin + gpui::point(px(frame.plot.x as f32), px(frame.plot.y as f32)),
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
    pub(super) fn sync_charts(
        &mut self,
        dirty: &[NodeId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.id) else {
            for state in self.charts.values() {
                state.borrow_mut().close(window);
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
                state.borrow_mut().close(window);
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
                    let state = Rc::new(RefCell::new(State {
                        input: input::Input::new(self.focus.clone(), cx),
                        node: *id,
                        window: self.id,
                        config: node.chart.clone().unwrap(),
                        handler: node.handler,
                        revision: tree.revision(),
                        lease: None,
                        job: None,
                        requested: None,
                        ready: None,
                        requested_frame: None,
                        ready_frame: None,
                        legend_scroll: Default::default(),
                        failure: None,
                        reported_ready: None,
                        reported_failure: None,
                        closed: false,
                        transport: Arc::downgrade(&self.transport),
                        session: Rc::downgrade(&self.session),
                    }));
                    input::install_blur(&state, window, cx);
                    state
                })
                .borrow_mut()
                .configure(node, tree.revision(), window);
        }
        for (id, state) in &self.charts {
            if !self.focus.borrow().visible(*id) {
                state.borrow_mut().suspend(window);
            } else {
                if !self.focus.borrow().allows(*id) || !pointer_enabled(tree, *id) {
                    state.borrow_mut().cancel_input(window);
                }
                // Original-data access must also work when mesh admission fails
                // immediately and cannot trigger a worker completion redraw.
                state.borrow_mut().refresh_source(window);
            }
        }
    }
    pub(super) fn hide_unvisited_charts(&self, window: &mut Window) {
        for (id, state) in &self.charts {
            if !self.visited.contains(id) {
                state.borrow_mut().suspend(window);
            }
        }
    }
    pub(super) fn charts_changed(
        &self,
        source: Option<ResourceId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut changed = false;
        for (id, state) in &self.charts {
            let mut state = state.borrow_mut();
            if source.is_none() || state.config.source == source {
                state.refresh_source(window);
                state.poll_ready(window);
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
        let (element, _) = apply_styles(element, &node.style, interaction, config.disabled);
        let Some(state) = self.charts.get(&node.id).cloned() else {
            return element.into_any_element();
        };
        let text = state.borrow().text_element(identity);
        let overlay = state.borrow().input_overlay();
        let data_view = input::data_element(state.clone());
        let element = input::keyboard(element, state.clone());
        let prepaint = state.clone();
        let budget = self.chart_budget.clone();
        let element = element
            .relative()
            .overflow_hidden()
            .child(
                canvas(
                    move |bounds, window, _| input::prepaint(&prepaint, bounds, window),
                    move |bounds, hitbox, window, cx| {
                        if bounds.size.width <= px(0.)
                            || bounds.size.height <= px(0.)
                            || !bounds.intersects(&window.content_mask().bounds)
                        {
                            state.borrow_mut().suspend(window);
                            return;
                        }
                        state
                            .borrow_mut()
                            .paint(bounds, &mut budget.borrow_mut(), window, cx);
                        input::paint(&state, hitbox, window);
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .children(text)
            .children(overlay)
            .children(data_view);
        crate::semantics::State {
            identity: None,
            busy: false,
            hidden: false,
            metadata: None,
            element,
            disabled: config.disabled,
            read_only: false,
            modal: false,
            live: None,
        }
        .into_any_element()
    }
}

#[cfg(feature = "native-canvas-tests")]
#[path = "chart_view_test.rs"]
pub(crate) mod test;
