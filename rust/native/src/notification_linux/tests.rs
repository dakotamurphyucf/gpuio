use super::*;
use gpuio_protocol::notification::{Event, Sound};
use std::sync::Mutex;
#[derive(Clone)]
struct Fake {
    signals: async_channel::Sender<Result<Signal, Error>>,
    dismissals: Arc<Mutex<Vec<u32>>>,
    closed: Arc<std::sync::atomic::AtomicBool>,
    gate: Option<async_channel::Receiver<()>>,
    entered: async_channel::Sender<()>,
    show_failure: Option<Error>,
    dismiss_failure: Option<Error>,
    early_action: bool,
    shows: Arc<std::sync::atomic::AtomicUsize>,
}
struct FakeEvents(async_channel::Receiver<Result<Signal, Error>>);
impl Source for FakeEvents {
    async fn next(&mut self) -> Result<Signal, Error> {
        self.0.recv().await.unwrap_or(Err(Error::Unavailable))
    }
}
impl Backend for Fake {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            body: true,
            actions: true,
            activation: true,
            replacement: true,
            dismissal: true,
            permission_request: false,
            sound: false,
        }
    }
    async fn show(
        &self,
        _: &Identity,
        _: &Receipt,
        _: i64,
        _: u32,
        _: &Content,
    ) -> Result<u32, Error> {
        self.shows
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let _ = self.entered.try_send(());
        if self.early_action {
            self.signals.send(Ok(Signal::Activated(42))).await.unwrap();
            // Capacity one forces the worker to drain before method reply.
            self.signals.send(Ok(Signal::Activated(42))).await.unwrap();
        }
        if let Some(gate) = &self.gate {
            future::or(
                async {
                    let _ = gate.recv().await;
                },
                async {
                    async_io::Timer::after(std::time::Duration::from_secs(10)).await;
                },
            )
            .await;
        }
        self.show_failure.map_or(Ok(42), Err)
    }
    async fn dismiss(&self, id: u32) -> Result<(), Error> {
        self.dismissals.lock().unwrap().push(id);
        self.dismiss_failure.map_or(Ok(()), Err)
    }
    async fn close(self) {
        self.closed
            .store(true, std::sync::atomic::Ordering::Release);
    }
}
fn identity() -> Identity {
    Identity {
        identifier: "com.gpuio.test".into(),
        name: "Test".into(),
        schemes: vec![],
    }
}
fn content() -> Content {
    Content {
        title: "Ready".into(),
        body: String::new(),
        actions: vec![],
        sound: Sound::Silent,
    }
}
fn fake(
    gate: Option<async_channel::Receiver<()>>,
) -> (Fake, FakeEvents, async_channel::Receiver<()>) {
    let (signals, events) = async_channel::bounded(1);
    let (entered, started) = async_channel::bounded(1);
    (
        Fake {
            signals,
            dismissals: Arc::default(),
            closed: Arc::default(),
            gate,
            entered,
            show_failure: None,
            dismiss_failure: None,
            early_action: true,
            shows: Arc::default(),
        },
        FakeEvents(events),
        started,
    )
}
fn request(service: &Service, id: i64, request: Request) -> async_channel::Receiver<Response> {
    let (send, receive) = async_channel::bounded(1);
    service.request(id, request, move |response| {
        send.try_send(response).unwrap();
    });
    receive
}
#[test]
fn worker_delivers_early_action_and_joins_with_exact_artifact_cleanup() {
    let (fake, events, _) = fake(None);
    let backend = fake.clone();
    let source = Arc::new(Mutex::new(Some(events)));
    let mut service = Service::start(identity(), Arc::new(|| ()), move || {
        let backend = backend.clone();
        let source = source.lock().unwrap().take().unwrap();
        async { Ok((backend, source)) }
    })
    .unwrap();
    let Response::Posted(receipt) = request(&service, 1, Request::Post("job".into(), content()))
        .recv_blocking()
        .unwrap()
    else {
        panic!("post failed")
    };
    assert_eq!(
        request(&service, 2, Request::TakeEvents)
            .recv_blocking()
            .unwrap(),
        Response::Events(vec![Event::Activated(receipt)])
    );
    service.close().unwrap().wait_before_quit();
    assert_eq!(*fake.dismissals.lock().unwrap(), vec![42]);
    assert!(fake.closed.load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(
        request(&service, 3, Request::Capabilities)
            .recv_blocking()
            .unwrap(),
        Response::Failed(Error::Closed)
    );
}
#[test]
fn close_during_notify_retires_callbacks_but_recovers_and_removes_native_id() {
    let (release, gate) = async_channel::bounded(1);
    let (fake, events, started) = fake(Some(gate));
    let backend = fake.clone();
    let source = Arc::new(Mutex::new(Some(events)));
    let mut service = Service::start(identity(), Arc::new(|| ()), move || {
        let backend = backend.clone();
        let source = source.lock().unwrap().take().unwrap();
        async { Ok((backend, source)) }
    })
    .unwrap();
    let response = request(&service, 1, Request::Post("job".into(), content()));
    started.recv_blocking().unwrap();
    let cleanup = service.close().unwrap();
    assert_eq!(
        response.recv_blocking().unwrap(),
        Response::Failed(Error::Closed)
    );
    release.close();
    future::block_on(cleanup.wait());
    assert_eq!(*fake.dismissals.lock().unwrap(), vec![42]);
    assert!(fake.closed.load(std::sync::atomic::Ordering::Acquire));
    assert!(response.try_recv().is_err());
}

#[test]
fn invalid_identity_never_starts_a_worker_or_connects() {
    let mut invalid = identity();
    invalid.identifier.clear();
    assert!(matches!(
        Service::new(invalid, || ()),
        Err(Error::InvalidRequest)
    ));
}

fn service(fake: &Fake, events: FakeEvents, notify: Notify) -> Service {
    let backend = fake.clone();
    let source = Arc::new(Mutex::new(Some(events)));
    Service::start(identity(), notify, move || {
        let backend = backend.clone();
        // A second connect would violate the fixed-owner session policy.
        let source = source.lock().unwrap().take().unwrap();
        async { Ok((backend, source)) }
    })
    .unwrap()
}
#[test]
fn bounded_admission_and_close_cancel_all_queued_work_without_starting_it() {
    let (release, gate) = async_channel::bounded(1);
    let (mut fake, events, started) = fake(Some(gate));
    fake.early_action = false;
    let mut service = service(&fake, events, Arc::new(|| ()));
    let mut responses = vec![request(&service, 1, Request::Post("job".into(), content()))];
    started.recv_blocking().unwrap();
    for id in 2..=16 {
        responses.push(request(&service, id, Request::Capabilities));
    }
    assert_eq!(
        request(&service, 17, Request::Capabilities)
            .recv_blocking()
            .unwrap(),
        Response::Failed(Error::Busy)
    );
    let cleanup = service.close().unwrap();
    for response in responses {
        assert_eq!(
            response.recv_blocking().unwrap(),
            Response::Failed(Error::Closed)
        );
    }
    release.close();
    cleanup.wait_before_quit();
    assert_eq!(fake.shows.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert_eq!(*fake.dismissals.lock().unwrap(), vec![42]);
}
#[test]
fn daemon_loss_is_delivered_once_and_does_not_reconnect_or_replay() {
    let (fake, events, _) = fake(None);
    let (wake, wakes) = async_channel::bounded(1);
    let mut service = service(
        &fake,
        events,
        Arc::new(move || {
            let _ = wake.try_send(());
        }),
    );
    assert!(matches!(
        request(&service, 1, Request::Capabilities)
            .recv_blocking()
            .unwrap(),
        Response::Capabilities(_)
    ));
    fake.signals.send_blocking(Err(Error::Unavailable)).unwrap();
    wakes.recv_blocking().unwrap();
    assert_eq!(
        request(&service, 2, Request::TakeEvents)
            .recv_blocking()
            .unwrap(),
        Response::Events(vec![Event::Failed(Error::Unavailable)])
    );
    assert_eq!(
        request(&service, 3, Request::Post("job".into(), content()))
            .recv_blocking()
            .unwrap(),
        Response::Failed(Error::Unavailable)
    );
    assert_eq!(
        request(&service, 4, Request::TakeEvents)
            .recv_blocking()
            .unwrap(),
        Response::Events(vec![])
    );
    service.close().unwrap().wait_before_quit();
    assert_eq!(fake.shows.load(std::sync::atomic::Ordering::Relaxed), 0);
}
#[test]
fn uncertain_show_is_not_replayed_and_cannot_claim_native_cleanup() {
    let (mut fake, events, _) = fake(None);
    fake.early_action = false;
    fake.show_failure = Some(Error::NativeFailure);
    let mut service = service(&fake, events, Arc::new(|| ()));
    for id in 1..=2 {
        assert_eq!(
            request(&service, id, Request::Post("job".into(), content()))
                .recv_blocking()
                .unwrap(),
            Response::Failed(Error::NativeFailure)
        );
    }
    assert_eq!(
        request(&service, 3, Request::TakeEvents)
            .recv_blocking()
            .unwrap(),
        Response::Events(vec![Event::Failed(Error::NativeFailure)])
    );
    service.close().unwrap().wait_before_quit();
    assert_eq!(fake.shows.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert!(fake.dismissals.lock().unwrap().is_empty());
}
#[test]
fn failed_dismissal_retires_actions_but_retains_the_lease_for_shutdown_retry() {
    let (mut fake, events, _) = fake(None);
    fake.early_action = false;
    fake.dismiss_failure = Some(Error::NativeFailure);
    let mut service = service(&fake, events, Arc::new(|| ()));
    let Response::Posted(receipt) = request(&service, 1, Request::Post("job".into(), content()))
        .recv_blocking()
        .unwrap()
    else {
        panic!("post failed")
    };
    assert_eq!(
        request(&service, 2, Request::Dismiss(receipt.clone()))
            .recv_blocking()
            .unwrap(),
        Response::Failed(Error::NativeFailure)
    );
    assert_eq!(
        request(&service, 3, Request::Replace(receipt, content()))
            .recv_blocking()
            .unwrap(),
        Response::Failed(Error::Stale)
    );
    assert_eq!(
        request(&service, 4, Request::Post("job".into(), content()))
            .recv_blocking()
            .unwrap(),
        Response::Failed(Error::Busy)
    );
    service.close().unwrap().wait_before_quit();
    assert_eq!(*fake.dismissals.lock().unwrap(), vec![42, 42]);
}
