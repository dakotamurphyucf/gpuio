//! Window-owned, latest-value keybinding observations. Sampling never executes an
//! action, enters OCaml, starts a timer or requests another frame.
use super::{InputContext, InputGate, View, keystroke};
use crate::tree::Tree;
use gpui::{App, AsKeystroke, Context as ViewContext, KeyContext, Keystroke, Window};
use gpuio_protocol::{HandlerId, NodeId, command_binding::*, v1::*};
use std::{collections::BTreeMap, rc::Rc, sync::Arc};

const MAX_SAMPLE_WORK: usize = 65_536;
type Definition<'a> = (NodeId, &'a Arc<CommandConfig>);
type Chord = (NodeId, String, u8, bool);

pub(crate) struct Owner {
    config: Arc<Config>,
    handler: HandlerId,
    context: Option<KeyContext>,
    observed: Option<Observation>,
}
impl Owner {
    fn new(config: Arc<Config>, handler: HandlerId) -> Self {
        let context = match &config.context {
            gpuio_protocol::command_binding::Context::NativeContext(text) => parse_context(text),
            _ => None,
        };
        Self {
            config,
            handler,
            context,
            observed: None,
        }
    }
    fn next(&self, state: State) -> Option<Observation> {
        if self
            .observed
            .as_ref()
            .is_some_and(|old| old.state == state || old.state == State::EpochExhausted)
        {
            return None;
        }
        let epoch = self
            .observed
            .as_ref()
            .map_or(1, |old| old.epoch.saturating_add(1));
        Some(Observation {
            epoch,
            state: if epoch == i64::MAX {
                State::EpochExhausted
            } else {
                state
            },
        })
    }
}

/// KeyContext's pinned parser recurses on malformed punctuation without consuming
/// it. Build the documented facts grammar iteratively instead. No fork required.
fn parse_context(source: &str) -> Option<KeyContext> {
    if source.len() > 1024 {
        return None;
    }
    let identifier = |text: &str| {
        text.char_indices()
            .find(|(_, ch)| !ch.is_alphanumeric() && *ch != '_' && *ch != '-')
            .map_or(text.len(), |(offset, _)| offset)
    };
    let mut rest = source.trim_start();
    if rest.is_empty() {
        return None;
    }
    let mut result = KeyContext::default();
    while !rest.is_empty() {
        let end = identifier(rest);
        if end == 0 {
            return None;
        }
        let key = rest[..end].to_owned();
        rest = rest[end..].trim_start();
        if let Some(value) = rest.strip_prefix('=') {
            rest = value.trim_start();
            let end = identifier(rest);
            if end == 0 {
                return None;
            }
            result.set(key, rest[..end].to_owned());
            rest = &rest[end..];
            if rest.chars().next().is_some_and(|ch| !ch.is_whitespace()) {
                return None;
            }
            rest = rest.trim_start();
        } else {
            result.add(key);
        }
    }
    Some(result)
}

struct Declarations<'a> {
    ordered: Vec<Definition<'a>>,
    indices: BTreeMap<&'a str, usize>,
}
struct Sampler<'a> {
    view: &'a View,
    tree: &'a Tree,
    window: &'a Window,
    cx: &'a App,
    input: InputContext,
    focused: Option<NodeId>,
    remaining: usize,
    contexts: BTreeMap<NodeId, Option<Rc<Declarations<'a>>>>,
    winners: BTreeMap<Chord, Option<Definition<'a>>>,
}
impl<'a> Sampler<'a> {
    fn work(&mut self) -> Result<(), ()> {
        self.remaining = self.remaining.checked_sub(1).ok_or(())?;
        Ok(())
    }
    fn declarations(&mut self, start: NodeId) -> Result<Rc<Declarations<'a>>, ()> {
        self.work()?;
        self.contexts
            .entry(start)
            .or_insert_with(|| {
                let ordered = self
                    .tree
                    .bounded_commands_from(start, &mut self.remaining)?;
                let indices = ordered
                    .iter()
                    .enumerate()
                    .map(|(i, (_, command))| (command.id.as_str(), i))
                    .collect();
                Some(Rc::new(Declarations { ordered, indices }))
            })
            .clone()
            .ok_or(())
    }
    fn winner(
        &mut self,
        start: NodeId,
        key: &Keystroke,
        priority: ShortcutPriority,
    ) -> Result<Option<Definition<'a>>, ()> {
        self.work()?;
        let cache_key = (
            start,
            key.key.clone(),
            modifiers(key),
            priority == ShortcutPriority::Override,
        );
        if let Some(result) = self.winners.get(&cache_key) {
            return Ok(*result);
        }
        let commands = self.declarations(start)?;
        let mut result = None;
        'find: for &(node, command) in &commands.ordered {
            for shortcut in &command.shortcuts {
                self.work()?;
                if self.input.matches(shortcut, key, priority) {
                    result = Some((node, command));
                    break 'find;
                }
            }
        }
        self.winners.insert(cache_key, result);
        Ok(result)
    }
    fn unavailable(&self, scope: NodeId, command: &CommandConfig) -> Option<Suppression> {
        if !command.enabled {
            Some(Suppression::Disabled)
        } else if self.view.focus.borrow().blocks_pointer(scope) {
            Some(Suppression::ScopeBlocked)
        } else if !self.view.command_available(command, self.window, self.cx) {
            Some(Suppression::NativeUnavailable)
        } else {
            None
        }
    }
    fn override_conflict(
        &mut self,
        start: NodeId,
        key: &Keystroke,
    ) -> Result<Option<Suppression>, ()> {
        Ok(self
            .winner(start, key, ShortcutPriority::Override)?
            .filter(|(scope, command)| self.unavailable(*scope, command).is_none())
            .map(|(_, command)| Suppression::Conflict(command.id.clone())))
    }
    fn registry(&mut self, start: NodeId, id: &str, focused: bool) -> Result<Entry, ()> {
        let declarations = self.declarations(start)?;
        let Some(index) = declarations.indices.get(id) else {
            return Ok(Entry::MissingCommand);
        };
        let (scope, command) = declarations.ordered[*index];
        let mut candidates = Vec::with_capacity(command.shortcuts.len());
        for shortcut in &command.shortcuts {
            self.work()?;
            let disposition = if !focused {
                Disposition::Declared
            } else {
                let key = keystroke(shortcut);
                let mut suppressed = self.unavailable(scope, command).or_else(|| {
                    self.input.gate(shortcut, &key).map(|gate| match gate {
                        InputGate::Composition => Suppression::Composition,
                        InputGate::TextInput => Suppression::TextInput,
                        InputGate::NativeNavigation => Suppression::NativeNavigation,
                    })
                });
                if suppressed.is_none() && shortcut.priority == ShortcutPriority::NativeFirst {
                    suppressed = self.override_conflict(start, &key)?.filter(
                        |reason| !matches!(reason, Suppression::Conflict(id) if id == &command.id),
                    );
                }
                if suppressed.is_none() {
                    suppressed = self
                        .winner(start, &key, shortcut.priority)?
                        .filter(|(_, winner)| winner.id != command.id)
                        .map(|(_, winner)| Suppression::Conflict(winner.id.clone()));
                }
                suppressed.map_or_else(
                    || match shortcut.priority {
                        ShortcutPriority::Override => Disposition::Override,
                        ShortcutPriority::NativeFirst => Disposition::NativeFirst,
                    },
                    Disposition::Unavailable,
                )
            };
            candidates.push(Candidate {
                shortcut: shortcut.clone(),
                disposition,
            });
        }
        Ok(Entry::Registry {
            enabled: command.enabled,
            candidates,
        })
    }
    fn native(
        &mut self,
        owner: &Owner,
        action: NativeCommand,
        start: Option<NodeId>,
    ) -> Result<Entry, ()> {
        self.work()?;
        let action = native_action(action);
        let focused = owner.config.context == gpuio_protocol::command_binding::Context::Focused;
        let binding = match &owner.config.context {
            gpuio_protocol::command_binding::Context::Focused => {
                self.window.focused(self.cx).and_then(|focus| {
                    self.window
                        .highest_precedence_binding_for_action_in(action.as_ref(), &focus)
                })
            }
            gpuio_protocol::command_binding::Context::Editor(_, node) => {
                let Some(editor) = self.view.editors.get(node) else {
                    return Ok(Entry::NativeUnbound);
                };
                self.window.highest_precedence_binding_for_action_in(
                    action.as_ref(),
                    &editor.focus_handle(self.cx),
                )
            }
            gpuio_protocol::command_binding::Context::NativeContext(_) => self
                .window
                .highest_precedence_binding_for_action_in_context(
                    action.as_ref(),
                    owner.context.clone().expect("validated context"),
                ),
            gpuio_protocol::command_binding::Context::Here => {
                unreachable!("validated target context")
            }
        };
        let Some(binding) = binding else {
            return Ok(Entry::NativeUnbound);
        };
        if binding.keystrokes().len() > MAX_STROKES {
            return Ok(Entry::NativeUnsupported(Unsupported::SequenceTooLong));
        }
        if binding.keystrokes().is_empty() {
            return Ok(Entry::NativeUnsupported(Unsupported::InvalidStroke));
        }
        let strokes: Vec<_> = binding
            .keystrokes()
            .iter()
            .map(|key| {
                let key = key.as_keystroke();
                Stroke {
                    key: key.key.clone(),
                    modifiers: i64::from(modifiers(key)),
                }
            })
            .collect();
        if !strokes.iter().all(Stroke::is_valid) {
            return Ok(Entry::NativeUnsupported(Unsupported::InvalidStroke));
        }
        let disposition = if !focused {
            Disposition::Declared
        } else if !self.window.is_action_available(action.as_ref(), self.cx) {
            Disposition::Unavailable(Suppression::NativeUnavailable)
        } else if let Some(start) = start
            && let Some(reason) =
                self.override_conflict(start, binding.keystrokes()[0].as_keystroke())?
        {
            Disposition::Unavailable(reason)
        } else {
            Disposition::Widget
        };
        Ok(Entry::NativeBinding {
            strokes,
            disposition,
        })
    }
    fn sample(&mut self, node: NodeId, owner: &Owner) -> State {
        if !self.view.binding_rendered.contains(&node)
            || !self.view.focus.borrow().highlight_visible(self.tree, node)
        {
            return State::Suspended;
        }
        let focused = owner.config.context == gpuio_protocol::command_binding::Context::Focused;
        let start = match &owner.config.context {
            gpuio_protocol::command_binding::Context::Focused => self.focused.or(self.tree.root()),
            gpuio_protocol::command_binding::Context::Here => Some(node),
            gpuio_protocol::command_binding::Context::Editor(_, editor) => {
                if !self.tree.get(*editor).is_some_and(|n| n.editor.is_some()) {
                    return State::ContextGone;
                }
                if !self.view.binding_rendered.contains(editor)
                    || !self
                        .view
                        .focus
                        .borrow()
                        .highlight_visible(self.tree, *editor)
                {
                    return State::Suspended;
                }
                Some(*editor)
            }
            gpuio_protocol::command_binding::Context::NativeContext(_) => {
                if owner.context.is_none() {
                    return State::InvalidContext;
                }
                None
            }
        };
        let result: Result<Vec<_>, ()> = owner
            .config
            .targets
            .iter()
            .map(|target| match target {
                Target::Command(id) => match start {
                    Some(start) => self.registry(start, id, focused),
                    None => Ok(Entry::MissingCommand),
                },
                Target::NativeAction(action) => self.native(owner, *action, start),
            })
            .collect();
        result.map_or(State::Capacity, State::Ready)
    }
}
fn modifiers(key: &Keystroke) -> u8 {
    u8::from(key.modifiers.control)
        | (u8::from(key.modifiers.alt) << 1)
        | (u8::from(key.modifiers.shift) << 2)
        | (u8::from(key.modifiers.platform) << 3)
        | (u8::from(key.modifiers.function) << 4)
}
fn native_action(action: NativeCommand) -> Box<dyn gpui::Action> {
    match action {
        NativeCommand::Copy => Box::new(gpui_base::input::Copy),
        NativeCommand::Cut => Box::new(gpui_base::input::Cut),
        NativeCommand::Paste => Box::new(gpui_base::input::Paste),
        NativeCommand::SelectAll => Box::new(gpui_base::input::SelectAll),
        NativeCommand::Undo => Box::new(gpui_base::input::Undo),
        NativeCommand::Redo => Box::new(gpui_base::input::Redo),
    }
}
impl View {
    pub(in crate::host) fn sync_binding_queries(&mut self) {
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.id) else {
            self.binding_queries.clear();
            return;
        };
        self.transport
            .mailbox
            .lock()
            .expect("mailbox poisoned")
            .retain_bindings(self.id, |id, handler| {
                tree.get(id).is_some_and(|node| {
                    node.command_binding.is_some() && node.handler == Some(handler)
                })
            });
        self.binding_queries.retain(|id, owner| {
            tree.get(*id).is_some_and(|node| {
                node.command_binding.as_ref() == Some(&owner.config)
                    && node.handler == Some(owner.handler)
            })
        });
        for id in tree.binding_owners() {
            self.binding_queries.entry(id).or_insert_with(|| {
                let node = tree.get(id).expect("indexed binding owner");
                Owner::new(
                    node.command_binding.clone().expect("query"),
                    node.handler.expect("query handler"),
                )
            });
        }
    }
    pub(in crate::host) fn finish_binding_paint(
        &mut self,
        window: &Window,
        cx: &mut ViewContext<Self>,
    ) {
        if self.binding_queries.is_empty() {
            return;
        }
        let mut owners = std::mem::take(&mut self.binding_queries);
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.id) else {
            return;
        };
        let focused = self.focus.borrow().focused_node(window, cx);
        let kind = focused.and_then(|id| tree.get(id).map(|node| node.kind));
        let input = self.command_input_context(kind, window, cx);
        let mut sampler = Sampler {
            view: self,
            tree,
            window,
            cx,
            input,
            focused,
            remaining: MAX_SAMPLE_WORK,
            contexts: BTreeMap::new(),
            winners: BTreeMap::new(),
        };
        for (&id, owner) in &mut owners {
            let state = sampler.sample(id, owner);
            if let Some(observation) = owner.next(state)
                && let Some(event) = session.command_binding_observed(
                    self.id,
                    id,
                    owner.handler,
                    observation.clone(),
                )
                && self.transport.input(event)
            {
                owner.observed = Some(observation);
            }
        }
        drop(sampler);
        drop(session);
        self.binding_queries = owners;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_context_facts_preserve_first_definition_without_recursive_parsing() {
        for source in [
            "Input",
            "Input mode=visible",
            " Input\tmode = visible other-name=日本語 ",
            "Input mode=a mode=b",
        ] {
            assert_eq!(
                parse_context(source),
                Some(KeyContext::parse(source).unwrap())
            );
        }
        for source in [
            "",
            "   ",
            "!",
            "Input!",
            "Input && Editor",
            "a==b",
            "a=",
            "=value",
            "a=b=c",
            "Input\0",
        ] {
            assert!(parse_context(source).is_none(), "{source:?}");
        }
        assert!(parse_context(&"a ".repeat(512)).is_some());
        assert!(parse_context(&"a".repeat(1025)).is_none());
        let context = parse_context("Input mode=first mode=second").unwrap();
        assert_eq!(
            context.get("mode").map(|value| value.as_ref()),
            Some("first")
        );
    }
    #[test]
    fn unchanged_samples_are_silent_and_exhaustion_is_terminal() {
        let mut owner = Owner::new(
            Arc::new(Config {
                context: gpuio_protocol::command_binding::Context::Focused,
                targets: vec![Target::Command("run".into())],
            }),
            HandlerId::from_parts(0, 1).unwrap(),
        );
        let first = owner
            .next(State::Ready(vec![Entry::MissingCommand]))
            .unwrap();
        assert_eq!(first.epoch, 1);
        owner.observed = Some(first.clone());
        assert!(owner.next(first.state).is_none());
        assert_eq!(owner.next(State::Suspended).unwrap().epoch, 2);
        owner.observed = Some(Observation {
            epoch: i64::MAX - 1,
            state: State::Suspended,
        });
        let last = owner.next(State::ContextGone).unwrap();
        assert_eq!(
            last,
            Observation {
                epoch: i64::MAX,
                state: State::EpochExhausted
            }
        );
        owner.observed = Some(last);
        assert!(owner.next(State::Suspended).is_none());
    }
}
