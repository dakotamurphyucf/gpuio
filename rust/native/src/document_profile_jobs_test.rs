use super::*;
use crate::document_profile_jobs::{Highlights, Request as ProfileRequest};
use gpuio_document_sdk as sdk;
use gpuio_protocol::{
    document::Mode,
    document_profile::{Config, Instance, Stage},
    extension::{Payload, Schema},
};
use std::sync::atomic::AtomicUsize;
const FINGERPRINT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
type Pause = (std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>);
#[derive(Default)]
struct Counts {
    configurations: AtomicUsize,
    alive: AtomicUsize,
    pause: std::sync::Mutex<Option<Pause>>,
}
struct Factory(Arc<Counts>);
impl sdk::Factory for Factory {
    fn descriptor(&self) -> sdk::Descriptor {
        sdk::Descriptor {
            name: "test.worker",
            version: 1,
            fingerprint: FINGERPRINT,
            sdk_version: sdk::SDK_VERSION,
            gpui_revision: sdk::GPUI_REVISION,
            base_revision: sdk::BASE_REVISION,
            max_properties: 1,
            max_event: 16,
            max_retained_bytes: 4096,
        }
    }
    fn validate_properties(&self, bytes: &[u8]) -> Result<(), sdk::Error> {
        if bytes.len() == 1 && bytes[0] <= 5 {
            Ok(())
        } else {
            Err(sdk::Error::InvalidProperties)
        }
    }
    fn configure(
        &self,
        bytes: &[u8],
        cx: &sdk::PrepareContext<'_>,
    ) -> Result<sdk::Profile, sdk::Error> {
        cx.check()?;
        self.0.configurations.fetch_add(1, Ordering::SeqCst);
        let pause = self.0.pause.lock().unwrap().take();
        if let Some((entered, resume)) = pause {
            entered.send(()).unwrap();
            resume
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
        }
        assert_ne!(bytes, [1], "configuration panic");
        self.0.alive.fetch_add(1, Ordering::SeqCst);
        Ok(sdk::Profile::new().with_highlighter(Arc::new(Paint {
            mode: bytes[0],
            counts: self.0.clone(),
        })))
    }
}
struct Paint {
    mode: u8,
    counts: Arc<Counts>,
}
impl Drop for Paint {
    fn drop(&mut self) {
        self.counts.alive.fetch_sub(1, Ordering::SeqCst);
    }
}
impl sdk::Highlighter for Paint {
    fn highlight(
        &self,
        code: &sdk::Code<'_>,
        cx: &sdk::PrepareContext<'_>,
    ) -> Result<Vec<sdk::Highlight>, sdk::Error> {
        cx.check()?;
        assert_ne!(self.mode, 2, "highlighter panic");
        if self.mode == 4 {
            return Ok(vec![]);
        }
        let mut style = sdk::Highlight::new(
            if self.mode == 3 {
                1..2
            } else {
                0..code.text().len()
            },
            [10, 20, 30],
        );
        style.weight = 537;
        style.strikethrough = true;
        style.italic = true;
        style.underline = true;
        style.background = Some([30, 20, 10]);
        if self.mode == 5 {
            let mut oversized = Vec::with_capacity(sdk::MAX_RUNS * 8);
            oversized.push(style);
            Ok(oversized)
        } else {
            Ok(vec![style])
        }
    }
}
fn profile(counts: &Arc<Counts>, epoch: i64, mode: u8) -> ProfileRequest {
    let registry =
        sdk::Registry::new([Arc::new(Factory(counts.clone())) as Arc<dyn sdk::Factory>]).unwrap();
    let binding = registry
        .bind("test.worker", 1, FINGERPRINT, &[mode])
        .unwrap();
    ProfileRequest::for_test(
        Arc::new(Config {
            epoch,
            instance: Some(Instance {
                schema: Schema {
                    name: "test.worker".into(),
                    version: 1,
                    fingerprint: FINGERPRINT.into(),
                },
                generation: 1,
                properties: Payload(vec![mode]),
            }),
        }),
        binding,
    )
}
fn request(counts: &Arc<Counts>, epoch: i64, mode: u8) -> Request {
    Request {
        profile: Some(profile(counts, epoch, mode)),
        ..super::tests::request("```txt\n世界\n```\n\n```txt\n世界\n```\n")
    }
}
#[test]
fn worker_profiles_keep_complete_styles_duplicate_blocks_and_budget_until_drop() {
    let counts = Arc::new(Counts::default());
    let mut pool = Pool::default();
    let request = request(&counts, 1, 0);
    let expected =
        4096 + request.snapshot.text.len() * 32 + request.profile.as_ref().unwrap().work_units();
    let handle = pool.request(request).unwrap();
    assert_eq!(counts.configurations.load(Ordering::SeqCst), 0);
    let work = pool.next_work().unwrap();
    assert_eq!(
        pool.reserved_bytes(),
        expected + gpui_base::text::RenderedText::max_preparation_units()
    );
    assert_eq!(counts.configurations.load(Ordering::SeqCst), 0);
    // Real separate worker thread; native resources/callbacks are unavailable.
    pool.complete(std::thread::spawn(move || work.run()).join().unwrap());
    let ready = handle.take_ready().unwrap().unwrap();
    let Prepared::Markdown {
        document,
        code,
        profile,
    } = &ready.prepared
    else {
        panic!("profile Markdown")
    };
    assert!(document.plain_text().contains("世界"));
    assert_eq!(code.len(), 1);
    let highlights = code.values().next().unwrap();
    assert!(matches!(highlights, Highlights::Profile(_)));
    let styles = highlights.styles();
    assert_eq!(styles[0].0, 0..6);
    assert_eq!(styles[0].1.font_weight, Some(gpui::FontWeight(537.)));
    assert!(styles[0].1.strikethrough.is_some());
    assert!(styles[0].1.underline.is_some());
    assert!(styles[0].1.background_color.is_some());
    assert_eq!(profile.as_ref().unwrap().parser_epoch, 1);
    assert_eq!(counts.alive.load(Ordering::SeqCst), 1);
    assert!(pool.reserved_bytes() < expected / 8);
    assert!(pool.reserved_bytes() >= 4096 + 4096);
    drop(ready);
    assert_eq!(counts.alive.load(Ordering::SeqCst), 0);
    assert_eq!(pool.reserved_bytes(), 0);
}
#[test]
fn unchanged_requests_are_silent_and_profile_epochs_cancel_obsolete_work() {
    let counts = Arc::new(Counts::default());
    let mut pool = Pool::default();
    let mut request = request(&counts, 1, 0);
    let handle = pool.request(request.clone()).unwrap();
    assert!(!handle.update(request.clone()).unwrap());
    let obsolete = pool.next_work().unwrap();
    request.profile = Some(profile(&counts, 2, 4));
    assert!(handle.update(request).unwrap());
    pool.complete(obsolete.run());
    assert!(handle.take_ready().is_none());
    assert_eq!(counts.configurations.load(Ordering::SeqCst), 0);
    assert_eq!(pool.reserved_bytes(), 0);
    let current = pool.next_work().unwrap();
    pool.complete(current.run());
    let ready = handle.take_ready().unwrap().unwrap();
    let Prepared::Markdown { profile, code, .. } = &ready.prepared else {
        panic!("Markdown")
    };
    assert_eq!(profile.as_ref().unwrap().parser_epoch, 2);
    assert!(
        code.values()
            .all(|runs| matches!(runs, Highlights::Profile(v) if v.is_empty()))
    );
    drop(ready);
    assert_eq!(counts.alive.load(Ordering::SeqCst), 0);
    assert_eq!(pool.reserved_bytes(), 0);
}
#[test]
fn profile_failures_keep_source_fallback_and_precise_stage_without_live_plugins() {
    let counts = Arc::new(Counts::default());
    for (mode, stage, error) in [
        (1, Stage::Configure, sdk::Error::Panicked),
        (2, Stage::Highlight, sdk::Error::Panicked),
        (3, Stage::Highlight, sdk::Error::InvalidHighlight),
    ] {
        let mut pool = Pool::default();
        let handle = pool.request(request(&counts, 1, mode)).unwrap();
        let work = pool.next_work().unwrap();
        pool.complete(work.run());
        let ready = handle.take_ready().unwrap().unwrap();
        assert!(
            matches!(ready.prepared, Prepared::Source(Error::Profile(s,e)) if s == stage && e == error)
        );
        assert_eq!(counts.alive.load(Ordering::SeqCst), 0);
        drop(ready);
        assert_eq!(pool.reserved_bytes(), 0);
    }
}
#[test]
fn unpublished_and_over_budget_profiles_never_invoke_configuration() {
    let counts = Arc::new(Counts::default());
    let mut pool = Pool::default();
    let mut store = crate::document_store::Store::default();
    let id = store.create().unwrap();
    let mut request = request(&counts, 1, 0);
    request.snapshot = store.acquire(id).unwrap().snapshot();
    let handle = pool.request(request.clone()).unwrap();
    assert!(pool.next_work().is_none());
    assert!(handle.take_ready().is_none());
    assert!(handle.is_pending());
    assert_eq!(pool.reserved_bytes(), 0);
    assert_eq!(counts.configurations.load(Ordering::SeqCst), 0);
    request.snapshot = super::tests::request("published").snapshot;
    handle.update(request).unwrap();
    let charge = pool.reserve(MAX_RESERVED_BYTES - 1).unwrap();
    assert!(pool.next_work().is_none());
    assert!(matches!(
        handle.take_ready(),
        Some(Err(Error::ResourceLimit))
    ));
    assert_eq!(counts.configurations.load(Ordering::SeqCst), 0);
    drop(charge);
    drop(handle);
    assert_eq!(pool.reserved_bytes(), 0);
}
#[test]
fn html_profiles_keep_html_parsing_and_cancelled_retention_releases() {
    let counts = Arc::new(Counts::default());
    let mut pool = Pool::default();
    let request = Request {
        mode: Mode::Html,
        profile: Some(profile(&counts, 1, 0)),
        ..super::tests::request(
            "<p>Reader</p><pre><code>世界</code></pre><img src='https://invalid/' alt='safe'/>",
        )
    };
    let handle = pool.request(request).unwrap();
    let work = pool.next_work().unwrap();
    pool.complete(work.run());
    let ready = handle.take_ready().unwrap().unwrap();
    let Prepared::Markdown {
        document,
        code,
        profile,
    } = &ready.prepared
    else {
        panic!("HTML")
    };
    assert!(document.plain_text().contains("Reader"));
    assert!(document.plain_text().contains("safe"));
    assert!(profile.is_some());
    assert!(code.values().all(|v| matches!(v, Highlights::Profile(_))));
    drop(ready);
    let handle = pool.request(self::request(&counts, 2, 0)).unwrap();
    let work = pool.next_work().unwrap();
    drop(handle);
    pool.complete(work.run());
    assert_eq!(counts.alive.load(Ordering::SeqCst), 0);
    assert_eq!(pool.reserved_bytes(), 0);
}

#[test]
fn invalid_mode_updates_preserve_the_existing_request_and_parser_failures_are_typed() {
    let counts = Arc::new(Counts::default());
    let mut pool = Pool::default();
    let valid = request(&counts, 1, 0);
    let handle = pool.request(valid.clone()).unwrap();
    let mut invalid = valid.clone();
    invalid.mode = Mode::Code("txt".into());
    assert!(matches!(
        pool.request(invalid.clone()),
        Err(Error::Profile(
            Stage::Configure,
            sdk::Error::InvalidProperties
        ))
    ));
    assert_eq!(
        handle.update(invalid),
        Err(Error::Profile(
            Stage::Configure,
            sdk::Error::InvalidProperties
        ))
    );
    assert!(!handle.update(valid).unwrap());
    let mut malformed = Request {
        profile: Some(profile(&counts, 2, 0)),
        ..super::tests::request("<Card>never closed")
    };
    malformed.markdown_options.mdx = true;
    handle.update(malformed).unwrap();
    let work = pool.next_work().unwrap();
    pool.complete(work.run());
    let ready = handle.take_ready().unwrap().unwrap();
    assert!(matches!(
        ready.prepared,
        Prepared::Source(Error::Profile(Stage::Parse, sdk::Error::Parse))
    ));
    drop(ready);
    assert_eq!(counts.alive.load(Ordering::SeqCst), 0);
    assert_eq!(pool.reserved_bytes(), 0);
}

#[test]
fn cancellation_during_configuration_discards_the_new_profile_and_releases_its_charge() {
    use std::sync::mpsc::channel;
    let counts = Arc::new(Counts::default());
    let (entered, started) = channel();
    let (resume, resumed) = channel();
    *counts.pause.lock().unwrap() = Some((entered, resumed));
    let mut pool = Pool::default();
    let mut request = request(&counts, 1, 0);
    let handle = pool.request(request.clone()).unwrap();
    let work = pool.next_work().unwrap();
    let worker = std::thread::spawn(move || work.run());
    started
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    request.profile = Some(profile(&counts, 2, 4));
    handle.update(request).unwrap();
    resume.send(()).unwrap();
    pool.complete(worker.join().unwrap());
    assert!(handle.take_ready().is_none());
    assert_eq!(pool.discarded, 1);
    assert_eq!(counts.alive.load(Ordering::SeqCst), 0);
    assert_eq!(pool.reserved_bytes(), 0);
    let work = pool.next_work().unwrap();
    pool.complete(work.run());
    let ready = handle.take_ready().unwrap().unwrap();
    assert_eq!(counts.configurations.load(Ordering::SeqCst), 2);
    drop(ready);
    assert_eq!(counts.alive.load(Ordering::SeqCst), 0);
    assert_eq!(pool.reserved_bytes(), 0);
}
#[test]
fn many_retained_profiles_share_the_global_budget_before_configuration() {
    let counts = Arc::new(Counts::default());
    let mut pool = Pool::default();
    let request = request(&counts, 1, 0);
    let units =
        4096 + request.snapshot.text.len() * 32 + request.profile.as_ref().unwrap().work_units();
    let handles: Vec<_> = (0..MAX_VIEWS)
        .map(|_| pool.request(request.clone()).unwrap())
        .collect();
    while let Some(work) = pool.next_work() {
        pool.complete(work.run());
    }
    let mut retained = vec![];
    let mut rejected = 0;
    for handle in handles {
        match handle.take_ready().unwrap() {
            Ok(ready) => retained.push(ready),
            Err(Error::ResourceLimit) => rejected += 1,
            Err(error) => panic!("unexpected {error:?}"),
        }
    }
    assert_eq!(retained.len(), MAX_VIEWS);
    assert_eq!(rejected, 0);
    assert_eq!(counts.configurations.load(Ordering::SeqCst), retained.len());
    assert!(pool.reserved_bytes() < retained.len() * units / 8);
    assert!(pool.peak_reserved_bytes <= MAX_RESERVED_BYTES);
    drop(retained);
    assert_eq!(counts.alive.load(Ordering::SeqCst), 0);
    assert_eq!(pool.reserved_bytes(), 0);
}

#[test]
fn excessive_retained_vector_capacity_rejects_the_entire_prepared_profile() {
    let counts = Arc::new(Counts::default());
    let mut pool = Pool::default();
    let request = request(&counts, 1, 5);
    let base = 4096 + request.snapshot.text.len() * 32;
    let handle = pool.request(request).unwrap();
    let work = pool.next_work().unwrap();
    pool.complete(work.run());
    let ready = handle.take_ready().unwrap().unwrap();
    assert!(matches!(
        ready.prepared,
        Prepared::Source(Error::Profile(_, sdk::Error::LimitExceeded))
    ));
    assert_eq!(counts.alive.load(Ordering::SeqCst), 0);
    assert_eq!(pool.reserved_bytes(), base);
    drop(ready);
    assert_eq!(pool.reserved_bytes(), 0);
}
