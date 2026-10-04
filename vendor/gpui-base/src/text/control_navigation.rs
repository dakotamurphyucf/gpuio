//! Realize virtual blocks before admitting their native controls to keyboard focus.
//! AST candidates are hints, never focus targets; only painted tab stops qualify.
use std::{collections::BTreeMap, sync::Arc};

use gpui::{App, Context, FocusHandle, Window};

use super::{
    document::ParsedDocument, node::BlockNode, state::TextViewState, text_view::LinkFocusGuardFn,
};

#[derive(Default)]
pub(super) struct Navigation {
    pub scopes: BTreeMap<usize, FocusHandle>,
    prepared: bool,
    pending: Option<Pending>,
    scheduled: bool,
    epoch: Arc<()>,
}
struct Pending {
    block: usize,
    backward: bool,
    anchor: FocusHandle,
    target: Option<FocusHandle>,
    guard: Option<Arc<LinkFocusGuardFn>>,
}
impl Navigation {
    pub fn is_pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn invalidate(&mut self) {
        self.cancel();
        self.prepared = false;
    }
    pub fn cancel(&mut self) {
        if self.pending.take().is_some() || self.scheduled {
            self.epoch = Arc::new(());
        }
        self.scheduled = false;
    }
    pub fn prepare(&mut self, document: &ParsedDocument, cx: &App) {
        if self.prepared {
            return;
        }
        self.scopes
            .retain(|ix, _| document.blocks.get(*ix).is_some_and(candidate));
        for (ix, block) in document.blocks.iter().enumerate() {
            if candidate(block) {
                self.scopes.entry(ix).or_insert_with(|| cx.focus_handle());
            }
        }
        self.prepared = true;
    }
    fn next(&self, block: Option<usize>, backward: bool) -> Option<usize> {
        if backward {
            self.scopes
                .keys()
                .rev()
                .copied()
                .find(|ix| block.is_none_or(|block| *ix < block))
        } else {
            self.scopes
                .keys()
                .copied()
                .find(|ix| block.is_none_or(|block| *ix > block))
        }
    }
}
fn candidate(node: &BlockNode) -> bool {
    match node {
        BlockNode::Root { children, .. }
        | BlockNode::Blockquote { children, .. }
        | BlockNode::List { children, .. }
        | BlockNode::ListItem { children, .. } => children.iter().any(candidate),
        BlockNode::Paragraph(p) | BlockNode::Heading { children: p, .. } => {
            p.children.iter().any(|inline| inline.custom.is_some())
        }
        BlockNode::CodeBlock(_) | BlockNode::Table(_) | BlockNode::Custom(_) => true,
        BlockNode::DescriptionList(list) => list.entries.iter().any(|entry| {
            entry
                .label
                .children
                .iter()
                .chain(entry.value.children.iter())
                .any(|inline| inline.custom.is_some())
        }),
        BlockNode::Break { .. }
        | BlockNode::HorizontalRule { .. }
        | BlockNode::Definition { .. }
        | BlockNode::Unknown => false,
    }
}
impl TextViewState {
    fn controls_allowed(&self, cx: &App) -> bool {
        self.link_focus_guard.as_ref().is_none_or(|guard| guard(cx))
    }
    fn virtual_controls(&self) -> bool {
        self.scrollable && self.max_lines.is_none()
    }

    /// Called after logical links, or while a native descendant owns focus.
    pub(super) fn tab_controls(
        &mut self,
        backward: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.controls_allowed(cx) {
            self.control_navigation.cancel();
            return false;
        }
        if let Some(pending) = &self.control_navigation.pending {
            // Coalesce key repeat until a frame establishes the next stop. A
            // reversed request cancels rather than acting on an unpainted row.
            if pending.backward != backward {
                self.control_navigation.cancel();
            }
            return true;
        }
        let current = self
            .control_navigation
            .scopes
            .iter()
            .find(|(_, scope)| scope.contains_focused(window, cx))
            .map(|(ix, _)| *ix);
        if let Some(block) = current {
            let focused = window.focused(cx);
            if self.try_controls(block, focused.as_ref(), backward, window, cx) {
                return true;
            }
        } else if !self.focus_handle.is_focused(window) {
            return false;
        }
        let next = self.control_navigation.next(current, backward);
        if next.is_none() && backward && current.is_some() {
            self.focus_handle.focus(window, cx);
            self.link_navigation.active = None;
            return true;
        }
        self.seek_control(next, backward, window, cx)
    }

    /// Native order within a realized block, including measured reveal hints.
    /// A clipped target gets one reveal attempt; failure resumes after it.
    fn try_controls(
        &mut self,
        block: usize,
        after: Option<&FocusHandle>,
        backward: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let scope = &self.control_navigation.scopes[&block];
        let eligible = window.tab_stops_within(scope);
        let mut candidates = window.tab_candidates_within(scope);
        if backward {
            candidates.reverse();
        }
        let start = match after {
            Some(after) => match candidates.iter().position(|(handle, _)| handle == after) {
                Some(index) => index + 1,
                None => return false,
            },
            None => 0,
        };
        for (handle, bounds) in candidates.into_iter().skip(start) {
            if eligible.contains(&handle) {
                self.link_navigation.active = None;
                handle.focus(window, cx);
                return true;
            }
            if self.virtual_controls() {
                self.request_control(block, backward, Some((handle, bounds)), window, cx);
                return true;
            }
        }
        false
    }

    fn request_control(
        &mut self,
        block: usize,
        backward: bool,
        target: Option<(FocusHandle, gpui::Bounds<gpui::Pixels>)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = target
            .as_ref()
            .and_then(|(_, bounds)| {
                self.list_state
                    .bounds_for_item(block)
                    .map(|row| (bounds.top() - row.top()).max(gpui::px(0.)))
            })
            .unwrap_or(gpui::px(0.));
        self.link_navigation.active = None;
        self.focus_handle.focus(window, cx);
        self.control_navigation.pending = Some(Pending {
            block,
            backward,
            anchor: self.focus_handle.clone(),
            target: target.map(|(handle, _)| handle),
            guard: self.link_focus_guard.clone(),
        });
        self.list_state.scroll_to(gpui::ListOffset {
            item_ix: block,
            offset_in_item: offset,
        });
        cx.notify();
    }

    fn seek_control(
        &mut self,
        mut block: Option<usize>,
        backward: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        while let Some(ix) = block {
            if self.try_controls(ix, None, backward, window, cx) {
                return true;
            }
            // A virtual row absent from the painted frame must be realized
            // before deciding that it has no enabled controls.
            if self.virtual_controls() {
                self.request_control(ix, backward, None, window, cx);
                return true;
            }
            block = self.control_navigation.next(Some(ix), backward);
        }
        false
    }

    /// Queued by an actual paint, never an idle timer. Each callback either
    /// focuses an eligible stop, advances monotonically, or realizes one block/
    /// control for the next frame. Stale requests cannot outlive their source.
    pub(super) fn schedule_control_focus(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.control_navigation.pending.is_none() || self.control_navigation.scheduled {
            return;
        }
        self.control_navigation.scheduled = true;
        let epoch = self.control_navigation.epoch.clone();
        let weak = cx.entity().downgrade();
        window.defer(cx, move |window, cx| {
            let mut exit_handler = None;
            let _ = weak.update(cx, |state, cx| {
                if !Arc::ptr_eq(&epoch, &state.control_navigation.epoch) {
                    return;
                }
                state.control_navigation.scheduled = false;
                let Some(pending) = state.control_navigation.pending.take() else {
                    return;
                };
                if !window.is_window_active()
                    || !pending.anchor.is_focused(window)
                    || pending.guard.as_ref().is_some_and(|guard| !guard(cx))
                    || !state.controls_allowed(cx)
                {
                    return;
                }
                let Some(scope) = state.control_navigation.scopes.get(&pending.block) else {
                    return;
                };
                if let Some(target) = &pending.target
                    && window.tab_stops_within(scope).contains(target)
                {
                    target.focus(window, cx);
                    return;
                }
                if state.try_controls(
                    pending.block,
                    pending.target.as_ref(),
                    pending.backward,
                    window,
                    cx,
                ) {
                    return;
                }
                let next = state
                    .control_navigation
                    .next(Some(pending.block), pending.backward);
                if !state.seek_control(next, pending.backward, window, cx) {
                    if pending.backward {
                        state.focus_handle.focus(window, cx);
                    } else if let Some(exit) = state.tab_exit_handler.clone() {
                        exit_handler = Some((exit, pending.backward));
                    } else if let Some(next) =
                        window.tab_stop_outside(&state.focus_handle, pending.backward)
                    {
                        next.focus(window, cx);
                    }
                }
            });
            // The host may read the text entity while restoring its composite
            // focus anchor. Invoke it only after releasing this entity's borrow.
            if let Some((exit, backward)) = exit_handler {
                exit(backward, window, cx);
            }
        });
    }
}
