use super::*;
use std::{future::Future, os::unix::net::UnixStream, sync::Mutex};
use zbus::connection::Builder;

fn run(test: impl Future<Output = ()>) {
    future::block_on(future::or(test, async {
        async_io::Timer::after(Duration::from_secs(10)).await;
        panic!("instance test timed out");
    }));
}

async fn peer() -> (MessageStream, Connection, MessageStream, Connection) {
    let (a, b) = UnixStream::pair().unwrap();
    let guid = zbus::Guid::generate();
    let build = |socket, name| {
        Builder::authenticated_socket(async_io::Async::new(socket).unwrap(), guid.clone())
            .unwrap()
            .p2p()
            .unique_name(name)
            .unwrap()
            .method_timeout(Duration::from_millis(300))
            .build_message_stream()
    };
    let (a, b) = future::zip(build(a, ":1.10"), build(b, ":1.20")).await;
    let (a, b) = (a.unwrap(), b.unwrap());
    let (ac, bc) = (Connection::from(&a), Connection::from(&b));
    (a, ac, b, bc)
}

#[test]
fn primary_admits_startup_before_serving_and_empty_secondary_means_reopen() {
    run(async {
        let (mut bus, bus_connection, app, _) = peer().await;
        let admitted = Arc::new(Mutex::new(Vec::new()));
        let capture = admitted.clone();
        let admit: Admit = Arc::new(move |links| {
            capture.lock().unwrap().push(links);
            Ok(())
        });
        let serve_bus = async {
            let message = bus.next().await.unwrap().unwrap();
            assert_eq!(message.header().member().unwrap().as_str(), "RequestName");
            assert_eq!(
                message.body().deserialize::<(&str, u32)>().unwrap(),
                ("com.example.app", 4)
            );
            bus_connection
                .reply(&message.header(), &1u32)
                .await
                .unwrap();
        };
        let (launch, ()) = future::zip(
            claim(app, "com.example.app", vec!["startup".into()], admit),
            serve_bus,
        )
        .await;
        let Launch::Primary(server) = launch.unwrap() else {
            panic!("expected primary")
        };
        assert_eq!(*admitted.lock().unwrap(), vec![vec!["startup".to_owned()]]);
        let (cancel, receive) = async_channel::bounded(1);
        let calls = async {
            forward(&bus_connection, ":1.20", &["later".into(), "later".into()])
                .await
                .unwrap();
            forward(&bus_connection, ":1.20", &[]).await.unwrap();
            cancel.close();
        };
        let (result, ()) = future::zip(server.serve(receive), calls).await;
        assert_eq!(result, Ok(()));
        assert_eq!(
            *admitted.lock().unwrap(),
            vec![
                vec!["startup".to_owned()],
                vec!["later".to_owned(); 2],
                vec![]
            ]
        );
    });
}

#[test]
fn secondary_pins_owner_and_propagates_backpressure_without_retry() {
    run(async {
        for busy in [false, true] {
            let (mut bus, connection, app, _) = peer().await;
            let serve = async {
                let request = bus.next().await.unwrap().unwrap();
                connection.reply(&request.header(), &3u32).await.unwrap();
                let owner = bus.next().await.unwrap().unwrap();
                assert_eq!(owner.header().member().unwrap().as_str(), "GetNameOwner");
                connection.reply(&owner.header(), &":1.99").await.unwrap();
                let open = bus.next().await.unwrap().unwrap();
                assert_eq!(open.header().destination().unwrap().as_str(), ":1.99");
                assert_eq!(decode_links(&open).unwrap(), ["opaque malformed input"]);
                if busy {
                    connection
                        .reply_error(&open.header(), "org.gpuio.Application1.Error.Busy", &"full")
                        .await
                        .unwrap();
                } else {
                    connection.reply(&open.header(), &()).await.unwrap();
                }
            };
            let (result, ()) = future::zip(
                claim(
                    app,
                    "com.example.app",
                    vec!["opaque malformed input".into()],
                    Arc::new(|_| panic!("secondary must not initialize")),
                ),
                serve,
            )
            .await;
            if busy {
                assert!(matches!(result, Err(Error::Busy)));
            } else {
                assert!(matches!(result, Ok(Launch::Forwarded)));
            }
        }
    });
}

#[test]
fn server_refuses_invalid_batches_and_preserves_typed_admission_errors() {
    run(async {
        let (stream, connection, _, client) = peer().await;
        let seen = Arc::new(Mutex::new(0));
        let capture = seen.clone();
        let server = Server {
            connection,
            stream,
            identifier: "com.example.app".into(),
            admit: Arc::new(move |_| {
                *capture.lock().unwrap() += 1;
                Err(Error::Closed)
            }),
        };
        let (cancel, receive) = async_channel::bounded(1);
        let calls = async {
            for links in [
                vec!["x".into(); MAX_LINKS + 1],
                vec!["x".repeat(MAX_LINK_BYTES + 1)],
                vec!["x".repeat(MAX_LINK_BYTES); 17],
            ] {
                assert_eq!(
                    forward(&client, ":1.10", &links).await,
                    Err(Error::InvalidRequest)
                );
            }
            assert_eq!(
                forward(&client, ":1.10", &["valid".into()]).await,
                Err(Error::Closed)
            );
            let wrong = client
                .call_method(Some(":1.10"), PATH, Some(INTERFACE), "OpenLinks", &42u32)
                .await;
            assert_eq!(
                wrong.map_err(map_error).map(|_| ()),
                Err(Error::InvalidRequest)
            );
            cancel.close();
        };
        assert_eq!(future::zip(server.serve(receive), calls).await.0, Ok(()));
        assert_eq!(*seen.lock().unwrap(), 1);
    });
}

#[test]
fn bus_loss_is_observable_and_empty_initial_launch_is_not_reopen() {
    run(async {
        let (mut stream, connection, app, _) = peer().await;
        let bus = async {
            let claim = stream.next().await.unwrap().unwrap();
            connection.reply(&claim.header(), &1u32).await.unwrap();
        };
        let (result, ()) = future::zip(
            claim(
                app,
                "com.example.app",
                vec![],
                Arc::new(|_| panic!("empty startup is not reopen")),
            ),
            bus,
        )
        .await;
        let Launch::Primary(server) = result.unwrap() else {
            panic!("expected primary")
        };
        let (_cancel, receive) = async_channel::bounded(1);
        connection.close().await.unwrap();
        assert_eq!(server.serve(receive).await, Err(Error::Unavailable));
    });
}

#[test]
fn local_invalid_input_does_not_attempt_session_bus() {
    run(async {
        for (id, links) in [
            ("bad", vec![]),
            ("com.example.app", vec!["bad\0input".into()]),
        ] {
            assert!(matches!(
                launch(id, links, Arc::new(|_| panic!("not admitted"))).await,
                Err(Error::InvalidRequest)
            ));
        }
    });
}
