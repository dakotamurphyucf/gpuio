//! Native-thread scheduling and worker fences for retained chart plans. Mirrors
//! image_host's shutdown contract; background workers never await UI callbacks.
use crate::chart_jobs::{self as jobs, Error, Ready, Request};
use gpui::{AnyWindowHandle, App, AsyncApp, Window};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::{Rc, Weak},
};

type Done = async_channel::Receiver<()>;
type Output = (jobs::Completion, Done);
type Shared = Rc<RefCell<Service>>;
struct Global(Shared);
impl gpui::Global for Global {}
struct Service {
    pool: jobs::Pool,
    dirty: HashSet<gpui::WindowId>,
    windows: HashMap<gpui::WindowId, AnyWindowHandle>,
    output: async_channel::Sender<Output>,
    input: async_channel::Receiver<Output>,
    workers: Vec<Done>,
    delivering: Option<jobs::Completion>,
    scheduled: bool,
    closed: bool,
}
impl Drop for Service {
    fn drop(&mut self) {
        self.pool.close();
        self.input.close();
        self.delivering = None;
        while self.input.try_recv().is_ok() {}
        for done in &self.workers {
            let _ = done.recv_blocking();
        }
    }
}

pub struct Handle {
    window: gpui::WindowId,
    service: Weak<RefCell<Service>>,
    job: jobs::Handle,
}

pub fn init(cx: &mut App) {
    if cx.has_global::<Global>() {
        return;
    }
    let (output, input) = async_channel::bounded(2);
    let service = Rc::new(RefCell::new(Service {
        pool: jobs::Pool::default(),
        dirty: HashSet::new(),
        windows: HashMap::new(),
        output,
        input: input.clone(),
        workers: Vec::new(),
        delivering: None,
        scheduled: false,
        closed: false,
    }));
    cx.set_global(Global(service.clone()));
    let weak = Rc::downgrade(&service);
    cx.on_window_closed(move |_, id| {
        if let Some(service) = weak.upgrade() {
            service.borrow_mut().windows.remove(&id);
        }
    })
    .detach();
    cx.on_app_quit(|cx| {
        finish_before_quit(cx);
        std::future::ready(())
    })
    .detach();
    let weak = Rc::downgrade(&service);
    cx.spawn(async move |cx| {
        while let Ok((completion, done)) = input.recv().await {
            let Some(service) = weak.upgrade() else {
                break;
            };
            service.borrow_mut().delivering = Some(completion);
            drop(service);
            let _ = done.recv().await;
            let Some(service) = weak.upgrade() else {
                break;
            };
            let completion = service.borrow_mut().delivering.take();
            if let Some(completion) = completion {
                let mut state = service.borrow_mut();
                if let Some(window) = state.pool.complete(completion) {
                    state.dirty.insert(window);
                }
            }
            cx.update(|cx| pump(&service, cx));
        }
    })
    .detach();
}
fn schedule(service: &Shared, cx: &mut App) {
    let mut state = service.borrow_mut();
    if state.closed || state.scheduled {
        return;
    }
    state.scheduled = true;
    let weak = Rc::downgrade(service);
    cx.defer(move |cx| {
        if let Some(service) = weak.upgrade() {
            service.borrow_mut().scheduled = false;
            pump(&service, cx);
        }
    });
}
fn pump(service: &Shared, cx: &mut App) {
    if service.borrow().closed {
        return;
    }
    service
        .borrow_mut()
        .workers
        .retain(|done| !done.is_closed());
    for _ in 0..jobs::MAX_WORKERS {
        let Some(work) = service.borrow_mut().pool.next_work() else {
            break;
        };
        let (finished, done) = async_channel::bounded::<()>(1);
        let output = {
            let mut state = service.borrow_mut();
            state.workers.push(done.clone());
            state.output.clone()
        };
        cx.background_executor()
            .spawn(async move {
                let completion = work.run();
                let _ = output.try_send((completion, done));
                drop(finished);
            })
            .detach();
    }
    let windows: Vec<_> = {
        let mut state = service.borrow_mut();
        let dirty = std::mem::take(&mut state.dirty);
        dirty
            .into_iter()
            .filter_map(|id| state.windows.get(&id).copied())
            .collect()
    };
    for handle in windows {
        crate::host::refresh_chart_window(handle, cx);
    }
}
pub fn request(mut request: Request, window: &mut Window, cx: &mut App) -> Result<Handle, Error> {
    init(cx);
    let service = cx.global::<Global>().0.clone();
    let mut state = service.borrow_mut();
    let window = window.window_handle();
    if state.closed {
        return Err(Error::Closed);
    }
    if state.windows.len() >= 32 && !state.windows.contains_key(&window.window_id()) {
        return Err(Error::LimitExceeded);
    }
    request.observer = Some(window.window_id());
    let job = state.pool.request(request)?;
    state.dirty.insert(window.window_id());
    state.windows.insert(window.window_id(), window);
    drop(state);
    schedule(&service, cx);
    Ok(Handle {
        window: window.window_id(),
        service: Rc::downgrade(&service),
        job,
    })
}
impl Handle {
    pub fn update(&self, mut request: Request, cx: &mut App) -> Result<(), Error> {
        let service = self.service.upgrade().ok_or(Error::Closed)?;
        if !cx
            .try_global::<Global>()
            .is_some_and(|global| Rc::ptr_eq(&global.0, &service))
        {
            return Err(Error::Closed);
        }
        if service.borrow().closed || !service.borrow().windows.contains_key(&self.window) {
            return Err(Error::Closed);
        }
        request.observer = Some(self.window);
        if self.job.update(request)? {
            service.borrow_mut().dirty.insert(self.window);
            schedule(&service, cx);
        }
        Ok(())
    }
    pub fn take_ready(&self) -> Option<Result<Ready, Error>> {
        self.job.take_ready()
    }
}
fn close_service(service: &Shared) -> Vec<Done> {
    let mut state = service.borrow_mut();
    if !state.closed {
        state.closed = true;
        state.pool.close();
        if let Some(completion) = state.delivering.take() {
            state.pool.complete(completion);
        }
        state.input.close();
        while let Ok((completion, _)) = state.input.try_recv() {
            state.pool.complete(completion);
        }
    }
    state.workers.clone()
}
fn begin_close(cx: &mut App) -> Option<(Shared, Vec<Done>)> {
    let service = cx.try_global::<Global>()?.0.clone();
    let workers = close_service(&service);
    Some((service, workers))
}

pub async fn shutdown(cx: &mut AsyncApp) {
    let Some((_service, workers)) = cx.update(begin_close) else {
        return;
    };
    for done in workers {
        let _ = done.recv().await;
    }
}
pub fn finish_before_quit(cx: &mut App) {
    let Some((_service, workers)) = begin_close(cx) else {
        return;
    };
    for done in workers {
        let _ = done.recv_blocking();
    }
}

#[cfg(feature = "native-tests")]
pub fn measurements(cx: &App) -> Option<(usize, usize, usize, usize, usize)> {
    let state = cx.try_global::<Global>()?.0.borrow();
    Some((
        state.pool.completed,
        state.pool.discarded,
        state.pool.peak_workers,
        state.pool.reserved_bytes(),
        state.pool.workspace_bytes(),
    ))
}

#[cfg(feature = "native-canvas-tests")]
pub(crate) fn hold_remaining_budget_for_test(cx: &App) -> Box<dyn std::any::Any> {
    cx.global::<Global>()
        .0
        .borrow()
        .pool
        .hold_remaining_budget_for_test()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn service(done: Done) -> Shared {
        let (output, input) = async_channel::bounded(jobs::MAX_WORKERS);
        Rc::new(RefCell::new(Service {
            pool: jobs::Pool::default(),
            dirty: HashSet::new(),
            windows: HashMap::new(),
            output,
            input,
            workers: vec![done],
            delivering: None,
            scheduled: false,
            closed: false,
        }))
    }
    #[test]
    fn repeated_shutdown_keeps_worker_fences_for_every_closer() {
        let (finished, done) = async_channel::bounded::<()>(1);
        let service = service(done);
        let first = close_service(&service);
        let second = close_service(&service);
        assert_eq!((first.len(), second.len()), (1, 1));
        assert!(!first[0].is_closed());
        assert!(service.borrow().input.is_closed());
        drop(finished);
        assert!(first[0].recv_blocking().is_err());
        assert!(second[0].recv_blocking().is_err());
    }
    #[test]
    fn drop_closes_output_then_joins_worker_without_ui_dependency() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        let (finished, done) = async_channel::bounded::<()>(1);
        let service = service(done);
        let output = service.borrow().output.clone();
        let exited = Arc::new(AtomicBool::new(false));
        let worker_exited = exited.clone();
        let worker = std::thread::spawn(move || {
            futures_lite::future::block_on(output.closed());
            worker_exited.store(true, Ordering::Release);
            drop(finished);
        });
        drop(service);
        assert!(exited.load(Ordering::Acquire));
        worker.join().unwrap();
    }
}
