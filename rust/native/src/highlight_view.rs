//! Mounted scope state, independent of paint order and native worker epochs.
//! Source/config changes retire paint immediately. Weak projection references
//! never keep rejected or retired source outside worker reservation ownership.
use super::View;
use crate::{
    highlight_collect as collect, highlight_host, highlight_jobs as jobs, highlight_paint as paint,
    highlight_projection::{self as projection, Projection, RunKey},
    tree::{Node, Tree},
};
use gpui::{Context, IntoElement, Window, canvas, prelude::*};
use gpuio_protocol::{
    HandlerId, NodeId,
    highlight::{
        Config, Count, Failure, InvalidRange, Limit, Observation, RangeError, State as Outcome,
    },
};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    rc::Rc,
    sync::{Arc, Weak},
};

pub(super) type Shared = Rc<RefCell<State>>;
struct Stamp {
    revision: i64,
    visibility: Rc<()>,
    documents: Rc<()>,
}

#[derive(Clone)]
pub(super) struct DocumentBinding {
    scope: std::rc::Weak<RefCell<State>>,
    key: RunKey,
}
impl DocumentBinding {
    pub(super) fn paint(&self, fragment: u32, source: &projection::Source) -> Option<paint::Paint> {
        let scope = self.scope.upgrade()?;
        let mut state = scope.borrow_mut();
        let jobs::Status::Ready(ready) = state.job.as_ref()?.status() else {
            return None;
        };
        let key = RunKey {
            fragment,
            ..self.key
        };
        if !ready.source().has_source(key, source) {
            return None;
        }
        state.paint(key).map(|(paint, _)| paint)
    }
}
struct CachedPaint {
    ready: Arc<jobs::Ready>,
    config: Arc<Config>,
    paint: paint::Paint,
    geometry: paint::SharedCache,
}
#[derive(Default)]
pub(super) struct State {
    stamp: Option<Stamp>,
    config: Option<Arc<Config>>,
    source: Weak<Projection>,
    collection_error: Option<collect::Error>,
    job: Option<highlight_host::Handle>,
    fallback: Option<Outcome>,
    epoch: i64,
    observed: Option<(HandlerId, Observation)>,
    paints: BTreeMap<RunKey, CachedPaint>,
    painted: bool,
}
fn job_error(error: jobs::Error) -> Outcome {
    match error {
        jobs::Error::Closed => Outcome::Failed(Failure::SourceUnavailable),
        jobs::Error::AdmissionLimit => Outcome::Capacity(Limit::Admission),
        jobs::Error::EpochExhausted => Outcome::Failed(Failure::EpochExhausted),
        jobs::Error::WorkerFailed => Outcome::Failed(Failure::WorkerFailed),
        jobs::Error::Match(projection::Error::WorkLimit) => Outcome::Capacity(Limit::Work),
        jobs::Error::Match(projection::Error::InvalidRange {
            spec_index,
            range_index,
            reason,
        }) => Outcome::InvalidRange(InvalidRange {
            spec_index: spec_index as i64,
            range_index: range_index as i64,
            reason: match reason {
                projection::RangeError::OutOfBounds => RangeError::OutOfBounds,
                projection::RangeError::ScalarBoundary => RangeError::ScalarBoundary,
            },
        }),
        jobs::Error::Match(projection::Error::Cancelled | projection::Error::InvalidConfig) => {
            Outcome::Failed(Failure::WorkerFailed)
        }
    }
}
fn collection_error(error: collect::Error) -> Outcome {
    match error {
        collect::Error::VisitLimit => Outcome::Capacity(Limit::Work),
        collect::Error::Projection(_) => Outcome::Capacity(Limit::Source),
        collect::Error::Document(collect::DocumentError::Pending) => Outcome::Pending,
        collect::Error::MissingScope
        | collect::Error::Document(collect::DocumentError::Unavailable) => {
            Outcome::Failed(Failure::SourceUnavailable)
        }
    }
}
impl State {
    fn prepare(
        &mut self,
        projection: Result<Projection, collect::Error>,
        config: Arc<Config>,
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        if matches!(
            self.fallback,
            Some(Outcome::Failed(Failure::EpochExhausted))
        ) {
            return;
        }
        let same_matcher = self
            .config
            .as_ref()
            .is_some_and(|old| old.same_matchers(&config));
        let old_source = self.source.upgrade();
        let (source, error) = match projection {
            Ok(next) => {
                let source = old_source
                    .as_ref()
                    .filter(|old| old.same_source(&next))
                    .cloned()
                    .unwrap_or_else(|| Arc::new(next));
                (Some(source), None)
            }
            Err(error) => (None, Some(error)),
        };
        let same_source = match (&source, &old_source) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (None, None) => self.collection_error == error,
            _ => false,
        } || (config.0.is_empty()
            && self.config.as_ref().is_some_and(|old| old.0.is_empty()));
        if self.epoch == 0 || !same_matcher || !same_source {
            self.paints.clear();
            let Some(epoch) = self.epoch.checked_add(1) else {
                self.job = None;
                self.source = Weak::new();
                self.fallback = Some(Outcome::Failed(Failure::EpochExhausted));
                return;
            };
            self.epoch = epoch;
        }
        self.config = Some(config.clone());
        self.collection_error = error;
        if let Some(error) = error {
            self.job = None;
            self.source = Weak::new();
            self.paints.clear();
            self.fallback = Some(collection_error(error));
            return;
        }
        if config.0.is_empty() {
            self.job = None;
            self.source = Weak::new();
            self.paints.clear();
            self.fallback = Some(Outcome::Ready(vec![]));
            return;
        }
        let source = source.expect("successful collection");
        self.source = Arc::downgrade(&source);
        let result = if let Some(job) = &self.job {
            job.update(source, config, cx).map(|_| ())
        } else {
            highlight_host::request(source, config, window, cx).map(|job| self.job = Some(job))
        };
        self.fallback = result.err().map(job_error);
        if self.fallback.is_some() {
            self.source = Weak::new();
            self.paints.clear();
        }
    }
    pub(super) fn observation(&self) -> Observation {
        let state = if let Some(fallback) = &self.fallback {
            fallback.clone()
        } else if let Some(job) = &self.job {
            match job.status() {
                jobs::Status::Pending => Outcome::Pending,
                jobs::Status::Failed(error) => job_error(error),
                jobs::Status::Ready(ready) => Outcome::Ready(
                    ready
                        .matches
                        .counts
                        .iter()
                        .map(|c| Count {
                            total: c.total,
                            stored: c.stored,
                        })
                        .collect(),
                ),
            }
        } else {
            Outcome::Pending
        };
        Observation {
            epoch: self.epoch,
            state,
        }
    }
    fn paint(&mut self, key: RunKey) -> Option<(paint::Paint, paint::SharedCache)> {
        if self.fallback.is_some() {
            return None;
        }
        let jobs::Status::Ready(ready) = self.job.as_ref()?.status() else {
            return None;
        };
        let config = self.config.as_ref()?;
        if let Some(cached) = self.paints.get(&key)
            && Arc::ptr_eq(&cached.ready, &ready)
            && Arc::ptr_eq(&cached.config, config)
        {
            return Some((cached.paint.clone(), cached.geometry.clone()));
        }
        let paint = paint::resolve(ready.clone(), key, config);
        if paint.is_empty() {
            self.paints.remove(&key);
            return None;
        }
        let geometry = self
            .paints
            .get(&key)
            .map(|p| p.geometry.clone())
            .unwrap_or_default();
        self.paints.insert(
            key,
            CachedPaint {
                ready,
                config: config.clone(),
                paint: paint.clone(),
                geometry: geometry.clone(),
            },
        );
        Some((paint, geometry))
    }
}
impl View {
    pub(super) fn prepare_highlight(
        &mut self,
        tree: &Tree,
        node: &Node,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Shared {
        let scope = self.highlights.entry(node.id).or_default().clone();
        let visibility = self.focus.borrow().visibility_identity();
        let documents = self.highlight_documents.borrow().clone();
        let mut state = scope.borrow_mut();
        if state.stamp.as_ref().is_none_or(|s| {
            s.revision != tree.revision()
                || !Rc::ptr_eq(&s.visibility, &visibility)
                || !Rc::ptr_eq(&s.documents, &documents)
        }) {
            let focus = self.focus.borrow();
            let projection = collect::collect(
                tree,
                node.id,
                |node| focus.highlight_visible(tree, node.id),
                |node| {
                    if node.document.as_ref().is_some_and(|c| c.source.is_none()) {
                        Ok(vec![])
                    } else {
                        self.documents
                            .get(&node.id)
                            .ok_or(collect::DocumentError::Pending)?
                            .highlight_groups(cx)
                    }
                },
            );
            drop(focus);
            state.prepare(
                projection,
                node.highlight_scope.as_ref().unwrap().clone(),
                window,
                cx,
            );
            state.stamp = Some(Stamp {
                revision: tree.revision(),
                visibility,
                documents,
            });
        }
        drop(state);
        scope
    }
    pub(super) fn highlight_for(
        &mut self,
        tree: &Tree,
        node: NodeId,
    ) -> Option<(paint::Paint, paint::SharedCache)> {
        let mut cursor = Some(node);
        while let Some(id) = cursor {
            let current = tree.get(id)?;
            if current.highlight_scope.is_some() {
                return self
                    .highlights
                    .get(&id)?
                    .borrow_mut()
                    .paint(RunKey { node, fragment: 0 });
            }
            cursor = current.parent;
        }
        None
    }
    pub(super) fn document_highlight(&self, tree: &Tree, node: NodeId) -> Option<DocumentBinding> {
        let mut cursor = Some(node);
        while let Some(id) = cursor {
            let current = tree.get(id)?;
            if current.highlight_scope.is_some() {
                return Some(DocumentBinding {
                    scope: Rc::downgrade(self.highlights.get(&id)?),
                    key: RunKey { node, fragment: 0 },
                });
            }
            cursor = current.parent;
        }
        None
    }
    pub(super) fn begin_highlight_paint(&mut self) {
        for scope in self.highlights.values() {
            scope.borrow_mut().painted = false;
        }
    }
    pub(super) fn finish_highlight_paint(&mut self, cx: &mut Context<Self>) {
        let previous = self.highlights.len();
        self.highlights.retain(|_, scope| scope.borrow().painted);
        let released = self.highlights.len() != previous;
        let visibility = self.focus.borrow().visibility_identity();
        let documents = self.highlight_documents.borrow().clone();
        let events = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                self.highlights.clear();
                return;
            };
            self.highlights
                .iter()
                .filter_map(|(id, scope)| {
                    let node = tree.get(*id)?;
                    let handler = node.handler?;
                    let mut state = scope.borrow_mut();
                    let stamp = state.stamp.as_ref()?;
                    if stamp.revision != tree.revision()
                        || !Rc::ptr_eq(&stamp.visibility, &visibility)
                        || !Rc::ptr_eq(&stamp.documents, &documents)
                    {
                        return None;
                    }
                    let sample = state.observation();
                    if state.observed.as_ref() == Some(&(handler, sample.clone())) {
                        return None;
                    }
                    let event = session.highlight_observed(
                        self.id,
                        *id,
                        handler,
                        tree.revision(),
                        sample.clone(),
                    )?;
                    state.observed = Some((handler, sample));
                    Some(event)
                })
                .collect::<Vec<_>>()
        };
        for event in events {
            if !self.transport.input(event) && self.session.borrow_mut().overload(self.id) {
                self.transport.fault(self.id);
            }
        }
        // Retry admission once after actual scope reclamation; never idle-poll.
        if released {
            let mut retry = false;
            for scope in self.highlights.values() {
                let mut state = scope.borrow_mut();
                if matches!(
                    state.observation().state,
                    Outcome::Capacity(Limit::Admission)
                ) {
                    state.stamp = None;
                    retry = true;
                }
            }
            if retry {
                cx.notify();
            }
        }
    }
}
pub(super) fn marker(scope: Shared) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |_, _, _, _| scope.borrow_mut().painted = true,
    )
    .absolute()
    .size_full()
}

#[cfg(feature = "native-image-tests")]
#[path = "highlight_view_test.rs"]
pub(crate) mod test;
