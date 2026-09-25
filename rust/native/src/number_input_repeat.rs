//! Native step-button hold lifetime. There is no timer while idle and no strong
//! owner/editor reference in the task or paint callbacks.
use super::*;
use std::time::Duration;

pub(super) const DELAY: Duration = Duration::from_millis(400);
pub(super) const INTERVAL: Duration = Duration::from_millis(75);

struct Hold {
    direction: Direction,
    hitbox: HitboxId,
    bounds: Bounds<Pixels>,
    token: Rc<()>,
}
#[derive(Default)]
pub(super) struct Repeat {
    hold: Option<Hold>,
    task: Option<Task<()>>,
    #[cfg(feature = "native-tests")]
    pub ticks: usize,
    #[cfg(feature = "native-tests")]
    pub buttons: [Bounds<Pixels>; 2],
}
impl Repeat {
    pub(super) fn is_active(&self) -> bool {
        self.hold.is_some()
    }
    #[cfg(feature = "native-tests")]
    pub(super) fn has_task(&self) -> bool {
        self.task.is_some()
    }
}
fn editor_allows(entity: &WeakEntity<InputState>, window: &Window, cx: &App) -> bool {
    entity
        .read_with(cx, |state, cx| {
            state.focus_handle(cx).is_focused(window) && state.bridge_composition().is_none()
        })
        .unwrap_or(false)
}
impl Owner {
    pub(super) fn stop_repeat(&mut self, window: &mut Window) -> bool {
        self.repeat.task = None;
        let Some(hold) = self.repeat.hold.take() else {
            return false;
        };
        if window.captured_hitbox() == Some(hold.hitbox) {
            window.release_pointer();
        }
        window.refresh();
        true
    }
    fn repeat_allowed(&self, window: &Window) -> bool {
        let config = self.model.config();
        self.current()
            && !config.disabled
            && !config.read_only
            && config.step_controls != n::StepControls::Hidden
            && window.is_window_active()
            && self.route.gate.borrow().allows(self.route.node)
            && self
                .route
                .session
                .borrow()
                .tree(self.route.window)
                .is_some_and(|tree| super::super::pointer_enabled(tree, self.route.node))
    }
    fn has_next_step(&self, direction: Direction) -> bool {
        let n::Value::Number(value) = self.model.snapshot().committed else {
            return false;
        };
        match direction {
            Direction::Increase => value < self.model.config().domain.max(),
            Direction::Decrease => value > self.model.config().domain.min(),
        }
    }
    pub(super) fn check_repeat(&mut self, window: &mut Window) {
        if self.repeat.hold.is_some() && !self.repeat_allowed(window) {
            self.stop_repeat(window);
        }
    }
}

fn begin(
    weak: &Weak<RefCell<Owner>>,
    entity: &WeakEntity<InputState>,
    direction: Direction,
    hitbox: &Hitbox,
    window: &mut Window,
    cx: &mut App,
) {
    let Some(owner) = weak.upgrade() else {
        return;
    };
    if window.default_prevented()
        || window.captured_hitbox().is_some()
        || !owner.borrow().repeat_allowed(window)
    {
        return;
    }
    perform(
        weak,
        entity,
        &n::Command::Focus,
        n::Source::Programmatic,
        window,
        cx,
    );
    let response = perform(
        weak,
        entity,
        &n::Command::Step(direction),
        n::Source::Stepper,
        window,
        cx,
    );
    window.prevent_default();
    cx.stop_propagation();
    if !matches!(response, n::Response::Applied(_)) || !owner.borrow().has_next_step(direction) {
        return;
    }
    let token = Rc::new(());
    window.capture_pointer(hitbox.id);
    owner.borrow_mut().repeat.hold = Some(Hold {
        direction,
        hitbox: hitbox.id,
        bounds: hitbox.bounds,
        token: token.clone(),
    });
    let lease = Rc::downgrade(&token);
    let weak = weak.clone();
    let entity = entity.clone();
    owner.borrow_mut().repeat.task = Some(window.spawn(cx, async move |cx| {
        let mut delay = DELAY;
        loop {
            cx.background_executor().timer(delay).await;
            let keep = cx
                .update(|window, cx| {
                    let (Some(owner), Some(token)) = (weak.upgrade(), lease.upgrade()) else {
                        return false;
                    };
                    // Motion handlers cancel on leaving the captured button.
                    // GPUI's separately polled cursor position may not belong to
                    // the event stream that initiated this captured gesture.
                    let allowed = {
                        let owner = owner.borrow();
                        owner.repeat.hold.as_ref().is_some_and(|hold| {
                            Rc::ptr_eq(&hold.token, &token)
                                && window.captured_hitbox() == Some(hold.hitbox)
                        }) && owner.repeat_allowed(window)
                            && editor_allows(&entity, window, cx)
                    };
                    if !allowed {
                        owner.borrow_mut().stop_repeat(window);
                        return false;
                    }
                    #[cfg(feature = "native-tests")]
                    {
                        owner.borrow_mut().repeat.ticks += 1;
                    }
                    let response = perform(
                        &weak,
                        &entity,
                        &n::Command::Step(direction),
                        n::Source::Stepper,
                        window,
                        cx,
                    );
                    let keep = matches!(response, n::Response::Applied(_))
                        && owner.borrow().has_next_step(direction);
                    if !keep {
                        owner.borrow_mut().stop_repeat(window);
                    }
                    keep
                })
                .unwrap_or(false);
            if !keep {
                break;
            }
            delay = INTERVAL;
        }
    }));
    window.refresh();
}

pub(super) struct Button<E> {
    pub element: E,
    pub owner: Weak<RefCell<Owner>>,
    pub entity: WeakEntity<InputState>,
    pub direction: Direction,
    pub enabled: bool,
}
impl<E: Element> IntoElement for Button<E> {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl<E: Element> Element for Button<E> {
    type RequestLayoutState = E::RequestLayoutState;
    type PrepaintState = (Hitbox, E::PrepaintState);
    fn id(&self) -> Option<ElementId> {
        self.element.id()
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.element.source_location()
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.element.request_layout(id, inspector, window, cx)
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let hitbox = window.insert_hitbox(bounds, HitboxBehavior::BlockMouse);
        if let Some(owner) = self.owner.upgrade() {
            let mut owner = owner.borrow_mut();
            #[cfg(feature = "native-tests")]
            {
                owner.repeat.buttons[if self.direction == Direction::Increase {
                    1
                } else {
                    0
                }] = hitbox.bounds;
            }
            let allowed = self.enabled
                && owner.repeat_allowed(window)
                && editor_allows(&self.entity, window, cx);
            if let Some(hold) = owner
                .repeat
                .hold
                .as_mut()
                .filter(|hold| hold.direction == self.direction)
            {
                if allowed
                    && hold.bounds == hitbox.bounds
                    && window.captured_hitbox() == Some(hold.hitbox)
                {
                    window.capture_pointer(hitbox.id);
                    hold.hitbox = hitbox.id;
                } else {
                    owner.stop_repeat(window);
                }
            }
        }
        (
            hitbox,
            self.element
                .prepaint(id, inspector, bounds, layout, window, cx),
        )
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.enabled {
            let weak = self.owner.clone();
            let entity = self.entity.clone();
            let hitbox = prepaint.0.clone();
            let direction = self.direction;
            window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                if phase.bubble() && event.button == MouseButton::Left && hitbox.is_hovered(window)
                {
                    begin(&weak, &entity, direction, &hitbox, window, cx);
                }
            });
        }
        let weak = self.owner.clone();
        let direction = self.direction;
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, _| {
            if !phase.capture() {
                return;
            }
            if let Some(owner) = weak.upgrade() {
                let mut owner = owner.borrow_mut();
                if let Some(hold) = owner
                    .repeat
                    .hold
                    .as_ref()
                    .filter(|h| h.direction == direction)
                    && (event.pressed_button != Some(MouseButton::Left)
                        || !hold.bounds.contains(&event.position)
                        || window.captured_hitbox() != Some(hold.hitbox))
                {
                    owner.stop_repeat(window);
                }
            }
        });
        self.element
            .paint(id, inspector, bounds, layout, &mut prepaint.1, window, cx);
        let weak = self.owner.clone();
        window.on_mouse_event(move |_: &MouseUpEvent, phase, window, cx| {
            if !phase.bubble() {
                return;
            }
            if let Some(owner) = weak.upgrade() {
                let held = owner
                    .borrow()
                    .repeat
                    .hold
                    .as_ref()
                    .is_some_and(|h| h.direction == direction);
                if held {
                    owner.borrow_mut().stop_repeat(window);
                    window.prevent_default();
                    cx.stop_propagation();
                }
            }
        });
    }
    fn a11y_role(&self) -> Option<accesskit::Role> {
        self.element.a11y_role()
    }
    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.element.write_a11y_info(node);
    }
    fn a11y_synthetic_children(
        &mut self,
        prepaint: &mut Self::PrepaintState,
        builder: &mut A11ySubtreeBuilder,
    ) {
        self.element
            .a11y_synthetic_children(&mut prepaint.1, builder);
    }
}
