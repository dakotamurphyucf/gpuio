//! GPUI main-thread adapter for encoded leases, background decode tickets and
//! per-window atlas ownership. Image views must retain a Handle while mounted.
use crate::{asset_cache, asset_store::Lease};
use gpui::{AnyWindowHandle, App, AsyncApp, RenderImage, Window};
use std::{
    cell::RefCell,
    collections::HashMap,
    rc::{Rc, Weak},
    sync::Arc,
};

pub use asset_cache::Error;
const MAX_WINDOWS: usize = 32;
const MAX_ATLAS_BYTES: usize = 256 * 1024 * 1024;
const MAX_ATLAS_ENTRIES: usize = 1024;
const MAX_ATLAS_FRAMES: usize = 4096;
type Done = async_channel::Receiver<()>;
type Output = (asset_cache::Completion, Done);
type Shared = Rc<RefCell<Service>>;
struct Global(Shared);
impl gpui::Global for Global {}
struct WindowUse {
    window: AnyWindowHandle,
    images: HashMap<gpui::ImageId, Arc<RenderImage>>,
}
struct Service {
    cache: asset_cache::Cache,
    windows: HashMap<gpui::WindowId, WindowUse>,
    atlas_bytes: usize,
    atlas_entries: usize,
    atlas_frames: usize,
    output: async_channel::Sender<Output>,
    input: async_channel::Receiver<Output>,
    workers: Vec<Done>,
    delivering: Option<asset_cache::Completion>,
    scheduled: bool,
    closed: bool,
}
#[derive(Clone)]
pub struct Handle {
    service: Weak<RefCell<Service>>,
    lease: asset_cache::Handle,
}
fn bytes(image: &RenderImage) -> usize {
    (0..image.frame_count())
        .map(|index| image.as_bytes(index).map_or(0, <[u8]>::len))
        .sum()
}

/// Initialize once on the GPUI thread. No threads or idle timers are started;
/// background executor tasks exist only for admitted image work.
pub fn init(cx: &mut App) {
    if cx.has_global::<Global>() {
        return;
    }
    let (output, input) = async_channel::bounded(2);
    let service = Rc::new(RefCell::new(Service {
        cache: asset_cache::Cache::default(),
        windows: HashMap::new(),
        atlas_bytes: 0,
        atlas_entries: 0,
        atlas_frames: 0,
        output,
        input: input.clone(),
        workers: Vec::new(),
        delivering: None,
        scheduled: false,
        closed: false,
    }));
    cx.set_global(Global(service.clone()));
    let weak = Rc::downgrade(&service);
    cx.on_window_closed(move |cx, id| {
        if let Some(service) = weak.upgrade() {
            let mut state = service.borrow_mut();
            if let Some(window) = state.windows.remove(&id) {
                state.atlas_entries -= window.images.len();
                state.atlas_frames -= window
                    .images
                    .values()
                    .map(|image| image.frame_count())
                    .sum::<usize>();
                state.atlas_bytes -= window
                    .images
                    .values()
                    .map(|image| bytes(image))
                    .sum::<usize>();
            }
            drop(state);
            schedule(&service, cx);
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
            // Keep the result in the service while awaiting the worker fence.
            // Terminal cleanup can reclaim it without polling this foreground task.
            service.borrow_mut().delivering = Some(completion);
            drop(service);
            let _ = done.recv().await;
            let Some(service) = weak.upgrade() else {
                break;
            };
            let completion = service.borrow_mut().delivering.take();
            if let Some(completion) = completion {
                service.borrow_mut().cache.complete(completion);
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
fn flush(service: &Shared, cx: &mut App) {
    let operations = {
        let mut state = service.borrow_mut();
        let evictions = state.cache.take_evictions();
        let mut operations = Vec::new();
        for image in evictions {
            let mut count = 0;
            for window in state.windows.values_mut() {
                if window.images.remove(&image.id).is_some() {
                    operations.push((window.window, image.clone()));
                    count += 1;
                }
            }
            state.atlas_entries -= count;
            state.atlas_frames -= count * image.frame_count();
            state.atlas_bytes -= count * bytes(&image);
        }
        operations
    };
    for (handle, image) in operations {
        let _ = handle.update(cx, |_, window, _| window.drop_image(image));
    }
}
fn pump(service: &Shared, cx: &mut App) {
    flush(service, cx);
    if service.borrow().closed {
        return;
    }
    service
        .borrow_mut()
        .workers
        .retain(|done| !done.is_closed());
    for _ in 0..asset_cache::MAX_WORKERS {
        let mut work = service.borrow_mut().cache.next_work();
        if work.is_none() && service.borrow_mut().cache.stats().evictions > 0 {
            // Retry after actually releasing the atlas/cleanup references.
            flush(service, cx);
            work = service.borrow_mut().cache.next_work();
        }
        let Some(work) = work else {
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
                // Two reservations imply at most two undelivered results. Closing
                // the receiver drops late output without waiting on the UI thread.
                let _ = output.try_send((completion, done));
                drop(finished);
            })
            .detach();
    }
    let windows: Vec<_> = service
        .borrow()
        .windows
        .values()
        .map(|window| window.window)
        .collect();
    for handle in windows {
        let _ = handle.update(cx, |_, window, _| window.refresh());
    }
}

/// Acquire once for a new/replaced mounted binding. The Lease must come from
/// this application's native encoded store, after validating its generation.
pub fn request(source: Lease, window: &mut Window, cx: &mut App) -> Result<Handle, Error> {
    init(cx);
    let service = cx.global::<Global>().0.clone();
    let window_handle = window.window_handle();
    let mut state = service.borrow_mut();
    if state.closed {
        return Err(Error::Closed);
    }
    if !state.windows.contains_key(&window_handle.window_id()) && state.windows.len() >= MAX_WINDOWS
    {
        return Err(Error::ResourceLimit);
    }
    let lease = state.cache.request(source)?;
    state
        .windows
        .entry(window_handle.window_id())
        .or_insert_with(|| WindowUse {
            window: window_handle,
            images: HashMap::new(),
        });
    drop(state);
    schedule(&service, cx);
    Ok(Handle {
        service: Rc::downgrade(&service),
        lease,
    })
}

/// Observe ready pixels and reserve their full animation footprint for this
/// window before returning them to img/paint_image. None means still loading.
/// Reuse of the same image in a window does not charge a second atlas copy.
pub fn image(
    handle: &Handle,
    window: &mut Window,
    cx: &mut App,
) -> Result<Option<Arc<RenderImage>>, Error> {
    let service = handle.service.upgrade().ok_or(Error::Closed)?;
    if !cx
        .try_global::<Global>()
        .is_some_and(|global| Rc::ptr_eq(&global.0, &service))
    {
        return Err(Error::Closed);
    }
    let mut state = service.borrow_mut();
    if state.closed {
        return Err(Error::Closed);
    }
    let id = window.window_handle().window_id();
    if !state.windows.contains_key(&id) && state.windows.len() >= MAX_WINDOWS {
        return Err(Error::ResourceLimit);
    }
    // A cloned loading handle can be mounted in another window before pixels
    // exist. Register that observer now, not only when admitting its atlas copy.
    state.windows.entry(id).or_insert_with(|| WindowUse {
        window: window.window_handle(),
        images: HashMap::new(),
    });
    let image = match state.cache.state(&handle.lease) {
        asset_cache::State::Loading => return Ok(None),
        asset_cache::State::Failed(error) => return Err(error),
        asset_cache::State::Ready(image) => image,
    };
    if state.windows[&id].images.contains_key(&image.id) {
        return Ok(Some(image));
    }
    let bytes = bytes(&image);
    if state.atlas_entries >= MAX_ATLAS_ENTRIES
        || image.frame_count() > MAX_ATLAS_FRAMES - state.atlas_frames
        || bytes > MAX_ATLAS_BYTES - state.atlas_bytes
    {
        return Err(Error::ResourceLimit);
    }
    state
        .windows
        .entry(id)
        .or_insert_with(|| WindowUse {
            window: window.window_handle(),
            images: HashMap::new(),
        })
        .images
        .insert(image.id, image.clone());
    state.atlas_entries += 1;
    state.atlas_frames += image.frame_count();
    state.atlas_bytes += bytes;
    Ok(Some(image))
}

fn begin_close(cx: &mut App) -> Option<(Shared, Vec<Done>)> {
    let service = cx.try_global::<Global>()?.0.clone();
    let mut state = service.borrow_mut();
    state.closed = true;
    state.cache.close();
    if let Some(completion) = state.delivering.take() {
        state.cache.complete(completion);
    }
    state.input.close();
    while let Ok((completion, _)) = state.input.try_recv() {
        state.cache.complete(completion);
    }
    let workers = std::mem::take(&mut state.workers);
    drop(state);
    Some((service, workers))
}
/// Nonblocking ordinary shutdown: worker completion never requires a UI update.
pub async fn shutdown(cx: &mut AsyncApp) {
    let Some((service, workers)) = cx.update(begin_close) else {
        return;
    };
    for done in workers {
        let _ = done.recv().await;
    }
    cx.update(|cx| {
        service.borrow_mut().cache.stats();
        flush(&service, cx);
    });
}
/// GPUI's unconditional quit destroys windows before polling quit futures. Drain
/// CPU-only jobs here, before that disposal. Ordinary shutdown uses async above.
pub fn finish_before_quit(cx: &mut App) {
    let Some((service, workers)) = begin_close(cx) else {
        return;
    };
    for done in workers {
        let _ = done.recv_blocking();
    }
    service.borrow_mut().cache.stats();
    flush(&service, cx);
}

#[cfg(feature = "native-image-tests")]
#[path = "image_host_test.rs"]
pub(crate) mod test;
