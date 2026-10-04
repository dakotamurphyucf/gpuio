//! Paint-only caret timing. One cancellable task, with weak lifetime captures.
use super::*;
use std::{rc::Rc, time::Duration};

pub(super) const INTERVAL: Duration = Duration::from_millis(500);

pub(super) struct Cursor {
    pub visible: bool,
    pub in_view: bool,
    token: Option<Rc<()>>,
    task: Option<Task<()>>,
}
impl Default for Cursor {
    fn default() -> Self {
        Self {
            visible: true,
            in_view: false,
            token: None,
            task: None,
        }
    }
}
impl Cursor {
    /// Reset phase without altering text, selection, layout geometry or revision.
    pub(super) fn reset(&mut self) {
        self.token = None;
        self.task = None;
        self.visible = true;
    }
    #[cfg(all(test, feature = "native-image-tests"))]
    pub(super) fn running(&self) -> bool {
        self.task.is_some()
    }
}
impl Input {
    fn cursor_can_blink(&self, window: &Window, cx: &App) -> bool {
        self.cursor.in_view
            && window.is_window_active()
            && self.focus.is_focused(window)
            && self.access() == Access::Allowed
            && !self.model.config().disabled
            && !self.model.config().read_only
            && !self.model.editor().is_composing()
            && self.model.editor().selection().range().is_empty()
            && !cx.reduce_motion()
    }
    pub(super) fn sync_cursor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.cursor_can_blink(window, cx) {
            self.cursor.reset();
            return;
        }
        if self.cursor.task.is_some() {
            return;
        }
        let token = Rc::new(());
        let lease = Rc::downgrade(&token);
        self.cursor.token = Some(token);
        let weak = cx.entity().downgrade();
        self.cursor.task = Some(window.spawn(cx, async move |cx| {
            loop {
                cx.background_executor().timer(INTERVAL).await;
                let keep = cx
                    .update(|window, cx| {
                        let Some(token) = lease.upgrade() else {
                            return false;
                        };
                        weak.update(cx, |state, cx| {
                            if !state
                                .cursor
                                .token
                                .as_ref()
                                .is_some_and(|current| Rc::ptr_eq(current, &token))
                            {
                                return false;
                            }
                            if !state.cursor_can_blink(window, cx) {
                                state.cursor.reset();
                                cx.notify();
                                return false;
                            }
                            state.cursor.visible = !state.cursor.visible;
                            cx.notify();
                            true
                        })
                        .unwrap_or(false)
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
            }
        }));
    }
}
