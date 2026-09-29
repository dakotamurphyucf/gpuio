//! Native component ownership. Hooks execute on the GPUI thread; sinks enqueue.
use super::*;
use gpuio_extension_sdk::{Component, Error, EventLease, contain};
use gpuio_protocol::{
    HandlerId,
    extension::{Config, Payload, Signal},
};

pub(super) struct State {
    config: Arc<Config>,
    handler: HandlerId,
    component: Option<Box<dyn Component>>,
    lease: EventLease,
    pub(super) focus: gpui::FocusHandle,
    command: i64,
    failure: Option<Error>,
}
impl State {
    fn dispose(&mut self) {
        self.lease.close();
        if let Some(mut component) = self.component.take() {
            let _ = contain(|| {
                component.unmount();
                Ok(())
            });
            let _ = contain(|| {
                drop(component);
                Ok(())
            });
        }
    }
}
impl Drop for State {
    fn drop(&mut self) {
        self.dispose();
    }
}

fn lease(
    transport: &Arc<Transport>,
    window: WindowId,
    node: NodeId,
    handler: HandlerId,
    revision: i64,
    config: &Config,
    maximum: usize,
) -> EventLease {
    let weak = Arc::downgrade(transport);
    let generation = config.generation;
    EventLease::new(maximum, move |bytes| {
        let transport = weak.upgrade().ok_or(Error::Closed)?;
        if transport.input(Event::ExtensionEvent(
            window,
            node,
            handler,
            revision,
            generation,
            Signal::Data(Payload(bytes)),
        )) {
            Ok(())
        } else {
            Err(Error::Overloaded)
        }
    })
    .expect("validated component event limit")
}

impl View {
    pub(super) fn hide_unvisited_extensions(&self) {
        for (id, state) in &self.extensions {
            if !self.visited.contains(id) {
                state.lease.set_visible(false);
            }
        }
    }

    fn extension_signal(
        &self,
        id: NodeId,
        handler: HandlerId,
        revision: i64,
        generation: i64,
        signal: Signal,
    ) {
        if !self.transport.input(Event::ExtensionEvent(
            self.id, id, handler, revision, generation, signal,
        )) && self.session.borrow_mut().overload(self.id)
        {
            self.transport.fault(self.id);
        }
    }

    pub(super) fn sync_extensions(
        &mut self,
        dirty: &[NodeId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (nodes, revision) = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                self.extensions.clear();
                return;
            };
            self.extensions.retain(|id, state| {
                tree.get(*id)
                    .and_then(|node| node.extension.as_ref())
                    .is_some_and(|config| {
                        config.generation == state.config.generation
                            && config.schema == state.config.schema
                    })
            });
            (
                dirty
                    .iter()
                    .filter_map(|id| tree.get(*id))
                    .filter(|node| node.extension.is_some())
                    .cloned()
                    .collect::<Vec<_>>(),
                tree.revision(),
            )
        };
        for node in nodes {
            let config = node.extension.expect("filtered extension");
            let handler = node.handler.expect("validated extension handler");
            let id = node.id;
            let (descriptor, factory) = crate::extensions::registry()
                .resolve(
                    &config.schema.name,
                    config.schema.version as u16,
                    &config.schema.fingerprint,
                )
                .expect("registry validated before tree acceptance");
            let mounted = !self.extensions.contains_key(&id);
            let state = self.extensions.entry(id).or_insert_with(|| State {
                lease: lease(
                    &self.transport,
                    self.id,
                    id,
                    handler,
                    revision,
                    &config,
                    descriptor.max_event,
                ),
                config: config.clone(),
                handler,
                component: None,
                focus: cx.focus_handle().tab_stop(true),
                command: 0,
                failure: None,
            });
            if state.failure.is_none()
                && let Some(error) = state.lease.failure()
            {
                state.failure = Some(error);
                state.dispose();
                self.extension_signal(
                    id,
                    handler,
                    revision,
                    config.generation,
                    Signal::Failed(crate::extensions::wire_error(error)),
                );
                continue;
            }
            let changed = state.config != config || state.handler != handler;
            if changed {
                state.lease.close();
                state.lease = lease(
                    &self.transport,
                    self.id,
                    id,
                    handler,
                    revision,
                    &config,
                    descriptor.max_event,
                );
                state.config = config.clone();
                state.handler = handler;
            }
            state
                .lease
                .set_visible(!config.disabled && self.focus.borrow().allows(id));
            if state.failure.is_some() {
                continue;
            }
            let mut context = gpuio_extension_sdk::Context {
                window,
                app: cx,
                focus: state.focus.clone(),
                events: state.lease.sink(),
            };
            let result = if mounted {
                contain(|| factory.mount(&config.properties.0, &mut context))
                    .map(|component| state.component = Some(component))
            } else if changed {
                contain(|| {
                    state
                        .component
                        .as_mut()
                        .expect("mounted component")
                        .update(&config.properties.0, &mut context)
                })
            } else {
                Ok(())
            };
            let mut signals = Vec::new();
            if result.is_ok() && mounted {
                signals.push(Signal::Mounted);
            }
            let result = result.and_then(|()| {
                if let Some(command) = &config.command
                    && command.sequence > state.command
                {
                    // Advance before invoking: failures must never replay side effects.
                    state.command = command.sequence;
                    contain(|| {
                        state
                            .component
                            .as_mut()
                            .expect("mounted component")
                            .command(&command.payload.0, &mut context)
                    })?;
                    signals.push(Signal::CommandCompleted(command.sequence));
                }
                Ok(())
            });
            if let Err(error) = result {
                state.failure = Some(error);
                state.dispose();
                signals.push(Signal::Failed(crate::extensions::wire_error(error)));
            }
            for signal in signals {
                self.extension_signal(id, handler, revision, config.generation, signal);
            }
        }
        let session = self.session.borrow();
        for (id, state) in &self.extensions {
            state.lease.set_pointer_enabled(
                session
                    .tree(self.id)
                    .is_some_and(|tree| pointer_enabled(tree, *id)),
            );
            state
                .lease
                .set_visible(!state.config.disabled && self.focus.borrow().allows(*id));
        }
    }

    pub(super) fn extension_element(
        &mut self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        interaction: Interaction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let id = node.id;
        self.visited.insert(id);
        let Some(state) = self.extensions.get_mut(&id) else {
            return div().into_any_element();
        };
        let enabled = !state.config.disabled && self.focus.borrow().allows(id);
        state.lease.set_pointer_enabled(pointer_enabled(tree, id));
        state.lease.set_visible(enabled);
        let focus = state.focus.clone();
        let element = div()
            .id((
                "gpuio-extension",
                ((id.generation() as u64) << 32) | id.slot() as u64,
            ))
            .role(gpui::Role::Group)
            .aria_label(state.config.label.clone());
        let (mut element, _) = apply_styles(element, &node.style, interaction, !enabled);
        if state.failure.is_none() {
            let mut context = gpuio_extension_sdk::Context {
                window,
                app: cx,
                focus: focus.clone(),
                events: state.lease.sink(),
            };
            let result = match state.lease.failure() {
                Some(error) => Err(error),
                None => contain(|| {
                    state
                        .component
                        .as_mut()
                        .ok_or(Error::Closed)?
                        .render(&mut context)
                }),
            };
            match result {
                Ok(child) => element = element.child(child),
                Err(error) => {
                    state.failure = Some(error);
                    state.dispose();
                    let (handler, generation) = (state.handler, state.config.generation);
                    let event = Event::ExtensionEvent(
                        self.id,
                        id,
                        handler,
                        tree.revision(),
                        generation,
                        Signal::Failed(crate::extensions::wire_error(error)),
                    );
                    let session = self.session.clone();
                    let transport = self.transport.clone();
                    let window_id = self.id;
                    // Render holds an immutable tree borrow; overload mutation follows it.
                    cx.defer(move |_| {
                        if !transport.input(event) && session.borrow_mut().overload(window_id) {
                            transport.fault(window_id);
                        }
                    });
                }
            }
        }
        let gate = self.focus.clone();
        let record_focus = focus.clone();
        element = element.child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    gate.borrow_mut().record(
                        id,
                        record_focus.clone(),
                        enabled,
                        record_focus.is_focused(window),
                        bounds,
                    );
                },
            )
            .absolute()
            .size_full(),
        );
        let gate = self.focus.clone();
        element = element.on_a11y_action(gpui::AccessibleAction::Focus, move |_, window, cx| {
            if enabled && gate.borrow().allows(id) {
                window.focus(&focus, cx);
            }
        });
        crate::semantics::State {
            hidden: false,
            metadata: None,
            element,
            disabled: !enabled,
            read_only: false,
            modal: false,
            live: None,
        }
        .into_any_element()
    }
}
