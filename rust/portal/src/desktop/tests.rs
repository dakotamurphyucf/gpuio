use super::*;
use futures_lite::StreamExt;
use std::{
    future::Future,
    io::{Read, Seek, Write},
    os::unix::{ffi::OsStringExt, net::UnixStream},
    sync::atomic::{AtomicU64, Ordering},
};
use zbus::{
    MessageStream,
    connection::Builder,
    zvariant::{OwnedFd, OwnedValue},
};

const OWNER: &str = ":1.10";
const HANDLE: &str = "/org/freedesktop/portal/desktop/request/1_20/desktop_test";

fn run(test: impl Future<Output = ()>) {
    future::block_on(future::or(test, async {
        async_io::Timer::after(Duration::from_secs(10)).await;
        panic!("desktop portal test timed out");
    }));
}

async fn peer() -> (MessageStream, Connection, Connection) {
    let (a, b) = UnixStream::pair().unwrap();
    let guid = zbus::Guid::generate();
    let server = Builder::authenticated_socket(async_io::Async::new(a).unwrap(), guid.clone())
        .unwrap()
        .p2p()
        .unique_name(OWNER)
        .unwrap()
        .build_message_stream();
    let client = Builder::authenticated_socket(async_io::Async::new(b).unwrap(), guid)
        .unwrap()
        .p2p()
        .unique_name(":1.20")
        .unwrap()
        .method_timeout(Duration::from_millis(300))
        .build();
    let (server, client) = future::zip(server, client).await;
    let stream = server.unwrap();
    let server = Connection::from(&stream);
    (stream, server, client.unwrap())
}

struct Fixture {
    path: std::path::PathBuf,
    file: File,
}
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let name = format!(
            "gpuio-desktop-{}-{}-",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let mut name = name.into_bytes();
        #[cfg(target_os = "linux")]
        name.push(255);
        #[cfg(not(target_os = "linux"))]
        name.extend_from_slice("résumé".as_bytes());
        let path = std::env::temp_dir().join(std::ffi::OsString::from_vec(name));
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        file.write_all(b"native file identity").unwrap();
        file.rewind().unwrap();
        Self { path, file }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_file(&self.path).unwrap();
    }
}

#[test]
fn descriptors_options_and_portal_responses_are_preserved() {
    run(async {
        for (operation, code, expected) in [
            (FileOperation::Open, 0u32, Ok(())),
            (FileOperation::Reveal, 0, Ok(())),
            (FileOperation::Open, 1, Err(Error::Denied)),
            (FileOperation::Open, 2, Err(Error::NativeFailure)),
        ] {
            let fixture = Fixture::new();
            let (mut stream, server, client) = peer().await;
            let (_keep, cancel) = async_channel::bounded(1);
            let serve = async {
                let call = stream.next().await.unwrap().unwrap();
                assert_eq!(call.header().interface().unwrap().as_str(), INTERFACE);
                assert_eq!(call.header().member().unwrap().as_str(), operation.method());
                let (parent, fd, options): (String, OwnedFd, HashMap<String, OwnedValue>) =
                    call.body().deserialize().unwrap();
                assert_eq!(parent, "");
                assert_eq!(
                    <&str>::try_from(options.get("handle_token").unwrap()).unwrap(),
                    "desktop_test"
                );
                if operation == FileOperation::Open {
                    assert!(bool::try_from(options.get("writable").unwrap()).unwrap());
                    assert!(!bool::try_from(options.get("ask").unwrap()).unwrap());
                } else {
                    assert!(!options.contains_key("ask") && !options.contains_key("writable"));
                }
                let mut received = File::from(std::os::fd::OwnedFd::from(fd));
                let mut contents = String::new();
                received.read_to_string(&mut contents).unwrap();
                assert_eq!(contents, "native file identity");
                // Subscribe-before-call preserves a completion before the method reply.
                server
                    .emit_signal(
                        None::<&str>,
                        HANDLE,
                        "org.freedesktop.portal.Request",
                        "Response",
                        &(code, HashMap::<&str, Value<'_>>::new()),
                    )
                    .await
                    .unwrap();
                server
                    .reply(&call.header(), &OwnedObjectPath::try_from(HANDLE).unwrap())
                    .await
                    .unwrap();
            };
            let (result, ()) = future::zip(
                file_on(
                    &client,
                    OWNER,
                    3,
                    &fixture.file,
                    operation,
                    "desktop_test",
                    &cancel,
                ),
                serve,
            )
            .await;
            assert_eq!(result, expected);
        }
    });
}

#[test]
fn versions_gate_open_and_reveal_before_sending_requests() {
    run(async {
        let fixture = Fixture::new();
        for (operation, version) in [(FileOperation::Open, 1), (FileOperation::Reveal, 2)] {
            let (mut stream, _server, client) = peer().await;
            let (_keep, cancel) = async_channel::bounded(1);
            assert_eq!(
                file_on(
                    &client,
                    OWNER,
                    version,
                    &fixture.file,
                    operation,
                    "desktop_test",
                    &cancel
                )
                .await,
                Err(Error::Unsupported)
            );
            assert!(future::poll_once(stream.next()).await.is_none());
        }
    });
}

#[test]
fn cancellation_closes_the_actual_portal_request() {
    run(async {
        let fixture = Fixture::new();
        let (mut stream, server, client) = peer().await;
        let (cancel, receiver) = async_channel::bounded(1);
        let serve = async {
            let call = stream.next().await.unwrap().unwrap();
            server
                .reply(&call.header(), &OwnedObjectPath::try_from(HANDLE).unwrap())
                .await
                .unwrap();
            cancel.close();
            let close = stream.next().await.unwrap().unwrap();
            assert_eq!(close.header().member().unwrap().as_str(), "Close");
            assert_eq!(close.header().path().unwrap().as_str(), HANDLE);
            server.reply(&close.header(), &()).await.unwrap();
        };
        let (result, ()) = future::zip(
            file_on(
                &client,
                OWNER,
                3,
                &fixture.file,
                FileOperation::Open,
                "desktop_test",
                &receiver,
            ),
            serve,
        )
        .await;
        assert_eq!(result, Err(Error::Closed));
    });
}
