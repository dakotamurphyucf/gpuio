//! A private wire-protocol peer, not a desktop/compositor GUI. libwayland is the
//! real client: the host reads the socket and the guest dispatches only its queue.
use gpuio_wayland::{Display, Error, Export};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    os::unix::net::UnixStream,
    ptr::NonNull,
    thread::JoinHandle,
    time::Duration,
};
use wayland_client::{
    Connection, EventQueue, Proxy,
    protocol::{wl_compositor, wl_registry, wl_surface},
};

struct Host;
wayland_client::delegate_noop!(Host: ignore wl_registry::WlRegistry);
wayland_client::delegate_noop!(Host: ignore wl_compositor::WlCompositor);
wayland_client::delegate_noop!(Host: ignore wl_surface::WlSurface);

#[derive(Default, Debug)]
struct Observed {
    registries: usize,
    exports: Vec<(u32, u32)>,
    destroyed: usize,
    exporters_destroyed: usize,
}

fn integer(bytes: &[u8]) -> u32 {
    u32::from_ne_bytes(bytes[..4].try_into().unwrap())
}
fn string(value: &str) -> Vec<u8> {
    let mut bytes = ((value.len() + 1) as u32).to_ne_bytes().to_vec();
    bytes.extend_from_slice(value.as_bytes());
    bytes.push(0);
    bytes.resize(bytes.len().next_multiple_of(4), 0);
    bytes
}
fn event(socket: &mut UnixStream, id: u32, opcode: u32, body: &[u8]) {
    socket.write_all(&id.to_ne_bytes()).unwrap();
    socket
        .write_all(&(((body.len() as u32 + 8) << 16) | opcode).to_ne_bytes())
        .unwrap();
    socket.write_all(body).unwrap();
}
fn global(socket: &mut UnixStream, registry: u32, name: u32, interface: &str) {
    let mut body = name.to_ne_bytes().to_vec();
    body.extend(string(interface));
    body.extend(1u32.to_ne_bytes());
    event(socket, registry, 0, &body);
}
fn serve(mut socket: UnixStream, versions: &[u32], value: &str) -> Observed {
    socket
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut objects = BTreeMap::from([(1, "wl_display".to_owned())]);
    let mut observed = Observed::default();
    loop {
        let mut header = [0u8; 8];
        match socket.read_exact(&mut header) {
            Ok(()) => (),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::ConnectionReset
                ) =>
            {
                break;
            }
            Err(error) => panic!("private Wayland peer: {error}"),
        }
        let id = integer(&header);
        let header = integer(&header[4..]);
        let opcode = header & 0xffff;
        let size = (header >> 16) as usize;
        assert!((8..=65536).contains(&size));
        let mut body = vec![0; size - 8];
        socket.read_exact(&mut body).unwrap();
        match (objects.get(&id).unwrap().as_str(), opcode) {
            ("wl_display", 1) => {
                let registry = integer(&body);
                objects.insert(registry, "wl_registry".into());
                observed.registries += 1;
                global(&mut socket, registry, 10, "wl_compositor");
                for version in versions {
                    global(
                        &mut socket,
                        registry,
                        10 + version,
                        &format!("zxdg_exporter_v{version}"),
                    );
                }
            }
            ("wl_display", 0) => {
                let callback = integer(&body);
                event(&mut socket, callback, 0, &0u32.to_ne_bytes());
                event(&mut socket, 1, 1, &callback.to_ne_bytes());
            }
            ("wl_registry", 0) => {
                let length = integer(&body[4..]) as usize;
                let interface = std::str::from_utf8(&body[8..8 + length - 1])
                    .unwrap()
                    .to_owned();
                let new_id = integer(&body[body.len() - 4..]);
                objects.insert(new_id, interface);
            }
            ("wl_compositor", 0) => {
                objects.insert(integer(&body), "wl_surface".into());
            }
            ("zxdg_exporter_v1" | "zxdg_exporter_v2", 1) => {
                let version = if objects[&id].ends_with("v2") { 2 } else { 1 };
                let exported = integer(&body);
                let surface = integer(&body[4..]);
                assert_eq!(objects[&surface], "wl_surface");
                observed.exports.push((version, surface));
                objects.insert(exported, "exported".into());
                event(&mut socket, exported, 0, &string(value));
            }
            ("zxdg_exporter_v1" | "zxdg_exporter_v2", 0) => {
                observed.exporters_destroyed += 1;
                objects.remove(&id);
                event(&mut socket, 1, 1, &id.to_ne_bytes());
            }
            ("exported", 0) => {
                observed.destroyed += 1;
                objects.remove(&id);
                event(&mut socket, 1, 1, &id.to_ne_bytes());
            }
            other => panic!("unexpected Wayland request {other:?}"),
        }
    }
    observed
}

struct Fixture {
    connection: Connection,
    queue: EventQueue<Host>,
    first: wl_surface::WlSurface,
    second: wl_surface::WlSurface,
    peer: JoinHandle<Observed>,
}
impl Fixture {
    fn new(versions: &'static [u32], value: &'static str) -> Self {
        let (client, server) = UnixStream::pair().unwrap();
        let peer = std::thread::spawn(move || serve(server, versions, value));
        let connection = Connection::from_socket(client).unwrap();
        let mut queue = connection.new_event_queue::<Host>();
        let handle = queue.handle();
        let registry = connection.display().get_registry(&handle, ());
        // These IDs are the private fixture's declared globals, not environment
        // discovery. Initial host roundtrip creates two distinct native surfaces.
        let compositor: wl_compositor::WlCompositor = registry.bind(10, 1, &handle, ());
        let first = compositor.create_surface(&handle, ());
        let second = compositor.create_surface(&handle, ());
        queue.roundtrip(&mut Host).unwrap();
        Self {
            connection,
            queue,
            first,
            second,
            peer,
        }
    }
    fn display(&self) -> Display {
        // SAFETY: tests explicitly drop this guest and all exports before host
        // connection teardown. Both surfaces belong to that same connection.
        unsafe {
            Display::new(NonNull::new(self.connection.backend().display_ptr().cast()).unwrap())
        }
    }
    fn export(&self, display: &Display, second: bool) -> Export {
        let surface = if second { &self.second } else { &self.first };
        unsafe {
            display
                .export(NonNull::new(surface.id().as_ptr().cast()).unwrap())
                .unwrap()
        }
    }
    fn ready(&mut self, export: &mut Export) -> Result<String, Error> {
        for _ in 0..8 {
            if let Some(parent) = export.poll()? {
                return Ok(parent);
            }
            // ONLY the host reads. The guest implementation must work using
            // dispatch_pending alone, including events read by this other queue.
            self.queue.roundtrip(&mut Host).unwrap();
        }
        panic!("guest did not receive its exported handle")
    }
    fn finish(mut self) -> Observed {
        // Flush all destructors; a successful host sync proves that dropping the
        // guest did not disconnect the host or destroy its native surfaces.
        self.queue.roundtrip(&mut Host).unwrap();
        drop(self.queue);
        drop(self.connection);
        self.peer.join().unwrap()
    }
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "requires Linux system libwayland-client"
)]
fn exact_surfaces_share_one_registry_and_dispose_every_export() {
    let mut fixture = Fixture::new(&[1, 2], "parent-token");
    let display = fixture.display();
    let first_id = fixture.first.id().protocol_id();
    let second_id = fixture.second.id().protocol_id();
    for _ in 0..32 {
        let mut first = fixture.export(&display, false);
        let mut second = fixture.export(&display, true);
        assert_eq!(fixture.ready(&mut first).unwrap(), "wayland:parent-token");
        assert_eq!(fixture.ready(&mut second).unwrap(), "wayland:parent-token");
        drop(first);
        drop(second);
    }
    drop(display);
    let observed = fixture.finish();
    assert_eq!(
        observed.registries, 2,
        "one host registry and one shared guest registry"
    );
    assert_eq!(observed.exports.len(), 64);
    assert_eq!(observed.destroyed, 64);
    assert_eq!(observed.exporters_destroyed, 1);
    for pair in observed.exports.chunks_exact(2) {
        assert_eq!(pair, [(2, first_id), (2, second_id)]);
    }
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "requires Linux system libwayland-client"
)]
fn cancellation_before_registry_and_before_handle_keeps_host_usable() {
    let mut fixture = Fixture::new(&[1], "v1-token");
    let display = fixture.display();
    drop(fixture.export(&display, false)); // cancelled before registry dispatch
    let mut pending = fixture.export(&display, true);
    assert_eq!(pending.poll().unwrap(), None);
    fixture.queue.roundtrip(&mut Host).unwrap();
    assert_eq!(pending.poll().unwrap(), None); // export issued, handle not dispatched
    drop(pending);
    let mut next = fixture.export(&display, false);
    assert_eq!(fixture.ready(&mut next).unwrap(), "wayland:v1-token");
    drop(next);
    drop(display);
    let observed = fixture.finish();
    assert_eq!(observed.registries, 2);
    assert_eq!(observed.exports.len(), 2);
    assert_eq!(observed.destroyed, 2);
    assert_eq!(observed.exporters_destroyed, 1);
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "requires Linux system libwayland-client"
)]
fn absent_export_protocol_and_invalid_handle_fail_explicitly() {
    for (versions, value, expected) in [
        (&[][..], "unused", Error::Unsupported),
        (&[2][..], "", Error::NativeFailure),
    ] {
        let mut fixture = Fixture::new(versions, value);
        let display = fixture.display();
        let mut export = fixture.export(&display, false);
        assert_eq!(fixture.ready(&mut export), Err(expected));
        drop(export);
        drop(display);
        let observed = fixture.finish();
        assert_eq!(observed.exports.len(), observed.destroyed);
    }
}
