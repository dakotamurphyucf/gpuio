//! GPUI executor/wakeup ownership for subtree matching. Only Work/Completion
//! cross threads. Window handles, scope handles and application callbacks stay
//! on the native thread; there are no synchronous OCaml calls or idle timers.
use crate::{highlight_jobs as jobs, highlight_projection::Projection};
use gpui::{AnyWindowHandle, App, AsyncApp, Window};
use gpuio_protocol::highlight::Config;
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet},
    rc::{Rc, Weak},
    sync::Arc,
};

type Done = async_channel::Receiver<()>;
type Output = (jobs::Completion, Done);
type Shared = Rc<RefCell<Service>>;
struct Global(Shared);
impl gpui::Global for Global {}
struct Route {
    window: gpui::WindowId,
    job: jobs::Handle,
}
struct Service {
    pool: jobs::Pool,
    routes: BTreeMap<jobs::ScopeId, Route>,
    windows: HashMap<gpui::WindowId, AnyWindowHandle>,
    dirty: HashSet<gpui::WindowId>,
    output: async_channel::Sender<Output>,
    input: async_channel::Receiver<Output>,
    workers: Vec<Done>,
    delivering: Option<jobs::Completion>,
    scheduled: bool,
    closed: bool,
}
pub struct Handle {
    service: Weak<RefCell<Service>>,
    job: jobs::Handle,
}
impl Service {
    fn close_window(&mut self, window: gpui::WindowId) {
        self.windows.remove(&window);
        self.dirty.remove(&window);
        self.routes.retain(|_, route| {
            if route.window == window {
                route.job.close();
                false
            } else {
                true
            }
        });
    }
    fn accept(&mut self, completion: jobs::Completion) {
        let scope = completion.scope_id();
        if self.pool.complete(completion)
            && let Some(route) = self.routes.get(&scope)
        {
            self.dirty.insert(route.window);
        }
    }
}
pub fn init(cx: &mut App) {
    if cx.has_global::<Global>() {
        return;
    }
    let (output, input) = async_channel::bounded(jobs::MAX_WORKERS);
    let service = Rc::new(RefCell::new(Service {
        pool: jobs::Pool::default(),
        routes: BTreeMap::new(),
        windows: HashMap::new(),
        dirty: HashSet::new(),
        output,
        input: input.clone(),
        workers: Vec::new(),
        delivering: None,
        scheduled: false,
        closed: false,
    }));
    cx.set_global(Global(service.clone()));
    let weak = Rc::downgrade(&service);
    cx.on_window_closed(move |_, window| {
        if let Some(service) = weak.upgrade() {
            service.borrow_mut().close_window(window);
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
            // Keep the completion reachable by synchronous quit cleanup while
            // waiting for the worker's destruction fence.
            service.borrow_mut().delivering = Some(completion);
            drop(service);
            let _ = done.recv().await;
            let Some(service) = weak.upgrade() else {
                break;
            };
            let completion = service.borrow_mut().delivering.take();
            if let Some(completion) = completion {
                service.borrow_mut().accept(completion);
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
                // At most two dispatched jobs can own an undelivered output, and
                // pool worker slots remain occupied until their completion is read.
                // A worker never awaits the UI, including during synchronous quit.
                let _ = output.try_send((completion, done));
                drop(finished);
            })
            .detach();
    }
    let windows: Vec<_> = {
        let mut state = service.borrow_mut();
        std::mem::take(&mut state.dirty)
            .into_iter()
            .filter_map(|id| state.windows.get(&id).copied())
            .collect()
    };
    for handle in windows {
        let _ = handle.update(cx, |_, window, _| window.refresh());
    }
}
pub fn request(
    source: Arc<Projection>,
    config: Arc<Config>,
    window: &mut Window,
    cx: &mut App,
) -> Result<Handle, jobs::Error> {
    init(cx);
    let service = cx.global::<Global>().0.clone();
    let mut state = service.borrow_mut();
    if state.closed {
        return Err(jobs::Error::Closed);
    }
    let window = window.window_handle();
    if state.windows.len() >= 32 && !state.windows.contains_key(&window.window_id()) {
        return Err(jobs::Error::AdmissionLimit);
    }
    let job = state.pool.request(source, config)?;
    state.routes.insert(
        job.scope_id(),
        Route {
            window: window.window_id(),
            job: job.clone(),
        },
    );
    state.windows.insert(window.window_id(), window);
    state.dirty.insert(window.window_id());
    drop(state);
    schedule(&service, cx);
    Ok(Handle {
        service: Rc::downgrade(&service),
        job,
    })
}
impl Handle {
    pub fn epoch(&self) -> i64 {
        self.job.epoch()
    }
    pub fn status(&self) -> jobs::Status {
        self.job.status()
    }
    pub fn config(&self) -> Option<Arc<Config>> {
        self.job.config()
    }
    pub fn update(
        &self,
        source: Arc<Projection>,
        config: Arc<Config>,
        cx: &mut App,
    ) -> Result<jobs::Update, jobs::Error> {
        let service = self.service.upgrade().ok_or(jobs::Error::Closed)?;
        if !cx
            .try_global::<Global>()
            .is_some_and(|g| Rc::ptr_eq(&g.0, &service))
            || service.borrow().closed
        {
            return Err(jobs::Error::Closed);
        }
        let updated = self.job.update(source, config);
        // Even a rejected update cancels the old job. Pump once to allow other
        // queued scopes to advance and deliver any released worker slots.
        if !matches!(updated, Ok(jobs::Update::Unchanged)) {
            let mut state = service.borrow_mut();
            if let Some(window) = state.routes.get(&self.job.scope_id()).map(|r| r.window) {
                state.dirty.insert(window);
            }
            drop(state);
            schedule(&service, cx);
        }
        updated
    }
}
impl Drop for Handle {
    fn drop(&mut self) {
        self.job.close();
        if let Some(service) = self.service.upgrade() {
            service.borrow_mut().routes.remove(&self.job.scope_id());
        }
    }
}
fn begin_close(cx: &mut App) -> Option<(Shared, Vec<Done>)> {
    let service = cx.try_global::<Global>()?.0.clone();
    let mut state = service.borrow_mut();
    if !state.closed {
        state.closed = true;
        state.pool.close();
        state.routes.clear();
        state.windows.clear();
        state.dirty.clear();
        if let Some(completion) = state.delivering.take() {
            state.pool.complete(completion);
        }
        state.input.close();
        while let Ok((completion, _)) = state.input.try_recv() {
            state.pool.complete(completion);
        }
    }
    // Keep fences reachable if a synchronous platform quit arrives while an
    // asynchronous shutdown is awaiting these same workers. Channel closure
    // wakes every receiver, so both cleanup paths may safely await a clone.
    let workers = state.workers.clone();
    drop(state);
    Some((service, workers))
}
fn finish_close(service: &Shared) {
    let mut state = service.borrow_mut();
    state.workers.retain(|done| !done.is_closed());
    state.pool.reap_abandoned();
}
pub async fn shutdown(cx: &mut AsyncApp) {
    let Some((service, workers)) = cx.update(begin_close) else {
        return;
    };
    for done in workers {
        let _ = done.recv().await;
    }
    finish_close(&service);
}
pub fn finish_before_quit(cx: &mut App) {
    let Some((service, workers)) = begin_close(cx) else {
        return;
    };
    for done in workers {
        let _ = done.recv_blocking();
    }
    finish_close(&service);
}

#[cfg(feature = "native-tests")]
#[path = "highlight_host_test.rs"]
pub(crate) mod test;
