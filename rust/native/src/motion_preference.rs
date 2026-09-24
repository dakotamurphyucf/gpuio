//! One application policy for all native motion. OS changes never enter Bonsai.
use gpui::{App, Global, Task};
use gpuio_protocol::animation::Preference;
use std::{cell::RefCell, rc::Rc};

struct State {
    preference: Preference,
    system: Option<bool>,
    #[cfg(feature = "native-tests")]
    updates: usize,
}
impl Global for State {}
fn resolve(preference: Preference, system: Option<bool>) -> bool {
    match preference {
        Preference::System => system.unwrap_or(false),
        Preference::Reduce => true,
        Preference::Full => false,
    }
}
pub(crate) fn set(preference: Preference, cx: &mut App) {
    let state = cx.global_mut::<State>();
    state.preference = preference;
    let reduced = resolve(state.preference, state.system);
    cx.set_reduce_motion(reduced);
}
fn system_changed(system: Option<bool>, cx: &mut App) {
    let state = cx.global_mut::<State>();
    state.system = system;
    #[cfg(feature = "native-tests")]
    {
        state.updates += 1;
    }
    let reduced = resolve(state.preference, system);
    cx.set_reduce_motion(reduced);
}

pub(crate) struct Watch {
    // Remove the OS producer before cancelling its consumer.
    _source: platform::Source,
    _consumer: Task<()>,
}
pub(crate) fn init(cx: &mut App) -> Rc<RefCell<Option<Watch>>> {
    cx.set_global(State {
        preference: Preference::System,
        system: None,
        #[cfg(feature = "native-tests")]
        updates: 0,
    });
    let (source, receiver, initial) = platform::start(cx);
    system_changed(initial, cx);
    let consumer = cx.spawn(async move |cx| {
        while let Ok(value) = receiver.recv().await {
            cx.update(|cx| system_changed(platform::value(value), cx));
        }
    });
    Rc::new(RefCell::new(Some(Watch {
        _source: source,
        _consumer: consumer,
    })))
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use block2::RcBlock;
    use objc2::rc::Retained;
    use objc2::runtime::ProtocolObject;
    use objc2_app_kit::{NSWorkspace, NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification};
    use objc2_foundation::{NSNotificationCenter, NSObjectProtocol};

    pub(super) struct Source {
        center: Retained<NSNotificationCenter>,
        observer: Retained<ProtocolObject<dyn NSObjectProtocol>>,
    }
    impl Drop for Source {
        fn drop(&mut self) {
            // The token belongs to exactly this center and remains retained here.
            unsafe {
                self.center.removeObserver((*self.observer).as_ref());
            }
        }
    }
    pub(super) fn value((): ()) -> Option<bool> {
        Some(NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion())
    }
    pub(super) fn start(_: &mut App) -> (Source, async_channel::Receiver<()>, Option<bool>) {
        let (sender, receiver) = async_channel::bounded(1);
        let center = NSWorkspace::sharedWorkspace().notificationCenter();
        let callback = RcBlock::new(move |_| {
            let _ = sender.try_send(());
        });
        // The callback only signals a bounded channel. AppKit is read on the GPUI
        // main thread, even if the notification was posted from another thread.
        let observer = unsafe {
            center.addObserverForName_object_queue_usingBlock(
                Some(NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification),
                None,
                None,
                &callback,
            )
        };
        // Subscribe before reading, so a concurrent change is re-read by consumer.
        (Source { center, observer }, receiver, value(()))
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    pub(super) type Source = Task<()>;
    pub(super) fn value(value: Option<bool>) -> Option<bool> {
        value
    }
    pub(super) fn start(
        cx: &mut App,
    ) -> (Source, async_channel::Receiver<Option<bool>>, Option<bool>) {
        let (sender, receiver) = async_channel::bounded(1);
        let source = cx.background_executor().spawn(async move {
            gpuio_portal::watch_motion(move |value| {
                let _ = sender.force_send(value);
            })
            .await;
        });
        (source, receiver, None)
    }
}

#[cfg(feature = "native-tests")]
pub(crate) async fn test(cx: &mut gpui::AsyncApp, watch: &Rc<RefCell<Option<Watch>>>) {
    cx.update(|cx| {
        set(Preference::System, cx);
        system_changed(Some(true), cx);
        assert!(cx.reduce_motion());
        set(Preference::Full, cx);
        assert!(!cx.reduce_motion());
        system_changed(Some(false), cx);
        system_changed(Some(true), cx);
        assert!(
            !cx.reduce_motion(),
            "explicit full overrides live system updates"
        );
        set(Preference::System, cx);
        assert!(
            cx.reduce_motion(),
            "system resumes the latest observed preference"
        );
        set(Preference::Reduce, cx);
        system_changed(None, cx);
        assert!(
            cx.reduce_motion(),
            "explicit reduce survives unavailable system preference"
        );
        set(Preference::System, cx);
        assert!(
            !cx.reduce_motion(),
            "unavailable system preference uses full motion"
        );
        set(Preference::Full, cx);
    });
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{
            NSWorkspace, NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification,
        };
        fn post() {
            // Exercise the real observer without changing the user's OS settings.
            unsafe {
                NSWorkspace::sharedWorkspace()
                    .notificationCenter()
                    .postNotificationName_object(
                        NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification,
                        None,
                    );
            }
        }
        let before = cx.update(|cx| cx.global::<State>().updates);
        post();
        for _ in 0..100 {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(10))
                .await;
            if cx.update(|cx| cx.global::<State>().updates > before) {
                break;
            }
        }
        cx.update(|cx| {
            assert!(
                cx.global::<State>().updates > before,
                "AppKit notification reaches GPUI"
            );
            assert_eq!(cx.global::<State>().system, platform::value(()));
            assert!(
                !cx.reduce_motion(),
                "OS notification preserves Full override"
            );
        });
        watch.borrow_mut().take();
        let before = cx.update(|cx| cx.global::<State>().updates);
        post();
        cx.background_executor()
            .timer(std::time::Duration::from_millis(80))
            .await;
        assert_eq!(
            cx.update(|cx| cx.global::<State>().updates),
            before,
            "disposed observer stops delivery"
        );
    }
    watch.borrow_mut().take();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overrides_and_unknown_system_values() {
        for system in [None, Some(false), Some(true)] {
            assert!(resolve(Preference::Reduce, system));
            assert!(!resolve(Preference::Full, system));
            assert_eq!(resolve(Preference::System, system), system == Some(true));
        }
    }
}
