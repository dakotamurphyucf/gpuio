//! One bounded worker owns each fixed-daemon session. No GPUI/OCaml calls occur
//! here: completions and wake hints carry owned values to the application bridge.
mod routing;
use crate::notification_operations::{Operations, Ticket};
use crate::notification_state::Token;
use futures_lite::future;
use gpuio_portal::notifications::{Client, Events, Signal};
use gpuio_protocol::{
    desktop::Identity,
    notification::{Authorization, Capabilities, Content, Error, Receipt, Request, Response},
};
use routing::Routing;
use std::{future::Future, sync::Arc, thread::JoinHandle};

type Notify = Arc<dyn Fn() + Send + Sync>;
struct Command {
    request: Request,
    ticket: Ticket,
}
pub(crate) struct Service {
    commands: async_channel::Sender<Command>,
    cancel: async_channel::Sender<()>,
    operations: Arc<Operations>,
    cleanup: Option<Cleanup>,
}
#[must_use = "join notification worker before native application shutdown"]
pub(crate) struct Cleanup {
    done: async_channel::Receiver<()>,
    thread: Option<JoinHandle<()>>,
}
impl Cleanup {
    pub async fn wait(self) {
        let _ = self.done.recv().await;
        self.join();
    }
    pub fn wait_before_quit(self) {
        let _ = self.done.recv_blocking();
        self.join();
    }
    fn join(mut self) {
        if self
            .thread
            .take()
            .is_some_and(|thread| thread.join().is_err())
        {
            eprintln!("GPUIO_NOTIFICATION_CLEANUP_FAILED: worker panicked");
        }
    }
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        if self
            .thread
            .take()
            .is_some_and(|thread| thread.join().is_err())
        {
            eprintln!("GPUIO_NOTIFICATION_CLEANUP_FAILED: worker panicked");
        }
    }
}
struct Done(async_channel::Sender<()>, Arc<Operations>);
impl Drop for Done {
    fn drop(&mut self) {
        self.1.close();
        self.0.close();
    }
}
impl Service {
    pub fn new(
        identity: Identity,
        notify: impl Fn() + Send + Sync + 'static,
    ) -> Result<Self, Error> {
        Self::start(identity, Arc::new(notify), Client::connect)
    }
    fn start<
        B: Backend + Send + 'static,
        S: Source + Send + 'static,
        F: Future<Output = Result<(B, S), Error>>,
    >(
        identity: Identity,
        notify: Notify,
        connect: impl Fn() -> F + Send + 'static,
    ) -> Result<Self, Error> {
        if !identity.is_valid() {
            return Err(Error::InvalidRequest);
        }
        let (commands, receive) = async_channel::bounded(16);
        let (cancel, stop) = async_channel::bounded(1);
        let (done, receive_done) = async_channel::bounded(1);
        let operations = Arc::new(Operations::default());
        let worker_operations = operations.clone();
        let thread = std::thread::Builder::new()
            .name("gpuio-notifications".into())
            .spawn(move || {
                let _done = Done(done, worker_operations);
                future::block_on(worker(identity, notify, receive, stop, connect));
            })
            .map_err(|_| Error::NativeFailure)?;
        Ok(Self {
            commands,
            cancel,
            operations,
            cleanup: Some(Cleanup {
                done: receive_done,
                thread: Some(thread),
            }),
        })
    }
    pub fn request(
        &self,
        correlation: i64,
        request: Request,
        complete: impl FnOnce(Response) + Send + 'static,
    ) {
        // Keep the completion available when admission fails.
        let complete = Arc::new(std::sync::Mutex::new(Some(complete)));
        let admitted_complete = complete.clone();
        let ticket = self.operations.admit(correlation, move |response| {
            if let Some(complete) = admitted_complete
                .lock()
                .expect("notification callback poisoned")
                .take()
            {
                complete(response);
            }
        });
        match ticket {
            Ok(ticket) => {
                if !request.is_valid() {
                    ticket.finish(Response::Failed(Error::InvalidRequest));
                    return;
                }
                if let Err(error) = self.commands.try_send(Command { request, ticket }) {
                    let closed = error.is_closed();
                    error
                        .into_inner()
                        .ticket
                        .finish(Response::Failed(if closed {
                            Error::Closed
                        } else {
                            Error::Busy
                        }));
                }
            }
            Err(error) => {
                if let Some(complete) = complete
                    .lock()
                    .expect("notification callback poisoned")
                    .take()
                {
                    complete(Response::Failed(error));
                }
            }
        }
    }
    pub fn close(&mut self) -> Option<Cleanup> {
        self.operations.close();
        self.cancel.close();
        self.commands.close();
        self.cleanup.take()
    }
}
impl Drop for Service {
    fn drop(&mut self) {
        // Explicit host cleanup is asynchronous. This is a last-resort ownership
        // guard for initialization failure/unwinding, never a detached thread.
        if let Some(cleanup) = self.close() {
            cleanup.wait_before_quit();
        }
    }
}
trait Backend {
    fn capabilities(&self) -> Capabilities;
    fn show(
        &self,
        identity: &Identity,
        receipt: &Receipt,
        revision: i64,
        replaces: u32,
        content: &Content,
    ) -> impl Future<Output = Result<u32, Error>>;
    fn dismiss(&self, id: u32) -> impl Future<Output = Result<(), Error>>;
    fn close(self) -> impl Future<Output = ()>;
}
trait Source {
    fn next(&mut self) -> impl Future<Output = Result<Signal, Error>>;
}
impl Backend for Client {
    fn capabilities(&self) -> Capabilities {
        self.capabilities()
    }
    async fn show(
        &self,
        identity: &Identity,
        receipt: &Receipt,
        revision: i64,
        replaces: u32,
        content: &Content,
    ) -> Result<u32, Error> {
        self.show(identity, receipt, revision, replaces, content)
            .await
    }
    async fn dismiss(&self, id: u32) -> Result<(), Error> {
        self.dismiss(id).await
    }
    async fn close(self) {
        self.close().await;
    }
}
impl Source for Events {
    async fn next(&mut self) -> Result<Signal, Error> {
        self.next_signal().await
    }
}
struct Session {
    routing: Routing,
    failure: Option<Error>,
    events_failed: bool,
    uncertain_submission: bool,
    stopping: bool,
    notify: Notify,
}
impl Session {
    fn fail(&mut self, error: Error) {
        if self.failure.is_none() {
            self.failure = Some(error);
            if self.routing.state.fail(error) {
                (self.notify)();
            }
        }
    }
    fn signal(&mut self, signal: Result<Signal, Error>, pending: Option<&Token>) {
        match signal {
            Ok(signal) => match self.routing.signal(signal, pending) {
                Ok(true) => (self.notify)(),
                Ok(false) => (),
                Err(error) => self.fail(error),
            },
            Err(error) => {
                self.events_failed = true;
                self.fail(error);
            }
        }
    }
    // Retain a submitted method across cancellation to learn its exact native ID.
    // Portal methods have a five-second deadline; close retires logical state now.
    async fn method<T>(
        &mut self,
        events: &mut impl Source,
        stop: &async_channel::Receiver<()>,
        pending: Option<&Token>,
        method: impl Future<Output = T>,
    ) -> T {
        let mut method = std::pin::pin!(method);
        loop {
            enum Next<T> {
                Reply(T),
                Signal(Result<Signal, Error>),
                Stop,
            }
            let next = future::or(
                async { Next::Reply(method.as_mut().await) },
                future::or(
                    async {
                        if self.events_failed {
                            future::pending().await
                        } else {
                            Next::Signal(events.next().await)
                        }
                    },
                    async {
                        if self.stopping {
                            future::pending().await
                        } else {
                            let _ = stop.recv().await;
                            Next::Stop
                        }
                    },
                ),
            )
            .await;
            match next {
                Next::Reply(reply) => return reply,
                Next::Signal(signal) => self.signal(signal, pending),
                Next::Stop => {
                    self.stopping = true;
                    self.routing.state.close();
                }
            }
        }
    }
    async fn cleanup(
        &mut self,
        client: &impl Backend,
        events: &mut impl Source,
        stop: &async_channel::Receiver<()>,
        ids: Vec<u32>,
    ) {
        if ids.is_empty() {
            return;
        }
        // At most 128 parallel bounded calls: shutdown is one deadline, not 128
        // sequential deadlines. Continue draining signals to avoid bus blockage.
        let methods = ids
            .iter()
            .map(|id| Box::pin(client.dismiss(*id)))
            .collect::<Vec<_>>();
        let mut methods = methods.into_iter().map(Some).collect::<Vec<_>>();
        let mut results = vec![None; methods.len()];
        let replies = self
            .method(
                events,
                stop,
                None,
                future::poll_fn(move |cx| {
                    let mut ready = true;
                    for (index, method) in methods.iter_mut().enumerate() {
                        if let Some(future) = method {
                            match future.as_mut().poll(cx) {
                                std::task::Poll::Ready(result) => {
                                    results[index] = Some(result);
                                    *method = None;
                                }
                                std::task::Poll::Pending => ready = false,
                            }
                        }
                    }
                    if ready {
                        std::task::Poll::Ready(std::mem::take(&mut results))
                    } else {
                        std::task::Poll::Pending
                    }
                }),
            )
            .await;
        for (id, result) in ids.into_iter().zip(replies) {
            if matches!(result, Some(Ok(()))) {
                self.routing.released(id);
            }
        }
    }
    async fn request(
        &mut self,
        client: &impl Backend,
        events: &mut impl Source,
        stop: &async_channel::Receiver<()>,
        identity: &Identity,
        request: Request,
    ) -> Response {
        if matches!(request, Request::TakeEvents) {
            return match self.routing.state.take() {
                Ok(events) => Response::Events(events),
                Err(error) => Response::Failed(error),
            };
        }
        if let Some(error) = self.failure {
            return Response::Failed(error);
        }
        match request {
            Request::Capabilities => Response::Capabilities(client.capabilities()),
            Request::Authorization | Request::RequestAuthorization => {
                Response::Authorization(Authorization::NotRequired)
            }
            Request::Post(tag, content) => {
                if !self.routing.can_post() {
                    return Response::Failed(Error::Busy);
                }
                match self.routing.state.post(tag, content.clone()) {
                    Ok(token) => {
                        self.show(client, events, stop, identity, token, content, 0)
                            .await
                    }
                    Err(error) => Response::Failed(error),
                }
            }
            Request::Replace(receipt, content) => {
                let id = match self.routing.native_id(&receipt) {
                    Ok(id) => id,
                    Err(error) => return Response::Failed(error),
                };
                match self.routing.state.replace(&receipt, content.clone()) {
                    Ok(token) => {
                        self.show(client, events, stop, identity, token, content, id)
                            .await
                    }
                    Err(error) => Response::Failed(error),
                }
            }
            Request::Dismiss(receipt) => {
                let id = match self.routing.native_id(&receipt) {
                    Ok(id) => id,
                    Err(error) => return Response::Failed(error),
                };
                let token = match self.routing.state.dismiss(&receipt) {
                    Ok(token) => token,
                    Err(error) => return Response::Failed(error),
                };
                let result = self.method(events, stop, None, client.dismiss(id)).await;
                let completion = self.routing.state.complete(&token, result);
                if result.is_ok() {
                    self.routing.released(id);
                }
                match completion.result {
                    Ok(()) => Response::DismissRequested,
                    Err(error) => Response::Failed(error),
                }
            }
            Request::TakeEvents => unreachable!("event drain handled before session failure"),
            Request::Close => Response::Failed(Error::InvalidRequest),
        }
    }
    #[allow(clippy::too_many_arguments)]
    async fn show(
        &mut self,
        client: &impl Backend,
        events: &mut impl Source,
        stop: &async_channel::Receiver<()>,
        identity: &Identity,
        token: Token,
        content: Content,
        replaces: u32,
    ) -> Response {
        self.routing.begin_show(replaces);
        let result = self
            .method(
                events,
                stop,
                (replaces == 0).then_some(&token),
                client.show(identity, &token.receipt, token.serial, replaces, &content),
            )
            .await;
        let result = match result {
            Ok(id) => {
                let result = self.routing.bind(id, &token);
                if let Err(error) = result {
                    self.fail(error);
                }
                result
            }
            Err(error) => {
                // A missing/invalid reply can follow actual submission. Fault the
                // fixed-owner session; never retry an uncertain Show automatically.
                if matches!(error, Error::NativeFailure | Error::Unavailable) {
                    self.uncertain_submission = true;
                    self.fail(error);
                }
                Err(error)
            }
        };
        let completion = self.routing.state.complete(&token, result);
        if completion.notify {
            (self.notify)();
        }
        match completion.result {
            Ok(()) if replaces == 0 => Response::Posted(token.receipt),
            Ok(()) => Response::Replaced,
            Err(error) => Response::Failed(self.failure.unwrap_or(error)),
        }
    }
}
async fn worker<B: Backend, S: Source, F: Future<Output = Result<(B, S), Error>>>(
    identity: Identity,
    notify: Notify,
    commands: async_channel::Receiver<Command>,
    stop: async_channel::Receiver<()>,
    connect: impl Fn() -> F,
) {
    // Setup failures have not minted receipts and can be explicitly retried.
    let (client, mut events, first) = loop {
        let command = future::or(async { commands.recv().await.ok() }, async {
            let _ = stop.recv().await;
            None
        })
        .await;
        let Some(command) = command else {
            return;
        };
        let connected = future::or(connect(), async {
            let _ = stop.recv().await;
            Err(Error::Closed)
        })
        .await;
        match connected {
            Ok((client, events)) => break (client, events, command),
            Err(error) => {
                command.ticket.finish(Response::Failed(error));
                if error == Error::Closed {
                    return;
                }
            }
        }
    };
    let mut session = Session {
        routing: Routing::default(),
        failure: None,
        events_failed: false,
        uncertain_submission: false,
        stopping: false,
        notify,
    };
    let mut command = Some(first);
    loop {
        if let Some(command) = command.take() {
            let response = session
                .request(&client, &mut events, &stop, &identity, command.request)
                .await;
            command.ticket.finish(response);
        }
        if session.stopping || stop.is_closed() {
            break;
        }
        let ids = session.routing.cleanup_ids();
        // Only attempt automatic terminal cleanup once. A failure faults the
        // session, retains its bounded leases, and retries on final shutdown.
        if session.failure.is_none() && !ids.is_empty() {
            session
                .cleanup(&client, &mut events, &stop, ids.clone())
                .await;
            if ids.iter().any(|id| session.routing.all_ids().contains(id)) {
                session.fail(Error::NativeFailure);
            }
        }
        if session.stopping || stop.is_closed() {
            break;
        }
        enum Next {
            Command(Option<Command>),
            Signal(Result<Signal, Error>),
            Stop,
        }
        match future::or(
            async {
                let _ = stop.recv().await;
                Next::Stop
            },
            future::or(async { Next::Command(commands.recv().await.ok()) }, async {
                if session.events_failed {
                    future::pending().await
                } else {
                    Next::Signal(events.next().await)
                }
            }),
        )
        .await
        {
            Next::Command(Some(next)) => command = Some(next),
            Next::Command(None) | Next::Stop => break,
            Next::Signal(signal) => session.signal(signal, None),
        }
    }
    session.stopping = true;
    session.routing.state.close();
    let ids = session.routing.all_ids();
    session.cleanup(&client, &mut events, &stop, ids).await;
    if !session.routing.all_ids().is_empty() {
        eprintln!("GPUIO_NOTIFICATION_CLEANUP_FAILED: daemon removal was not confirmed");
    }
    if session.uncertain_submission {
        eprintln!(
            "GPUIO_NOTIFICATION_CLEANUP_UNCERTAIN: a submitted notification ID was not acknowledged"
        );
    }
    client.close().await;
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod bus_tests;
