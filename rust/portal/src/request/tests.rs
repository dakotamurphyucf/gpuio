use super::*;
use gpuio_protocol::file_path::FilePath;
use std::{future::Future, os::unix::net::UnixStream};
use zbus::{Message, connection::Builder, zvariant::OwnedValue};

const OWNER: &str = ":1.10";
const PREDICTED: &str = "/org/freedesktop/portal/desktop/request/1_20/gpuio_test";
const ALTERNATE: &str = "/org/freedesktop/portal/desktop/request/1_20/legacy";

fn run(test: impl Future<Output = ()>) {
    future::block_on(future::or(test, async {
        async_io::Timer::after(Duration::from_secs(10)).await;
        panic!("portal test timed out");
    }));
}
async fn peer() -> (MessageStream, Connection, Connection) {
    let (a, b) = UnixStream::pair().unwrap();
    // A private socket pair is already authenticated by this test process.
    // Supplying names on this path models the bus-assigned unique names without
    // depending on a user/session bus or testing zbus's SASL implementation.
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
    let (stream, client) = future::zip(server, client).await;
    let stream = stream.unwrap();
    let server = Connection::from(&stream);
    (stream, server, client.unwrap())
}
fn config() -> FileDialogConfig {
    FileDialogConfig::Open(OpenFileConfig {
        selection: FileSelection::Files,
        multiple: true,
        title: "Open native files".into(),
        accept_label: "Choose".into(),
        directory: Some(FilePath::new(b"/tmp/\xff".to_vec()).unwrap()),
    })
}

#[test]
fn capability_queries_use_the_discovered_version_without_presenting_a_picker() {
    run(async {
        for version in [0, 1, 2, 3, 4] {
            let (mut stream, _server, client) = peer().await;
            let (_keep, cancel) = async_channel::bounded(1);
            let result = choose_on(
                client,
                OWNER,
                version,
                FileDialogConfig::Capabilities,
                "x11:2a",
                "probe",
                cancel,
            )
            .await;
            if version == 0 {
                assert_eq!(result, failed(FileDialogError::Unsupported));
            } else {
                assert_eq!(
                    result,
                    FileDialogResult::Capabilities(FileDialogCapabilities {
                        files: FileSelectionSupport::Multiple,
                        directories: if version >= 3 {
                            FileSelectionSupport::Multiple
                        } else {
                            FileSelectionSupport::Unsupported
                        },
                        files_and_directories: FileSelectionSupport::Unsupported,
                        save: true,
                    })
                );
            }
            assert!(
                stream.next().await.is_none_or(|message| message.is_err()),
                "capability probe must not send OpenFile, SaveFile or a request subscription"
            );
        }
        let (mut stream, _server, client) = peer().await;
        let (cancel, receiver) = async_channel::bounded(1);
        cancel.send(()).await.unwrap();
        assert_eq!(
            choose_peer(client, FileDialogConfig::Capabilities, receiver).await,
            failed(FileDialogError::Closed)
        );
        assert!(stream.next().await.is_none_or(|message| message.is_err()));
    });
}
async fn method(stream: &mut MessageStream, member: &str, path: &str) -> Message {
    let message = stream.next().await.unwrap().unwrap();
    assert_eq!(message.message_type(), Type::MethodCall);
    assert_eq!(message.header().member().unwrap().as_str(), member);
    assert_eq!(message.header().path().unwrap().as_str(), path);
    message
}
async fn reply_handle(server: &Connection, call: &Message, path: &str) {
    server
        .reply(&call.header(), &OwnedObjectPath::try_from(path).unwrap())
        .await
        .unwrap();
}
async fn response(server: &Connection, path: &str, code: u32, uris: &[&str]) {
    server
        .emit_signal(
            None::<&str>,
            path,
            REQUEST,
            "Response",
            &(code, HashMap::from([("uris", Value::from(uris.to_vec()))])),
        )
        .await
        .unwrap();
}
fn choose_peer(
    client: Connection,
    config: FileDialogConfig,
    cancel: Receiver<()>,
) -> impl Future<Output = FileDialogResult> {
    choose_on(client, OWNER, 4, config, "x11:2a", "gpuio_test", cancel)
}

#[test]
fn response_before_method_reply_preserves_exact_request_options() {
    run(async {
        let (mut stream, server, client) = peer().await;
        let (_keep, cancel) = async_channel::bounded(1);
        let serve = async {
            let call = method(&mut stream, "OpenFile", DESKTOP).await;
            let (parent, title, mut options): (String, String, HashMap<String, OwnedValue>) =
                call.body().deserialize().unwrap();
            assert_eq!(parent, "x11:2a");
            assert_eq!(title, "Open native files");
            assert_eq!(
                String::try_from(options.remove("handle_token").unwrap()).unwrap(),
                "gpuio_test"
            );
            assert_eq!(
                String::try_from(options.remove("accept_label").unwrap()).unwrap(),
                "Choose"
            );
            assert!(bool::try_from(options.remove("multiple").unwrap()).unwrap());
            assert!(bool::try_from(options.remove("modal").unwrap()).unwrap());
            assert!(!bool::try_from(options.remove("directory").unwrap()).unwrap());
            assert_eq!(
                Vec::<u8>::try_from(options.remove("current_folder").unwrap()).unwrap(),
                b"/tmp/\xff\0"
            );
            assert!(options.is_empty());
            // An unrelated response in the same namespace must not win. The
            // actual legacy handle responds before OpenFile returns its path.
            response(&server, PREDICTED, 0, &["file:///wrong"]).await;
            response(
                &server,
                ALTERNATE,
                0,
                &["file:///tmp/%FF", "file:///tmp/two"],
            )
            .await;
            reply_handle(&server, &call, ALTERNATE).await;
        };
        let (result, ()) = future::zip(choose_peer(client, config(), cancel), serve).await;
        assert_eq!(
            result,
            FileDialogResult::Selected(vec![
                FilePath::new(b"/tmp/\xff".to_vec()).unwrap(),
                FilePath::new(b"/tmp/two".to_vec()).unwrap()
            ])
        );
        assert!(
            stream.next().await.is_none_or(|event| event.is_err()),
            "selected requests close their dedicated connection"
        );
    });
}

#[test]
fn cancellation_before_handle_creation_retries_the_returned_handle() {
    run(async {
        let (mut stream, server, client) = peer().await;
        let (cancel, receiver) = async_channel::bounded(1);
        let serve = async {
            let open = method(&mut stream, "OpenFile", DESKTOP).await;
            cancel.send(()).await.unwrap();
            let early_close = method(&mut stream, "Close", PREDICTED).await;
            server
                .reply_error(
                    &early_close.header(),
                    "org.freedesktop.DBus.Error.UnknownObject",
                    &"not created yet",
                )
                .await
                .unwrap();
            reply_handle(&server, &open, ALTERNATE).await;
            let actual_close = method(&mut stream, "Close", ALTERNATE).await;
            server.reply(&actual_close.header(), &()).await.unwrap();
        };
        let (result, ()) = future::zip(choose_peer(client, config(), receiver), serve).await;
        assert_eq!(result, failed(FileDialogError::Closed));
    });
}

#[test]
fn dropping_owner_closes_a_live_request_without_waiting_for_response() {
    run(async {
        let (mut stream, server, client) = peer().await;
        let (cancel, receiver) = async_channel::bounded(1);
        let serve = async {
            let open = method(&mut stream, "OpenFile", DESKTOP).await;
            reply_handle(&server, &open, PREDICTED).await;
            drop(cancel);
            let close = method(&mut stream, "Close", PREDICTED).await;
            server.reply(&close.header(), &()).await.unwrap();
            // Request.Close deliberately emits no Response signal.
        };
        let (result, ()) = future::zip(choose_peer(client, config(), receiver), serve).await;
        assert_eq!(result, failed(FileDialogError::Closed));
    });
}

#[test]
fn pre_cancelled_request_never_calls_open_file() {
    run(async {
        let (mut stream, _server, client) = peer().await;
        let (sender, receiver) = async_channel::bounded(1);
        drop(sender);
        assert_eq!(
            choose_peer(client, config(), receiver).await,
            failed(FileDialogError::Closed)
        );
        assert!(stream.next().await.is_none_or(|event| event.is_err()));
    });
}

#[test]
fn foreign_handle_is_rejected_without_closing_another_request() {
    run(async {
        let (mut stream, server, client) = peer().await;
        let (_keep, receiver) = async_channel::bounded(1);
        let serve = async {
            let open = method(&mut stream, "OpenFile", DESKTOP).await;
            reply_handle(
                &server,
                &open,
                "/org/freedesktop/portal/desktop/request/1_99/other",
            )
            .await;
            let close = method(&mut stream, "Close", PREDICTED).await;
            server.reply(&close.header(), &()).await.unwrap();
        };
        let (result, ()) = future::zip(choose_peer(client, config(), receiver), serve).await;
        assert_eq!(result, failed(FileDialogError::NativeFailure));
    });
}

#[test]
fn timeout_cleans_up_a_handle_even_without_an_open_reply() {
    run(async {
        let (mut stream, server, client) = peer().await;
        let (_keep, receiver) = async_channel::bounded(1);
        let serve = async {
            let _open = method(&mut stream, "OpenFile", DESKTOP).await;
            let close = method(&mut stream, "Close", PREDICTED).await;
            server.reply(&close.header(), &()).await.unwrap();
        };
        let (result, ()) = future::zip(choose_peer(client, config(), receiver), serve).await;
        assert_eq!(result, failed(FileDialogError::NativeFailure));
    });
}

#[test]
fn service_disconnect_does_not_leave_the_request_waiting() {
    run(async {
        let (mut stream, server, client) = peer().await;
        let (_keep, receiver) = async_channel::bounded(1);
        let serve = async {
            let open = method(&mut stream, "OpenFile", DESKTOP).await;
            reply_handle(&server, &open, PREDICTED).await;
            server.close().await.unwrap();
        };
        let (result, ()) = future::zip(choose_peer(client, config(), receiver), serve).await;
        assert_eq!(result, failed(FileDialogError::NativeFailure));
    });
}

#[test]
fn save_is_exact_and_unsupported_modes_are_explicit() {
    let save = FileDialogConfig::Save(SaveFileConfig {
        directory: FilePath::new(b"/tmp".to_vec()).unwrap(),
        suggested_name: "report.sql.s".into(),
        title: "Save report".into(),
        accept_label: "Save".into(),
    });
    let (method, title, values) = options(&save, "test", 1).unwrap();
    assert_eq!(method, "SaveFile");
    assert_eq!(title, "Save report");
    assert!(
        matches!(values.get("current_name"), Some(Value::Str(name)) if name.as_str() == "report.sql.s")
    );
    let FileDialogConfig::Open(mut open) = config() else {
        unreachable!()
    };
    open.selection = FileSelection::Directories;
    assert!(matches!(
        options(&FileDialogConfig::Open(open.clone()), "test", 2),
        Err(FileDialogError::Unsupported)
    ));
    assert!(options(&FileDialogConfig::Open(open.clone()), "test", 3).is_ok());
    open.selection = FileSelection::FilesAndDirectories;
    assert!(matches!(
        options(&FileDialogConfig::Open(open), "test", 4),
        Err(FileDialogError::Unsupported)
    ));
    for parent in ["", "x11:0", "x11:no", "wayland:", "wayland:a\0b"] {
        assert!(!valid_request(&save, parent, "valid"));
    }
    assert!(!valid_request(&save, "x11:2a", "bad/token"));
}

#[test]
fn owner_watch_filters_the_exact_departing_service() {
    run(async {
        let (_stream, server, client) = peer().await;
        let mut watch = watch_owner(&client, OWNER).await.unwrap();
        for (name, old, new) in [
            (":1.99", ":1.99", ""),
            (OWNER, "", OWNER),
            (OWNER, OWNER, ""),
        ] {
            let signal = Message::signal(
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
                "NameOwnerChanged",
            )
            .unwrap()
            .sender("org.freedesktop.DBus")
            .unwrap()
            .build(&(name, old, new))
            .unwrap();
            server.send(&signal).await.unwrap();
        }
        let event = watch.next().await.unwrap().unwrap();
        let values: (String, String, String) = event.body().deserialize().unwrap();
        assert_eq!(values, (OWNER.into(), OWNER.into(), String::new()));
    });
}

#[test]
fn discovery_reads_capabilities_from_the_pinned_owner_after_activation() {
    run(async {
        let (mut stream, server, client) = peer().await;
        let serve = async {
            let activate = method(&mut stream, "Get", DESKTOP).await;
            assert_eq!(
                activate.header().destination().unwrap().as_str(),
                DESTINATION
            );
            let property: (String, String) = activate.body().deserialize().unwrap();
            assert_eq!(property, (CHOOSER.into(), "version".into()));
            server
                .reply(&activate.header(), &Value::from(4u32))
                .await
                .unwrap();
            let owner = method(&mut stream, "GetNameOwner", "/org/freedesktop/DBus").await;
            assert_eq!(owner.body().deserialize::<String>().unwrap(), DESTINATION);
            server.reply(&owner.header(), &OWNER).await.unwrap();
            let version = method(&mut stream, "Get", DESKTOP).await;
            assert_eq!(version.header().destination().unwrap().as_str(), OWNER);
            server
                .reply(&version.header(), &Value::from(2u32))
                .await
                .unwrap();
        };
        let (result, ()) = future::zip(discover(&client), serve).await;
        assert_eq!(result.unwrap(), (OWNER.into(), 2));
    });
}

#[test]
fn missing_portal_is_reported_as_unsupported() {
    run(async {
        let (mut stream, server, client) = peer().await;
        let serve = async {
            let property = method(&mut stream, "Get", DESKTOP).await;
            server
                .reply_error(
                    &property.header(),
                    "org.freedesktop.DBus.Error.ServiceUnknown",
                    &"portal unavailable",
                )
                .await
                .unwrap();
        };
        let (result, ()) = future::zip(discover(&client), serve).await;
        assert_eq!(result, Err(FileDialogError::Unsupported));
    });
}

#[test]
fn a_failed_close_is_not_reported_as_acknowledged_cancellation() {
    run(async {
        let (mut stream, server, client) = peer().await;
        let (cancel, receiver) = async_channel::bounded(1);
        let serve = async {
            let open = method(&mut stream, "OpenFile", DESKTOP).await;
            // Make cancellation precede the method reply so both cleanup
            // attempts are exercised; both receive an explicit failure.
            cancel.send(()).await.unwrap();
            let close = method(&mut stream, "Close", PREDICTED).await;
            server
                .reply_error(
                    &close.header(),
                    "org.freedesktop.portal.Error.Failed",
                    &"cannot close",
                )
                .await
                .unwrap();
            reply_handle(&server, &open, PREDICTED).await;
            let retry = method(&mut stream, "Close", PREDICTED).await;
            server
                .reply_error(
                    &retry.header(),
                    "org.freedesktop.portal.Error.Failed",
                    &"cannot close",
                )
                .await
                .unwrap();
        };
        let (result, ()) = future::zip(choose_peer(client, config(), receiver), serve).await;
        assert_eq!(result, failed(FileDialogError::NativeFailure));
        assert!(stream.next().await.is_none_or(|event| event.is_err()));
    });
}
