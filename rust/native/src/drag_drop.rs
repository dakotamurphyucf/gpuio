//! Native typed dragging. GPUI owns the source data through its actual gesture;
//! this adapter owns only weak source references and bounded external snapshots.
use super::choice::Route;
use gpui::{
    A11ySubtreeBuilder, App, Bounds, Context, Element, ElementId, GlobalElementId, Hitbox,
    HitboxBehavior, InspectorElementId, IntoElement, LayoutId, Pixels, Window, div, prelude::*, px,
    rgba,
};
use gpuio_protocol::{
    NodeId, WindowId,
    drag_drop::*,
    v1::{Event, Field, PointerModifiers, Style},
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    os::unix::ffi::OsStrExt,
    rc::{Rc, Weak},
    sync::Arc,
};

type Shared = Rc<RefCell<Manager>>;
#[derive(Default)]
struct Global(Shared);
impl gpui::Global for Global {}
fn shared(cx: &mut App) -> Shared {
    cx.default_global::<Global>().0.clone()
}

fn send(route: &Route, event: Option<Event>) {
    if let Some(event) = event
        && !route.transport.input(event)
        && route.session.borrow_mut().overload(route.window)
    {
        route.transport.fault(route.window);
    }
}
fn source_event(route: &Route, gesture: i64, phase: SourcePhase) {
    let event = route.session.borrow().drag_source_event(
        route.window,
        route.node,
        route.handler,
        route.revision,
        SourceSample { gesture, phase },
    );
    send(route, event);
}
fn target_event(route: &Route, sample: TargetSample) {
    let event = route.session.borrow().drop_target_event(
        route.window,
        route.node,
        route.handler,
        route.revision,
        sample,
    );
    send(route, event);
}

// Current policy, rather than the source render's snapshot, determines whether
// the owner can continue participating. Payload changes do not cancel a gesture.
fn unavailable(route: &Route, source: bool) -> Option<CancelReason> {
    let session = route.session.borrow();
    let Some(tree) = session.tree(route.window) else {
        return Some(CancelReason::WindowClosed);
    };
    let Some(node) = tree.get(route.node) else {
        return Some(CancelReason::Removed);
    };
    if node.handler != Some(route.handler) {
        return Some(CancelReason::Reconfigured);
    }
    let disabled = if source {
        match &node.drag_source {
            Some(config) => config.disabled(),
            None => return Some(CancelReason::Removed),
        }
    } else {
        match &node.drop_target {
            Some(config) => config.disabled(),
            None => return Some(CancelReason::Removed),
        }
    };
    if disabled {
        return Some(CancelReason::Disabled);
    }
    if !route.gate.borrow().visible(route.node) {
        return Some(CancelReason::Hidden);
    }
    if !route.gate.borrow().allows(route.node) {
        return Some(CancelReason::Blocked);
    }
    let mut cursor = Some(route.node);
    while let Some(id) = cursor {
        let Some(node) = tree.get(id) else {
            return Some(CancelReason::Removed);
        };
        for style in node.style.iter().rev() {
            if let Style::Fields(fields) = style {
                for field in fields.iter().rev() {
                    if let Field::PointerEvents(value) = field {
                        return (!value).then_some(CancelReason::Disabled);
                    }
                }
            }
        }
        cursor = node.parent;
    }
    None
}

struct Lease {
    gesture: i64,
    route: Route,
    config: Arc<Source>,
    ended: Cell<bool>,
    offered: Cell<bool>,
}
impl Lease {
    fn finish(&self, outcome: Outcome) {
        if !self.ended.replace(true) {
            source_event(&self.route, self.gesture, SourcePhase::Ended(outcome));
        }
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        self.finish(Outcome::Unconfirmed);
    }
}
struct SourceData {
    route: Route,
    config: Arc<Source>,
    manager: Shared,
    lease: RefCell<Option<Rc<Lease>>>,
}
struct Preview(gpui::SharedString);
impl Render for Preview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        // The preview owns text only: retaining a rendered scene must not retain
        // the gesture lease, source route or full data payload.
        div()
            .px(px(10.))
            .py(px(6.))
            .rounded(px(4.))
            .bg(rgba(0x333940ee))
            .text_color(rgba(0xffffffff))
            .child(self.0.clone())
    }
}

#[derive(Clone)]
enum CandidateData {
    Internal(Weak<Lease>),
    External {
        identity: usize,
        payload: Result<Arc<Payload>, Rejection>,
    },
}
#[derive(Clone)]
struct Candidate {
    gesture: i64,
    window: WindowId,
    data: CandidateData,
}
enum SnapshotData {
    Internal(Rc<Lease>),
    External(Result<Arc<Payload>, Rejection>),
}
struct Snapshot {
    gesture: i64,
    data: SnapshotData,
}
impl Snapshot {
    fn payload(&self) -> Result<&Payload, Rejection> {
        match &self.data {
            SnapshotData::Internal(lease) => Ok(lease.config.payload()),
            SnapshotData::External(payload) => payload.as_deref().map_err(|error| *error),
        }
    }
    fn origin(&self) -> Origin {
        match self.data {
            SnapshotData::Internal(_) => Origin::Internal,
            SnapshotData::External(_) => Origin::Desktop,
        }
    }
}
impl Candidate {
    fn snapshot(&self) -> Option<Snapshot> {
        let data = match &self.data {
            CandidateData::Internal(lease) => {
                let lease = lease.upgrade()?;
                if lease.ended.get() {
                    return None;
                }
                SnapshotData::Internal(lease)
            }
            CandidateData::External { payload, .. } => SnapshotData::External(payload.clone()),
        };
        Some(Snapshot {
            gesture: self.gesture,
            data,
        })
    }
}
struct Hover {
    route: Route,
    sample: TargetSample,
}
#[derive(Default)]
struct Manager {
    serial: i64,
    candidate: Option<Candidate>,
    hovers: BTreeMap<(WindowId, NodeId), Hover>,
}
impl Manager {
    fn next_id(&mut self) -> Option<i64> {
        self.serial = self.serial.checked_add(1)?;
        Some(self.serial)
    }
    fn snapshot(&self) -> Option<Snapshot> {
        self.candidate.as_ref()?.snapshot()
    }
    fn set_candidate(&mut self, candidate: Candidate) {
        if let Some(previous) = &self.candidate
            && previous.gesture != candidate.gesture
        {
            self.clear(previous.gesture);
        }
        self.candidate = Some(candidate);
    }

    fn leave(&mut self, key: (WindowId, NodeId)) {
        if let Some(mut hover) = self.hovers.remove(&key) {
            hover.sample.phase = TargetPhase::Left;
            target_event(&hover.route, hover.sample);
        }
    }
    fn clear(&mut self, gesture: i64) {
        if self
            .candidate
            .as_ref()
            .is_some_and(|c| c.gesture == gesture)
        {
            self.candidate = None;
            let keys: Vec<_> = self.hovers.keys().copied().collect();
            for key in keys {
                self.leave(key);
            }
        }
    }
    fn leave_window(&mut self, window: WindowId) {
        let keys: Vec<_> = self
            .hovers
            .keys()
            .filter(|(w, _)| *w == window)
            .copied()
            .collect();
        for key in keys {
            self.leave(key);
        }
        if self
            .candidate
            .as_ref()
            .is_some_and(|c| c.window == window && matches!(c.data, CandidateData::External { .. }))
        {
            self.candidate = None;
        }
    }
    fn external(&mut self, window: WindowId, paths: &gpui::ExternalPaths) {
        let identity = paths as *const _ as usize;
        if self.candidate.as_ref().is_some_and(|c| c.window == window && matches!(c.data, CandidateData::External { identity: old, .. } if old == identity)) { return; }
        if let Some(previous) = &self.candidate {
            self.clear(previous.gesture);
        }
        let Some(gesture) = self.next_id() else {
            return;
        };
        self.candidate = Some(Candidate {
            gesture,
            window,
            data: CandidateData::External {
                identity,
                payload: external_payload(paths).map(Arc::new),
            },
        });
    }
}
fn external_payload(paths: &gpui::ExternalPaths) -> Result<Payload, Rejection> {
    if paths.paths().len() > MAX_FILES {
        return Err(Rejection::LimitExceeded);
    }
    let mut total = 0;
    let mut files = Vec::with_capacity(paths.paths().len());
    for path in paths.paths() {
        let bytes = path.as_os_str().as_bytes();
        total += bytes.len();
        if total > MAX_DATA_BYTES || bytes.len() > gpuio_protocol::file_path::MAX_PATH_BYTES {
            return Err(Rejection::LimitExceeded);
        }
        let path = gpuio_protocol::file_path::FilePath::new(bytes.to_vec())
            .map_err(|_| Rejection::InvalidData)?;
        files.push(File {
            path,
            is_directory: None,
        });
    }
    Payload::files(files).map_err(|_| Rejection::InvalidData)
}
fn desktop_supported(window: &Window) -> bool {
    HasWindowHandle::window_handle(window).is_ok_and(|handle| {
        matches!(
            handle.as_raw(),
            RawWindowHandle::AppKit(_) | RawWindowHandle::Wayland(_)
        )
    })
}

pub(super) fn source(
    element: gpui::Stateful<gpui::Div>,
    route: Route,
    config: Arc<Source>,
    cx: &mut App,
) -> gpui::Stateful<gpui::Div> {
    if unavailable(&route, true).is_some() {
        return element;
    }
    let manager = shared(cx);
    element
        .on_drag(
            SourceData {
                route,
                config,
                manager,
                lease: RefCell::new(None),
            },
            |data, _, window, cx| {
                let preview = cx.new(|_| Preview(data.config.label().to_owned().into()));
                let Some(gesture) = data.manager.borrow_mut().next_id() else {
                    return preview;
                };
                let reason = unavailable(&data.route, true)
                    .or_else(|| window.captured_hitbox().map(|_| CancelReason::Blocked));
                let lease = Rc::new(Lease {
                    gesture,
                    route: data.route.clone(),
                    config: data.config.clone(),
                    ended: Cell::new(reason.is_some()),
                    offered: Cell::new(false),
                });
                data.manager.borrow_mut().set_candidate(Candidate {
                    gesture,
                    window: data.route.window,
                    data: CandidateData::Internal(Rc::downgrade(&lease)),
                });
                *data.lease.borrow_mut() = Some(lease);
                if reason.is_none() {
                    source_event(
                        &data.route,
                        gesture,
                        SourcePhase::Started(data.config.payload().clone()),
                    );
                    if data.config.allow_desktop_files() && !desktop_supported(window) {
                        source_event(&data.route, gesture, SourcePhase::DesktopUnavailable);
                    }
                } else {
                    let manager = data.manager.clone();
                    window.defer(cx, move |window, cx| {
                        let ours = manager
                            .borrow()
                            .candidate
                            .as_ref()
                            .is_some_and(|c| c.gesture == gesture);
                        if ours {
                            manager.borrow_mut().clear(gesture);
                            cx.stop_active_drag(window);
                        }
                    });
                }
                preview
            },
        )
        .external_drag_payload::<SourceData>(|data, _, _| {
            let lease = data.lease.borrow().clone()?;
            if lease.ended.get()
                || !lease.config.allow_desktop_files()
                || unavailable(&lease.route, true).is_some()
            {
                return None;
            }
            let PayloadRef::Files(files) = lease.config.payload().as_ref() else {
                return None;
            };
            let entries = files.iter().map(|file| {
                (
                    std::path::PathBuf::from(std::ffi::OsStr::from_bytes(file.path.as_bytes())),
                    file.is_directory.expect("validated desktop metadata"),
                )
            });
            let payload = gpui::ExternalDragPayload::Files(gpui::FileDragPaths::new(entries));
            if !lease.offered.replace(true) {
                source_event(&lease.route, lease.gesture, SourcePhase::DesktopOffered);
            }
            Some(payload)
        })
}

fn sample(
    gesture: i64,
    phase: TargetPhase,
    point: gpui::Point<Pixels>,
    bounds: Bounds<Pixels>,
    keys: gpui::Modifiers,
) -> TargetSample {
    TargetSample {
        gesture,
        phase,
        window_x: f32::from(point.x) as f64,
        window_y: f32::from(point.y) as f64,
        local_x: f32::from(point.x - bounds.origin.x) as f64,
        local_y: f32::from(point.y - bounds.origin.y) as f64,
        modifiers: PointerModifiers {
            shift: keys.shift,
            control: keys.control,
            alt: keys.alt,
            command: keys.platform,
            function: keys.function,
        },
    }
}
fn accepts(route: &Route, payload: &Payload) -> bool {
    unavailable(route, false).is_none()
        && route
            .session
            .borrow()
            .tree(route.window)
            .and_then(|tree| tree.get(route.node))
            .and_then(|node| node.drop_target.as_ref())
            .is_some_and(|config| config.accepts(payload))
}
fn hover(
    manager: &Shared,
    route: &Route,
    hitbox: &Hitbox,
    event: &gpui::MouseMoveEvent,
    window: &Window,
) {
    let snapshot = manager.borrow().snapshot();
    let key = (route.window, route.node);
    let eligible = snapshot.as_ref().is_some_and(|snapshot| {
        snapshot
            .payload()
            .is_ok_and(|payload| accepts(route, payload))
    }) && hitbox.is_hovered_at(event.position, window);
    if !eligible {
        manager.borrow_mut().leave(key);
        return;
    }
    let snapshot = snapshot.unwrap();
    let exists =
        manager.borrow().hovers.get(&key).is_some_and(|h| {
            h.sample.gesture == snapshot.gesture && h.route.handler == route.handler
        });
    let phase = if exists {
        TargetPhase::Moved
    } else {
        manager.borrow_mut().leave(key);
        TargetPhase::Entered(Offer::new(snapshot.payload().unwrap(), snapshot.origin()))
    };
    let sample = sample(
        snapshot.gesture,
        phase,
        event.position,
        hitbox.bounds,
        event.modifiers,
    );
    manager.borrow_mut().hovers.insert(
        key,
        Hover {
            route: route.clone(),
            sample: sample.clone(),
        },
    );
    target_event(route, sample);
}
fn drop_on(
    manager: &Shared,
    route: &Route,
    hitbox: &Hitbox,
    event: &gpui::MouseUpEvent,
    window: &mut Window,
    cx: &mut App,
) {
    if event.button != gpui::MouseButton::Left
        || !cx.has_active_drag()
        || !hitbox.is_hovered_at(event.position, window)
        || unavailable(route, false).is_some()
    {
        return;
    }
    let Some(snapshot) = manager.borrow().snapshot() else {
        return;
    };
    let phase = match snapshot.payload() {
        Ok(payload) if accepts(route, payload) => TargetPhase::Dropped(payload.clone()),
        Ok(_) => return, // An accepting ancestor may handle this drag.
        Err(reason) => {
            let files = route
                .session
                .borrow()
                .tree(route.window)
                .and_then(|tree| tree.get(route.node))
                .and_then(|node| node.drop_target.as_ref())
                .is_some_and(|c| c.accepted_formats().contains(&Format::Files));
            if !files {
                return;
            }
            TargetPhase::Rejected(reason)
        }
    };
    target_event(
        route,
        sample(
            snapshot.gesture,
            phase,
            event.position,
            hitbox.bounds,
            event.modifiers,
        ),
    );
    if let SnapshotData::Internal(lease) = &snapshot.data {
        lease.finish(Outcome::InternalDrop);
    }
    manager.borrow_mut().clear(snapshot.gesture);
    // Release all manager borrows before GPUI drops its source lease.
    cx.stop_active_drag(window);
    cx.stop_propagation();
    window.refresh();
}

pub(super) fn sync(window_id: WindowId, window: &mut Window, cx: &mut App) {
    let Some(manager) = cx.try_global::<Global>().map(|g| g.0.clone()) else {
        return;
    };
    let snapshot = manager.borrow().snapshot();
    if let Some(Snapshot {
        data: SnapshotData::Internal(lease),
        ..
    }) = snapshot
        && lease.route.window == window_id
        && let Some(reason) = unavailable(&lease.route, true)
    {
        lease.finish(Outcome::Cancelled(reason));
        manager.borrow_mut().clear(lease.gesture);
        cx.stop_active_drag(window);
    }
    let snapshot = manager.borrow().snapshot();
    let keys: Vec<_> = manager
        .borrow()
        .hovers
        .iter()
        .filter(|((w, _), hover)| {
            *w == window_id
                && snapshot.as_ref().is_none_or(|snapshot| {
                    !snapshot
                        .payload()
                        .is_ok_and(|payload| accepts(&hover.route, payload))
                })
        })
        .map(|(key, _)| *key)
        .collect();
    for key in keys {
        manager.borrow_mut().leave(key);
    }
}
pub(super) fn cancel(
    window_id: WindowId,
    reason: CancelReason,
    window: &mut Window,
    cx: &mut App,
) -> bool {
    let Some(manager) = cx.try_global::<Global>().map(|g| g.0.clone()) else {
        return false;
    };
    let snapshot = manager.borrow().snapshot();
    let Some(snapshot) = snapshot else {
        manager.borrow_mut().leave_window(window_id);
        return false;
    };
    let applicable = match &snapshot.data {
        SnapshotData::Internal(lease) => lease.route.window == window_id,
        SnapshotData::External(_) => manager
            .borrow()
            .candidate
            .as_ref()
            .is_some_and(|c| c.window == window_id),
    };
    if !applicable {
        return false;
    }
    if reason == CancelReason::WindowInactive
        && matches!(&snapshot.data, SnapshotData::Internal(lease) if lease.offered.get())
    {
        return false;
    }
    if let SnapshotData::Internal(lease) = &snapshot.data {
        lease.finish(Outcome::Cancelled(reason));
    }
    manager.borrow_mut().clear(snapshot.gesture);
    cx.stop_active_drag(window);
    true
}

pub(super) struct Region<E> {
    pub element: E,
    pub route: Route,
}
impl<E: Element> IntoElement for Region<E> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl<E: Element> Element for Region<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = (Hitbox, E::PrepaintState);
    fn id(&self) -> Option<ElementId> {
        self.element.id()
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.element.source_location()
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.element.request_layout(id, inspector, window, cx)
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        let child = self
            .element
            .prepaint(id, inspector, bounds, layout, window, cx);
        (hitbox, child)
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        state: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let manager = shared(cx);
        let route = self.route.clone();
        let hitbox = state.0.clone();
        window.on_mouse_event(move |event: &gpui::MouseMoveEvent, phase, window, _| {
            if phase.capture() {
                hover(&manager, &route, &hitbox, event, window);
            }
        });
        // Register before descendants: their drop targets run first in bubbling.
        let manager = shared(cx);
        let route = self.route.clone();
        let hitbox = state.0.clone();
        window.on_mouse_event(move |event: &gpui::MouseUpEvent, phase, window, cx| {
            if phase.bubble() {
                drop_on(&manager, &route, &hitbox, event, window, cx);
            }
        });
        self.element
            .paint(id, inspector, bounds, layout, &mut state.1, window, cx);
    }
    fn a11y_role(&self) -> Option<gpui::accesskit::Role> {
        self.element.a11y_role()
    }
    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        self.element.write_a11y_info(node);
    }
    fn a11y_synthetic_children(
        &mut self,
        state: &mut Self::PrepaintState,
        builder: &mut A11ySubtreeBuilder,
    ) {
        self.element.a11y_synthetic_children(&mut state.1, builder);
    }
}

pub(super) fn root(element: gpui::Div, window_id: WindowId, cx: &mut App) -> gpui::Div {
    let manager = shared(cx);
    let external_manager = manager.clone();
    element
        .on_drag_move::<gpui::ExternalPaths>(move |event, _, _| {
            let paths = event
                .dragged_item()
                .downcast_ref::<gpui::ExternalPaths>()
                .expect("GPUI typed drag");
            external_manager.borrow_mut().external(window_id, paths);
        })
        .on_drag_move::<SourceData>(move |event, _, _| {
            let data = event
                .dragged_item()
                .downcast_ref::<SourceData>()
                .expect("GPUI typed drag");
            if let Some(lease) = data.lease.borrow().as_ref()
                && !lease.ended.get()
            {
                manager.borrow_mut().set_candidate(Candidate {
                    gesture: lease.gesture,
                    window: lease.route.window,
                    data: CandidateData::Internal(Rc::downgrade(lease)),
                });
            }
        })
}

pub(super) fn install_cleanup(window_id: WindowId, window: &mut Window, cx: &mut App) {
    let manager = shared(cx);
    window.on_mouse_event(move |_: &gpui::MouseUpEvent, phase, window, cx| {
        if phase.capture() {
            let gesture = manager.borrow().candidate.as_ref().map(|c| c.gesture);
            if let Some(gesture) = gesture {
                let manager = manager.clone();
                window.defer(cx, move |window, _| {
                    manager.borrow_mut().clear(gesture);
                    window.refresh();
                });
            }
        }
    });
    let manager = shared(cx);
    window.on_mouse_event(move |event: &gpui::FileDropEvent, phase, _, _| {
        if phase.capture()
            && matches!(
                event,
                gpui::FileDropEvent::Exited | gpui::FileDropEvent::Ended
            )
        {
            manager.borrow_mut().leave_window(window_id);
        }
    });
}

pub(super) fn shutdown(cx: &mut App) {
    let Some(manager) = cx.try_global::<Global>().map(|g| g.0.clone()) else {
        return;
    };
    let snapshot = manager.borrow().snapshot();
    if let Some(snapshot) = snapshot {
        if let SnapshotData::Internal(lease) = &snapshot.data {
            lease.finish(Outcome::Cancelled(CancelReason::WindowClosed));
        }
        manager.borrow_mut().clear(snapshot.gesture);
        for handle in cx.windows() {
            let _ = cx.update_window(handle, |_, window, cx| {
                cx.stop_active_drag(window);
            });
        }
    }
    // Includes stale weak source references and hovers whose owners already closed.
    let keys: Vec<_> = manager.borrow().hovers.keys().copied().collect();
    for key in keys {
        manager.borrow_mut().leave(key);
    }
    manager.borrow_mut().candidate = None;
}

#[cfg(feature = "native-tests")]
pub(super) fn source_config_weak(cx: &App) -> Option<std::sync::Weak<Source>> {
    let manager = &cx.try_global::<Global>()?.0;
    let snapshot = manager.borrow().snapshot()?;
    match snapshot.data {
        SnapshotData::Internal(lease) => Some(Arc::downgrade(&lease.config)),
        SnapshotData::External(_) => None,
    }
}
#[cfg(feature = "native-tests")]
pub(super) fn is_empty(cx: &App) -> bool {
    cx.try_global::<Global>().is_none_or(|g| {
        let manager = g.0.borrow();
        manager.candidate.is_none() && manager.hovers.is_empty()
    })
}
