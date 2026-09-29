//! Native-thread-owned window state. No OCaml callbacks or runtime references.
use crate::tree::{Applied, Tree};
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};

#[derive(Clone, Copy, Debug)]
pub struct CommandInvocation<'a> {
    pub scope: NodeId,
    pub handler: HandlerId,
    pub revision: i64,
    pub command: &'a str,
    pub generation: i64,
    pub source: CommandSource,
}

#[derive(Default)]
struct Slot {
    generation: u32,
    window: Option<Window>,
}

struct Window {
    tree: Tree,
    rendered: Option<i64>,
    requested_frame: Option<i64>,
    overloaded: bool,
}

#[derive(Default)]
pub struct Session {
    ready: bool,
    stopped: bool,
    slots: Vec<Slot>,
    retained_bytes: usize,
    assets: crate::asset_store::Store,
    documents: crate::document_store::Store,
    canvases: crate::canvas_store::Store,
    charts: crate::chart_store::Store,
    motion: std::rc::Rc<std::cell::RefCell<crate::motion_host::Store>>,
}

pub enum ChartDispatch {
    Immediate(gpuio_protocol::chart_resource::Response),
    Publish(crate::chart_store::Work),
}

impl Session {
    pub fn motion(&self) -> std::rc::Rc<std::cell::RefCell<crate::motion_host::Store>> {
        self.motion.clone()
    }
    pub fn container_selected(
        &self,
        window: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        snapshot: gpuio_protocol::container_query::Snapshot,
    ) -> Option<Event> {
        let state = self.window(window).ok()?;
        let current = state.tree.get(node)?;
        let config = current.container_query.as_ref()?;
        (!state.overloaded
            && current.handler == Some(handler)
            && revision >= 0
            && revision <= state.tree.revision()
            && snapshot.is_valid()
            && snapshot.generation == config.generation
            && config.select(snapshot.width, snapshot.height) == Some(snapshot.branch as usize))
        .then_some(Event::ContainerSelected(
            window, node, handler, revision, snapshot,
        ))
    }
    pub fn animation_program_event(
        &self,
        window: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        signals: Vec<gpuio_protocol::animation_program::Signal>,
    ) -> Option<Event> {
        let state = self.window(window).ok()?;
        let current = state.tree.get(node)?;
        let config = current.animation_program.as_ref()?;
        (!state.overloaded
            && current.handler == Some(handler)
            && revision >= 0
            && revision <= state.tree.revision()
            && gpuio_protocol::animation_program::Signal::valid_batch(&signals)
            && signals[0].generation <= config.generation)
            .then_some(Event::AnimationProgramEvent(
                window, node, handler, revision, signals,
            ))
    }

    #[cfg(feature = "native-canvas-tests")]
    pub(crate) fn retained_canvas_bytes(&self) -> usize {
        self.canvases.reserved_bytes()
    }

    pub fn hello(&mut self, version: i64, capabilities: i64) -> Result<Event, ErrorCode> {
        if self.stopped {
            return Err(ErrorCode::Closed);
        }
        if version != VERSION {
            return Err(ErrorCode::UnsupportedVersion);
        }
        if capabilities < 0 || capabilities & !CAPABILITIES != 0 {
            return Err(ErrorCode::UnsupportedCapability);
        }
        if self.ready {
            return Err(ErrorCode::Busy);
        }
        self.ready = true;
        Ok(Event::Welcome(VERSION, CAPABILITIES))
    }

    pub(crate) fn check_ready(&self) -> Result<(), ErrorCode> {
        if self.stopped {
            Err(ErrorCode::Closed)
        } else if !self.ready {
            Err(ErrorCode::NotReady)
        } else {
            Ok(())
        }
    }

    /// Application-owned encoded assets. The bridge must negotiate before
    /// registration, and shutdown permanently closes this acquisition surface.
    pub fn assets(&mut self) -> Result<&mut crate::asset_store::Store, ErrorCode> {
        self.check_ready()?;
        Ok(&mut self.assets)
    }

    pub fn acquire_image(
        &self,
        id: gpuio_protocol::ResourceId,
    ) -> Result<crate::asset_store::Lease, ImageError> {
        self.check_ready().map_err(|_| ImageError::Released)?;
        self.assets.acquire(id).map_err(|_| ImageError::Released)
    }

    pub fn table_input(
        &self,
        window: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        input: gpuio_protocol::table::Input,
    ) -> Option<Event> {
        let state = self.window(window).ok()?;
        let current = state.tree.get(node)?;
        let config = current.table.as_ref()?;
        let index = current.list_index.as_ref()?;
        (!state.overloaded
            && state.tree.accepts_handler(node, handler)
            && revision >= 0
            && revision <= state.tree.revision()
            && input.is_valid()
            && input.schema_revision == config.schema_revision
            && input.query_generation == config.query_generation
            && config.allows_request(&input.request, |row| index.position(row).is_some()))
        .then_some(Event::TableInput(window, node, handler, revision, input))
    }

    pub fn tree_input(
        &self,
        window: WindowId,
        node: NodeId,
        handler: gpuio_protocol::HandlerId,
        revision: i64,
        request: gpuio_protocol::tree_input::Request,
    ) -> Option<Event> {
        let state = self.window(window).ok()?;
        let current = state.tree.get(node)?;
        if state.overloaded
            || !current.tree_input
            || !request.is_valid()
            || !state.tree.accepts_handler(node, handler)
            || revision < 0
            || revision > state.tree.revision()
        {
            return None;
        }
        if matches!(request, gpuio_protocol::tree_input::Request::Move { .. })
            && !current.tree_moves
        {
            return None;
        }
        for target in request.targets() {
            let row = current.list_rows.iter().find(|row| row.id == target)?;
            let item = state.tree.get(row.node)?;
            let Some(gpuio_protocol::accessibility::Role::TreeItem(metadata)) =
                item.accessibility.as_ref()?.role
            else {
                return None;
            };
            if metadata.disabled
                || (matches!(
                    request,
                    gpuio_protocol::tree_input::Request::SetExpanded(..)
                ) && metadata.expanded.is_none())
                || (matches!(request, gpuio_protocol::tree_input::Request::Move { destination, placement: gpuio_protocol::tree_input::Placement::Inside, .. } if destination == target)
                    && metadata.expanded.is_none())
            {
                return None;
            }
        }
        Some(Event::TreeInput(window, node, handler, revision, request))
    }

    pub fn list_viewport(
        &self,
        window: WindowId,
        node: NodeId,
        handler: gpuio_protocol::HandlerId,
        revision: i64,
        viewport: gpuio_protocol::list::Viewport,
    ) -> Option<Event> {
        let state = self.window(window).ok()?;
        let current = state.tree.get(node)?;
        let index = current.list_index.as_ref()?;
        let config = current.list_config.as_ref()?;
        (!state.overloaded
            && state.tree.accepts_handler(node, handler)
            && revision >= 0
            && revision <= state.tree.revision()
            && viewport.is_valid()
            && viewport.order_revision == index.revision()
            && viewport.visible_last as usize <= index.len()
            && viewport
                .requested
                .iter()
                .chain(&viewport.pinned)
                .all(|id| index.position(*id).is_some())
            && viewport
                .requested
                .iter()
                .chain(&viewport.pinned)
                .collect::<std::collections::HashSet<_>>()
                .len()
                <= config.max_active as usize
            && viewport
                .anchor
                .is_none_or(|(id, _)| index.position(id).is_some()))
        .then_some(Event::ListViewport(
            window, node, handler, revision, viewport,
        ))
    }

    pub fn animation_endpoint(
        &self,
        window: WindowId,
        node: NodeId,
        handler: gpuio_protocol::HandlerId,
        revision: i64,
        endpoint: gpuio_protocol::animation::Endpoint,
    ) -> Option<Event> {
        let tree = self.tree(window)?;
        let current = tree.get(node)?;
        (current.handler == Some(handler)
            && revision >= 0
            && revision <= tree.revision()
            && endpoint.generation > 0
            && endpoint.generation <= current.animation.as_ref()?.generation)
            .then_some(Event::AnimationEndpoint(
                window, node, handler, revision, endpoint,
            ))
    }
    pub fn image_state(
        &self,
        window: WindowId,
        node: NodeId,
        handler: gpuio_protocol::HandlerId,
        revision: i64,
        source: ImageSource,
        image_state: ImageState,
    ) -> Option<Event> {
        let state = self.window(window).ok()?;
        let config = state.tree.get(node)?.image.as_ref()?;
        (!state.overloaded
            && state.tree.accepts_handler(node, handler)
            && revision >= 0
            && revision <= state.tree.revision()
            && config.source == source
            && image_state.is_valid())
        .then_some(Event::ImageState(
            window,
            node,
            handler,
            revision,
            image_state,
        ))
    }

    pub fn document_request(
        &mut self,
        request: gpuio_protocol::document::Request,
    ) -> gpuio_protocol::document::Response {
        use gpuio_protocol::document::{Error, Request, Response};
        if let Err(error) = self.check_ready() {
            return Response::Failed(if error == ErrorCode::Closed {
                Error::Closed
            } else {
                Error::NotReady
            });
        }
        let result = match request {
            Request::Create => {
                return match self.documents.create() {
                    Ok(id) => Response::Created(id),
                    Err(error) => Response::Failed(error),
                };
            }
            Request::Begin(update) => self.documents.begin(update),
            Request::Chunk(id, revision, offset, bytes) => usize::try_from(offset)
                .map_err(|_| Error::InvalidRange)
                .and_then(|offset| self.documents.chunk(id, revision, offset, bytes.as_bytes())),
            Request::Publish(id, revision) => self.documents.publish(id, revision),
            Request::Abort(id, revision) => self.documents.abort(id, revision),
            Request::Release(id) => self.documents.release(id),
        };
        match result {
            Ok(()) => Response::Ack,
            Err(error) => Response::Failed(error),
        }
    }

    pub fn chart_request(
        &mut self,
        request: gpuio_protocol::chart_resource::Request,
    ) -> ChartDispatch {
        use gpuio_protocol::chart_resource::{Error, Request, Response};
        if let Err(error) = self.check_ready() {
            return ChartDispatch::Immediate(Response::Failed(if error == ErrorCode::Closed {
                Error::Closed
            } else {
                Error::NotReady
            }));
        }
        let result = match request {
            Request::Create => {
                return ChartDispatch::Immediate(match self.charts.create() {
                    Ok(id) => Response::Created(id),
                    Err(error) => Response::Failed(error),
                });
            }
            Request::Begin(update) => self.charts.begin(update),
            Request::Chunk(id, revision, offset, bytes) => usize::try_from(offset)
                .map_err(|_| Error::InvalidRange)
                .and_then(|offset| self.charts.chunk(id, revision, offset, bytes.as_bytes())),
            Request::Publish(id, revision) => {
                return match self.charts.publish(id, revision) {
                    Ok(work) => ChartDispatch::Publish(work),
                    Err(error) => ChartDispatch::Immediate(Response::Failed(error)),
                };
            }
            Request::Abort(id, revision) => self.charts.abort(id, revision),
            Request::Release(id) => self.charts.release(id),
        };
        ChartDispatch::Immediate(match result {
            Ok(()) => Response::Ack,
            Err(error) => Response::Failed(error),
        })
    }
    pub fn complete_chart(
        &mut self,
        completion: crate::chart_store::Completion,
    ) -> gpuio_protocol::chart_resource::Response {
        use gpuio_protocol::chart_resource::Response;
        match self.charts.complete(completion) {
            Ok(()) => Response::Ack,
            Err(error) => Response::Failed(error),
        }
    }
    pub fn chart(
        &self,
        id: gpuio_protocol::ResourceId,
    ) -> Result<crate::chart_store::Lease, gpuio_protocol::chart_resource::Error> {
        self.charts.acquire(id)
    }
    pub fn close_charts(&mut self) {
        self.charts.close();
    }
    pub fn chart_bytes(&self) -> usize {
        self.charts.reserved_bytes()
    }

    pub fn canvas_request(
        &mut self,
        request: gpuio_protocol::canvas_resource::Request,
    ) -> gpuio_protocol::canvas_resource::Response {
        use gpuio_protocol::canvas_resource::{Error, Request, Response};
        if let Err(error) = self.check_ready() {
            return Response::Failed(if error == ErrorCode::Closed {
                Error::Closed
            } else {
                Error::NotReady
            });
        }
        let result = match request {
            Request::Create => {
                return match self.canvases.create() {
                    Ok(id) => Response::Created(id),
                    Err(error) => Response::Failed(error),
                };
            }
            Request::Begin(update) => self.canvases.begin(update),
            Request::Chunk(id, revision, offset, bytes) => usize::try_from(offset)
                .map_err(|_| Error::InvalidRange)
                .and_then(|offset| self.canvases.chunk(id, revision, offset, bytes.as_bytes())),
            Request::Publish(id, revision) => self.canvases.publish(id, revision, &self.assets),
            Request::Abort(id, revision) => self.canvases.abort(id, revision),
            Request::Release(id) => self.canvases.release(id),
        };
        match result {
            Ok(()) => Response::Ack,
            Err(error) => Response::Failed(error),
        }
    }

    pub fn canvas(
        &self,
        id: gpuio_protocol::ResourceId,
    ) -> Result<crate::canvas_store::Lease, gpuio_protocol::canvas_resource::Error> {
        use gpuio_protocol::canvas_resource::Error;
        self.check_ready().map_err(|error| {
            if error == ErrorCode::Closed {
                Error::Closed
            } else {
                Error::NotReady
            }
        })?;
        self.canvases.acquire(id)
    }

    pub fn document(
        &self,
        id: gpuio_protocol::ResourceId,
    ) -> Result<crate::document_store::Lease, gpuio_protocol::document::Error> {
        self.documents.acquire(id)
    }

    pub fn asset_request(
        &mut self,
        request: gpuio_protocol::asset::Request,
    ) -> gpuio_protocol::asset::Response {
        use gpuio_protocol::asset::{Error, Request, Response};
        if let Err(error) = self.check_ready() {
            return Response::Failed(match error {
                ErrorCode::Closed => Error::Closed,
                ErrorCode::NotReady => Error::NotReady,
                _ => Error::NativeFailure,
            });
        }
        match request {
            Request::Begin(format, length) => {
                match usize::try_from(length)
                    .map_err(|_| Error::InvalidSize)
                    .and_then(|length| self.assets.begin(format, length))
                {
                    Ok(id) => Response::Begun(id),
                    Err(error) => Response::Failed(error),
                }
            }
            request => {
                let result = match request {
                    Request::Append(id, offset, data) => match usize::try_from(offset) {
                        Ok(offset) => self.assets.append(id, offset, data.as_bytes()),
                        Err(_) => self.assets.release(id).and(Err(Error::InvalidChunk)),
                    },
                    Request::Finish(id) => self.assets.finish(id),
                    Request::Release(id) => self.assets.release(id),
                    Request::Begin(..) => unreachable!(),
                };
                match result {
                    Ok(()) => Response::Ack,
                    Err(error) => Response::Failed(error),
                }
            }
        }
    }

    pub fn validate_open(
        &self,
        id: WindowId,
        title: &str,
        width: f64,
        height: f64,
    ) -> Result<(), ErrorCode> {
        self.check_ready()?;
        if title.len() > MAX_TEXT_BYTES || id.slot() > self.slots.len() || id.slot() >= MAX_WINDOWS
        {
            return Err(ErrorCode::LimitExceeded);
        }
        if ![width, height]
            .iter()
            .all(|n| n.is_finite() && (1.0..=32768.0).contains(n))
        {
            return Err(ErrorCode::Malformed);
        }
        let generation = match self.slots.get(id.slot()) {
            Some(slot) if slot.window.is_some() => return Err(ErrorCode::Busy),
            Some(slot) => slot.generation,
            None => 0,
        };
        if generation.checked_add(1) != Some(id.generation()) {
            return Err(ErrorCode::StaleHandle);
        }
        Ok(())
    }

    /// Call only after the platform has successfully created the window.
    pub fn open(
        &mut self,
        correlation: i64,
        id: WindowId,
        title: &str,
        width: f64,
        height: f64,
    ) -> Result<Event, ErrorCode> {
        self.validate_open(id, title, width, height)?;
        if id.slot() == self.slots.len() {
            self.slots.push(Slot::default());
        }
        self.slots[id.slot()] = Slot {
            generation: id.generation(),
            window: Some(Window {
                tree: Tree::new(id),
                rendered: None,
                requested_frame: None,
                overloaded: false,
            }),
        };
        Ok(Event::Opened(correlation, id))
    }

    fn window(&self, id: WindowId) -> Result<&Window, ErrorCode> {
        self.check_ready()?;
        self.slots
            .get(id.slot())
            .filter(|s| s.generation == id.generation())
            .and_then(|s| s.window.as_ref())
            .ok_or(ErrorCode::StaleHandle)
    }

    fn window_mut(&mut self, id: WindowId) -> Result<&mut Window, ErrorCode> {
        self.window(id)?;
        Ok(self.slots[id.slot()]
            .window
            .as_mut()
            .expect("validated window"))
    }

    pub fn tree(&self, id: WindowId) -> Option<&Tree> {
        self.window(id).ok().map(|w| &w.tree)
    }
    /// Closed or terminally overloaded windows must not accept further input.
    pub fn accepts_input(&self, id: WindowId) -> bool {
        self.window(id).is_ok_and(|window| !window.overloaded)
    }
    pub fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }

    pub fn apply(&mut self, tx: &Transaction) -> Result<Applied, ErrorCode> {
        self.apply_guarded(tx, &[])
            .map_err(crate::tree::ApplyFailure::rejection)
    }

    pub fn apply_guarded(
        &mut self,
        tx: &Transaction,
        pins: &[gpuio_protocol::list::Retained],
    ) -> Result<Applied, crate::tree::ApplyFailure> {
        let window = self.window(tx.window)?;
        if window.overloaded {
            return Err(ErrorCode::Overloaded.into());
        }
        for operation in &tx.operations {
            if let Op::SetExtension(_, config) = operation {
                crate::extensions::validate(config).map_err(|_| ErrorCode::InvalidTree)?;
            }
        }
        let before = window.tree.retained_bytes();
        let budget = MAX_SESSION_BYTES - (self.retained_bytes - before);
        let motion = self.motion.clone();
        let window = self.window_mut(tx.window)?;
        let result = window
            .tree
            .apply_with_admission(tx, budget, pins, |changes| {
                motion.borrow_mut().admit(tx.window, changes)
            })?;
        let after = window.tree.retained_bytes();
        self.retained_bytes = self.retained_bytes - before + after;
        Ok(result)
    }

    /// Frame requests complete only from the paint callback, even at an unchanged revision.
    pub fn request_frame(&mut self, correlation: i64, id: WindowId) -> Result<(), ErrorCode> {
        let window = self.window_mut(id)?;
        if window.requested_frame.is_some() {
            return Err(ErrorCode::Busy);
        }
        window.requested_frame = Some(correlation);
        Ok(())
    }

    pub fn painted(&mut self, id: WindowId, revision: i64) -> Vec<Event> {
        let Ok(window) = self.window_mut(id) else {
            return vec![];
        };
        if revision != window.tree.revision() {
            return vec![];
        }
        let mut events = Vec::with_capacity(2);
        if window.rendered != Some(revision) {
            window.rendered = Some(revision);
            events.push(Event::Rendered(id, revision));
        }
        if let Some(correlation) = window.requested_frame.take() {
            events.push(Event::FrameRequested(correlation, id, revision));
        }
        events
    }

    pub fn press(
        &self,
        id: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
    ) -> Option<Event> {
        let window = self.window(id).ok()?;
        (!window.overloaded
            && revision <= window.tree.revision()
            && revision >= 0
            && window.tree.get(node).is_some_and(|node| {
                node.image.is_none()
                    && node.input_region.is_none()
                    && node.highlight_scope.is_none()
                    && node.slider.is_none()
                    && node.number_input.is_none()
                    && node.otp_input.is_none()
                    && node.calendar.is_none()
                    && node.color_input.is_none()
                    && !node.control.is_some_and(Control::disabled)
                    && !node.link.as_ref().is_some_and(|config| config.disabled)
            })
            && window.tree.accepts_handler(node, handler))
        .then_some(Event::Press(id, node, handler, revision))
    }

    pub fn slider_event(
        &self,
        id: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        event: gpuio_protocol::slider::Event,
    ) -> Option<Event> {
        let window = self.window(id).ok()?;
        let slider = window.tree.get(node)?.slider.as_ref()?;
        (!window.overloaded
            && revision >= 0
            && revision <= window.tree.revision()
            && window.tree.accepts_handler(node, handler)
            && event.is_valid()
            && slider.initial.same_mode(event.snapshot().value))
        .then_some(Event::SliderEvent(id, node, handler, revision, event))
    }

    pub fn color_input_event(
        &self,
        id: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        event: gpuio_protocol::color_input::Event,
    ) -> Option<Event> {
        let window = self.window(id).ok()?;
        window.tree.get(node)?.color_input.as_ref()?;
        (!window.overloaded
            && revision >= 0
            && revision <= window.tree.revision()
            && window.tree.accepts_handler(node, handler)
            && event.is_valid())
        .then_some(Event::ColorInputEvent(id, node, handler, revision, event))
    }

    pub fn calendar_event(
        &self,
        id: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        event: gpuio_protocol::calendar_input::Event,
    ) -> Option<Event> {
        let window = self.window(id).ok()?;
        let mount = window.tree.get(node)?.calendar.as_ref()?;
        (!window.overloaded
            && revision >= 0
            && revision <= window.tree.revision()
            && window.tree.accepts_handler(node, handler)
            && event.is_valid()
            && event.snapshot().mode == mount.config.mode)
            .then_some(Event::CalendarEvent(id, node, handler, revision, event))
    }

    pub fn otp_input_event(
        &self,
        id: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        event: gpuio_protocol::otp_input::Event,
    ) -> Option<Event> {
        let window = self.window(id).ok()?;
        let mount = window.tree.get(node)?.otp_input.as_ref()?;
        (!window.overloaded
            && revision >= 0
            && revision <= window.tree.revision()
            && window.tree.accepts_handler(node, handler)
            && event.is_valid()
            && event.snapshot().policy == mount.config.policy)
            .then_some(Event::OtpInputEvent(id, node, handler, revision, event))
    }

    pub fn number_input_event(
        &self,
        id: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        event: gpuio_protocol::number_input::Event,
    ) -> Option<Event> {
        let window = self.window(id).ok()?;
        window.tree.get(node)?.number_input.as_ref()?;
        (!window.overloaded
            && revision >= 0
            && revision <= window.tree.revision()
            && window.tree.accepts_handler(node, handler)
            && event.is_valid())
        .then_some(Event::NumberInputEvent(id, node, handler, revision, event))
    }

    pub fn request_carousel(
        &self,
        id: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        request: gpuio_protocol::carousel::Request,
    ) -> Option<Event> {
        let window = self.window(id).ok()?;
        (!window.overloaded
            && revision >= 0
            && revision <= window.tree.revision()
            && window.tree.accepts_handler(node, handler)
            && request.is_valid()
            && window
                .tree
                .get(node)?
                .carousel
                .as_ref()?
                .accepts_request(&request))
        .then_some(Event::CarouselRequested(
            id, node, handler, revision, request,
        ))
    }

    pub fn request_rating(
        &self,
        id: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        request: gpuio_protocol::rating::Request,
    ) -> Option<Event> {
        let window = self.window(id).ok()?;
        (!window.overloaded
            && revision >= 0
            && revision <= window.tree.revision()
            && window.tree.accepts_handler(node, handler)
            && window.tree.get(node)?.rating.as_ref()?.can_apply(request))
        .then_some(Event::RatingRequested(id, node, handler, revision, request))
    }

    pub fn choose(
        &self,
        id: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        selected: &str,
    ) -> Option<Event> {
        let window = self.window(id).ok()?;
        (!window.overloaded
            && revision >= 0
            && revision <= window.tree.revision()
            && window.tree.accepts_handler(node, handler)
            && window.tree.get(node)?.choice.as_ref()?.can_select(selected))
        .then(|| Event::Choice(id, node, handler, revision, selected.to_owned()))
    }

    pub fn dismiss(
        &self,
        id: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        reason: Dismissal,
    ) -> Option<Event> {
        let window = self.window(id).ok()?;
        (!window.overloaded
            && revision >= 0
            && revision <= window.tree.revision()
            && window.tree.accepts_handler(node, handler)
            && window.tree.get(node)?.overlay.as_ref()?.allows(reason))
        .then_some(Event::OverlayDismissed(id, node, handler, revision, reason))
    }

    pub fn drag_source_event(
        &self,
        window: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        sample: gpuio_protocol::drag_drop::SourceSample,
    ) -> Option<Event> {
        let state = self.window(window).ok()?;
        let config = state.tree.get(node)?.drag_source.as_ref()?;
        (!state.overloaded
            && sample.is_valid()
            && state.tree.accepts_handler(node, handler)
            && revision >= 0
            && revision <= state.tree.revision()
            && (matches!(
                sample.phase,
                gpuio_protocol::drag_drop::SourcePhase::Ended(_)
            ) || !config.disabled()))
        .then_some(Event::DragSourceEvent(
            window, node, handler, revision, sample,
        ))
    }
    pub fn drop_target_event(
        &self,
        window: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        sample: gpuio_protocol::drag_drop::TargetSample,
    ) -> Option<Event> {
        use gpuio_protocol::drag_drop::TargetPhase;
        let state = self.window(window).ok()?;
        let config = state.tree.get(node)?.drop_target.as_ref()?;
        let allowed = match &sample.phase {
            TargetPhase::Dropped(payload) => config.accepts(payload),
            TargetPhase::Left => true,
            _ => !config.disabled(),
        };
        (!state.overloaded
            && sample.is_valid()
            && allowed
            && state.tree.accepts_handler(node, handler)
            && revision >= 0
            && revision <= state.tree.revision())
        .then_some(Event::DropTargetEvent(
            window, node, handler, revision, sample,
        ))
    }

    /// The presenter supplies provenance from its exact installed snapshot.
    /// Keep old same-generation pictures interactive while preparation runs,
    /// but retire source/config/handler identities immediately when replaced.
    pub fn document_diff_event(
        &self,
        window: WindowId,
        node: NodeId,
        handler: HandlerId,
        source: gpuio_protocol::ResourceId,
        event: gpuio_protocol::document_diff::Event,
    ) -> Option<Event> {
        let state = self.window(window).ok()?;
        let target = state.tree.get(node)?;
        let config = target.document_diff.as_ref()?;
        let snapshot = self.documents.acquire(source).ok()?.snapshot();
        (!state.overloaded
            && event.is_valid()
            && event.observation.valid_for(config)
            && target.document_diff_epoch == event.config_epoch
            && target.document.as_ref()?.source == Some(source)
            && snapshot.generation == event.source_generation
            && event.source_revision >= snapshot.generation_first_revision
            && event.source_revision <= snapshot.revision
            && state.tree.accepts_handler(node, handler))
        .then_some(Event::DocumentDiffEvent(
            window,
            node,
            handler,
            state.tree.revision(),
            source,
            event,
        ))
    }

    pub fn highlight_observed(
        &self,
        window: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        observation: gpuio_protocol::highlight::Observation,
    ) -> Option<Event> {
        let state = self.window(window).ok()?;
        let config = state.tree.get(node)?.highlight_scope.as_ref()?;
        (!state.overloaded
            && observation.valid_for(config)
            && state.tree.accepts_handler(node, handler)
            && revision >= 0
            && revision <= state.tree.revision())
        .then_some(Event::HighlightObserved(
            window,
            node,
            handler,
            revision,
            observation,
        ))
    }

    pub fn input_observed(
        &self,
        window: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        event: gpuio_protocol::input::Event,
    ) -> Option<Event> {
        let state = self.window(window).ok()?;
        let config = state.tree.get(node)?.input_region.as_ref()?;
        (!state.overloaded
            && !config.disabled
            && event.is_valid()
            && config.subscription(event.kind()).is_some()
            && state.tree.accepts_handler(node, handler)
            && revision >= 0
            && revision <= state.tree.revision())
        .then_some(Event::InputObserved(window, node, handler, revision, event))
    }

    pub fn pointer_event(
        &self,
        window: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        sample: PointerSample,
    ) -> Option<Event> {
        let state = self.window(window).ok()?;
        let config = state.tree.get(node)?.pointer.as_ref()?;
        (!state.overloaded
            && sample.is_valid()
            && state.tree.accepts_handler(node, handler)
            && revision >= 0
            && revision <= state.tree.revision()
            && (matches!(sample.phase, PointerPhase::Cancelled(_))
                || (!config.disabled && config.button == sample.button)))
            .then_some(Event::PointerEvent(window, node, handler, revision, sample))
    }

    pub fn toast_dismissed(
        &self,
        window: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        reason: ToastDismissal,
    ) -> Option<Event> {
        let state = self.window(window).ok()?;
        let config = state.tree.get(node)?.toast.as_ref()?;
        (!state.overloaded
            && state.tree.accepts_handler(node, handler)
            && revision >= 0
            && revision <= state.tree.revision()
            && config.allows(reason))
        .then_some(Event::ToastDismissed(
            window, node, handler, revision, reason,
        ))
    }

    pub fn palette_dismissed(
        &self,
        window: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        reason: PaletteDismissal,
    ) -> Option<Event> {
        let state = self.window(window).ok()?;
        let config = state.tree.get(node)?.palette.as_ref()?;
        (!state.overloaded
            && state.tree.accepts_handler(node, handler)
            && revision >= 0
            && revision <= state.tree.revision()
            && config.allows(&reason))
        .then_some(Event::PaletteDismissed(
            window, node, handler, revision, reason,
        ))
    }

    pub fn tooltip_open_changed(
        &self,
        id: WindowId,
        node: NodeId,
        handler: HandlerId,
        revision: i64,
        open: bool,
    ) -> Option<Event> {
        let window = self.window(id).ok()?;
        (!window.overloaded
            && revision >= 0
            && revision <= window.tree.revision()
            && window.tree.accepts_handler(node, handler)
            && (!open || !window.tree.get(node)?.tooltip.as_ref()?.disabled)
            && window.tree.get(node)?.tooltip.is_some())
        .then_some(Event::TooltipOpenChanged(id, node, handler, revision, open))
    }

    pub fn command_target(
        &self,
        id: WindowId,
        request: CommandInvocation<'_>,
    ) -> Option<CommandTarget> {
        let window = self.window(id).ok()?;
        let config = window
            .tree
            .get(request.scope)?
            .commands
            .as_ref()?
            .iter()
            .find(|entry| entry.id == request.command)?;
        let valid_source = match request.source {
            CommandSource::Button(button) => {
                window
                    .tree
                    .get(button)
                    .is_some_and(|node| node.command_ref.as_deref() == Some(request.command))
                    && window
                        .tree
                        .command(button, request.command)
                        .is_some_and(|(scope, _)| scope == request.scope)
            }
            CommandSource::Menu(menu) => {
                window
                    .tree
                    .get(menu)
                    .and_then(|node| node.menu.as_ref())
                    .is_some_and(|config| config.permits(request.command))
                    && (window
                        .tree
                        .get(menu)
                        .and_then(|node| node.menu.as_ref())
                        .is_some_and(|config| config.presentation == MenuPresentation::PlatformBar)
                        || window
                            .tree
                            .command(menu, request.command)
                            .is_some_and(|(scope, _)| scope == request.scope))
            }
            CommandSource::Shortcut => true,
            CommandSource::Palette(palette) => {
                window
                    .tree
                    .get(palette)
                    .and_then(|node| node.palette.as_ref())
                    .is_some_and(|config| config.permits(request.command))
                    && window
                        .tree
                        .command(palette, request.command)
                        .is_some_and(|(scope, _)| scope == request.scope)
            }
        };
        (!window.overloaded
            && request.revision >= 0
            && request.revision <= window.tree.revision()
            && window.tree.accepts_handler(request.scope, request.handler)
            && config.enabled
            && config.generation == request.generation
            && valid_source)
            .then_some(config.target)
    }

    pub fn invoke_command(&self, id: WindowId, request: CommandInvocation<'_>) -> Option<Event> {
        (self.command_target(id, request)? == CommandTarget::Callback).then(|| {
            Event::CommandInvoked(
                id,
                request.scope,
                request.handler,
                request.revision,
                request.command.to_owned(),
                request.generation,
                request.source,
            )
        })
    }

    pub fn overload(&mut self, id: WindowId) -> bool {
        let Ok(window) = self.window_mut(id) else {
            return false;
        };
        !std::mem::replace(&mut window.overloaded, true)
    }

    /// Returns the pending frame correlation so its reserved response can fail Closed.
    pub fn close(&mut self, id: WindowId) -> Result<Option<i64>, ErrorCode> {
        self.window(id)?;
        let window = self.slots[id.slot()]
            .window
            .take()
            .expect("validated window");
        self.retained_bytes -= window.tree.retained_bytes();
        self.motion.borrow_mut().close_window(id);
        Ok(window.requested_frame)
    }

    pub fn shutdown(&mut self) -> Vec<Event> {
        self.motion.borrow_mut().close();
        self.canvases.close();
        self.charts.close();
        self.assets.close();
        self.documents.close();
        let mut events = Vec::new();
        for slot in &mut self.slots {
            if let Some(window) = slot.window.take()
                && let Some(correlation) = window.requested_frame
            {
                events.push(Event::Failed(correlation, ErrorCode::Closed));
            }
        }
        self.retained_bytes = 0;
        self.stopped = true;
        events.push(Event::Stopped);
        events
    }
}
