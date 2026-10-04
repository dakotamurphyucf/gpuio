//! Installed native profiles. UI callbacks retain only accounted data; input
//! ownership is revocable independently of cached elements and their resources.
use super::*;
use gpuio_document_sdk as sdk;
use gpuio_protocol::document_profile::{
    Config as ProfileConfig, Event as ProfileEvent, Signal, Stage,
};
use std::sync::{
    Weak,
    atomic::{AtomicBool, Ordering},
};

pub(super) type Owner = Rc<RefCell<Option<Weak<Installed>>>>;
pub(super) struct Attachment {
    pub(super) environment: Environment,
    pub(super) owner: Owner,
}
impl Drop for State {
    fn drop(&mut self) {
        if let Some(owner) = self.profile_owner.borrow().as_ref().and_then(Weak::upgrade) {
            owner.revoke();
        }
    }
}
#[derive(Clone)]
pub(super) struct Environment {
    transport: Weak<crate::transport::Transport>,
    window: gpuio_protocol::WindowId,
    node: NodeId,
    handler: Option<gpuio_protocol::HandlerId>,
    revision: i64,
    source: Option<ResourceId>,
    allowed: bool,
    pointer: bool,
}
impl Environment {
    pub(super) fn new(root: &View, tree: &Tree, node: &Node) -> Self {
        Self {
            transport: Arc::downgrade(&root.transport),
            window: root.id,
            node: node.id,
            handler: node.handler,
            revision: tree.revision(),
            source: node.document.as_ref().and_then(|d| d.source),
            allowed: root.focus.borrow().allows(node.id),
            pointer: super::super::pointer_enabled(tree, node.id),
        }
    }
    fn emit(&self, config: &ProfileConfig, snapshot: &Snapshot, signal: Signal) {
        let (Some(transport), Some(handler), Some(source), Some(instance)) = (
            self.transport.upgrade(),
            self.handler,
            self.source,
            config.instance.as_ref(),
        ) else {
            return;
        };
        let event = ProfileEvent {
            config_epoch: config.epoch,
            instance_generation: instance.generation,
            source_generation: snapshot.generation,
            source_revision: snapshot.revision,
            signal,
        };
        if !event.is_valid() {
            return;
        }
        if !transport.input(Event::DocumentProfileEvent(
            self.window,
            self.node,
            handler,
            self.revision,
            source,
            event,
        )) {
            transport.fault(self.window);
        }
    }
}
pub(super) struct Installed {
    profile: Arc<sdk::Profile>,
    context: sdk::RenderContext,
    lease: sdk::EventLease,
    parser_epoch: u64,
    config: Arc<ProfileConfig>,
    snapshot: Arc<Snapshot>,
    environment: Environment,
    revoked: AtomicBool,
    failed: AtomicBool,
    _charge: Arc<document_jobs::Charge>,
}
impl Installed {
    #[cfg(all(test, feature = "native-image-tests"))]
    pub(super) fn events(&self) -> sdk::EventSink {
        self.context.events().clone()
    }
    fn new(
        prepared: crate::document_profile_jobs::Prepared,
        config: Arc<ProfileConfig>,
        snapshot: Arc<Snapshot>,
        environment: Environment,
        charge: Arc<document_jobs::Charge>,
    ) -> Result<Arc<Self>, sdk::Error> {
        let source_stamp = sdk::Source::new(snapshot.generation, snapshot.revision)?;
        let instance = config
            .instance
            .as_ref()
            .ok_or(sdk::Error::InvalidProperties)?;
        let env = environment.clone();
        let epoch = config.epoch;
        let generation = instance.generation;
        let source = environment.source.ok_or(sdk::Error::InvalidSource)?;
        let handler = environment.handler.ok_or(sdk::Error::InvalidProperties)?;
        let lease = sdk::EventLease::new(prepared.descriptor.max_event, move |bytes| {
            use gpuio_extension_sdk::Error;
            let transport = env.transport.upgrade().ok_or(Error::Closed)?;
            if transport.input(Event::DocumentProfileEvent(
                env.window,
                env.node,
                handler,
                env.revision,
                source,
                ProfileEvent {
                    config_epoch: epoch,
                    instance_generation: generation,
                    source_revision: source_stamp.revision(),
                    source_generation: source_stamp.generation(),
                    signal: Signal::Data(gpuio_protocol::extension::Payload(bytes)),
                },
            )) {
                Ok(())
            } else {
                transport.fault(env.window);
                Err(Error::Overloaded)
            }
        })
        .map_err(|_| sdk::Error::LimitExceeded)?;
        lease.set_visible(false);
        let sink = lease.sink().with_retained_resource(charge.clone());
        Ok(Arc::new_cyclic(|weak: &Weak<Self>| {
            let weak = weak.clone();
            let context =
                sdk::RenderContext::new(source_stamp, sink).with_failure_reporter(move |error| {
                    if let Some(owner) = weak.upgrade() {
                        owner.fail(Stage::Render, error);
                    }
                });
            Self {
                profile: prepared.profile,
                parser_epoch: prepared.parser_epoch,
                context,
                lease,
                config,
                snapshot,
                environment,
                revoked: AtomicBool::new(false),
                failed: AtomicBool::new(false),
                _charge: charge,
            }
        }))
    }
    pub(super) fn revoke(&self) {
        self.revoked.store(true, Ordering::Release);
        self.lease.close();
    }
    fn fail(&self, stage: Stage, error: sdk::Error) {
        if self.revoked.load(Ordering::Acquire) || self.failed.swap(true, Ordering::AcqRel) {
            return;
        }
        self.lease.close();
        self.environment.emit(
            &self.config,
            &self.snapshot,
            Signal::Failed(stage, crate::document_profiles::wire_error(error)),
        );
    }
    pub(super) fn poll_failure(&self) {
        if self.lease.failure().is_some() {
            self.fail(Stage::Input, sdk::Error::Panicked);
        }
    }
    fn extensions(
        &self,
        base: gpui_base::text::MarkdownExtensions,
    ) -> Result<gpui_base::text::MarkdownExtensions, sdk::Error> {
        self.profile
            .extensions(
                base,
                self.parser_epoch,
                Arc::new(AtomicBool::new(false)),
                Some(self.context.clone()),
            )
            .map(|adapted| adapted.extensions)
    }
    pub(super) fn update_policy(&self, allowed: bool, pointer: bool) {
        self.lease.set_pointer_enabled(pointer);
        if !allowed {
            self.hide();
        }
    }
    pub(super) fn source_changed(&self, generation: i64) {
        if self.snapshot.generation != generation {
            self.revoke();
        }
    }
    pub(super) fn hide(&self) {
        self.lease.set_visible(false);
    }
    fn paint(
        &self,
        allowed: bool,
        pointer: bool,
        bounds: gpui::Bounds<gpui::Pixels>,
        window: &Window,
    ) {
        let clip = bounds.intersect(&window.content_mask().bounds);
        let visible = allowed
            && !self.revoked.load(Ordering::Acquire)
            && !self.failed.load(Ordering::Acquire)
            && clip.size.width > px(0.)
            && clip.size.height > px(0.)
            && window.element_opacity() > 0.;
        self.lease.set_pointer_enabled(pointer);
        self.lease.set_visible(visible);
        self.poll_failure();
    }
    pub(super) fn code(
        &self,
        block: &sdk::CodeBlock,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<gpui::AnyElement> {
        self.render_action(|renderer| renderer.code(block, &self.context, window, cx))
    }
    pub(super) fn table(
        &self,
        table: &sdk::TableData,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<gpui::AnyElement> {
        self.render_action(|renderer| renderer.table(table, &self.context, window, cx))
    }
    fn render_action(
        &self,
        render: impl FnOnce(&dyn sdk::ActionRenderer) -> Result<Option<gpui::AnyElement>, sdk::Error>,
    ) -> Option<gpui::AnyElement> {
        if self.failed.load(Ordering::Acquire) {
            return Some(
                div()
                    .id("profile-render-error")
                    .role(gpui::Role::Alert)
                    .child("Document extension unavailable")
                    .into_any_element(),
            );
        }
        let renderer = self.profile.actions()?;
        match sdk::contain(|| render(renderer.as_ref())) {
            Ok(value) => value,
            Err(error) => {
                self.fail(Stage::Render, error);
                Some(
                    div()
                        .id("profile-render-error")
                        .role(gpui::Role::Alert)
                        .child("Document extension unavailable")
                        .into_any_element(),
                )
            }
        }
    }
}
impl Drop for Presentation {
    fn drop(&mut self) {
        self.revoke_profile();
    }
}
impl Presentation {
    pub(in crate::host) fn revoke_profile(&mut self) {
        if let Some(installed) = self.profile_install.take() {
            installed.revoke();
            let mut owner = self.profile_owner.borrow_mut();
            if owner
                .as_ref()
                .and_then(Weak::upgrade)
                .is_some_and(|current| Arc::ptr_eq(&current, &installed))
            {
                *owner = None;
            }
        }
    }
    pub(super) fn refresh_profile(
        &mut self,
        config: Option<Arc<ProfileConfig>>,
        environment: Environment,
        next_document: &Config,
        options: gpuio_protocol::document::MarkdownOptions,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let observes = self
            .profile_config
            .as_ref()
            .is_some_and(|c| c.instance.is_some())
            || config.as_ref().is_some_and(|c| c.instance.is_some());
        let handler_changed = observes && self.profile_environment.handler != environment.handler;
        self.profile_environment = environment;
        if self.profile_config == config && !handler_changed {
            return;
        }
        self.revoke_profile();
        self.profile_config = config;
        self.profile_request = self
            .profile_config
            .as_ref()
            .filter(|c| c.instance.is_some())
            .map(|c| {
                crate::document_profile_jobs::Request::bind(c.clone())
                    .map(|request| request.with_observer(self.profile_environment.handler))
            })
            .transpose();
        self.profile_failure = None;
        self.ready = false;
        let request = self
            .profile_request
            .clone()
            .map(|profile| document_jobs::Request {
                observer: None,
                profile,
                snapshot: self.lease.snapshot(),
                mode: next_document.mode.clone(),
                markdown_options: options,
                dark: next_document.dark,
                search: next_document.search.clone(),
            });
        match request {
            Ok(request) => {
                if let Ok(job) = &self.job {
                    let _ = job.update(request, cx);
                } else {
                    self.job = document_host::request(request, window, cx);
                }
            }
            Err(error) => self.job = Err(error),
        }
        cx.notify();
    }
    pub(super) fn profile_extensions(&self) -> gpui_base::text::MarkdownExtensions {
        let base = document_markdown::extensions_with_options(
            document_markdown::Images(self.images.0.clone()),
            self.installed_markdown_options,
        );
        if let Some(profile) = &self.profile_install {
            match profile.extensions(base.clone()) {
                Ok(extensions) => extensions,
                Err(error) => {
                    profile.fail(Stage::Render, error);
                    base
                }
            }
        } else {
            base
        }
    }
    pub(super) fn install_profile(
        &mut self,
        prepared: Option<crate::document_profile_jobs::Prepared>,
        charge: Arc<document_jobs::Charge>,
    ) {
        self.revoke_profile();
        if let (Some(prepared), Some(config)) = (prepared, &self.profile_config) {
            match Installed::new(
                prepared,
                config.clone(),
                self.snapshot.clone(),
                self.profile_environment.clone(),
                charge,
            ) {
                Ok(installed) => {
                    *self.profile_owner.borrow_mut() = Some(Arc::downgrade(&installed));
                    self.profile_install = Some(installed);
                }
                Err(error) => {
                    self.report_profile_failure(document_jobs::Error::Profile(Stage::Render, error))
                }
            }
        }
    }
    pub(super) fn report_profile_failure(&mut self, error: document_jobs::Error) {
        let Some(config) = self
            .profile_config
            .as_ref()
            .filter(|c| c.instance.is_some())
        else {
            return;
        };
        if self.snapshot.revision <= 0 {
            return;
        }
        let (stage, error) = match error {
            document_jobs::Error::Profile(stage, error) => (stage, error),
            document_jobs::Error::Highlight => (Stage::Highlight, sdk::Error::Highlight),
            document_jobs::Error::ResourceLimit => (Stage::Parse, sdk::Error::LimitExceeded),
            document_jobs::Error::Parse => (Stage::Parse, sdk::Error::Parse),
            document_jobs::Error::Closed => (Stage::Configure, sdk::Error::Closed),
            document_jobs::Error::Cancelled => return,
        };
        let stamp = (
            config.epoch,
            self.snapshot.generation,
            self.snapshot.revision,
        );
        if self.profile_failure == Some(stamp) {
            return;
        }
        self.profile_failure = Some(stamp);
        self.profile_environment.emit(
            config,
            &self.snapshot,
            Signal::Failed(stage, crate::document_profiles::wire_error(error)),
        );
    }
    pub(super) fn profile_paint_gate(&self) -> Option<gpui::AnyElement> {
        let owner = self.profile_install.clone()?;
        let current = self.lease.snapshot();
        let allowed = self.profile_environment.allowed
            && !self.collapsed
            && !self.source_mode
            && current.generation == owner.snapshot.generation;
        let pointer = self.profile_environment.pointer;
        Some(
            gpui::canvas(
                |_, _, _| (),
                move |bounds, _, window, _| owner.paint(allowed, pointer, bounds, window),
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .into_any_element(),
        )
    }
}
impl View {
    pub(in crate::host) fn begin_document_profile_paint(&self) {
        for state in self.documents.values() {
            if let Some(owner) = state
                .profile_owner
                .borrow()
                .as_ref()
                .and_then(Weak::upgrade)
            {
                owner.hide();
                owner.poll_failure();
            }
        }
    }
}
