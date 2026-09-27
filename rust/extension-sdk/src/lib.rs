//! Source-compatible, statically linked GPUIO component authoring API.
//! This is trusted Rust code, not a sandbox or stable binary plugin ABI.
pub use gpui;
use std::{
    collections::BTreeMap,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex, Weak},
};

pub const SDK_VERSION: u32 = 1;
pub const GPUI_REVISION: &str = "a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b";
pub const MAX_COMPONENTS: usize = 64;
pub const MAX_PROPERTIES: usize = 65_536;
pub const MAX_MESSAGE: usize = 16_384;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidSchema,
    IncompatibleSdk,
    DuplicateComponent,
    LimitExceeded,
    UnknownComponent,
    IncompatibleSchema,
    InvalidProperties,
    InvalidCommand,
    Closed,
    Hidden,
    Stale,
    Overloaded,
    Panicked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub name: &'static str,
    pub version: u16,
    /// SHA-256 of the package's documented complete property/command/event schema.
    pub fingerprint: &'static str,
    pub sdk_version: u32,
    pub gpui_revision: &'static str,
    pub max_properties: usize,
    pub max_command: usize,
    pub max_event: usize,
}
impl Descriptor {
    pub fn validate(&self) -> Result<(), Error> {
        let mut segments = self.name.split('.');
        let valid_segment = |s: &str| {
            s.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                && s.bytes()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
        };
        if self.name.len() > 128
            || segments.clone().count() < 2
            || !segments.all(valid_segment)
            || self.version == 0
            || self.fingerprint.len() != 64
            || !self
                .fingerprint
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(Error::InvalidSchema);
        }
        if self.sdk_version != SDK_VERSION || self.gpui_revision != GPUI_REVISION {
            return Err(Error::IncompatibleSdk);
        }
        if !(1..=MAX_PROPERTIES).contains(&self.max_properties)
            || !(1..=MAX_MESSAGE).contains(&self.max_command)
            || !(1..=MAX_MESSAGE).contains(&self.max_event)
        {
            return Err(Error::LimitExceeded);
        }
        Ok(())
    }
}

/// Only these hooks run on the native UI thread. A component may retain native
/// entities/resources, but must release them and cancel its work on unmount.
/// Use EventSink::guard for callbacks installed in native elements. Arbitrary
/// direct GPUI callbacks remain the trusted author's panic-containment duty.
pub trait Component {
    fn update(&mut self, properties: &[u8], cx: &mut Context<'_>) -> Result<(), Error>;
    fn command(&mut self, command: &[u8], cx: &mut Context<'_>) -> Result<(), Error>;
    fn render(&mut self, cx: &mut Context<'_>) -> Result<gpui::AnyElement, Error>;
    fn unmount(&mut self) {}
}

/// Validation is pure, bounded and must check nested lengths before allocating.
/// A factory must never call into OCaml. One factory is registered per name.
pub trait Factory: Send + Sync + 'static {
    fn descriptor(&self) -> Descriptor;
    fn validate_properties(&self, properties: &[u8]) -> Result<(), Error>;
    fn validate_command(&self, command: &[u8]) -> Result<(), Error>;
    fn mount(&self, properties: &[u8], cx: &mut Context<'_>) -> Result<Box<dyn Component>, Error>;
}

pub struct Context<'a> {
    pub window: &'a mut gpui::Window,
    pub app: &'a mut gpui::App,
    pub focus: gpui::FocusHandle,
    pub events: EventSink,
}

/// Contains ordinary unwinding Rust panics. This cannot recover aborts or unsafe
/// code corruption. Host hook boundaries and extension-installed callbacks use it.
pub fn contain<T>(f: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or(Err(Error::Panicked))
}

pub struct Registry {
    factories: BTreeMap<&'static str, (Descriptor, Arc<dyn Factory>)>,
}
impl Registry {
    pub fn new(factories: impl IntoIterator<Item = Arc<dyn Factory>>) -> Result<Self, Error> {
        let mut result = Self {
            factories: BTreeMap::new(),
        };
        for factory in factories {
            if result.factories.len() == MAX_COMPONENTS {
                return Err(Error::LimitExceeded);
            }
            let descriptor = contain(|| Ok(factory.descriptor()))?;
            descriptor.validate()?;
            if result
                .factories
                .insert(descriptor.name, (descriptor, factory))
                .is_some()
            {
                return Err(Error::DuplicateComponent);
            }
        }
        Ok(result)
    }

    pub fn descriptors(&self) -> impl Iterator<Item = Descriptor> + '_ {
        self.factories.values().map(|(descriptor, _)| *descriptor)
    }

    pub fn resolve(
        &self,
        name: &str,
        version: u16,
        fingerprint: &str,
    ) -> Result<(Descriptor, Arc<dyn Factory>), Error> {
        let (descriptor, factory) = self.factories.get(name).ok_or(Error::UnknownComponent)?;
        if descriptor.version != version || descriptor.fingerprint != fingerprint {
            return Err(Error::IncompatibleSchema);
        }
        Ok((*descriptor, factory.clone()))
    }
}

struct EventState {
    generation: u64,
    visible: bool,
    pointer_enabled: bool,
    closed: bool,
    panicked: bool,
}
type Deliver = dyn Fn(Vec<u8>) -> Result<(), Error> + Send + Sync;
struct LeaseInner {
    state: Mutex<EventState>,
    maximum: usize,
    deliver: Box<Deliver>,
}

/// Host-owned revocation handle. Sinks hold only Weak references, so background
/// callbacks cannot keep a closed component or its transport alive. Delivery is
/// serialized with revocation. The supplied delivery closure must enqueue only,
/// must not reenter this lease, and must not run application code.
pub struct EventLease(Arc<LeaseInner>);
impl EventLease {
    pub fn new(
        maximum: usize,
        deliver: impl Fn(Vec<u8>) -> Result<(), Error> + Send + Sync + 'static,
    ) -> Result<Self, Error> {
        if !(1..=MAX_MESSAGE).contains(&maximum) {
            return Err(Error::LimitExceeded);
        }
        Ok(Self(Arc::new(LeaseInner {
            state: Mutex::new(EventState {
                generation: 1,
                visible: true,
                pointer_enabled: true,
                closed: false,
                panicked: false,
            }),
            maximum,
            deliver: Box::new(deliver),
        })))
    }

    pub fn sink(&self) -> EventSink {
        let generation = self
            .0
            .state
            .lock()
            .expect("event lease poisoned")
            .generation;
        EventSink {
            lease: Arc::downgrade(&self.0),
            generation,
        }
    }

    /// Invalidate callbacks derived from obsolete properties. Exhaustion closes
    /// the lease permanently instead of allowing identity reuse.
    pub fn advance(&self) -> Result<EventSink, Error> {
        let mut state = self.0.state.lock().expect("event lease poisoned");
        if state.closed {
            return Err(Error::Closed);
        }
        let Some(next) = state.generation.checked_add(1) else {
            state.closed = true;
            return Err(Error::Closed);
        };
        state.generation = next;
        Ok(EventSink {
            lease: Arc::downgrade(&self.0),
            generation: next,
        })
    }

    /// A guarded callback panic closes the instance. The host observes this on
    /// its next lifecycle/render boundary and delivers a failure observation.
    pub fn failure(&self) -> Option<Error> {
        self.0
            .state
            .lock()
            .expect("event lease poisoned")
            .panicked
            .then_some(Error::Panicked)
    }

    pub fn set_visible(&self, visible: bool) {
        self.0.state.lock().expect("event lease poisoned").visible = visible;
    }

    pub fn set_pointer_enabled(&self, enabled: bool) {
        self.0
            .state
            .lock()
            .expect("event lease poisoned")
            .pointer_enabled = enabled;
    }

    pub fn close(&self) {
        self.0.state.lock().expect("event lease poisoned").closed = true;
    }
}
impl Drop for EventLease {
    fn drop(&mut self) {
        self.close();
    }
}

#[derive(Clone)]
pub struct EventSink {
    lease: Weak<LeaseInner>,
    generation: u64,
}
impl EventSink {
    pub fn check(&self) -> Result<(), Error> {
        self.check_input(false)
    }

    fn check_input(&self, pointer: bool) -> Result<(), Error> {
        let lease = self.lease.upgrade().ok_or(Error::Closed)?;
        let state = lease.state.lock().expect("event lease poisoned");
        if state.closed {
            Err(Error::Closed)
        } else if state.generation != self.generation {
            Err(Error::Stale)
        } else if !state.visible || (pointer && !state.pointer_enabled) {
            Err(Error::Hidden)
        } else {
            Ok(())
        }
    }

    pub fn emit(&self, payload: Vec<u8>) -> Result<(), Error> {
        let lease = self.lease.upgrade().ok_or(Error::Closed)?;
        if payload.len() > lease.maximum {
            return Err(Error::LimitExceeded);
        }
        let state = lease.state.lock().expect("event lease poisoned");
        if state.closed {
            return Err(Error::Closed);
        }
        if state.generation != self.generation {
            return Err(Error::Stale);
        }
        if !state.visible {
            return Err(Error::Hidden);
        }
        // The lock fences close/update against the enqueue, not OCaml dispatch.
        // Host delivery additionally stamps native generation-checked identity.
        contain(|| (lease.deliver)(payload))
    }

    /// Pointer callbacks additionally honor the inherited PointerEvents style.
    /// Keyboard and accessibility actions use [guard], so disabling pointer
    /// interaction does not remove an otherwise enabled control's keyboard use.
    pub fn guard_pointer<T>(&self, f: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
        self.check_input(true)?;
        self.guard(f)
    }

    /// Guard an input callback on the native UI thread. Obsolete/hidden callbacks
    /// do not run. A panic permanently closes this lease; callers must report
    /// the error through their host failure path. This is not a background-task
    /// synchronization primitive for arbitrary mutable native state.
    pub fn guard<T>(&self, f: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
        self.check()?;
        let result = contain(f);
        if matches!(result, Err(Error::Panicked))
            && let Some(lease) = self.lease.upgrade()
        {
            let mut state = lease.state.lock().expect("event lease poisoned");
            state.closed = true;
            state.panicked = true;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const DESCRIPTOR: Descriptor = Descriptor {
        name: "example.counter",
        version: 1,
        fingerprint: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        sdk_version: SDK_VERSION,
        gpui_revision: GPUI_REVISION,
        max_properties: 64,
        max_command: 16,
        max_event: 16,
    };
    struct Example;
    impl Factory for Example {
        fn descriptor(&self) -> Descriptor {
            DESCRIPTOR
        }
        fn validate_properties(&self, _: &[u8]) -> Result<(), Error> {
            Ok(())
        }
        fn validate_command(&self, _: &[u8]) -> Result<(), Error> {
            Ok(())
        }
        fn mount(&self, _: &[u8], _: &mut Context<'_>) -> Result<Box<dyn Component>, Error> {
            unreachable!("pure registry tests do not mount native components")
        }
    }

    #[test]
    fn schema_and_registry_reject_incompatible_or_ambiguous_components() {
        assert_eq!(DESCRIPTOR.validate(), Ok(()));
        for name in [
            "counter",
            "Example.counter",
            "example..counter",
            "example.λ",
        ] {
            assert_eq!(
                Descriptor { name, ..DESCRIPTOR }.validate(),
                Err(Error::InvalidSchema)
            );
        }
        assert_eq!(
            Descriptor {
                sdk_version: 2,
                ..DESCRIPTOR
            }
            .validate(),
            Err(Error::IncompatibleSdk)
        );
        assert_eq!(
            Descriptor {
                max_event: MAX_MESSAGE + 1,
                ..DESCRIPTOR
            }
            .validate(),
            Err(Error::LimitExceeded)
        );
        let factory: Arc<dyn Factory> = Arc::new(Example);
        assert!(matches!(
            Registry::new([factory.clone(), factory.clone()]),
            Err(Error::DuplicateComponent)
        ));
        let registry = Registry::new([factory]).unwrap();
        assert!(
            registry
                .resolve(DESCRIPTOR.name, 1, DESCRIPTOR.fingerprint)
                .is_ok()
        );
        assert!(matches!(
            registry.resolve(DESCRIPTOR.name, 2, DESCRIPTOR.fingerprint),
            Err(Error::IncompatibleSchema)
        ));
        assert!(matches!(
            registry.resolve("missing.component", 1, DESCRIPTOR.fingerprint),
            Err(Error::UnknownComponent)
        ));
    }

    #[test]
    fn sinks_are_bounded_revocable_and_do_not_keep_the_owner_alive() {
        let delivered = Arc::new(AtomicUsize::new(0));
        let count = delivered.clone();
        let lease = EventLease::new(4, move |_| {
            count.fetch_add(1, Ordering::Relaxed);
            Ok(())
        })
        .unwrap();
        let old = lease.sink();
        assert_eq!(old.emit(vec![0; 4]), Ok(()));
        assert_eq!(old.emit(vec![0; 5]), Err(Error::LimitExceeded));
        lease.set_visible(false);
        assert_eq!(old.emit(vec![]), Err(Error::Hidden));
        assert_eq!(
            old.guard(|| panic!("must not execute a hidden input")),
            Err::<(), _>(Error::Hidden)
        );
        lease.set_visible(true);
        let current = lease.advance().unwrap();
        assert_eq!(old.emit(vec![]), Err(Error::Stale));
        assert_eq!(
            old.guard(|| panic!("must not execute stale input")),
            Err::<(), _>(Error::Stale)
        );
        assert_eq!(current.emit(vec![]), Ok(()));
        drop(lease);
        assert_eq!(current.emit(vec![]), Err(Error::Closed));
        assert_eq!(delivered.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn guarded_input_panics_close_the_native_lease() {
        let lease = EventLease::new(4, |_| Ok(())).unwrap();
        let sink = lease.sink();
        assert_eq!(
            sink.guard(|| panic!("component callback")),
            Err::<(), _>(Error::Panicked)
        );
        assert_eq!(sink.emit(vec![]), Err(Error::Closed));
        assert_eq!(lease.failure(), Some(Error::Panicked));
        assert_eq!(lease.advance().err(), Some(Error::Closed));
    }

    #[test]
    fn overload_does_not_retain_a_payload_or_close_the_component() {
        let lease = EventLease::new(4, |_| Err(Error::Overloaded)).unwrap();
        assert_eq!(lease.sink().emit(vec![1]), Err(Error::Overloaded));
        assert_eq!(lease.sink().check(), Ok(()));
        lease.close();
        lease.set_visible(true);
        assert_eq!(lease.sink().check(), Err(Error::Closed));
    }
}
