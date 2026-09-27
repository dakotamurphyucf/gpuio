//! Correlated chart publication. Background jobs never await native/UI work;
//! shutdown closes the store, retires replies and joins all admitted workers.
use crate::{
    chart_store,
    session::{ChartDispatch, Session},
    transport::Transport,
};
use gpui::{App, AsyncApp};
use gpuio_protocol::chart_resource::{Error, Request, Response};
use std::{
    cell::RefCell,
    collections::BTreeSet,
    rc::{Rc, Weak},
    sync::Arc,
};

type Done = async_channel::Receiver<()>;
type Output = (i64, chart_store::Completion, Done);
type Shared = Rc<RefCell<Service>>;
struct Global(Shared);
impl gpui::Global for Global {}
struct Service {
    session: Weak<RefCell<Session>>,
    transport: Arc<Transport>,
    output: async_channel::Sender<Output>,
    input: async_channel::Receiver<Output>,
    pending: BTreeSet<i64>,
    workers: Vec<Done>,
    delivering: Option<(i64, chart_store::Completion)>,
    closed: bool,
}

impl Drop for Service {
    fn drop(&mut self) {
        // Fallback for host unwinding before its normal quit hooks. Workers do
        // not await UI callbacks, so joining here cannot depend on a live App.
        self.input.close();
        self.delivering = None;
        while self.input.try_recv().is_ok() {}
        if let Some(session) = self.session.upgrade()
            && let Ok(mut session) = session.try_borrow_mut()
        {
            session.close_charts();
        }
        for done in &self.workers {
            let _ = done.recv_blocking();
        }
    }
}

pub(crate) fn init(session: &Rc<RefCell<Session>>, transport: Arc<Transport>, cx: &mut App) {
    assert!(
        !cx.has_global::<Global>(),
        "one chart publisher per application"
    );
    let (output, input) = async_channel::bounded(chart_store::MAX_WORKERS);
    let service = Rc::new(RefCell::new(Service {
        session: Rc::downgrade(session),
        transport,
        output,
        input: input.clone(),
        pending: BTreeSet::new(),
        workers: vec![],
        delivering: None,
        closed: false,
    }));
    cx.set_global(Global(service.clone()));
    cx.on_app_quit(|cx| {
        finish_before_quit(cx);
        std::future::ready(())
    })
    .detach();
    let weak = Rc::downgrade(&service);
    cx.spawn(async move |cx| {
        while let Ok((correlation, completion, done)) = input.recv().await {
            let Some(service) = weak.upgrade() else {
                break;
            };
            service.borrow_mut().delivering = Some((correlation, completion));
            drop(service);
            let _ = done.recv().await;
            let Some(service) = weak.upgrade() else {
                break;
            };
            let mut state = service.borrow_mut();
            let Some((correlation, completion)) = state.delivering.take() else {
                continue;
            };
            if !state.pending.remove(&correlation) {
                continue;
            }
            let response = match state.session.upgrade() {
                Some(session) if !state.closed => session.borrow_mut().complete_chart(completion),
                _ => Response::Failed(Error::Closed),
            };
            let published = response == Response::Ack;
            state.transport.respond_chart(correlation, response);
            state.workers.retain(|done| !done.is_closed());
            drop(state);
            if published {
                cx.update(|cx| {
                    for handle in cx.windows() {
                        let _ = handle.update(cx, |_, window, _| window.refresh());
                    }
                });
            }
        }
    })
    .detach();
}

pub(crate) fn dispatch(correlation: i64, request: Request, cx: &mut App) {
    let service = cx.global::<Global>().0.clone();
    let mut state = service.borrow_mut();
    if state.pending.contains(&correlation) {
        state
            .transport
            .respond_chart(correlation, Response::Failed(Error::Busy));
        return;
    }
    let Some(session) = state.session.upgrade().filter(|_| !state.closed) else {
        state
            .transport
            .respond_chart(correlation, Response::Failed(Error::Closed));
        return;
    };
    match session.borrow_mut().chart_request(request) {
        ChartDispatch::Immediate(response) => state.transport.respond_chart(correlation, response),
        ChartDispatch::Publish(work) => {
            // Store admission already bounds both pending results and workers.
            state.pending.insert(correlation);
            state.workers.retain(|done| !done.is_closed());
            let (finished, done) = async_channel::bounded::<()>(1);
            state.workers.push(done.clone());
            let output = state.output.clone();
            cx.background_executor()
                .spawn(async move {
                    let completion = work.run();
                    let _ = output.try_send((correlation, completion, done));
                    drop(finished);
                })
                .detach();
        }
    }
}

fn begin_close(cx: &mut App) -> Vec<Done> {
    let Some(global) = cx.try_global::<Global>() else {
        return vec![];
    };
    close_service(&global.0)
}
fn close_service(service: &Shared) -> Vec<Done> {
    let mut state = service.borrow_mut();
    if state.closed {
        // An OS quit can interrupt an asynchronous shutdown already waiting.
        // Every closer must still join the same live workers.
        return state.workers.clone();
    }
    state.closed = true;
    if let Some(session) = state.session.upgrade() {
        session.borrow_mut().close_charts();
    }
    for correlation in std::mem::take(&mut state.pending) {
        state
            .transport
            .respond_chart(correlation, Response::Failed(Error::Closed));
    }
    state.input.close();
    state.delivering = None;
    while state.input.try_recv().is_ok() {}
    state.workers.clone()
}

pub(crate) async fn shutdown(cx: &mut AsyncApp) {
    for done in cx.update(begin_close) {
        let _ = done.recv().await;
    }
}
pub(crate) fn finish_before_quit(cx: &mut App) {
    for done in begin_close(cx) {
        let _ = done.recv_blocking();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpuio_protocol::v1::Event;
    use std::os::fd::AsRawFd;

    #[test]
    fn service_drop_closes_worker_output_and_waits_without_ui_callbacks() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let wake = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/null")
            .unwrap();
        let transport = Arc::new(Transport::new(wake.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        let (output, input) = async_channel::bounded(chart_store::MAX_WORKERS);
        let (finished, done) = async_channel::bounded::<()>(1);
        let worker_output = output.clone();
        let exited = Arc::new(AtomicBool::new(false));
        let worker_exited = exited.clone();
        let worker = std::thread::spawn(move || {
            futures_lite::future::block_on(worker_output.closed());
            worker_exited.store(true, Ordering::Release);
            drop(finished);
        });
        let service = Service {
            session: Rc::downgrade(&session),
            transport,
            output,
            input,
            pending: BTreeSet::new(),
            workers: vec![done],
            delivering: None,
            closed: false,
        };
        drop(service);
        assert!(exited.load(Ordering::Acquire));
        worker.join().unwrap();
    }

    #[test]
    fn repeated_closers_join_the_same_workers_and_reply_only_once() {
        let wake = std::fs::OpenOptions::new()
            .write(true)
            .open("/dev/null")
            .unwrap();
        let transport = Arc::new(Transport::new(wake.as_raw_fd()).unwrap());
        let session = Rc::new(RefCell::new(Session::default()));
        let (output, input) = async_channel::bounded(chart_store::MAX_WORKERS);
        let (finished, done) = async_channel::bounded::<()>(1);
        transport
            .submit(gpuio_protocol::v1::Message::Chart(7, Request::Create), 3)
            .unwrap();
        transport.mailbox.lock().unwrap().pop();
        let service = Rc::new(RefCell::new(Service {
            session: Rc::downgrade(&session),
            transport: transport.clone(),
            output,
            input,
            pending: BTreeSet::from([7]),
            workers: vec![done],
            delivering: None,
            closed: false,
        }));
        let first = close_service(&service);
        let second = close_service(&service);
        assert_eq!((first.len(), second.len()), (1, 1));
        assert!(!first[0].is_closed() && !second[0].is_closed());
        assert_eq!(
            transport.mailbox.lock().unwrap().drain(64),
            vec![Event::ChartResponse(7, Response::Failed(Error::Closed))]
        );
        assert!(service.borrow().input.is_closed());
        drop(finished);
        assert!(first[0].recv_blocking().is_err());
        assert!(second[0].recv_blocking().is_err());
        assert!(transport.mailbox.lock().unwrap().drain(64).is_empty());
        transport.mailbox.lock().unwrap().close();
        transport.respond_chart(99, Response::Ack);
        assert_eq!(
            transport.mailbox.lock().unwrap().drain(64),
            vec![Event::Stopped]
        );
    }
}
