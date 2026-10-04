use gpuio_document_sdk::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

const DESCRIPTOR: Descriptor = Descriptor {
    name: "example.document",
    version: 1,
    fingerprint: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    sdk_version: SDK_VERSION,
    gpui_revision: GPUI_REVISION,
    base_revision: BASE_REVISION,
    max_properties: 4,
    max_event: 16,
    max_retained_bytes: 1024,
};
struct Example {
    calls: Arc<AtomicUsize>,
}
impl Factory for Example {
    fn descriptor(&self) -> Descriptor {
        DESCRIPTOR
    }
    fn validate_properties(&self, bytes: &[u8]) -> Result<(), Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if bytes == b"yes" {
            Ok(())
        } else {
            Err(Error::InvalidProperties)
        }
    }
    fn configure(&self, bytes: &[u8], cx: &PrepareContext<'_>) -> Result<Profile, Error> {
        cx.check()?;
        assert_eq!(bytes, b"yes");
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Profile::new())
    }
}
fn factory(calls: &Arc<AtomicUsize>) -> Arc<dyn Factory> {
    Arc::new(Example {
        calls: calls.clone(),
    })
}
#[test]
fn registry_checks_schema_and_limits_before_factory_and_keeps_binding_owned() {
    let calls = Arc::new(AtomicUsize::new(0));
    let f = factory(&calls);
    assert!(matches!(
        Registry::new([f.clone(), f.clone()]),
        Err(Error::DuplicateProfile)
    ));
    let registry = Registry::new([f]).unwrap();
    assert_eq!(registry.descriptors().collect::<Vec<_>>(), vec![DESCRIPTOR]);
    assert!(matches!(
        registry.bind("other.profile", 1, DESCRIPTOR.fingerprint, b"yes"),
        Err(Error::UnknownProfile)
    ));
    assert!(matches!(
        registry.bind(DESCRIPTOR.name, 2, DESCRIPTOR.fingerprint, b"yes"),
        Err(Error::IncompatibleSchema)
    ));
    assert!(matches!(
        registry.bind(DESCRIPTOR.name, 1, "bad", b"yes"),
        Err(Error::IncompatibleSchema)
    ));
    assert!(matches!(
        registry.bind(DESCRIPTOR.name, 1, DESCRIPTOR.fingerprint, b"longer"),
        Err(Error::LimitExceeded)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(matches!(
        registry.bind(DESCRIPTOR.name, 1, DESCRIPTOR.fingerprint, b"no"),
        Err(Error::InvalidProperties)
    ));
    let mut bytes = b"yes".to_vec();
    let binding = registry
        .bind(DESCRIPTOR.name, 1, DESCRIPTOR.fingerprint, &bytes)
        .unwrap();
    bytes.fill(0);
    drop(registry);
    assert_eq!(binding.properties(), b"yes");
    assert_eq!(binding.descriptor(), DESCRIPTOR);
    assert!(matches!(
        binding.configure(&PrepareContext::new(&|| true)),
        Err(Error::Cancelled)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    std::thread::spawn(move || {
        let profile = binding.configure(&PrepareContext::new(&|| false)).unwrap();
        let code = Code::new("x", None, false).unwrap();
        assert_eq!(
            profile
                .highlight(&code, &PrepareContext::new(&|| false))
                .unwrap(),
            None
        );
    })
    .join()
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}
#[test]
fn descriptors_reject_incompatible_dependencies_and_malformed_schemas() {
    for d in [
        Descriptor {
            name: "unqualified",
            ..DESCRIPTOR
        },
        Descriptor {
            name: "example.Bad",
            ..DESCRIPTOR
        },
        Descriptor {
            version: 0,
            ..DESCRIPTOR
        },
        Descriptor {
            fingerprint: "bad",
            ..DESCRIPTOR
        },
    ] {
        assert_eq!(d.validate(), Err(Error::InvalidSchema));
    }
    for d in [
        Descriptor {
            sdk_version: 2,
            ..DESCRIPTOR
        },
        Descriptor {
            gpui_revision: "unknown",
            ..DESCRIPTOR
        },
        Descriptor {
            base_revision: "unknown",
            ..DESCRIPTOR
        },
    ] {
        assert_eq!(d.validate(), Err(Error::IncompatibleSdk));
    }
    for d in [
        Descriptor {
            max_properties: 0,
            ..DESCRIPTOR
        },
        Descriptor {
            max_properties: MAX_PROPERTIES + 1,
            ..DESCRIPTOR
        },
        Descriptor {
            max_event: MAX_EVENT + 1,
            ..DESCRIPTOR
        },
        Descriptor {
            max_retained_bytes: MAX_RETAINED_BYTES + 1,
            ..DESCRIPTOR
        },
    ] {
        assert_eq!(d.validate(), Err(Error::LimitExceeded));
    }
}
struct Runs(Vec<Highlight>);
impl Highlighter for Runs {
    fn highlight(&self, _: &Code<'_>, _: &PrepareContext<'_>) -> Result<Vec<Highlight>, Error> {
        Ok(self.0.clone())
    }
}
fn highlight(runs: Vec<Highlight>) -> Result<Option<Vec<Highlight>>, Error> {
    Profile::new()
        .with_highlighter(Arc::new(Runs(runs)))
        .highlight(
            &Code::new("a世界🙂", Some("example"), true).unwrap(),
            &PrepareContext::new(&|| false),
        )
}
#[test]
fn highlights_accept_unicode_boundaries_and_reject_the_entire_invalid_result() {
    let valid = vec![
        Highlight::new(0..1, [1, 2, 3]),
        Highlight::new(1..7, [4, 5, 6]),
        Highlight::new(7..11, [7, 8, 9]),
    ];
    assert_eq!(highlight(valid.clone()), Ok(Some(valid.clone())));
    assert_eq!(highlight(vec![]), Ok(Some(vec![])));
    for invalid in [0..0, 1..2, 2..7, 7..12, 8..11] {
        assert_eq!(
            highlight(vec![Highlight::new(invalid, [0; 3])]),
            Err(Error::InvalidHighlight)
        );
    }
    let mut overlap = valid.clone();
    overlap[1].bytes.start = 0;
    assert_eq!(highlight(overlap), Err(Error::InvalidHighlight));
    let mut unsorted = valid.clone();
    unsorted.reverse();
    assert_eq!(highlight(unsorted), Err(Error::InvalidHighlight));
    for weight in [0, 1001] {
        let mut bad = valid.clone();
        bad[2].weight = weight;
        assert_eq!(highlight(bad), Err(Error::InvalidHighlight));
    }
    assert_eq!(
        highlight(vec![Highlight::new(0..1, [0; 3]); MAX_RUNS + 1]),
        Err(Error::LimitExceeded)
    );
    let text = "x".repeat(MAX_CODE_BYTES);
    assert!(Code::new(&text, None, false).is_ok());
    assert!(matches!(
        Code::new(&(text + "x"), None, false),
        Err(Error::LimitExceeded)
    ));
    assert!(matches!(
        Code::new("", Some(&"x".repeat(4097)), false),
        Err(Error::LimitExceeded)
    ));
}
struct Cancels(Arc<AtomicBool>);
impl Highlighter for Cancels {
    fn highlight(&self, code: &Code<'_>, _: &PrepareContext<'_>) -> Result<Vec<Highlight>, Error> {
        assert_eq!(code.language(), Some("test"));
        assert!(code.dark());
        self.0.store(true, Ordering::SeqCst);
        Ok(vec![Highlight::new(0..code.text().len(), [0; 3])])
    }
}
struct Panic;
impl Highlighter for Panic {
    fn highlight(&self, _: &Code<'_>, _: &PrepareContext<'_>) -> Result<Vec<Highlight>, Error> {
        panic!("grammar failure")
    }
}
#[test]
fn worker_cancellation_and_panics_do_not_publish_partial_highlights() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let check = || cancelled.load(Ordering::SeqCst);
    let cx = PrepareContext::new(&check);
    let code = Code::new("test", Some("test"), true).unwrap();
    let profile = Profile::new().with_highlighter(Arc::new(Cancels(cancelled.clone())));
    assert_eq!(profile.highlight(&code, &cx), Err(Error::Cancelled));
    assert_eq!(
        Profile::new()
            .with_highlighter(Arc::new(Panic))
            .highlight(&code, &PrepareContext::new(&|| false)),
        Err(Error::Panicked)
    );
    assert_eq!(
        PrepareContext::new(&|| panic!("bad cancellation hook")).check(),
        Err(Error::Panicked)
    );
}
struct BadFactory {
    configure: bool,
}
impl Factory for BadFactory {
    fn descriptor(&self) -> Descriptor {
        DESCRIPTOR
    }
    fn validate_properties(&self, _: &[u8]) -> Result<(), Error> {
        if self.configure {
            Ok(())
        } else {
            panic!("decode")
        }
    }
    fn configure(&self, _: &[u8], _: &PrepareContext<'_>) -> Result<Profile, Error> {
        panic!("configure")
    }
}
#[test]
fn factory_panics_are_errors_at_admission_and_preparation_boundaries() {
    for configure in [false, true] {
        let registry =
            Registry::new([Arc::new(BadFactory { configure }) as Arc<dyn Factory>]).unwrap();
        let binding = registry.bind(DESCRIPTOR.name, 1, DESCRIPTOR.fingerprint, b"yes");
        if configure {
            assert!(matches!(
                binding.unwrap().configure(&PrepareContext::new(&|| false)),
                Err(Error::Panicked)
            ));
        } else {
            assert!(matches!(binding, Err(Error::Panicked)));
        }
    }
}
#[test]
fn renderer_provenance_and_revocable_events_do_not_keep_the_host_alive() {
    use std::sync::Mutex;
    assert_eq!(Source::new(0, 1), Err(Error::InvalidSource));
    assert_eq!(Source::new(1, 0), Err(Error::InvalidSource));
    let delivered = Arc::new(Mutex::new(vec![]));
    let target = delivered.clone();
    let lease = EventLease::new(4, move |bytes| {
        target.lock().unwrap().push(bytes);
        Ok(())
    })
    .unwrap();
    let cx = RenderContext::new(Source::new(3, 7).unwrap(), lease.sink());
    assert_eq!((cx.source().generation(), cx.source().revision()), (3, 7));
    cx.events().emit(vec![1, 2]).unwrap();
    assert_eq!(*delivered.lock().unwrap(), vec![vec![1, 2]]);
    lease.advance().unwrap();
    assert_eq!(
        cx.events().emit(vec![3]),
        Err(gpuio_extension_sdk::Error::Stale)
    );
    let next = RenderContext::new(Source::new(3, 8).unwrap(), lease.sink());
    drop(lease);
    assert_eq!(
        next.events().emit(vec![4]),
        Err(gpuio_extension_sdk::Error::Closed)
    );
}
