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
}

impl Session {
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
    pub fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }

    pub fn apply(&mut self, tx: &Transaction) -> Result<Applied, ErrorCode> {
        let window = self.window(tx.window)?;
        if window.overloaded {
            return Err(ErrorCode::Overloaded);
        }
        let before = window.tree.retained_bytes();
        let budget = MAX_SESSION_BYTES - (self.retained_bytes - before);
        let window = self.window_mut(tx.window)?;
        let result = window.tree.apply_with_budget(tx, budget)?;
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
                node.image.is_none() && !node.control.is_some_and(Control::disabled)
            })
            && window.tree.accepts_handler(node, handler))
        .then_some(Event::Press(id, node, handler, revision))
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
        Ok(window.requested_frame)
    }

    pub fn shutdown(&mut self) -> Vec<Event> {
        self.assets.close();
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
