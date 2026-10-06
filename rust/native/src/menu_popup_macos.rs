//! AppKit tracking approach adapted from the pinned gpui-component native menu.
//! Copyright 2024–2026 Longbridge; Apache-2.0. See
//! docs/catalog/sources/gpui-kit-LICENSE and component-native_menu-macos.rs.txt.
//! GPUIO changes: retained view, cancellable owner, weak lease and indexed routes.
//!
//! AppKit popup ownership. The runner is called only outside GPUI borrows;
//! Objective-C targets record an index and never call application code.
use block2::RcBlock;
use gpui::{ForegroundExecutor, Pixels, Point, Window};
use objc2::runtime::{AnyObject, NSObject};
use objc2::{AnyThread, DefinedClass, MainThreadMarker, define_class, msg_send, rc::Retained, sel};
use objc2_app_kit::{NSMenu, NSMenuItem, NSView};
use objc2_core_foundation::{CFRunLoop, kCFRunLoopCommonModes};
use objc2_foundation::{NSPoint, NSString};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

pub(super) enum Item {
    Separator,
    Row {
        label: String,
        enabled: bool,
        checked: bool,
        action: Option<usize>,
        icon: Option<std::sync::Arc<gpui::RenderImage>>,
    },
    Submenu {
        label: String,
        enabled: bool,
        items: Vec<Item>,
        icon: Option<std::sync::Arc<gpui::RenderImage>>,
    },
}

struct TargetState {
    selected: Cell<Option<usize>>,
}
define_class!(
    #[unsafe(super(NSObject))]
    #[name = "GPUIOPopupMenuTarget"]
    #[ivars = TargetState]
    struct Target;
    impl Target {
        #[unsafe(method(selectItem:))]
        fn select_item(&self, sender: &NSMenuItem) {
            self.ivars().selected.set(usize::try_from(sender.tag()).ok());
        }
    }
);
impl Target {
    fn new() -> Retained<Self> {
        let this = Self::alloc().set_ivars(TargetState {
            selected: Cell::new(None),
        });
        // SAFETY: initialize the allocated NSObject subclass on the main thread.
        unsafe { msg_send![super(this), init] }
    }
}

struct State {
    menu: Retained<NSMenu>,
    target: Retained<Target>,
    view: Retained<NSView>,
    cancelled: Cell<bool>,
    finished: Cell<bool>,
}
thread_local! {
    static ACTIVE: RefCell<Weak<State>> = const { RefCell::new(Weak::new()) };
}

/// The View owns this lease. Its destruction invalidates selection immediately,
/// and schedules AppKit cancellation without holding the View/Session borrow.
pub(super) struct Owner {
    state: Rc<State>,
    executor: ForegroundExecutor,
}
impl Drop for Owner {
    fn drop(&mut self) {
        if self.state.finished.get() || self.state.cancelled.replace(true) {
            return;
        }
        let state = self.state.clone();
        self.executor
            .spawn(async move {
                if !state.finished.get() {
                    state.menu.cancelTrackingWithoutAnimation();
                }
            })
            .detach();
    }
}
pub(super) struct Runner(Rc<State>);
impl Runner {
    pub(super) fn run(self, position: Point<Pixels>) -> Option<usize> {
        let state = &self.0;
        if state.cancelled.get() || state.view.window().is_none() {
            state.finished.set(true);
            return None;
        }
        let y = f32::from(position.y) as f64;
        let location = NSPoint::new(
            f32::from(position.x) as f64,
            if state.view.isFlipped() {
                y
            } else {
                state.view.bounds().size.height - y
            },
        );
        state
            .menu
            .popUpMenuPositioningItem_atLocation_inView(None, location, Some(&state.view));
        state.finished.set(true);
        if state.cancelled.get() {
            None
        } else {
            state.target.ivars().selected.get()
        }
    }
}

pub(super) fn busy() -> bool {
    ACTIVE.with(|active| {
        active
            .borrow()
            .upgrade()
            .is_some_and(|state| !state.finished.get())
    })
}

pub(super) fn prepare(
    items: &[Item],
    window: &Window,
    executor: ForegroundExecutor,
) -> Option<(Owner, Runner)> {
    let marker = MainThreadMarker::new()?;
    if busy() {
        return None;
    }
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    // SAFETY: GPUI lends a live NSView on the main thread. Retain it before the
    // borrowed handle and Window access end; no unowned pointer enters the task.
    let view = unsafe { Retained::retain(handle.ns_view.cast::<NSView>().as_ptr()) }?;
    view.window()?;
    let target = Target::new();
    let state = Rc::new(State {
        menu: build(
            items,
            &target,
            marker,
            &mut super::popup_icon::Budget::default(),
        ),
        target,
        view,
        cancelled: Cell::new(false),
        finished: Cell::new(false),
    });
    ACTIVE.with(|active| *active.borrow_mut() = Rc::downgrade(&state));
    Some((
        Owner {
            state: state.clone(),
            executor,
        },
        Runner(state),
    ))
}

fn build(
    items: &[Item],
    target: &Target,
    marker: MainThreadMarker,
    icons: &mut super::popup_icon::Budget,
) -> Retained<NSMenu> {
    let menu = NSMenu::new(marker);
    menu.setAutoenablesItems(false);
    for item in items {
        let row = match item {
            Item::Separator => NSMenuItem::separatorItem(marker),
            Item::Row {
                label,
                enabled,
                checked,
                action,
                icon,
            } => {
                let row = NSMenuItem::new(marker);
                row.setTitle(&NSString::from_str(label));
                row.setEnabled(*enabled && action.is_some());
                row.setState(isize::from(*checked));
                if let Some(image) = icon.as_ref().and_then(|image| icons.image(image)) {
                    row.setImage(Some(&image));
                }
                if let Some(action) = action.filter(|_| *enabled) {
                    row.setTag(action as isize);
                    // SAFETY: Target implements selectItem: with the NSMenuItem
                    // signature and is retained alongside this menu throughout tracking.
                    unsafe {
                        row.setTarget(Some(target as &AnyObject));
                        row.setAction(Some(sel!(selectItem:)));
                    }
                }
                row
            }
            Item::Submenu {
                label,
                enabled,
                items,
                icon,
            } => {
                let row = NSMenuItem::new(marker);
                row.setTitle(&NSString::from_str(label));
                row.setEnabled(*enabled);
                if let Some(image) = icon.as_ref().and_then(|image| icons.image(image)) {
                    row.setImage(Some(&image));
                }
                row.setSubmenu(Some(&build(items, target, marker, icons)));
                row
            }
        };
        menu.addItem(&row);
    }
    menu
}

/// Schedule outside the serial dispatch main queue as well as outside GPUI
/// borrows. A nested AppKit tracking loop inside a main-queue task prevents
/// other main-queue tasks (including window close) from running until dismissal.
pub(super) fn defer(f: impl FnOnce() + 'static) -> bool {
    if MainThreadMarker::new().is_none() {
        return false;
    }
    let Some(run_loop) = CFRunLoop::main() else {
        return false;
    };
    // SAFETY: this is the Foundation-provided constant CFString mode.
    let Some(mode) = (unsafe { kCFRunLoopCommonModes }) else {
        return false;
    };
    let callback = RefCell::new(Some(f));
    let block = RcBlock::new(move || {
        // Release the cell borrow before calling anything that can reenter.
        let callback = callback.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
    });
    // SAFETY: schedule only on the main run loop from the main thread. The
    // non-null mode is a CFString and the copied block remains owned by that
    // loop until invocation/disposal on the main thread. Captured !Send owners
    // never cross threads. CFRunLoopPerformBlock does not invoke it inline.
    unsafe {
        run_loop.perform_block(Some(mode), Some(&block));
    }
    run_loop.wake_up();
    true
}
