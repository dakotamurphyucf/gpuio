use super::*;
use gpuio_protocol::input_content_hint::{Hint, Status, Unavailability};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Desired {
    pub node: NodeId,
    pub focus: gpui::WeakFocusHandle,
    pub hint: Hint,
}
#[derive(Default)]
pub(super) struct State {
    desired: Option<Desired>,
    #[cfg(target_os = "macos")]
    binding: Option<
        Result<crate::input_content_macos::Binding, crate::input_content_macos::Unavailable>,
    >,
}
impl State {
    pub(super) fn clear(&mut self) {
        #[cfg(target_os = "macos")]
        if let Some(Ok(binding)) = &mut self.binding {
            binding.clear();
        }
        self.desired = None;
    }
    fn update(&mut self, desired: Option<Desired>, window: &Window) {
        #[cfg(target_os = "macos")]
        {
            let value = desired.as_ref().and_then(|d| d.hint.macos_value());
            if value.is_some() && self.binding.is_none() {
                self.binding = Some(crate::input_content_macos::Binding::for_window(window));
            }
            if let Some(Ok(binding)) = &mut self.binding {
                if value.is_none() {
                    binding.clear();
                } else if self.desired != desired || !binding.is_current() {
                    // The public status query will distinguish exposure failure;
                    // editing itself must remain usable without native hints.
                    let _ = binding.set(value);
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = window;
        self.desired = desired;
    }
}
impl View {
    pub(super) fn read_input_content_status(
        &mut self,
        node: NodeId,
        window: &Window,
        cx: &App,
    ) -> EditorResult {
        let hint = {
            let session = self.session.borrow();
            let Some(item) = session.tree(self.id).and_then(|tree| tree.get(node)) else {
                return EditorResult::Failed(EditorError::StaleEditor);
            };
            if !self.editors.contains_key(&node) {
                return EditorResult::Failed(EditorError::StaleEditor);
            }
            item.editor_content_hint
        };
        self.sync_input_content(window, cx);
        let status = match self
            .input_content
            .desired
            .as_ref()
            .filter(|owner| owner.node == node)
        {
            None => Status::Inactive(hint),
            Some(owner) => {
                #[cfg(not(target_os = "macos"))]
                {
                    Status::Unavailable(owner.hint, Unavailability::Backend)
                }
                #[cfg(target_os = "macos")]
                {
                    if owner.hint.macos_value().is_none() {
                        Status::Unavailable(owner.hint, Unavailability::Mapping)
                    } else {
                        use crate::input_content_macos::Unavailable;
                        match &self.input_content.binding {
                            Some(Ok(binding)) if binding.is_current() => {
                                Status::Exposed(owner.hint)
                            }
                            Some(Err(Unavailable::Protocol)) => {
                                Status::Unavailable(owner.hint, Unavailability::Backend)
                            }
                            _ => Status::Unavailable(owner.hint, Unavailability::NativeView),
                        }
                    }
                }
            }
        };
        EditorResult::ContentHintStatus(status)
    }
    pub(super) fn sync_input_content(&mut self, window: &Window, cx: &App) {
        let desired = self.editors.iter().find_map(|(node, editor)| {
            let hint = editor.content_hint()?;
            let focus = editor.focus_handle(cx);
            (focus.is_focused(window)
                && self.focus.borrow().allows(*node)
                && self.focus.borrow().visible(*node))
            .then(|| Desired {
                node: *node,
                focus: focus.downgrade(),
                hint,
            })
        });
        self.input_content.update(desired, window);
    }
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "input_content_test.rs"]
mod tests;
