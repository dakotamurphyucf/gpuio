//! Cancellable guest-queue Wayland exports. The host owns and reads the display;
//! this adapter never reads the socket, prepares a read, or performs a roundtrip.
//! The native owner must retain the host surface/display until this value drops.
use std::{
    collections::BTreeMap,
    ffi::c_void,
    ptr::NonNull,
    sync::{Arc, Mutex},
};
use wayland_backend::sys::client::{Backend, ObjectId};
use wayland_client::{
    Connection, Dispatch, EventQueue, Proxy, QueueHandle,
    protocol::{wl_callback, wl_registry, wl_surface::WlSurface},
};
use wayland_protocols::xdg::foreign::{
    zv1::client::{zxdg_exported_v1, zxdg_exporter_v1},
    zv2::client::{zxdg_exported_v2, zxdg_exporter_v2},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Unsupported,
    NativeFailure,
}

enum Exporter {
    V1(zxdg_exporter_v1::ZxdgExporterV1),
    V2(zxdg_exporter_v2::ZxdgExporterV2),
}
impl Exporter {
    fn export(&self, surface: &WlSurface, queue: &QueueHandle<State>, id: u64) -> Exported {
        match self {
            Self::V1(exporter) => Exported::V1(exporter.export(surface, queue, id)),
            Self::V2(exporter) => Exported::V2(exporter.export_toplevel(surface, queue, id)),
        }
    }
    fn destroy(&self) {
        match self {
            Self::V1(exporter) => exporter.destroy(),
            Self::V2(exporter) => exporter.destroy(),
        }
    }
}
enum Exported {
    V1(zxdg_exported_v1::ZxdgExportedV1),
    V2(zxdg_exported_v2::ZxdgExportedV2),
}
impl Exported {
    fn destroy(&self) {
        match self {
            Self::V1(exported) => exported.destroy(),
            Self::V2(exported) => exported.destroy(),
        }
    }
}

#[derive(Default)]
struct State {
    // Choose only after registry sync, so advertisement order cannot choose v1
    // prematurely when the compositor supports v2 too. Removed globals vanish.
    v1: Option<u32>,
    v2: Option<u32>,
    registry_ready: bool,
    parents: BTreeMap<u64, Option<Result<String, Error>>>,
}

/// One guest registry/queue per application display. Wayland retains registry
/// resources until disconnect, so creating a new registry per picker leaks server
/// resources even when its client proxy is dropped. Clone this shared owner.
#[derive(Clone)]
pub struct Display(Arc<Mutex<Inner>>);
struct Inner {
    connection: Connection,
    queue: EventQueue<State>,
    state: State,
    registry: Option<wl_registry::WlRegistry>,
    exporter: Option<Exporter>,
    next_id: u64,
}

/// One exported surface; does not destroy the host surface or close the display.
pub struct Export {
    display: Display,
    id: u64,
    surface: WlSurface,
    exported: Option<Exported>,
}

impl Display {
    /// Create the application's guest registry on a system-libwayland display.
    /// No blocking I/O, reader thread or process-global lookup occurs.
    ///
    /// # Safety
    /// `display` must remain live until ALL clones and exports have dropped,
    /// including on unconditional quit. The host must keep reading its display
    /// during export. Borrowed handles alone do not establish this lifetime.
    pub unsafe fn new(display: NonNull<c_void>) -> Self {
        // SAFETY: caller guarantees native identity and lifetime. Guest mode
        // leaves display ownership and socket reading with that caller.
        let backend = unsafe { Backend::from_foreign_display(display.as_ptr().cast()) };
        let connection = Connection::from_backend(backend);
        let queue = connection.new_event_queue();
        let handle = queue.handle();
        let registry = connection.display().get_registry(&handle, ());
        connection.display().sync(&handle, ());
        Self(Arc::new(Mutex::new(Inner {
            connection,
            queue,
            state: State::default(),
            registry: Some(registry),
            exporter: None,
            next_id: 0,
        })))
    }

    /// Reserve one export. The returned owner can be dropped before polling.
    ///
    /// # Safety
    /// `surface` must be a live toplevel wl_surface on THIS display and remain
    /// alive until the returned Export drops. It must not be destroyed by the
    /// host while a pending export or portal request still retains this value.
    pub unsafe fn export(&self, surface: NonNull<c_void>) -> Result<Export, Error> {
        let mut inner = self.0.lock().unwrap();
        let id = unsafe { ObjectId::from_ptr(WlSurface::interface(), surface.as_ptr().cast()) }
            .map_err(|_| Error::NativeFailure)?;
        let surface =
            WlSurface::from_id(&inner.connection, id).map_err(|_| Error::NativeFailure)?;
        let id = inner.next_id.checked_add(1).ok_or(Error::NativeFailure)?;
        inner.next_id = id;
        inner.state.parents.insert(id, None);
        Ok(Export {
            display: self.clone(),
            id,
            surface,
            exported: None,
        })
    }
}

impl Inner {
    fn dispatch(&mut self) -> Result<(), Error> {
        self.queue
            .dispatch_pending(&mut self.state)
            .map_err(|_| Error::NativeFailure)?;
        if self.state.registry_ready
            && let Some(registry) = self.registry.take()
        {
            let queue = self.queue.handle();
            self.exporter = if let Some(name) = self.state.v2 {
                Some(Exporter::V2(registry.bind(name, 1, &queue, ())))
            } else {
                self.state
                    .v1
                    .map(|name| Exporter::V1(registry.bind(name, 1, &queue, ())))
            };
            // Keep the bound exporter, but stop registry events after discovery.
            // Otherwise an idle app would accumulate guest global notifications
            // while no picker is polling. The server registry allocation stays
            // one per connection; availability is a connection-lifetime snapshot.
            self.connection
                .backend()
                .destroy_object(&registry.id())
                .map_err(|_| Error::NativeFailure)?;
        }
        if self.state.registry_ready && self.exporter.is_none() {
            return Err(Error::Unsupported);
        }
        Ok(())
    }
}

impl Export {
    /// Process already-read guest events and flush outgoing requests. None means
    /// pending. The caller supplies timeout/cancellation and may drop at any
    /// point. Retain the ready owner throughout the portal request.
    pub fn poll(&mut self) -> Result<Option<String>, Error> {
        let mut inner = self.display.0.lock().unwrap();
        inner.dispatch()?;
        if self.exported.is_none()
            && let Some(exporter) = &inner.exporter
        {
            self.exported = Some(exporter.export(&self.surface, &inner.queue.handle(), self.id));
        }
        flush(&inner.connection)?;
        inner
            .state
            .parents
            .get(&self.id)
            .unwrap()
            .clone()
            .transpose()
    }
}

impl Drop for Export {
    fn drop(&mut self) {
        // A protocol dispatch panic is contained by the native worker. Cleanup
        // must still release its owned objects without a second poison panic.
        let mut inner = self
            .display
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        inner.state.parents.remove(&self.id);
        // No reply or foreground callback is required. This also works before
        // Handle arrives; libwayland discards events for the destroyed proxy.
        if let Some(exported) = &self.exported {
            exported.destroy();
        }
        let _ = flush(&inner.connection);
    }
}
impl Drop for Inner {
    fn drop(&mut self) {
        // The final display owner drops after all exports, before the host
        // destroys its display. wl_registry has no protocol destructor; the
        // guest backend removes its client proxy and any unfinished sync proxy.
        if let Some(exporter) = &self.exporter {
            exporter.destroy();
        }
        let _ = flush(&self.connection);
    }
}

fn flush(connection: &Connection) -> Result<(), Error> {
    match connection.flush() {
        Ok(()) => Ok(()),
        Err(wayland_backend::client::WaylandError::Io(error))
            if error.kind() == std::io::ErrorKind::WouldBlock =>
        {
            Ok(())
        }
        Err(_) => Err(Error::NativeFailure),
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } if version >= 1 => match interface.as_str() {
                "zxdg_exporter_v1" => state.v1 = Some(name),
                "zxdg_exporter_v2" => state.v2 = Some(name),
                _ => (),
            },
            wl_registry::Event::GlobalRemove { name } => {
                if state.v1 == Some(name) {
                    state.v1 = None;
                }
                if state.v2 == Some(name) {
                    state.v2 = None;
                }
            }
            _ => (),
        }
    }
}
impl Dispatch<wl_callback::WlCallback, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            state.registry_ready = true;
        }
    }
}
fn handle(state: &mut State, id: u64, handle: String) {
    let Some(parent) = state.parents.get_mut(&id) else {
        return;
    };
    *parent = Some(
        if handle.is_empty() || handle.len() > 4096 || handle.contains('\0') {
            Err(Error::NativeFailure)
        } else {
            Ok(format!("wayland:{handle}"))
        },
    );
}
impl Dispatch<zxdg_exported_v1::ZxdgExportedV1, u64> for State {
    fn event(
        state: &mut Self,
        _: &zxdg_exported_v1::ZxdgExportedV1,
        event: zxdg_exported_v1::Event,
        id: &u64,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zxdg_exported_v1::Event::Handle { handle: value } = event {
            handle(state, *id, value);
        }
    }
}
impl Dispatch<zxdg_exported_v2::ZxdgExportedV2, u64> for State {
    fn event(
        state: &mut Self,
        _: &zxdg_exported_v2::ZxdgExportedV2,
        event: zxdg_exported_v2::Event,
        id: &u64,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zxdg_exported_v2::Event::Handle { handle: value } = event {
            handle(state, *id, value);
        }
    }
}
wayland_client::delegate_noop!(State: ignore zxdg_exporter_v1::ZxdgExporterV1);
wayland_client::delegate_noop!(State: ignore zxdg_exporter_v2::ZxdgExporterV2);
