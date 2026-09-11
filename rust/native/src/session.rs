//! Native-thread-owned window state. No OCaml callbacks or runtime references.
use crate::tree::{Applied, Tree};
use gpuio_protocol::{HandlerId, NodeId, WindowId, v1::*};

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

    fn check_ready(&self) -> Result<(), ErrorCode> {
        if self.stopped {
            Err(ErrorCode::Closed)
        } else if !self.ready {
            Err(ErrorCode::NotReady)
        } else {
            Ok(())
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
            && window.tree.accepts_handler(node, handler))
        .then_some(Event::Press(id, node, handler, revision))
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
