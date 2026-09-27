//! Actual private-bus transport through the native worker; the daemon is a
//! deterministic fixture, not evidence of Linux desktop presentation.
use super::*;
use gpuio_protocol::notification::{Action, Event, Sound};
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, AtomicU32, Ordering},
    time::Duration,
};
use zbus::{Connection, connection::Builder, zvariant::OwnedValue};
const SERVICE: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";
static BUS_TEST: std::sync::Mutex<()> = std::sync::Mutex::new(());
#[derive(Clone, Debug)]
struct Submitted {
    native_id: u32,
    actions: Vec<String>,
}
struct Server {
    sequence: AtomicU32,
    submitted: async_channel::Sender<Submitted>,
    closed: async_channel::Sender<u32>,
    early: Arc<AtomicBool>,
    hold_reply: Option<async_channel::Receiver<()>>,
}
#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Server {
    fn get_capabilities(&self) -> Vec<&str> {
        vec!["body", "actions"]
    }
    #[allow(clippy::too_many_arguments)]
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
        assert_eq!(app_name, "Test");
        assert_eq!(app_icon, "");
        assert_eq!(summary, "Ready 🦀");
        assert_eq!(body, "日本語");
        assert_eq!(expire_timeout, -1);
        assert_eq!(
            <&str>::try_from(hints.get("desktop-entry").unwrap()).unwrap(),
            "com.gpuio.test"
        );
        let id = if replaces_id == 0 {
            self.sequence.fetch_add(1, Ordering::Relaxed)
        } else {
            replaces_id
        };
        if self.early.load(Ordering::Acquire) {
            // Exceeds the socket's subscription queue. The worker must drain it
            // concurrently with Notify, then route the matching custom action.
            for _ in 0..256 {
                emit(connection, 999, "foreign-action").await;
            }
            emit(connection, id, &actions[2]).await;
        }
        self.submitted
            .send(Submitted {
                native_id: id,
                actions,
            })
            .await
            .unwrap();
        if let Some(hold) = &self.hold_reply {
            let _ = hold.recv().await;
        }
        id
    }
    async fn close_notification(&self, id: u32) {
        self.closed.send(id).await.unwrap();
    }
}
async fn emit(connection: &Connection, id: u32, action: &str) {
    connection
        .emit_signal(None::<&str>, PATH, SERVICE, "ActionInvoked", &(id, action))
        .await
        .unwrap();
}
async fn receive<T>(rx: &async_channel::Receiver<T>) -> T {
    future::or(async { rx.recv().await.expect("channel closed") }, async {
        async_io::Timer::after(Duration::from_secs(10)).await;
        panic!("native notification worker timed out");
    })
    .await
}
fn request(service: &Service, id: i64, request: Request) -> async_channel::Receiver<Response> {
    let (tx, rx) = async_channel::bounded(1);
    service.request(id, request, move |response| {
        let _ = tx.try_send(response);
    });
    rx
}
fn content(id: &str) -> Content {
    Content {
        title: "Ready 🦀".into(),
        body: "日本語".into(),
        actions: vec![Action {
            id: id.into(),
            label: "Open".into(),
        }],
        sound: Sound::Silent,
    }
}
#[test]
#[ignore = "requires an explicitly isolated dbus-run-session"]
fn private_bus_worker_routes_actions_replacement_and_owner_loss() {
    assert_eq!(std::env::var("GPUIO_PRIVATE_BUS_TEST").as_deref(), Ok("1"));
    let _guard = BUS_TEST.lock().unwrap();
    future::block_on(async {
        let early = Arc::new(AtomicBool::new(true));
        let (submitted, submissions) = async_channel::bounded(16);
        let (closed, dismissals) = async_channel::bounded(128);
        let server = Builder::session()
            .unwrap()
            .name(SERVICE)
            .unwrap()
            .serve_at(
                PATH,
                Server {
                    sequence: AtomicU32::new(42),
                    submitted,
                    closed,
                    early: early.clone(),
                    hold_reply: None,
                },
            )
            .unwrap()
            .build()
            .await
            .unwrap();
        let (wake, wakes) = async_channel::bounded(1);
        let mut service = Service::new(
            Identity {
                identifier: "com.gpuio.test".into(),
                name: "Test".into(),
                schemes: vec![],
            },
            move || {
                let _ = wake.try_send(());
            },
        )
        .unwrap();
        assert_eq!(
            receive(&request(&service, 1, Request::Authorization)).await,
            Response::Authorization(Authorization::NotRequired)
        );
        let Response::Posted(first) = receive(&request(
            &service,
            2,
            Request::Post("job".into(), content("open")),
        ))
        .await
        else {
            panic!("post failed")
        };
        assert_eq!(receive(&submissions).await.native_id, 42);
        receive(&wakes).await;
        assert_eq!(
            receive(&request(&service, 3, Request::TakeEvents)).await,
            Response::Events(vec![Event::Action(first, "open".into())])
        );
        assert_eq!(receive(&dismissals).await, 42);
        early.store(false, Ordering::Release);
        let Response::Posted(second) = receive(&request(
            &service,
            4,
            Request::Post("job".into(), content("open")),
        ))
        .await
        else {
            panic!("second post failed")
        };
        let original = receive(&submissions).await;
        assert_eq!(original.native_id, 43);
        assert_eq!(
            receive(&request(
                &service,
                5,
                Request::Post("job".into(), content("open"))
            ))
            .await,
            Response::Failed(Error::Busy)
        );
        assert_eq!(
            receive(&request(
                &service,
                6,
                Request::Replace(second.clone(), content("inspect"))
            ))
            .await,
            Response::Replaced
        );
        let replacement = receive(&submissions).await;
        assert_eq!(replacement.native_id, 43);
        assert_ne!(original.actions[2], replacement.actions[2]);
        emit(&server, 43, &original.actions[2]).await;
        emit(&server, 43, &replacement.actions[2]).await;
        receive(&wakes).await;
        assert_eq!(
            receive(&request(&service, 7, Request::TakeEvents)).await,
            Response::Events(vec![Event::Action(second, "inspect".into())])
        );
        assert_eq!(receive(&dismissals).await, 43);
        let Response::Posted(third) = receive(&request(
            &service,
            8,
            Request::Post("job".into(), content("open")),
        ))
        .await
        else {
            panic!("third post failed")
        };
        assert_eq!(receive(&submissions).await.native_id, 44);
        assert_eq!(
            receive(&request(&service, 9, Request::Dismiss(third))).await,
            Response::DismissRequested
        );
        assert_eq!(receive(&dismissals).await, 44);
        server.close().await.unwrap();
        receive(&wakes).await;
        assert_eq!(
            receive(&request(&service, 10, Request::TakeEvents)).await,
            Response::Events(vec![Event::Failed(Error::Unavailable)])
        );
        assert_eq!(
            receive(&request(
                &service,
                11,
                Request::Post("job".into(), content("open"))
            ))
            .await,
            Response::Failed(Error::Unavailable)
        );
        service.close().unwrap().wait().await;
        assert!(dismissals.try_recv().is_err());
    });
}

#[test]
#[ignore = "requires an explicitly isolated dbus-run-session"]
fn private_bus_close_waits_for_bounded_uncertain_submission_without_replay() {
    assert_eq!(std::env::var("GPUIO_PRIVATE_BUS_TEST").as_deref(), Ok("1"));
    let _guard = BUS_TEST.lock().unwrap();
    future::block_on(async {
        let (submitted, submissions) = async_channel::bounded(1);
        let (closed, dismissals) = async_channel::bounded(1);
        let (release, hold_reply) = async_channel::bounded(1);
        let server = Builder::session()
            .unwrap()
            .name(SERVICE)
            .unwrap()
            .serve_at(
                PATH,
                Server {
                    sequence: AtomicU32::new(42),
                    submitted,
                    closed,
                    early: Arc::new(AtomicBool::new(false)),
                    hold_reply: Some(hold_reply),
                },
            )
            .unwrap()
            .build()
            .await
            .unwrap();
        let mut service = Service::new(
            Identity {
                identifier: "com.gpuio.test".into(),
                name: "Test".into(),
                schemes: vec![],
            },
            || (),
        )
        .unwrap();
        let response = request(&service, 1, Request::Post("job".into(), content("open")));
        assert_eq!(receive(&submissions).await.native_id, 42);
        let cleanup = service.close().unwrap();
        assert_eq!(receive(&response).await, Response::Failed(Error::Closed));
        // The daemon accepted the submission but does not send a reply. Cleanup
        // must finish without guessing an ID, replaying Notify or hanging quit.
        future::or(cleanup.wait(), async {
            async_io::Timer::after(Duration::from_secs(9)).await;
            panic!("unacknowledged Notify blocked shutdown");
        })
        .await;
        assert!(dismissals.try_recv().is_err());
        assert!(submissions.try_recv().is_err());
        release.close();
        server.close().await.unwrap();
    });
}
