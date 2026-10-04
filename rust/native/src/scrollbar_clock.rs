//! GPUI scheduling for one retained scrollbar axis. Rendered drivers and queued
//! work are weak; dropping Owner cancels its sole idle task and releases state.
use crate::{
    scrollbar_lifecycle::{Deadline, Error, Frame, State as Model},
    scrollbar_presentation::Interaction,
};
use gpui::{App, Task, Window};
use gpuio_protocol::scrollbar::{Mode, Motion};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

struct State {
    model: Model,
    origin: Instant,
    stamp: Rc<()>,
    deadline: Option<Deadline>,
    timer: Option<Task<()>>,
    frame_pending: bool,
    frame_needed: bool,
    painted: bool,
    #[cfg(test)]
    notifications: usize,
}
impl State {
    fn now(&self, cx: &App) -> Duration {
        cx.background_executor()
            .now()
            .saturating_duration_since(self.origin)
    }
    fn changed(&mut self) {
        self.stamp = Rc::new(());
        self.timer = None;
        self.deadline = None;
        self.frame_needed = false;
        // An old queued callback can service a subsequent accepted paint of
        // this owner. Keep its pending slot so updates cannot stack callbacks.
    }
}

/// Strong owner held only by the native viewport, never its element or timer.
pub struct Owner(Rc<RefCell<State>>);
impl Owner {
    pub fn new(mode: Mode, motion: Motion, cx: &App) -> Result<Self, Error> {
        Ok(Self(Rc::new(RefCell::new(State {
            model: Model::new(mode, motion)?,
            origin: cx.background_executor().now(),
            stamp: Rc::new(()),
            deadline: None,
            timer: None,
            frame_pending: false,
            frame_needed: false,
            painted: false,
            #[cfg(test)]
            notifications: 0,
        }))))
    }
    pub fn set_policy(&self, mode: Mode, motion: Motion) -> Result<(), Error> {
        let mut state = self.0.borrow_mut();
        if state.model.set_policy(mode, motion)? {
            state.changed();
        }
        Ok(())
    }
    pub fn set_eligible(&self, eligible: bool) {
        let mut state = self.0.borrow_mut();
        if state.model.set_eligible(eligible) {
            state.changed();
        }
    }
    pub fn set_interaction(&self, interaction: Interaction, focused: bool, cx: &App) {
        let mut state = self.0.borrow_mut();
        let now = state.now(cx);
        if state.model.set_interaction(interaction, focused, now) {
            state.changed();
        }
    }
    pub fn activity(&self, cx: &App) {
        let mut state = self.0.borrow_mut();
        let now = state.now(cx);
        if state.model.activity(now) {
            state.changed();
        }
    }
    pub fn interaction(&self) -> Interaction {
        self.0.borrow().model.interaction()
    }
    pub fn is_eligible(&self) -> bool {
        self.0.borrow().model.is_eligible()
    }
    pub fn accepts_pointer(&self) -> bool {
        self.0.borrow().model.accepts_pointer()
    }
    /// Pair before rendering and after deferred painting. Omitted/fully clipped
    /// owners must not retain their last frame's timing eligibility.
    pub fn prepare_frame(&self) {
        self.0.borrow_mut().painted = false;
    }
    pub fn finish_frame(&self) {
        if !self.0.borrow().painted {
            self.set_eligible(false);
        }
    }
    pub fn close(&self) {
        let mut state = self.0.borrow_mut();
        if state.model.close() {
            state.changed();
        }
    }
    pub fn driver(&self) -> Driver {
        Driver {
            state: Rc::downgrade(&self.0),
            stamp: Rc::downgrade(&self.0.borrow().stamp),
        }
    }
}

pub struct Driver {
    state: Weak<RefCell<State>>,
    stamp: Weak<()>,
}
impl Driver {
    fn current(&self) -> Option<Rc<RefCell<State>>> {
        let stamp = self.stamp.upgrade()?;
        let state = self.state.upgrade()?;
        if Rc::ptr_eq(&stamp, &state.borrow().stamp) && state.borrow().model.is_eligible() {
            Some(state)
        } else {
            None
        }
    }
    pub fn sample(
        &self,
        track_width: f64,
        thumb_width: f64,
        cx: &App,
    ) -> Result<Option<Frame>, Error> {
        let Some(state) = self.current() else {
            return Ok(None);
        };
        let mut state = state.borrow_mut();
        // The driver remains valid for this sample; reduced-motion changes
        // invalidate earlier model samples and cancel existing timers/frames.
        if state.model.set_reduced(cx.reduce_motion()) {
            state.timer = None;
            state.deadline = None;
            state.frame_needed = false;
        }
        state
            .model
            .sample(state.now(cx), track_width, thumb_width)
            .map(Some)
    }
    pub fn painted(&self, frame: &Frame, window: &mut Window, cx: &mut App) -> bool {
        let Some(shared) = self.current() else {
            return false;
        };
        let (queue_frame, deadline) = {
            let mut state = shared.borrow_mut();
            if !state.model.painted(frame) {
                return false;
            }
            state.painted = true;
            state.frame_needed = frame.needs_frame();
            let queue = state.frame_needed && !state.frame_pending;
            if queue {
                state.frame_pending = true;
            }
            // Use sample time: a deadline may fall between layout and paint.
            // Its zero-delay timer must still deliver the hide notification.
            (queue, state.model.deadline(frame.at()))
        };
        if queue_frame {
            let weak = Rc::downgrade(&shared);
            let view = window.current_view();
            window.on_next_frame(move |_, cx| {
                let Some(shared) = weak.upgrade() else {
                    return;
                };
                let mut state = shared.borrow_mut();
                state.frame_pending = false;
                if state.frame_needed {
                    #[cfg(test)]
                    {
                        state.notifications += 1;
                    }
                    cx.notify(view);
                }
            });
        }
        schedule(&shared, deadline, window.current_view(), window, cx);
        true
    }
}

fn schedule(
    shared: &Rc<RefCell<State>>,
    deadline: Option<Deadline>,
    view: gpui::EntityId,
    window: &mut Window,
    cx: &mut App,
) {
    let mut state = shared.borrow_mut();
    let Some(deadline) = deadline else {
        state.timer = None;
        state.deadline = None;
        return;
    };
    if state
        .deadline
        .as_ref()
        .is_some_and(|d| d.same_schedule(&deadline))
    {
        return;
    }
    state.timer = None;
    state.deadline = Some(deadline.clone());
    let delay = deadline.at().saturating_sub(state.now(cx));
    let timer = cx.background_executor().timer(delay);
    let weak = Rc::downgrade(shared);
    state.timer = Some(window.spawn(cx, async move |cx| {
        timer.await;
        let _ = cx.update(|window, cx| {
            let Some(shared) = weak.upgrade() else {
                return;
            };
            let next = {
                let mut state = shared.borrow_mut();
                if !state
                    .deadline
                    .as_ref()
                    .is_some_and(|d| d.same_schedule(&deadline))
                {
                    return;
                }
                state.timer = None;
                state.deadline = None;
                let now = state.now(cx);
                if state.model.wake(&deadline, now) {
                    #[cfg(test)]
                    {
                        state.notifications += 1;
                    }
                    cx.notify(view);
                }
                state.model.deadline(now)
            };
            // A platform timer that fires early gets the remaining interval;
            // the completed task is cleared before rearming it.
            schedule(&shared, next, view, window, cx);
        });
    }));
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "scrollbar_clock_test.rs"]
mod test;
