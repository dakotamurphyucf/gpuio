//! Real isolated D-Bus transport against a deterministic notification-service
//! fixture. This does not claim a Linux desktop displayed a notification.
use futures_lite::future::{self, block_on};
use gpuio_portal::notifications::{Client, Events, Signal};
use gpuio_protocol::{
    desktop::Identity,
    notification::{Action, ClosedReason, Content, Error, Receipt, Sound},
};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use zbus::{Connection, connection::Builder, zvariant::OwnedValue};
const SERVICE: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";

#[derive(Debug)]
struct Submitted {
    replaces: u32,
    title: String,
    body: String,
    actions: Vec<String>,
    desktop: String,
    silent: bool,
}
struct Server {
    submitted: Arc<Mutex<Vec<Submitted>>>,
    closed: Arc<Mutex<Vec<u32>>>,
    bad_id: Arc<AtomicBool>,
    actions: bool,
    flood: Arc<AtomicBool>,
}
#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Server {
    fn get_capabilities(&self) -> Vec<&str> {
        if self.actions {
            vec!["body", "body-markup", "actions", "sound"]
        } else {
            vec!["body"]
        }
    }
    #[allow(clippy::too_many_arguments)] // Standard freedesktop Notify signature.
    async fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
        #[zbus(connection)] connection: &Connection,
    ) -> u32 {
        assert_eq!(app_name, "Example");
        assert_eq!(app_icon, "");
        assert_eq!(expire_timeout, -1);
        let desktop = <&str>::try_from(hints.get("desktop-entry").unwrap())
            .unwrap()
            .to_owned();
        let silent = bool::try_from(hints.get("suppress-sound").unwrap()).unwrap();
        assert!(!bool::try_from(hints.get("resident").unwrap()).unwrap());
        self.submitted.lock().unwrap().push(Submitted {
            replaces: replaces_id,
            title: summary.into(),
            body: body.into(),
            actions,
            desktop,
            silent,
        });
        if self.flood.load(Ordering::Acquire) {
            for _ in 0..256 {
                connection
                    .emit_signal(
                        None::<&str>,
                        PATH,
                        SERVICE,
                        "ActionInvoked",
                        &(999u32, "other-app-action"),
                    )
                    .await
                    .unwrap();
            }
        }
        if self.bad_id.load(Ordering::Acquire) {
            0
        } else if replaces_id != 0 {
            replaces_id
        } else {
            42
        }
    }
    fn close_notification(&self, id: u32) {
        self.closed.lock().unwrap().push(id);
    }
}
async fn next(events: &mut Events) -> Result<Signal, Error> {
    future::or(events.next_signal(), async {
        async_io::Timer::after(Duration::from_secs(3)).await;
        panic!("notification signal timed out")
    })
    .await
}
async fn emit(connection: &Connection, member: &str, id: u32, key: &str) {
    connection
        .emit_signal(None::<&str>, PATH, SERVICE, member, &(id, key))
        .await
        .unwrap();
}

#[test]
#[ignore = "requires an explicitly isolated dbus-run-session"]
fn real_bus_capabilities_replace_actions_owner_fencing_and_close() {
    assert_eq!(std::env::var("GPUIO_PRIVATE_BUS_TEST").as_deref(), Ok("1"));
    block_on(async {
        assert!(matches!(Client::connect().await, Err(Error::Unavailable)));
        let submitted = Arc::new(Mutex::new(Vec::new()));
        let closed = Arc::new(Mutex::new(Vec::new()));
        let bad_id = Arc::new(AtomicBool::new(false));
        let flood = Arc::new(AtomicBool::new(false));
        let server = Builder::session()
            .unwrap()
            .name(SERVICE)
            .unwrap()
            .serve_at(
                PATH,
                Server {
                    submitted: submitted.clone(),
                    closed: closed.clone(),
                    bad_id: bad_id.clone(),
                    actions: true,
                    flood: flood.clone(),
                },
            )
            .unwrap()
            .build()
            .await
            .unwrap();
        let (client, mut events) = Client::connect().await.unwrap();
        assert!(
            client.capabilities().actions
                && client.capabilities().body
                && client.capabilities().sound
        );
        let identity = Identity {
            identifier: "com.example".into(),
            name: "Example".into(),
            schemes: vec![],
        };
        let receipt = Receipt {
            id: 7,
            tag: "build".into(),
        };
        let content = Content {
            title: "Build 🦀".into(),
            body: "日本語 <b>&\"'".into(),
            actions: vec![Action {
                id: "open".into(),
                label: "Open build".into(),
            }],
            sound: Sound::Silent,
        };
        assert_eq!(
            client.show(&identity, &receipt, 3, 0, &content).await,
            Ok(42)
        );
        assert_eq!(
            client.show(&identity, &receipt, 4, 42, &content).await,
            Ok(42)
        );
        {
            let requests = submitted.lock().unwrap();
            assert_eq!(requests.len(), 2);
            assert_eq!((requests[0].replaces, requests[1].replaces), (0, 42));
            assert_eq!(requests[0].title, "Build 🦀");
            assert_eq!(requests[0].body, "日本語 &lt;b&gt;&amp;&quot;&apos;");
            assert_eq!(
                requests[0].actions,
                ["default", "Open", "gpuio/7/3/open", "Open build"]
            );
            assert_eq!(requests[1].actions[2], "gpuio/7/4/open");
            assert_eq!(requests[0].desktop, "com.example");
            assert!(requests[0].silent);
        }
        // More than the subscription's capacity arrives before Notify replies.
        // Concurrent draining must keep the connection live without forwarding
        // another application's unknown action keys to our application.
        flood.store(true, Ordering::Release);
        let result = future::or(client.show(&identity, &receipt, 5, 42, &content), async {
            let signal = events.next_signal().await;
            panic!("Unowned flood escaped filtering: {signal:?}")
        })
        .await;
        assert_eq!(result, Ok(42));
        flood.store(false, Ordering::Release);
        let stranger = Connection::session().await.unwrap();
        emit(&stranger, "ActionInvoked", 42, "default").await;
        emit(&server, "ActionInvoked", 42, "other-app-action").await;
        emit(&server, "ActionInvoked", 42, "gpuio/7/4/open").await;
        assert_eq!(
            next(&mut events).await,
            Ok(Signal::Action {
                native_id: 42,
                receipt: 7,
                revision: 4,
                id: "open".into()
            })
        );
        emit(&server, "ActionInvoked", 42, "default").await;
        assert_eq!(next(&mut events).await, Ok(Signal::Activated(42)));
        client.dismiss(42).await.unwrap();
        assert_eq!(*closed.lock().unwrap(), vec![42]);
        for (reason, expected) in [
            (1, ClosedReason::Expired),
            (2, ClosedReason::User),
            (3, ClosedReason::Platform),
            (4, ClosedReason::Platform),
        ] {
            server
                .emit_signal(
                    None::<&str>,
                    PATH,
                    SERVICE,
                    "NotificationClosed",
                    &(42u32, reason as u32),
                )
                .await
                .unwrap();
            assert_eq!(next(&mut events).await, Ok(Signal::Closed(42, expected)));
        }
        bad_id.store(true, Ordering::Release);
        assert_eq!(
            client.show(&identity, &receipt, 5, 0, &content).await,
            Err(Error::NativeFailure)
        );
        server.close().await.unwrap();
        assert_eq!(next(&mut events).await, Err(Error::Unavailable));
        // Reused IDs in a new daemon must never be addressed by the old client.
        let replacement = Builder::session()
            .unwrap()
            .name(SERVICE)
            .unwrap()
            .serve_at(
                PATH,
                Server {
                    submitted: submitted.clone(),
                    closed: closed.clone(),
                    bad_id: Arc::new(AtomicBool::new(false)),
                    actions: false,
                    flood: Arc::new(AtomicBool::new(false)),
                },
            )
            .unwrap()
            .build()
            .await
            .unwrap();
        assert_eq!(client.dismiss(42).await, Err(Error::Unavailable));
        assert_eq!(*closed.lock().unwrap(), vec![42]);
        client.close().await;
        let (reduced, _events) = Client::connect().await.unwrap();
        assert!(!reduced.capabilities().actions && !reduced.capabilities().sound);
        assert_eq!(
            reduced.show(&identity, &receipt, 6, 0, &content).await,
            Err(Error::Unsupported)
        );
        let plain = Content {
            actions: vec![],
            ..content.clone()
        };
        assert_eq!(
            reduced.show(&identity, &receipt, 7, 0, &plain).await,
            Ok(42)
        );
        assert_eq!(submitted.lock().unwrap().last().unwrap().body, plain.body);
        let noisy = Content {
            sound: Sound::Default,
            ..plain
        };
        assert_eq!(
            reduced.show(&identity, &receipt, 8, 0, &noisy).await,
            Err(Error::Unsupported)
        );
        reduced.close().await;
        replacement.close().await.unwrap();
        stranger.close().await.unwrap();
    });
}
