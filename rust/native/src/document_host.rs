//! Native-thread scheduling and worker fences for display documents. Mirrors
//! image_host's shutdown contract; background workers never await UI callbacks.
use crate::document_jobs::{self as jobs, Error, Ready, Request};
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
                if let Some(window) = completion.observer {
                    state.dirty.insert(window);
                }
                state.pool.complete(completion);
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
        let notifications = state.pool.take_notifications();
        state.dirty.extend(notifications);
        let dirty = std::mem::take(&mut state.dirty);
        dirty
            .into_iter()
            .filter_map(|id| state.windows.get(&id).copied())
            .collect()
    };
    for handle in windows {
        let _ = handle.update(cx, |_, window, _| window.refresh());
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
        return Err(Error::ResourceLimit);
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
        if service.borrow().closed {
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
fn begin_close(cx: &mut App) -> Option<(Shared, Vec<Done>)> {
    let service = cx.try_global::<Global>()?.0.clone();
    let mut state = service.borrow_mut();
    if state.closed {
        return None;
    }
    state.closed = true;
    state.pool.close();
    if let Some(completion) = state.delivering.take() {
        state.pool.complete(completion);
    }
    state.input.close();
    while let Ok((completion, _)) = state.input.try_recv() {
        state.pool.complete(completion);
    }
    let workers = std::mem::take(&mut state.workers);
    drop(state);
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
pub fn measurements(cx: &App) -> Option<(jobs::Measurements, usize, usize, usize, usize)> {
    let state = cx.try_global::<Global>()?.0.borrow();
    Some((
        state.pool.totals,
        state.pool.completed,
        state.pool.discarded,
        state.pool.peak_workers,
        state.pool.peak_reserved_bytes,
    ))
}
