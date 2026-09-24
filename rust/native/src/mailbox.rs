//! Bounded transport state, protected by one short-lived mutex in the FFI adapter.
//! Decoding and GPUI tree work happen outside that mutex.
use gpuio_protocol::{WindowId, v1::*};
use std::collections::{BTreeSet, VecDeque};

pub const MAX_COMMANDS: usize = 64;
pub const MAX_COMMAND_BYTES: usize = 4 * MAX_MESSAGE_BYTES;
pub const MAX_RESPONSES: usize = 128;
pub const MAX_INPUT_EVENTS: usize = 128;
pub const MAX_INPUT_BYTES: usize = 4 * MAX_MESSAGE_BYTES;

// Conservative encoded-size bound (all non-text fields fit within 256 bytes).
// Responses retain their count reservation. Editor text and selected path bytes
// have per-result bounds; each path also needs its bin_prot length prefix.
// Drain includes those prefixes when fitting a response batch into 1 MiB.
fn event_bytes(event: &Event) -> usize {
    256 + match event {
        Event::DragSourceEvent(_, _, _, _, sample) => sample.payload_bytes(),
        Event::DropTargetEvent(_, _, _, _, sample) => sample.payload_bytes(),
        Event::FileDialogResult(_, _, FileDialogResult::Selected(paths)) => {
            paths.iter().map(|path| path.as_bytes().len() + 9).sum()
        }
        Event::Choice(_, _, _, _, id)
        | Event::CommandInvoked(_, _, _, _, id, _, _)
        | Event::PaletteDismissed(_, _, _, _, PaletteDismissal::Selected(id)) => id.len(),
        Event::ComboboxSelected(_, _, _, _, id, snapshot) => id.len() + snapshot.text.len(),
        Event::EditorEvent(_, _, _, _, _, snapshot)
        | Event::EditorResult(_, _, _, EditorResult::Applied(snapshot)) => snapshot.text.len(),
        _ => 0,
    }
}

struct Queued {
    message: Message,
    bytes: usize,
}
struct Output {
    event: Event,
    class: Class,
}
#[derive(Clone, Copy)]
enum Class {
    Response,
    Input,
    Fault,
    Terminal,
    Control,
}

#[derive(Default)]
pub struct Mailbox {
    commands: VecDeque<Queued>,
    command_bytes: usize,
    events: VecDeque<Output>,
    reserved: usize,
    responses: usize,
    inputs: usize,
    input_bytes: usize,
    faults: BTreeSet<WindowId>,
    in_flight: BTreeSet<WindowId>,
    closed: bool,
    stopped_emitted: bool,
    close_requests: BTreeSet<WindowId>,
    controls: usize,
}

impl Mailbox {
    /// Successful submission reserves a response. Backpressure is synchronous and
    /// leaves the message unsubmitted; the caller retains its desired UI state.
    pub fn submit(&mut self, message: Message, bytes: usize) -> Result<(), ErrorCode> {
        if self.closed {
            return Err(ErrorCode::Closed);
        }
        if bytes > MAX_MESSAGE_BYTES {
            return Err(ErrorCode::LimitExceeded);
        }
        if self.commands.len() >= MAX_COMMANDS
            || self.command_bytes + bytes > MAX_COMMAND_BYTES
            || self.reserved + self.responses >= MAX_RESPONSES
        {
            return Err(ErrorCode::Busy);
        }
        if let Message::Apply(tx) = &message
            && !self.in_flight.insert(tx.window)
        {
            return Err(ErrorCode::Busy);
        }
        self.command_bytes += bytes;
        self.reserved += 1;
        self.commands.push_back(Queued { message, bytes });
        Ok(())
    }

    pub fn request_close(&mut self, id: WindowId) {
        if !self.closed {
            self.close_requests.insert(id);
        }
    }
    pub fn pop_close(&mut self) -> Option<WindowId> {
        self.close_requests.pop_first()
    }
    pub fn native_closed(&mut self, id: WindowId) {
        assert!(
            self.controls < MAX_WINDOWS,
            "undrained closed window generations"
        );
        self.controls += 1;
        self.events.push_back(Output {
            event: Event::Closed(0, id),
            class: Class::Control,
        });
    }
    pub fn pop(&mut self) -> Option<Message> {
        let queued = self.commands.pop_front()?;
        self.command_bytes -= queued.bytes;
        Some(queued.message)
    }

    pub fn respond(&mut self, event: Event) {
        assert!(self.reserved > 0, "response without reservation");
        self.reserved -= 1;
        self.responses += 1;
        if matches!(event, Event::Stopped) {
            self.stopped_emitted = true;
        }
        self.events.push_back(Output {
            event,
            class: Class::Response,
        });
    }

    /// Coalesce only consecutive render observations for the same window. Never
    /// cross input/response barriers. Other input is ordered and never discarded.
    pub fn input(&mut self, event: Event) -> Result<(), Box<Event>> {
        if let Event::Rendered(id, _) = &event
            && let Some(Output {
                event: Event::Rendered(last, revision),
                ..
            }) = self.events.back_mut()
            && last == id
        {
            let Event::Rendered(_, next) = event else {
                unreachable!()
            };
            *revision = next;
            return Ok(());
        }
        if let Event::PointerEvent(window, node, handler, revision, sample) = &event
            && sample.phase == PointerPhase::Moved
            && let Some(last) = self.events.back_mut()
            && let Event::PointerEvent(w, n, h, r, previous) = &last.event
            && previous.phase == PointerPhase::Moved
            && (window, node, handler, revision, sample.gesture) == (w, n, h, r, previous.gesture)
        {
            last.event = event;
            return Ok(());
        }
        if let Event::DropTargetEvent(window, node, handler, revision, sample) = &event
            && sample.phase == gpuio_protocol::drag_drop::TargetPhase::Moved
            && let Some(last) = self.events.back_mut()
            && let Event::DropTargetEvent(w, n, h, r, previous) = &last.event
            && previous.phase == gpuio_protocol::drag_drop::TargetPhase::Moved
            && (window, node, handler, revision, sample.gesture) == (w, n, h, r, previous.gesture)
        {
            last.event = event;
            return Ok(());
        }
        let bytes = event_bytes(&event);
        if let Event::EditorEvent(window, node, handler, revision, EditorEventKind::Changed, _) =
            &event
            && let Some(last) = self.events.back_mut()
            && let Event::EditorEvent(w, n, h, r, EditorEventKind::Changed, _) = &last.event
            && (window, node, handler, revision) == (w, n, h, r)
        {
            let next_bytes = self.input_bytes - event_bytes(&last.event) + bytes;
            if next_bytes > MAX_INPUT_BYTES {
                return Err(Box::new(event));
            }
            last.event = event;
            self.input_bytes = next_bytes;
            return Ok(());
        }
        if self.inputs >= MAX_INPUT_EVENTS || self.input_bytes + bytes > MAX_INPUT_BYTES {
            return Err(Box::new(event));
        }
        self.inputs += 1;
        self.input_bytes += bytes;
        self.events.push_back(Output {
            event,
            class: Class::Input,
        });
        Ok(())
    }

    /// One terminal overload notification per window generation. Reopening is
    /// refused by the host until the old window's output is drained.
    pub fn fault(&mut self, window: WindowId) {
        if self.faults.contains(&window) {
            return;
        }
        assert!(
            self.faults.len() < MAX_WINDOWS,
            "undrained window generations"
        );
        self.faults.insert(window);
        self.events.push_back(Output {
            event: Event::Overloaded(window),
            class: Class::Fault,
        });
    }

    pub fn has_window_output(&self, window_slot: usize) -> bool {
        self.events.iter().any(|output| match output.event {
            Event::Opened(_, id)
            | Event::Closed(_, id)
            | Event::Accepted(id, _)
            | Event::Rejected(id, ..)
            | Event::Rendered(id, _)
            | Event::FrameRequested(_, id, _)
            | Event::Press(id, ..)
            | Event::EditorEvent(id, ..)
            | Event::Choice(id, ..)
            | Event::OverlayDismissed(id, ..)
            | Event::TooltipOpenChanged(id, ..)
            | Event::CommandInvoked(id, ..)
            | Event::ToastDismissed(id, ..)
            | Event::DragSourceEvent(id, ..)
            | Event::ImageState(id, ..)
            | Event::AnimationEndpoint(id, ..)
            | Event::DropTargetEvent(id, ..)
            | Event::PointerEvent(id, ..)
            | Event::PaletteDismissed(id, ..)
            | Event::ComboboxSelected(id, ..)
            | Event::EditorResult(_, id, ..)
            | Event::FileDialogResult(_, id, ..)
            | Event::Overloaded(id) => id.slot() == window_slot,
            Event::Welcome(..) | Event::Failed(..) | Event::AssetResponse(..) | Event::Stopped => {
                false
            }
        })
    }

    pub fn drain(&mut self, maximum: usize) -> Vec<Event> {
        let mut result = Vec::with_capacity(maximum.min(self.events.len()).min(256));
        let mut bytes = 8; // Bin_prot list prefix.
        for _ in 0..maximum.min(256) {
            if let Some(output) = self.events.front() {
                let next = bytes + event_bytes(&output.event);
                if next > MAX_MESSAGE_BYTES {
                    break;
                }
                bytes = next;
            }
            let Some(output) = self.events.pop_front() else {
                break;
            };
            match output.class {
                Class::Response => self.responses -= 1,
                Class::Input => {
                    self.inputs -= 1;
                    self.input_bytes -= event_bytes(&output.event);
                }
                Class::Fault => {
                    if let Event::Overloaded(id) = output.event {
                        self.faults.remove(&id);
                    }
                }
                Class::Terminal => (),
                Class::Control => self.controls -= 1,
            }
            if let Event::Accepted(id, _) | Event::Rejected(id, ..) = output.event {
                self.in_flight.remove(&id);
            }
            result.push(output.event);
        }
        result
    }
    pub fn has_output(&self) -> bool {
        !self.events.is_empty()
    }
    /// Terminal stop cancels outstanding requests, including requests queued
    /// when the platform closed its last window. Always deliver it after output.
    pub fn close(&mut self) {
        self.closed = true;
        self.commands.clear();
        self.close_requests.clear();
        self.command_bytes = 0;
        self.reserved = 0;
        self.in_flight.clear();
        if !self.stopped_emitted {
            self.stopped_emitted = true;
            self.events.push_back(Output {
                event: Event::Stopped,
                class: Class::Terminal,
            });
        }
    }
}
