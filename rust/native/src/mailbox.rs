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
        Event::NotificationResponse(_, gpuio_protocol::notification::Response::Events(events)) => {
            events
                .iter()
                .map(|event| match event {
                    gpuio_protocol::notification::Event::Activated(receipt)
                    | gpuio_protocol::notification::Event::Closed(receipt, _) => {
                        receipt.tag.len() + 24
                    }
                    gpuio_protocol::notification::Event::Action(receipt, id) => {
                        receipt.tag.len() + id.len() + 32
                    }
                    gpuio_protocol::notification::Event::Failed(_) => 2,
                })
                .sum()
        }

        Event::DesktopResponse(_, gpuio_protocol::desktop::Response::Links(batch)) => {
            batch.links.iter().map(|link| link.len() + 9).sum()
        }
        Event::InputObserved(_, _, _, _, input) => input.payload_bytes(),
        Event::TableInput(_, _, _, _, input) => input.request.payload_bytes(),
        Event::TreeInput(
            _,
            _,
            _,
            _,
            gpuio_protocol::tree_input::Request::Typeahead { text, .. },
        ) => text.len(),
        Event::CarouselRequested(_, _, _, _, request) => match request {
            gpuio_protocol::carousel::Request::Select(id) => id.len(),
            gpuio_protocol::carousel::Request::AutoNext { from, target, .. } => {
                from.len() + target.len()
            }
            _ => 0,
        },
        Event::AnimationProgramEvent(_, _, _, _, signals) => signals.len() * 32,
        Event::ExtensionEvent(_, _, _, _, _, gpuio_protocol::extension::Signal::Data(payload)) => {
            payload.0.len()
        }
        Event::WindowChanged(_, snapshot)
        | Event::WindowResponse(_, _, gpuio_protocol::window::Response::Observed(snapshot)) => {
            snapshot.title.len()
                + snapshot
                    .document
                    .as_ref()
                    .and_then(|document| document.path.as_ref())
                    .map_or(0, |path| path.as_bytes().len() + 9)
        }
        Event::DragSourceEvent(_, _, _, _, sample) => sample.payload_bytes(),
        Event::DropTargetEvent(_, _, _, _, sample) => sample.payload_bytes(),
        Event::FileDialogResult(_, _, FileDialogResult::Selected(paths)) => {
            paths.iter().map(|path| path.as_bytes().len() + 9).sum()
        }
        Event::Choice(_, _, _, _, id)
        | Event::CommandInvoked(_, _, _, _, id, _, _)
        | Event::PaletteDismissed(_, _, _, _, PaletteDismissal::Selected(id)) => id.len(),
        Event::ComboboxSelected(_, _, _, _, id, snapshot) => id.len() + snapshot.text.len(),
        Event::NumberInputEvent(_, _, _, _, event) => event.snapshot().draft.len(),
        Event::ColorInputResult(
            _,
            _,
            _,
            gpuio_protocol::color_input::Response::Applied(snapshot),
        ) => snapshot.draft.as_ref().map_or(0, |d| d.text.len()),
        Event::ColorInputEvent(_, _, _, _, event) => {
            event.snapshot().draft.as_ref().map_or(0, |d| d.text.len())
        }
        Event::OtpInputEvent(_, _, _, _, event) => {
            event.snapshot().value.len() + event.snapshot().draft.len()
        }
        Event::OtpInputResult(_, _, _, gpuio_protocol::otp_input::Response::Applied(snapshot)) => {
            snapshot.value.len() + snapshot.draft.len()
        }
        Event::NumberInputResult(
            _,
            _,
            _,
            gpuio_protocol::number_input::Response::Applied(snapshot),
        ) => snapshot.draft.len(),
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

fn otp_coalesces(previous: &Output, next: &Event) -> bool {
    matches!((&previous.event, next),
        (Event::OtpInputEvent(w, n, h, r, old), Event::OtpInputEvent(window, node, handler, revision, new))
        if matches!(previous.class, Class::Input)
            && (w, n, h, r) == (window, node, handler, revision)
            && crate::otp_input_state::can_coalesce(old, new))
}

fn color_coalesces(previous: &Output, next: &Event) -> bool {
    use gpuio_protocol::color_input::Event as ColorEvent;
    matches!((&previous.event, next),
        (Event::ColorInputEvent(w,n,h,r,ColorEvent::Preview(old)), Event::ColorInputEvent(w2,n2,h2,r2,ColorEvent::Preview(new)))
        if matches!(previous.class, Class::Input) && (w,n,h,r) == (w2,n2,h2,r2)
            && old.is_valid() && new.is_valid() && old.interaction == new.interaction
            && old.committed == new.committed && old.committed_allowed == new.committed_allowed
            && new.revision > old.revision)
}

fn color_pair(
    a: &gpuio_protocol::color_input::Event,
    b: &gpuio_protocol::color_input::Event,
) -> bool {
    use gpuio_protocol::color_input::{DraftStatus, Event as E, InteractionKind, Source};
    match (a, b) {
        (E::Cancelled(..), E::Observed(_) | E::Committed(..)) => b.snapshot().interaction.is_none(),
        (E::Started(a), E::Preview(b)) => {
            a.interaction == b.interaction
                && a.committed == b.committed
                && a.committed_allowed == b.committed_allowed
        }
        (E::Preview(a), E::Committed(source, b)) => {
            a.value == b.value
                && a.channels == b.channels
                && a.value_allowed == b.value_allowed
                && b.value_allowed
                && match (a.interaction.map(|i| i.kind), source) {
                    (Some(InteractionKind::Drag(_)), Source::Pointer) => true,
                    (Some(InteractionKind::Text(_)), Source::Text) => a
                        .draft
                        .as_ref()
                        .is_some_and(|d| !d.composing && d.status == DraftStatus::Valid),
                    _ => false,
                }
        }
        _ => false,
    }
}

fn otp_completion_pair(events: &[Event; 2]) -> bool {
    match (&events[0], &events[1]) {
        (
            Event::OtpInputEvent(w, n, h, r, gpuio_protocol::otp_input::Event::Changed(changed)),
            Event::OtpInputEvent(
                window,
                node,
                handler,
                revision,
                gpuio_protocol::otp_input::Event::Complete(complete),
            ),
        ) => {
            if (w, n, h, r) != (window, node, handler, revision)
                || *r < 0
                || changed.revision <= 0
                || !changed.is_valid()
                || !complete.is_valid()
                || !complete.is_complete()
                || changed.revision.checked_add(1) != Some(complete.revision)
            {
                return false;
            }
            let mut expected = changed.clone();
            expected.revision = complete.revision;
            expected == *complete
        }
        _ => false,
    }
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
    peak_command_bytes: usize,
    events: VecDeque<Output>,
    reserved: usize,
    responses: usize,
    inputs: usize,
    input_bytes: usize,
    faults: BTreeSet<WindowId>,
    in_flight: BTreeSet<WindowId>,
    closed: bool,
    stopped_emitted: bool,
    controls: usize,
}

impl Mailbox {
    /// Encoded-size accounting for accepted commands awaiting host dispatch.
    /// Pop/close release current bytes; the lifetime high-water mark persists.
    pub fn command_queue(&self) -> (usize, usize, usize) {
        (
            self.commands.len(),
            self.command_bytes,
            self.peak_command_bytes,
        )
    }

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
        self.peak_command_bytes = self.peak_command_bytes.max(self.command_bytes);
        self.reserved += 1;
        self.commands.push_back(Queued { message, bytes });
        Ok(())
    }

    /// Reliable latest-state lifecycle lane, bounded by native window slots.
    /// It does not consume user-input capacity or command response reservations.
    pub fn control(&mut self, event: Event) {
        if self.closed {
            return;
        }
        let same = |old: &Event| match (old, &event) {
            (Event::CloseRequested(a), Event::CloseRequested(b)) => a == b,
            (Event::WindowChanged(a, _), Event::WindowChanged(b, _)) => a == b,
            (Event::QuitRequested, Event::QuitRequested)
            | (Event::NotificationPending, Event::NotificationPending)
            | (Event::DesktopPending, Event::DesktopPending)
            | (Event::ReopenRequested, Event::ReopenRequested)
            | (Event::WindowCapabilities(_), Event::WindowCapabilities(_)) => true,
            _ => false,
        };
        if let Some(output) = self.events.iter_mut().find(|output| same(&output.event)) {
            output.event = event;
            return;
        }
        assert!(matches!(
            event,
            Event::CloseRequested(_)
                | Event::WindowChanged(..)
                | Event::QuitRequested
                | Event::ReopenRequested
                | Event::WindowCapabilities(_)
                | Event::NotificationPending
                | Event::DesktopPending
        ));
        assert!(
            self.controls < MAX_WINDOWS * 2 + 5,
            "undrained lifecycle generations"
        );
        self.controls += 1;
        self.events.push_back(Output {
            event,
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

    /// OS callbacks can race final transport shutdown. Check terminal state and
    /// consume the response reservation under the same mailbox lock.
    pub(crate) fn desktop_response(
        &mut self,
        correlation: i64,
        response: gpuio_protocol::desktop::Response,
    ) -> bool {
        if self.closed || self.stopped_emitted {
            return false;
        }
        self.respond(Event::DesktopResponse(correlation, response));
        true
    }

    pub(crate) fn chart_response(
        &mut self,
        correlation: i64,
        response: gpuio_protocol::chart_resource::Response,
    ) -> bool {
        if self.closed || self.stopped_emitted {
            return false;
        }
        self.respond(Event::ChartResponse(correlation, response));
        true
    }

    pub(crate) fn notification_response(
        &mut self,
        correlation: i64,
        response: gpuio_protocol::notification::Response,
    ) -> bool {
        if self.closed || self.stopped_emitted {
            return false;
        }
        self.respond(Event::NotificationResponse(correlation, response));
        true
    }

    /// Coalesce only consecutive render observations for the same window. Never
    /// cross input/response barriers. Other input is ordered and never discarded.
    pub fn input(&mut self, event: Event) -> Result<(), Box<Event>> {
        if matches!(event, Event::ColorInputEvent(..)) {
            return self
                .color_batch(vec![event])
                .map_err(|mut events| Box::new(events.pop().expect("single color event")));
        }
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
        if let Event::InputObserved(
            window,
            node,
            handler,
            revision,
            gpuio_protocol::input::Event::MouseMove(sample),
        ) = &event
            && let Some(last) = self.events.back_mut()
            && matches!(last.class, Class::Input)
            && let Event::InputObserved(
                w,
                n,
                h,
                r,
                gpuio_protocol::input::Event::MouseMove(previous),
            ) = &last.event
            && (window, node, handler, revision) == (w, n, h, r)
            && sample.pressed_button == previous.pressed_button
            && sample.location.modifiers == previous.location.modifiers
            && sample.location.is_valid()
            && previous.location.is_valid()
        {
            last.event = event;
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
        // Only adjacent previews from the same routed owner may collapse. Start,
        // final, cancel, command responses and unrelated input are barriers.
        if let Event::SliderEvent(
            window,
            node,
            handler,
            revision,
            gpuio_protocol::slider::Event::Preview(snapshot),
        ) = &event
            && let Some(last) = self.events.back_mut()
            && let Event::SliderEvent(w, n, h, r, gpuio_protocol::slider::Event::Preview(previous)) =
                &last.event
            && (window, node, handler, revision) == (w, n, h, r)
            && snapshot.dragging == previous.dragging
            && snapshot.committed == previous.committed
            && snapshot.revision > previous.revision
        {
            last.event = event;
            return Ok(());
        }
        let bytes = event_bytes(&event);
        if self
            .events
            .back()
            .is_some_and(|previous| otp_coalesces(previous, &event))
        {
            let last = self.events.back_mut().expect("coalescing predecessor");
            let next_bytes = self.input_bytes - event_bytes(&last.event) + bytes;
            if next_bytes > MAX_INPUT_BYTES {
                return Err(Box::new(event));
            }
            last.event = event;
            self.input_bytes = next_bytes;
            return Ok(());
        }
        if let Event::NumberInputEvent(
            window,
            node,
            handler,
            revision,
            gpuio_protocol::number_input::Event::Changed(snapshot),
        ) = &event
            && let Some(last) = self.events.back_mut()
            && let Event::NumberInputEvent(
                w,
                n,
                h,
                r,
                gpuio_protocol::number_input::Event::Changed(previous),
            ) = &last.event
            && (window, node, handler, revision) == (w, n, h, r)
            && matches!(last.class, Class::Input)
            && snapshot.domain == previous.domain
            && snapshot.committed == previous.committed
            && snapshot.revision > previous.revision
        {
            let next_bytes = self.input_bytes - event_bytes(&last.event) + bytes;
            if next_bytes > MAX_INPUT_BYTES {
                return Err(Box::new(event));
            }
            last.event = event;
            self.input_bytes = next_bytes;
            return Ok(());
        }
        if let Event::ListViewport(window, node, handler, revision, viewport) = &event
            && let Some(last) = self.events.back_mut()
            && let Event::ListViewport(w, n, h, r, previous) = &last.event
            && (window, node, handler, revision, viewport.order_revision)
                == (w, n, h, r, previous.order_revision)
        {
            let next_bytes = self.input_bytes - event_bytes(&last.event) + bytes;
            if next_bytes > MAX_INPUT_BYTES {
                return Err(Box::new(event));
            }
            last.event = event;
            self.input_bytes = next_bytes;
            return Ok(());
        }
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

    /// Admit a routed OTP Changed/Complete pair under one mailbox lock. Failed
    /// validation/count/byte admission preserves the old queue, even when Changed
    /// could replace its tail. Never expose half a completion or lose its boundary.
    pub fn otp_completion(&mut self, events: [Event; 2]) -> Result<(), Box<[Event; 2]>> {
        if self.closed || !otp_completion_pair(&events) {
            return Err(Box::new(events));
        }
        let replacing = self
            .events
            .back()
            .is_some_and(|previous| otp_coalesces(previous, &events[0]));
        let replaced_bytes = if replacing {
            event_bytes(&self.events.back().expect("predecessor").event)
        } else {
            0
        };
        let count = self.inputs + 2 - usize::from(replacing);
        let bytes =
            self.input_bytes - replaced_bytes + event_bytes(&events[0]) + event_bytes(&events[1]);
        if count > MAX_INPUT_EVENTS || bytes > MAX_INPUT_BYTES {
            return Err(Box::new(events));
        }
        let [changed, complete] = events;
        if replacing {
            self.events.back_mut().expect("predecessor").event = changed;
        } else {
            self.events.push_back(Output {
                event: changed,
                class: Class::Input,
            });
        }
        self.events.push_back(Output {
            event: complete,
            class: Class::Input,
        });
        self.inputs = count;
        self.input_bytes = bytes;
        Ok(())
    }

    /// Calendar completion is a discrete, atomic Changed/Selected boundary.
    pub fn calendar_completion(&mut self, events: [Event; 2]) -> Result<(), Box<[Event; 2]>> {
        let valid = match (&events[0], &events[1]) {
            (
                Event::CalendarEvent(w, n, h, r, gpuio_protocol::calendar_input::Event::Changed(a)),
                Event::CalendarEvent(
                    w2,
                    n2,
                    h2,
                    r2,
                    gpuio_protocol::calendar_input::Event::Selected(b),
                ),
            ) => {
                let mut expected = a.clone();
                expected.revision = b.revision;
                (w, n, h, r) == (w2, n2, h2, r2)
                    && *r >= 0
                    && a.revision > 0
                    && a.is_valid()
                    && b.is_valid()
                    && b.selection_allowed
                    && b.selection.is_complete()
                    && a.revision.checked_add(1) == Some(b.revision)
                    && expected == *b
            }
            _ => false,
        };
        let bytes = event_bytes(&events[0]) + event_bytes(&events[1]);
        if self.closed
            || !valid
            || self.inputs + 2 > MAX_INPUT_EVENTS
            || self.input_bytes + bytes > MAX_INPUT_BYTES
        {
            return Err(Box::new(events));
        }
        self.inputs += 2;
        self.input_bytes += bytes;
        for event in events {
            self.events.push_back(Output {
                event,
                class: Class::Input,
            });
        }
        Ok(())
    }

    /// A color owner emits at most two observations per operation. Preflight
    /// count and bytes before changing even a coalescible tail preview.
    pub fn color_batch(&mut self, events: Vec<Event>) -> Result<(), Vec<Event>> {
        if self.closed || events.is_empty() || events.len() > 2 {
            return Err(events);
        }
        if !events.iter().all(|e| matches!(e, Event::ColorInputEvent(_,_,_,revision,event) if *revision >= 0 && event.is_valid())) {
            return Err(events);
        }
        if events.len() == 2 {
            let (Event::ColorInputEvent(w, n, h, r, a), Event::ColorInputEvent(w2, n2, h2, r2, b)) =
                (&events[0], &events[1])
            else {
                unreachable!("checked event kind")
            };
            if (w, n, h, r) != (w2, n2, h2, r2)
                || a.snapshot().revision.checked_add(1) != Some(b.snapshot().revision)
                || !color_pair(a, b)
            {
                return Err(events);
            }
        }
        let replacing = self
            .events
            .back()
            .is_some_and(|old| color_coalesces(old, &events[0]));
        let old_bytes = if replacing {
            event_bytes(&self.events.back().unwrap().event)
        } else {
            0
        };
        let count = self.inputs + events.len() - usize::from(replacing);
        let bytes = self.input_bytes - old_bytes + events.iter().map(event_bytes).sum::<usize>();
        if count > MAX_INPUT_EVENTS || bytes > MAX_INPUT_BYTES {
            return Err(events);
        }
        for (index, event) in events.into_iter().enumerate() {
            if index == 0 && replacing {
                self.events.back_mut().unwrap().event = event;
            } else {
                self.events.push_back(Output {
                    event,
                    class: Class::Input,
                });
            }
        }
        self.inputs = count;
        self.input_bytes = bytes;
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
            Event::CloseRequested(id)
            | Event::WindowChanged(id, _)
            | Event::WindowResponse(_, id, _)
            | Event::Opened(_, id)
            | Event::Closed(_, id)
            | Event::Accepted(id, _)
            | Event::Rejected(id, ..)
            | Event::ListRetained(id, ..)
            | Event::Rendered(id, _)
            | Event::FrameRequested(_, id, _)
            | Event::Press(id, ..)
            | Event::EditorEvent(id, ..)
            | Event::RatingRequested(id, ..)
            | Event::TableInput(id, ..)
            | Event::TreeInput(id, ..)
            | Event::CarouselRequested(id, ..)
            | Event::SliderEvent(id, ..)
            | Event::NumberInputEvent(id, ..)
            | Event::OtpInputEvent(id, ..)
            | Event::CalendarEvent(id, ..)
            | Event::ColorInputEvent(id, ..)
            | Event::Choice(id, ..)
            | Event::OverlayDismissed(id, ..)
            | Event::TooltipOpenChanged(id, ..)
            | Event::CommandInvoked(id, ..)
            | Event::ToastDismissed(id, ..)
            | Event::DragSourceEvent(id, ..)
            | Event::ImageState(id, ..)
            | Event::CanvasEvent(id, ..)
            | Event::ChartEvent(id, ..)
            | Event::DocumentNavigation(id, ..)
            | Event::ExtensionEvent(id, ..)
            | Event::SplitResized(id, ..)
            | Event::AnimationEndpoint(id, ..)
            | Event::AnimationProgramEvent(id, ..)
            | Event::ContainerSelected(id, ..)
            | Event::ListViewport(id, ..)
            | Event::DropTargetEvent(id, ..)
            | Event::PointerEvent(id, ..)
            | Event::InputObserved(id, ..)
            | Event::PaletteDismissed(id, ..)
            | Event::ComboboxSelected(id, ..)
            | Event::SliderResult(_, id, ..)
            | Event::NumberInputResult(_, id, ..)
            | Event::CalendarResult(_, id, ..)
            | Event::ColorInputResult(_, id, ..)
            | Event::OtpInputResult(_, id, ..)
            | Event::EditorResult(_, id, ..)
            | Event::FileDialogResult(_, id, ..)
            | Event::Overloaded(id) => id.slot() == window_slot,
            Event::QuitRequested
            | Event::NotificationPending
            | Event::DesktopPending
            | Event::NotificationResponse(..)
            | Event::DesktopResponse(..)
            | Event::ReopenRequested
            | Event::WindowCapabilities(_)
            | Event::Welcome(..)
            | Event::Failed(..)
            | Event::AssetResponse(..)
            | Event::DocumentResponse(..)
            | Event::ChartResponse(..)
            | Event::CanvasResponse(..)
            | Event::Stopped => false,
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
            if let Event::Accepted(id, _) | Event::Rejected(id, ..) | Event::ListRetained(id, ..) =
                output.event
            {
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

#[cfg(test)]
mod tree_typeahead_tests {
    use super::*;
    #[test]
    fn text_is_charged_and_relative_searches_are_not_coalesced() {
        let text = "a".repeat(256);
        let event = Event::TreeInput(
            WindowId::from_parts(0, 1).unwrap(),
            gpuio_protocol::NodeId::from_parts(0, 1).unwrap(),
            gpuio_protocol::HandlerId::from_parts(0, 1).unwrap(),
            1,
            gpuio_protocol::tree_input::Request::Typeahead {
                text,
                reset: true,
                cycle: false,
            },
        );
        assert_eq!(event_bytes(&event), 512);
        let mut mailbox = Mailbox::default();
        mailbox.input(event.clone()).unwrap();
        mailbox.input(event.clone()).unwrap();
        assert_eq!(mailbox.input_bytes, 1024);
        assert_eq!(mailbox.drain(128), vec![event.clone(), event]);
        assert_eq!(mailbox.input_bytes, 0);
    }
}

#[cfg(test)]
mod table_input_tests {
    use super::*;
    #[test]
    fn table_requests_remain_ordered_and_charge_every_column_string() {
        use binprot::BinProtWrite;
        use gpuio_protocol::{
            HandlerId, NodeId,
            table::{Input, Request},
        };
        let request = Request::Resize(
            (0..64)
                .map(|i| (format!("{i:03}{}", "x".repeat(253)), 160.))
                .collect(),
        );
        let input = Input {
            schema_revision: 1,
            query_generation: 0,
            request,
        };
        assert!(input.is_valid());
        let event = Event::TableInput(
            WindowId::from_parts(0, 1).unwrap(),
            NodeId::from_parts(0, 1).unwrap(),
            HandlerId::from_parts(0, 1).unwrap(),
            1,
            input,
        );
        let mut bytes = Vec::new();
        event.binprot_write(&mut bytes).unwrap();
        assert!(event_bytes(&event) >= bytes.len());
        assert!(event_bytes(&event) > 64 * 256);
        let mut mailbox = Mailbox::default();
        for _ in 0..MAX_INPUT_EVENTS {
            mailbox.input(event.clone()).unwrap();
        }
        assert!(mailbox.input(event.clone()).is_err());
        assert!(mailbox.has_window_output(0));
        let mut drained = Vec::new();
        while mailbox.has_window_output(0) {
            let batch = mailbox.drain(128);
            assert!(!batch.is_empty());
            drained.extend(batch);
        }
        assert_eq!(drained, vec![event; MAX_INPUT_EVENTS]);
        assert_eq!(mailbox.input_bytes, 0);
    }
}

#[cfg(test)]
mod desktop_tests {
    use super::*;
    use binprot::BinProtWrite;
    use gpuio_protocol::desktop::{LinkBatch, MAX_LINK_BYTES, Request, Response};

    #[test]
    fn asynchronous_completion_after_terminal_shutdown_is_discarded() {
        let mut mailbox = Mailbox::default();
        mailbox
            .submit(
                Message::Desktop(1, Request::RegisterScheme("example".into())),
                10,
            )
            .unwrap();
        mailbox.pop().unwrap();
        mailbox.close();
        assert!(!mailbox.desktop_response(1, Response::Registered));
        assert_eq!(mailbox.drain(256), vec![Event::Stopped]);
        assert_eq!(mailbox.reserved, 0);
        assert_eq!(mailbox.responses, 0);
    }

    #[test]
    fn asynchronous_completion_cannot_follow_an_emitted_stop() {
        let mut mailbox = Mailbox::default();
        mailbox
            .submit(
                Message::Desktop(1, Request::RegisterScheme("example".into())),
                10,
            )
            .unwrap();
        mailbox.pop().unwrap();
        mailbox.submit(Message::Shutdown, 1).unwrap();
        mailbox.pop().unwrap();
        mailbox.respond(Event::Stopped);
        assert!(!mailbox.desktop_response(1, Response::Registered));
        assert_eq!(mailbox.drain(256), vec![Event::Stopped]);
        mailbox.close();
        assert_eq!(mailbox.reserved, 0);
    }

    #[test]
    fn represented_document_paths_are_charged_to_response_batches() {
        use gpuio_protocol::{file_path::FilePath, window};
        let id = WindowId::from_parts(0, 1).unwrap();
        let mut path = vec![b'x'; gpuio_protocol::file_path::MAX_PATH_BYTES];
        path[0] = b'/';
        let snapshot = window::Snapshot {
            title: "Document".into(),
            x: 0.,
            y: 0.,
            width: 1.,
            height: 1.,
            content_width: 1.,
            content_height: 1.,
            active: false,
            fullscreen: false,
            maximized: false,
            document: Some(window::Document {
                path: Some(FilePath::new(path).unwrap()),
                edited: true,
            }),
        };
        let mut mailbox = Mailbox::default();
        for correlation in 1..=80 {
            mailbox
                .submit(
                    Message::WindowCommand(correlation, id, window::Command::Observe),
                    5,
                )
                .unwrap();
            mailbox.pop().unwrap();
            let event = Event::WindowResponse(
                correlation,
                id,
                window::Response::Observed(snapshot.clone()),
            );
            let mut encoded = Vec::new();
            event.binprot_write(&mut encoded).unwrap();
            assert!(event_bytes(&event) >= encoded.len());
            mailbox.respond(event);
        }
        let mut count = 0;
        loop {
            let batch = mailbox.drain(256);
            if batch.is_empty() {
                break;
            }
            assert!(batch.len() < 80);
            count += batch.len();
            let mut bytes = Vec::new();
            batch.binprot_write(&mut bytes).unwrap();
            assert!(bytes.len() <= MAX_MESSAGE_BYTES);
        }
        assert_eq!(count, 80);
        assert_eq!(mailbox.reserved, 0);
    }

    #[test]
    fn availability_is_coalesced_and_independent_of_window_input_capacity() {
        let mut mailbox = Mailbox::default();
        for _ in 0..MAX_INPUT_EVENTS {
            mailbox
                .input(Event::Press(
                    WindowId::from_parts(0, 1).unwrap(),
                    gpuio_protocol::NodeId::from_parts(0, 1).unwrap(),
                    gpuio_protocol::HandlerId::from_parts(0, 1).unwrap(),
                    1,
                ))
                .unwrap();
        }
        for _ in 0..1000 {
            mailbox.control(Event::DesktopPending);
        }
        assert_eq!(mailbox.controls, 1);
        mailbox
            .submit(Message::Desktop(7, Request::TakeLinks), 3)
            .unwrap();
        assert!(matches!(
            mailbox.pop(),
            Some(Message::Desktop(7, Request::TakeLinks))
        ));
        let response = Event::DesktopResponse(
            7,
            Response::Links(LinkBatch {
                links: vec!["x".repeat(MAX_LINK_BYTES); 16],
                dropped: 1,
            }),
        );
        mailbox.respond(response.clone());
        let all = mailbox.drain(256);
        assert_eq!(all.len(), MAX_INPUT_EVENTS + 2);
        assert_eq!(all[MAX_INPUT_EVENTS], Event::DesktopPending);
        assert_eq!(all[MAX_INPUT_EVENTS + 1], response);
        assert_eq!(mailbox.controls, 0);
        assert!(!mailbox.has_window_output(0));
    }

    #[test]
    fn large_batches_are_charged_and_split_before_transport_limit() {
        let mut mailbox = Mailbox::default();
        for correlation in 1..=8 {
            mailbox
                .submit(Message::Desktop(correlation, Request::TakeLinks), 3)
                .unwrap();
            mailbox.pop().unwrap();
            let event = Event::DesktopResponse(
                correlation,
                Response::Links(LinkBatch {
                    links: vec!["x".repeat(MAX_LINK_BYTES); 16],
                    dropped: 0,
                }),
            );
            let mut encoded = Vec::new();
            event.binprot_write(&mut encoded).unwrap();
            assert!(event_bytes(&event) >= encoded.len());
            mailbox.respond(event);
        }
        let mut count = 0;
        loop {
            let batch = mailbox.drain(256);
            if batch.is_empty() {
                break;
            }
            assert!(batch.len() < 8);
            count += batch.len();
            let mut bytes = Vec::new();
            batch.binprot_write(&mut bytes).unwrap();
            assert!(bytes.len() <= MAX_MESSAGE_BYTES);
        }
        assert_eq!(count, 8);
        assert_eq!(mailbox.responses, 0);
        assert_eq!(mailbox.reserved, 0);
    }
}

#[cfg(test)]
mod notification_tests {
    use super::*;
    use binprot::BinProtWrite;
    use gpuio_protocol::notification;

    #[test]
    fn notification_batches_are_bounded_and_availability_does_not_consume_response_slots() {
        let mut mailbox = Mailbox::default();
        for _ in 0..1000 {
            mailbox.control(Event::NotificationPending);
            mailbox.control(Event::DesktopPending);
        }
        assert_eq!(mailbox.controls, 2);
        for correlation in 1..=64 {
            mailbox
                .submit(
                    Message::Notification(correlation, notification::Request::TakeEvents),
                    3,
                )
                .unwrap();
            mailbox.pop().unwrap();
            let mut events = vec![
                notification::Event::Action(
                    notification::Receipt {
                        id: i64::MAX,
                        tag: "x".repeat(notification::MAX_TAG_BYTES)
                    },
                    "x".repeat(notification::MAX_ACTION_ID_BYTES)
                );
                notification::MAX_LIVE
            ];
            events.push(notification::Event::Failed(
                notification::Error::Unavailable,
            ));
            let response = notification::Response::Events(events);
            assert!(response.is_valid());
            let event = Event::NotificationResponse(correlation, response.clone());
            let mut bytes = Vec::new();
            event.binprot_write(&mut bytes).unwrap();
            assert!(event_bytes(&event) >= bytes.len());
            assert!(mailbox.notification_response(correlation, response));
        }
        assert!(!mailbox.has_window_output(0));
        let mut count = 0;
        while mailbox.has_output() {
            let batch = mailbox.drain(256);
            assert!(!batch.is_empty() && batch.len() < 64);
            count += batch.len();
            let mut bytes = Vec::new();
            batch.binprot_write(&mut bytes).unwrap();
            assert!(bytes.len() <= MAX_MESSAGE_BYTES);
        }
        assert_eq!(count, 66);
        assert_eq!(
            (mailbox.controls, mailbox.responses, mailbox.reserved),
            (0, 0, 0)
        );
        mailbox.close();
        assert!(!mailbox.notification_response(65, notification::Response::Closed));
        mailbox.control(Event::NotificationPending);
        assert_eq!(mailbox.drain(256), vec![Event::Stopped]);
    }
}

#[cfg(test)]
mod queue_measurement_tests {
    use super::*;
    #[test]
    fn accepted_queue_bytes_peak_survives_pop_close_and_ignores_rejection() {
        let mut mailbox = Mailbox::default();
        assert_eq!(mailbox.command_queue(), (0, 0, 0));
        mailbox
            .submit(
                Message::Chart(1, gpuio_protocol::chart_resource::Request::Create),
                30,
            )
            .unwrap();
        mailbox
            .submit(
                Message::Chart(2, gpuio_protocol::chart_resource::Request::Create),
                50,
            )
            .unwrap();
        assert_eq!(mailbox.command_queue(), (2, 80, 80));
        assert_eq!(
            mailbox.submit(
                Message::Chart(3, gpuio_protocol::chart_resource::Request::Create),
                MAX_MESSAGE_BYTES + 1
            ),
            Err(ErrorCode::LimitExceeded)
        );
        assert_eq!(mailbox.command_queue(), (2, 80, 80));
        mailbox.pop();
        assert_eq!(mailbox.command_queue(), (1, 50, 80));
        mailbox.close();
        assert_eq!(mailbox.command_queue(), (0, 0, 80));
    }
}
