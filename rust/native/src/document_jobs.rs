//! Bounded, latest-snapshot document jobs. No GPUI callbacks occur in Work::run.
//! Mounted views own handles; dropping a handle cancels queued/running work.
use crate::{document_highlight as highlight, document_store::Snapshot};
use gpui_base::text::PreparedMarkdown;
use gpuio_protocol::document::Mode;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::{Rc, Weak},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Instant,
};

pub const MAX_WORKERS: usize = 2;
pub const MAX_VIEWS: usize = 128;
pub const MAX_RESERVED_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Closed,
    ResourceLimit,
    Cancelled,
    Parse,
    Highlight,
    Profile(
        gpuio_protocol::document_profile::Stage,
        gpuio_document_sdk::Error,
    ),
}

pub struct Charge {
    used: Arc<AtomicUsize>,
    bytes: usize,
}
impl Charge {
    fn reduce_to(&mut self, bytes: usize) -> Result<(), Error> {
        if bytes > self.bytes {
            return Err(Error::ResourceLimit);
        }
        self.used.fetch_sub(self.bytes - bytes, Ordering::Relaxed);
        self.bytes = bytes;
        Ok(())
    }
}
impl Drop for Charge {
    fn drop(&mut self) {
        self.used.fetch_sub(self.bytes, Ordering::Relaxed);
    }
}
pub enum Prepared {
    Markdown {
        document: Box<PreparedMarkdown>,
        code: BTreeMap<(String, String), crate::document_profile_jobs::Highlights>,
        profile: Option<crate::document_profile_jobs::Prepared>,
    },
    Code(Vec<highlight::Run>),
    Diff {
        document: crate::document_diff::Diff,
        runs: Vec<highlight::Run>,
    },
    Source(Error),
}
pub struct Ready {
    pub prepared: Prepared,
    pub charge: Charge,
    pub search: crate::document_search::Matches,
}
#[derive(Default, Clone, Copy, Debug)]
pub struct Measurements {
    pub queue_us: u128,
    pub configure_us: u128,
    pub parse_us: u128,
    pub highlight_us: u128,
    pub search_us: u128,
    pub source_bytes: usize,
}

#[derive(Clone)]
pub struct Request {
    pub observer: Option<gpui::WindowId>,
    pub snapshot: Arc<Snapshot>,
    pub mode: Mode,
    pub profile: Option<crate::document_profile_jobs::Request>,
    pub markdown_options: gpuio_protocol::document::MarkdownOptions,
    pub dark: bool,
    pub search: String,
}
impl Request {
    fn validate(&self) -> Result<(), Error> {
        if self.profile.is_some() && !matches!(self.mode, Mode::Markdown | Mode::Html) {
            return Err(Error::Profile(
                gpuio_protocol::document_profile::Stage::Configure,
                gpuio_document_sdk::Error::InvalidProperties,
            ));
        }
        Ok(())
    }
    fn equal(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.snapshot, &other.snapshot)
            && self.profile.as_ref().map(|p| (p.config(), p.observer()))
                == other.profile.as_ref().map(|p| (p.config(), p.observer()))
            && self.mode == other.mode
            && self.markdown_options == other.markdown_options
            && self.dark == other.dark
            && self.search == other.search
    }
}
struct Entry {
    closed: bool,
    queued_at: Instant,
    serial: u64,
    request: Request,
    running: Option<u64>,
    completed: u64,
    cancel: Arc<AtomicBool>,
    ready: Option<Result<Ready, Error>>,
}
impl Drop for Entry {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
#[derive(Clone)]
pub struct Handle(Rc<RefCell<Entry>>);
impl Handle {
    pub fn update(&self, request: Request) -> Result<bool, Error> {
        let mut entry = self.0.borrow_mut();
        if entry.closed {
            return Err(Error::Closed);
        }
        request.validate()?;
        if entry.request.equal(&request) {
            return Ok(false);
        }
        let serial = entry.serial.checked_add(1).ok_or(Error::ResourceLimit)?;
        entry.cancel.store(true, Ordering::Relaxed);
        entry.cancel = Arc::new(AtomicBool::new(false));
        entry.queued_at = Instant::now();
        entry.serial = serial;
        entry.request = request;
        entry.ready = None;
        Ok(true)
    }
    pub fn take_ready(&self) -> Option<Result<Ready, Error>> {
        self.0.borrow_mut().ready.take()
    }
    pub fn is_pending(&self) -> bool {
        let entry = self.0.borrow();
        entry.serial != entry.completed
    }
}

pub struct Work {
    queued_at: Instant,
    id: u64,
    serial: u64,
    request: Request,
    cancel: Arc<AtomicBool>,
    charge: Charge,
}
pub struct Completion {
    pub observer: Option<gpui::WindowId>,
    id: u64,
    serial: u64,
    pub result: Result<Ready, Error>,
    pub measurements: Measurements,
}
impl Work {
    pub fn run(mut self) -> Completion {
        let mut measurements = Measurements {
            queue_us: self.queued_at.elapsed().as_micros(),
            source_bytes: self.request.snapshot.text.len(),
            ..Default::default()
        };
        let cancelled = || self.cancel.load(Ordering::Relaxed);
        let search_started = Instant::now();
        let search = crate::document_search::find(
            &self.request.snapshot.text,
            &self.request.search,
            cancelled,
        );
        measurements.search_us = search_started.elapsed().as_micros();
        let prepared = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if cancelled() {
                return Err(Error::Cancelled);
            }
            let max = match self.request.mode {
                Mode::Markdown | Mode::Html => 65536,
                _ => highlight::MAX_HIGHLIGHT_BYTES,
            };
            if self.request.snapshot.text.len() > max {
                return Err(Error::ResourceLimit);
            }
            let text = self.request.snapshot.text.to_string();
            let value = match &self.request.mode {
                Mode::Markdown | Mode::Html => {
                    let start = Instant::now();
                    let extensions = crate::document_markdown::extensions_with_options(
                        Default::default(),
                        self.request.markdown_options,
                    );
                    let (document, profile, mut custom) =
                        if let Some(profile) = &self.request.profile {
                            let (prepared, configure_time) = profile.prepare(
                                &text,
                                matches!(self.request.mode, Mode::Html),
                                extensions,
                                self.serial,
                                self.request.dark,
                                self.cancel.clone(),
                            )?;
                            measurements.configure_us = configure_time.as_micros();
                            measurements.parse_us = prepared.timings.parse.as_micros();
                            measurements.highlight_us = prepared.timings.highlight.as_micros();
                            let retained_units = profile.retained_units(&prepared);
                            (
                                prepared.document,
                                Some(crate::document_profile_jobs::Prepared {
                                    profile: prepared.profile,
                                    parser_epoch: prepared.parser_epoch,
                                    descriptor: profile.descriptor(),
                                    retained_units,
                                }),
                                prepared.highlights,
                            )
                        } else {
                            let document = if matches!(self.request.mode, Mode::Html) {
                                PreparedMarkdown::parse_html(
                                    &text,
                                    extensions,
                                    crate::document_markdown::html_image,
                                )
                            } else {
                                PreparedMarkdown::parse(&text, extensions)
                            }
                            .map_err(|_| Error::Parse)?;
                            measurements.parse_us = start.elapsed().as_micros();
                            (document, None, BTreeMap::new())
                        };
                    let start = Instant::now();
                    let mut code = BTreeMap::new();
                    let mut runs = 0;
                    for block in document.code_blocks() {
                        if cancelled() {
                            return Err(Error::Cancelled);
                        }
                        let language = block
                            .lang()
                            .map_or_else(|| "txt".to_string(), |s| s.to_string());
                        let text = block.code().to_string();
                        let key = (language, text);
                        // Identical blocks share one immutable highlight result.
                        // Do not consume a custom result then overwrite it with
                        // the default highlighter on a duplicate block.
                        if code.contains_key(&key) {
                            continue;
                        }
                        let styles = if let Some(styles) = custom.remove(&key) {
                            crate::document_profile_jobs::Highlights::Profile(styles)
                        } else {
                            crate::document_profile_jobs::Highlights::Native(
                                highlight::highlight(&key.1, &key.0, self.request.dark, cancelled)
                                    .map_err(|_| Error::Highlight)?,
                            )
                        };
                        runs += styles.len();
                        if runs > highlight::MAX_RUNS {
                            return Err(Error::ResourceLimit);
                        }
                        code.insert(key, styles);
                    }
                    measurements.highlight_us += start.elapsed().as_micros();
                    Prepared::Markdown {
                        document: Box::new(document),
                        code,
                        profile,
                    }
                }
                Mode::Code(language) => {
                    let start = Instant::now();
                    let runs = highlight::highlight(&text, language, self.request.dark, cancelled)
                        .map_err(|error| match error {
                            highlight::Error::Cancelled => Error::Cancelled,
                            highlight::Error::Limit => Error::ResourceLimit,
                            highlight::Error::Grammar => Error::Highlight,
                        })?;
                    measurements.highlight_us = start.elapsed().as_micros();
                    Prepared::Code(runs)
                }
                Mode::Diff => {
                    let start = Instant::now();
                    let diff = crate::document_diff::parse(&text, cancelled).ok_or_else(|| {
                        if cancelled() {
                            Error::Cancelled
                        } else {
                            Error::ResourceLimit
                        }
                    })?;
                    measurements.parse_us = start.elapsed().as_micros();
                    let start = Instant::now();
                    let runs = match crate::document_diff_syntax::highlight(
                        &text,
                        &diff,
                        self.request.dark,
                        cancelled,
                    ) {
                        Ok(runs) => runs,
                        Err(highlight::Error::Cancelled) => return Err(Error::Cancelled),
                        // Syntax is optional; keep complete diff semantics on a
                        // grammar/work-limit failure, never a colored prefix.
                        Err(highlight::Error::Limit | highlight::Error::Grammar) => {
                            crate::document_diff::highlights(&diff, self.request.dark)
                        }
                    };
                    measurements.highlight_us = start.elapsed().as_micros();
                    Prepared::Diff {
                        runs,
                        document: diff,
                    }
                }
            };
            if cancelled() {
                Err(Error::Cancelled)
            } else {
                Ok(value)
            }
        }))
        .unwrap_or(Err(Error::Parse));
        let prepared = match prepared {
            Err(Error::ResourceLimit | Error::Parse | Error::Highlight | Error::Profile(..)) => {
                Ok(Prepared::Source(prepared.err().unwrap()))
            }
            other => other,
        };
        // Workers need worst-case capacity. Published pictures keep only a
        // checked allowance for retained metadata, vector capacities and declared
        // opaque state; otherwise a handful of short messages exhaust the pool.
        let prepared = prepared.map(|prepared| {
            if self.request.profile.is_none()
                && !matches!(self.request.mode, Mode::Markdown | Mode::Html)
            {
                return prepared;
            }
            let base = 4096
                + self
                    .request
                    .snapshot
                    .text
                    .len()
                    .min(highlight::MAX_HIGHLIGHT_BYTES)
                    * 32;
            let extra = match &prepared {
                Prepared::Markdown {
                    profile: Some(profile),
                    code,
                    ..
                } => code
                    .iter()
                    .fold(profile.retained_units, |bytes, ((language, text), runs)| {
                        bytes
                            .saturating_add(language.capacity())
                            .saturating_add(text.capacity())
                            .saturating_add(256)
                            .saturating_add(runs.retained_units())
                    }),
                _ => 0,
            };
            let rendered = match &prepared {
                Prepared::Markdown { document, .. } => document.rendered_text().retained_units(),
                _ => 0,
            };
            if self
                .charge
                .reduce_to(base.saturating_add(extra).saturating_add(rendered))
                .is_err()
            {
                // Trusted hooks can return excessive backing capacity even when
                // their logical output length is small. Reject the whole result.
                drop(prepared);
                self.charge
                    .reduce_to(base)
                    .expect("base was reserved before work");
                Prepared::Source(if self.request.profile.is_some() {
                    Error::Profile(
                        gpuio_protocol::document_profile::Stage::Parse,
                        gpuio_document_sdk::Error::LimitExceeded,
                    )
                } else {
                    Error::ResourceLimit
                })
            } else {
                prepared
            }
        });
        Completion {
            observer: self.request.observer,
            id: self.id,
            serial: self.serial,
            result: prepared.and_then(|prepared| {
                Ok(Ready {
                    prepared,
                    charge: self.charge,
                    search: search.ok_or(Error::Cancelled)?,
                })
            }),
            measurements,
        }
    }
}

#[derive(Default)]
pub struct Pool {
    entries: BTreeMap<u64, Weak<RefCell<Entry>>>,
    running: BTreeMap<u64, Arc<AtomicBool>>,
    next: u64,
    cursor: u64,
    notifications: Vec<gpui::WindowId>,
    reserved: Arc<AtomicUsize>,
    closed: bool,
    pub totals: Measurements,
    pub peak_workers: usize,
    pub discarded: usize,
    pub completed: usize,
    pub peak_reserved_bytes: usize,
}
impl Pool {
    /// Reserve mounted presentation storage in the same budget as parser work.
    /// Call before allocating; dropping the charge releases these admission units.
    pub fn reserve(&mut self, bytes: usize) -> Result<Charge, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        let reserved = self.reserved.load(Ordering::Relaxed);
        if bytes > MAX_RESERVED_BYTES - reserved {
            return Err(Error::ResourceLimit);
        }
        self.reserved.fetch_add(bytes, Ordering::Relaxed);
        self.peak_reserved_bytes = self.peak_reserved_bytes.max(reserved + bytes);
        Ok(Charge {
            used: self.reserved.clone(),
            bytes,
        })
    }
    pub fn request(&mut self, request: Request) -> Result<Handle, Error> {
        if self.closed {
            return Err(Error::Closed);
        }
        request.validate()?;
        self.entries.retain(|_, entry| entry.strong_count() > 0);
        if self.entries.len() >= MAX_VIEWS {
            return Err(Error::ResourceLimit);
        }
        self.next = self.next.checked_add(1).ok_or(Error::ResourceLimit)?;
        let entry = Rc::new(RefCell::new(Entry {
            closed: false,
            queued_at: Instant::now(),
            serial: 1,
            request,
            running: None,
            completed: 0,
            cancel: Arc::new(AtomicBool::new(false)),
            ready: None,
        }));
        self.entries.insert(self.next, Rc::downgrade(&entry));
        Ok(Handle(entry))
    }
    pub fn next_work(&mut self) -> Option<Work> {
        if self.closed || self.running.len() >= MAX_WORKERS {
            return None;
        }
        self.entries.retain(|_, entry| entry.strong_count() > 0);
        let ids: Vec<_> = self.entries.keys().copied().collect();
        for id in ids
            .iter()
            .filter(|id| **id > self.cursor)
            .chain(ids.iter().filter(|id| **id <= self.cursor))
        {
            let entry = self.entries[id].upgrade()?;
            let mut entry = entry.borrow_mut();
            if entry.running.is_some() || entry.serial == entry.completed {
                continue;
            }
            // No unpublished source can supply the positive provenance required
            // by profile callbacks. Wait for a source update without invoking hooks.
            if entry.request.profile.is_some() && entry.request.snapshot.revision == 0 {
                continue;
            }
            // Conservative work/cache units, distinct from measured allocator RSS.
            let source_bytes = entry.request.snapshot.text.len();
            let bytes = if matches!(entry.request.mode, Mode::Diff) {
                crate::document_diff_syntax::work_units(source_bytes)
            } else {
                4096 + source_bytes.min(highlight::MAX_HIGHLIGHT_BYTES) * 32
            };
            // Rich preparations add bounded selection metadata. Reserve its
            // maximum during work, then retain only the measured allowance.
            let rendered = if matches!(entry.request.mode, Mode::Markdown | Mode::Html) {
                gpui_base::text::RenderedText::max_preparation_units()
            } else {
                0
            };
            let bytes =
                bytes + rendered + entry.request.profile.as_ref().map_or(0, |p| p.work_units());
            let reserved = self.reserved.load(Ordering::Relaxed);
            if bytes > MAX_RESERVED_BYTES - reserved {
                entry.completed = entry.serial;
                entry.ready = Some(Err(Error::ResourceLimit));
                if let Some(window) = entry.request.observer {
                    self.notifications.push(window);
                }
                continue;
            }
            self.reserved.fetch_add(bytes, Ordering::Relaxed);
            self.peak_reserved_bytes = self.peak_reserved_bytes.max(reserved + bytes);
            let charge = Charge {
                used: self.reserved.clone(),
                bytes,
            };
            entry.running = Some(entry.serial);
            self.running.insert(*id, entry.cancel.clone());
            self.peak_workers = self.peak_workers.max(self.running.len());
            let work = Work {
                queued_at: entry.queued_at,
                id: *id,
                serial: entry.serial,
                request: entry.request.clone(),
                cancel: entry.cancel.clone(),
                charge,
            };
            self.cursor = *id;
            return Some(work);
        }
        None
    }
    pub fn complete(&mut self, completion: Completion) {
        self.running.remove(&completion.id);
        self.completed += 1;
        self.totals.queue_us = self
            .totals
            .queue_us
            .saturating_add(completion.measurements.queue_us);
        self.totals.configure_us = self
            .totals
            .configure_us
            .saturating_add(completion.measurements.configure_us);
        self.totals.parse_us = self
            .totals
            .parse_us
            .saturating_add(completion.measurements.parse_us);
        self.totals.highlight_us = self
            .totals
            .highlight_us
            .saturating_add(completion.measurements.highlight_us);
        self.totals.search_us = self
            .totals
            .search_us
            .saturating_add(completion.measurements.search_us);
        self.totals.source_bytes = self
            .totals
            .source_bytes
            .saturating_add(completion.measurements.source_bytes);
        let Some(entry) = self.entries.get(&completion.id).and_then(Weak::upgrade) else {
            self.discarded += 1;
            return;
        };
        let mut entry = entry.borrow_mut();
        entry.running = None;
        if self.closed || entry.serial != completion.serial {
            self.discarded += 1;
            return;
        }
        entry.completed = completion.serial;
        entry.ready = Some(completion.result);
    }
    pub fn take_notifications(&mut self) -> Vec<gpui::WindowId> {
        std::mem::take(&mut self.notifications)
    }
    pub fn reserved_bytes(&self) -> usize {
        self.reserved.load(Ordering::Relaxed)
    }
    pub fn close(&mut self) {
        self.closed = true;
        for cancel in self.running.values() {
            cancel.store(true, Ordering::Relaxed);
        }
        for entry in self.entries.values().filter_map(Weak::upgrade) {
            let mut entry = entry.borrow_mut();
            entry.closed = true;
            entry.cancel.store(true, Ordering::Relaxed);
            entry.ready = None;
            entry.completed = entry.serial;
        }
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document_store::Store;

    #[test]
    fn presentation_reservations_share_the_work_budget_and_release_on_drop() {
        let mut pool = Pool::default();
        let first = pool.reserve(MAX_RESERVED_BYTES - 8).unwrap();
        assert!(matches!(pool.reserve(9), Err(Error::ResourceLimit)));
        let final_bytes = pool.reserve(8).unwrap();
        assert_eq!(pool.reserved_bytes(), MAX_RESERVED_BYTES);
        drop(first);
        assert_eq!(pool.reserved_bytes(), 8);
        pool.close();
        assert!(matches!(pool.reserve(1), Err(Error::Closed)));
        drop(final_bytes);
        assert_eq!(pool.reserved_bytes(), 0);
    }

    use gpuio_protocol::document::{Status, Update};

    pub(super) fn request(text: &str) -> Request {
        let mut store = Store::default();
        let id = store.create().unwrap();
        store
            .begin(Update {
                id,
                base: 0,
                revision: 1,
                generation: 1,
                from_byte: 0,
                suffix_bytes: text.len() as i64,
                status: Status::Complete,
            })
            .unwrap();
        for (i, bytes) in text.as_bytes().chunks(262144).enumerate() {
            store.chunk(id, 1, i * 262144, bytes).unwrap();
        }
        store.publish(id, 1).unwrap();
        Request {
            observer: None,
            snapshot: store.acquire(id).unwrap().snapshot(),
            mode: Mode::Markdown,
            profile: None,
            markdown_options: Default::default(),
            dark: true,
            search: String::new(),
        }
    }

    #[test]
    fn rendered_text_work_reservation_shrinks_and_retires_with_prepared_result() {
        let mut pool = Pool::default();
        let handle = pool.request(request("# Hi\n\nWorld 世界")).unwrap();
        let work = pool.next_work().unwrap();
        let maximum = gpui_base::text::RenderedText::max_preparation_units();
        let during = pool.reserved_bytes();
        assert!(during >= maximum);
        pool.complete(work.run());
        let ready = handle.take_ready().unwrap().unwrap();
        let Prepared::Markdown { document, .. } = &ready.prepared else {
            panic!("rich text")
        };
        let projection = document.rendered_text();
        assert!(projection.retained_units() < maximum);
        assert_eq!(
            during - pool.reserved_bytes(),
            maximum - projection.retained_units()
        );
        drop(handle);
        assert!(
            pool.reserved_bytes() > 0,
            "the published result still owns its charge"
        );
        drop(ready);
        assert_eq!(pool.reserved_bytes(), 0);
        assert_eq!(projection.text(), "Hi\nWorld 世界\n");
        assert!(projection.selected_fragment_ranges().is_empty());
    }

    #[test]
    fn markdown_options_supersede_work_without_republishing_source() {
        use gpuio_protocol::document::{Frontmatter, MarkdownOptions};
        let source = "---\nname: Native 世界\n---\n\n<Card>Child **text**</Card>\n\n{1 + 2}\n";
        let mut pool = Pool::default();
        let mut request = request(source);
        let original = request.snapshot.clone();
        let handle = pool.request(request.clone()).unwrap();
        let stale = pool.next_work().unwrap();
        request.markdown_options = MarkdownOptions {
            frontmatter: Frontmatter::CodeBlock,
            mdx: true,
        };
        assert!(handle.update(request.clone()).unwrap());
        assert!(!handle.update(request.clone()).unwrap());
        pool.complete(stale.run());
        assert!(handle.take_ready().is_none());
        let work = pool.next_work().unwrap();
        pool.complete(work.run());
        let ready = handle.take_ready().unwrap().unwrap();
        let Prepared::Markdown { document, code, .. } = &ready.prepared else {
            panic!("rich Markdown");
        };
        assert_eq!(document.source().as_ref(), source);
        assert!(document.plain_text().contains("Child text"));
        assert!(!document.plain_text().contains("<Card>"));
        assert!(
            code.keys()
                .any(|(language, text)| language == "yml" && text.contains("Native 世界"))
        );
        assert!(
            code.keys()
                .any(|(language, text)| language == "mdx" && text == "1 + 2")
        );
        assert!(Arc::ptr_eq(&original, &request.snapshot));
        drop(ready);
        request.markdown_options = Default::default();
        handle.update(request.clone()).unwrap();
        let work = pool.next_work().unwrap();
        pool.complete(work.run());
        let ready = handle.take_ready().unwrap().unwrap();
        let Prepared::Markdown { document, code, .. } = &ready.prepared else {
            panic!("default Markdown");
        };
        assert!(document.plain_text().contains("<Card>"));
        assert!(code.is_empty());
        drop(ready);
        drop(handle);
        assert_eq!(pool.reserved_bytes(), 0);
    }

    #[test]
    fn malformed_mdx_falls_back_and_releases_worker_reservations() {
        let mut pool = Pool::default();
        let mut request = request("<Card>never closed");
        request.markdown_options.mdx = true;
        let handle = pool.request(request).unwrap();
        let work = pool.next_work().unwrap();
        pool.complete(work.run());
        let ready = handle.take_ready().unwrap().unwrap();
        assert!(matches!(ready.prepared, Prepared::Source(Error::Parse)));
        drop(ready);
        drop(handle);
        assert_eq!(pool.reserved_bytes(), 0);
    }

    #[test]
    fn html_reader_uses_bounded_jobs_and_rejects_stale_mode_results() {
        let mut pool = Pool::default();
        let mut request = request("<h1>Hello 世界</h1><p>Second &amp; final.</p>");
        request.mode = Mode::Html;
        let handle = pool.request(request.clone()).unwrap();
        let stale = pool.next_work().unwrap();
        request.mode = Mode::Markdown;
        handle.update(request.clone()).unwrap();
        pool.complete(stale.run());
        assert!(handle.take_ready().is_none());
        let current = pool.next_work().unwrap();
        pool.complete(current.run());
        let current = handle.take_ready().unwrap().unwrap();
        let Prepared::Markdown { document, .. } = current.prepared else {
            panic!("rich literal Markdown");
        };
        assert!(document.plain_text().contains("<h1>"));
        drop(current.charge);
        for source in [
            "<p>Hello 世界</p>".to_owned(),
            "<div>".repeat(100),
            "<p>x</p>\n".repeat(257),
            "x".repeat(65537),
        ] {
            let mut request = super::tests::request(&source);
            request.mode = Mode::Html;
            handle.update(request).unwrap();
            let work = pool.next_work().unwrap();
            pool.complete(work.run());
            let ready = handle.take_ready().unwrap().unwrap();
            if source == "<p>Hello 世界</p>" {
                let Prepared::Markdown { document, .. } = ready.prepared else {
                    panic!("rich HTML");
                };
                assert_eq!(document.source().as_ref(), source);
                assert!(document.plain_text().contains("Hello 世界"));
            } else {
                assert!(matches!(ready.prepared, Prepared::Source(_)));
            }
            drop(ready.charge);
        }
        drop(handle);
        assert_eq!(pool.reserved_bytes(), 0);
    }

    #[test]
    fn diff_syntax_limit_keeps_complete_diff_controls_and_colors() {
        let text = format!(
            "--- /dev/null\n+++ b/many.ml\n@@ -0,0 +1,3000 @@\n{}",
            "+let value = 42 (* note *)\n".repeat(3000)
        );
        let mut request = request(&text);
        request.mode = Mode::Diff;
        let mut pool = Pool::default();
        let handle = pool.request(request).unwrap();
        let work = pool.next_work().unwrap();
        pool.complete(work.run());
        let ready = handle.take_ready().unwrap().unwrap();
        let Prepared::Diff { document, runs } = ready.prepared else {
            panic!("syntax limits must not disable diff controls")
        };
        assert_eq!(document.files[0].added, 3000);
        assert_eq!(runs, crate::document_diff::highlights(&document, true));
        assert_eq!(runs.last().unwrap().bytes.end, text.len());
        drop(ready.charge);
        assert_eq!(pool.reserved_bytes(), 0);
    }

    #[test]
    fn measurement_report() {
        let mut pool = Pool::default();
        let specifications = [
            ("**Streaming** λ paragraph.\n\n".repeat(100), Mode::Markdown),
            (
                "let answer = 42 (* λ *)\n".repeat(1000),
                Mode::Code("ml".into()),
            ),
            (
                "--- a/file\n+++ b/file\n@@ -1 +1 @@\n-old\n+new\n".repeat(100),
                Mode::Diff,
            ),
            (format!("{}needle", "x".repeat(1024 * 1024)), Mode::Markdown),
        ];
        let handles: Vec<_> = specifications
            .iter()
            .map(|(text, mode)| {
                let mut value = request(text);
                value.mode = mode.clone();
                value.search = "needle".into();
                pool.request(value).unwrap()
            })
            .collect();
        loop {
            let work: Vec<_> = (0..MAX_WORKERS).filter_map(|_| pool.next_work()).collect();
            if work.is_empty() {
                break;
            }
            for work in work {
                pool.complete(work.run());
            }
        }
        assert_eq!(pool.peak_workers, 2);
        for handle in &handles {
            assert!(handle.take_ready().unwrap().is_ok());
        }
        assert_eq!(pool.reserved_bytes(), 0);
        eprintln!(
            "GPUIO_DOCUMENT_WORKER_METRICS {:?}; completed={}; peak_workers={}; peak_reserved_bytes={}",
            pool.totals, pool.completed, pool.peak_workers, pool.peak_reserved_bytes
        );
    }

    #[test]
    fn concurrency_and_stale_completion_are_bounded() {
        let mut pool = Pool::default();
        let a = pool.request(request("old")).unwrap();
        let b = pool.request(request("second")).unwrap();
        let c = pool.request(request("third")).unwrap();
        let first = pool.next_work().unwrap();
        let second = pool.next_work().unwrap();
        assert!(pool.next_work().is_none());
        for i in 0..1000 {
            a.update(request(&format!("latest {i}"))).unwrap();
        }
        pool.complete(first.run());
        assert!(a.take_ready().is_none());
        assert_eq!(pool.discarded, 1);
        pool.complete(second.run());
        assert!(b.take_ready().unwrap().is_ok());
        while let Some(work) = pool.next_work() {
            pool.complete(work.run());
        }
        match a.take_ready().unwrap().unwrap().prepared {
            Prepared::Markdown { document, .. } => {
                assert_eq!(document.source().as_ref(), "latest 999")
            }
            _ => panic!("wrong prepared kind"),
        }
        assert!(c.take_ready().unwrap().is_ok());
        assert_eq!(pool.reserved_bytes(), 0);
        assert_eq!(pool.completed, 4);
    }

    #[test]
    fn dropped_views_cancel_workers_and_close_rejects_updates() {
        let mut pool = Pool::default();
        let handle = pool.request(request("hello")).unwrap();
        let work = pool.next_work().unwrap();
        drop(handle);
        let completion = work.run();
        assert!(matches!(completion.result, Err(Error::Cancelled)));
        pool.complete(completion);
        assert_eq!(pool.reserved_bytes(), 0);
        let handle = pool.request(request("next")).unwrap();
        let work = pool.next_work().unwrap();
        pool.close();
        assert_eq!(handle.update(request("closed")), Err(Error::Closed));
        pool.complete(work.run());
        assert_eq!(pool.reserved_bytes(), 0);
    }

    #[test]
    fn per_document_and_aggregate_admission_keep_plain_fallback_explicit() {
        let mut pool = Pool::default();
        let large = pool.request(request(&"x".repeat(65537))).unwrap();
        let work = pool.next_work().unwrap();
        pool.complete(work.run());
        assert!(matches!(
            large.take_ready(),
            Some(Ok(Ready {
                prepared: Prepared::Source(Error::ResourceLimit),
                ..
            }))
        ));
        assert_eq!(pool.reserved_bytes(), 0);
        let text = format!("{}\n", "x".repeat(15000)).repeat(16);
        let request = Request {
            mode: Mode::Code("txt".into()),
            ..request(&text)
        };
        let handles: Vec<_> = (0..12)
            .map(|_| pool.request(request.clone()).unwrap())
            .collect();
        let mut retained = Vec::new();
        for handle in &handles {
            while handle.is_pending() {
                if let Some(work) = pool.next_work() {
                    pool.complete(work.run());
                } else {
                    break;
                }
            }
            if let Some(Ok(ready)) = handle.take_ready() {
                retained.push(ready);
            }
        }
        assert!(pool.reserved_bytes() <= MAX_RESERVED_BYTES);
        assert!(!retained.is_empty() && retained.len() < handles.len());
        drop(retained);
        drop(handles);
        drop(large);
        pool.close();
        assert_eq!(pool.reserved_bytes(), 0);
    }
}

#[cfg(test)]
#[path = "document_profile_jobs_test.rs"]
mod profile_tests;
