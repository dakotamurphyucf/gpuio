//! Standalone native bars over an existing viewport. The Host must reconcile
//! live policy, finish deferred frames and close removed owners explicitly.
use crate::{
    scrollbar_clock as clock,
    scrollbar_geometry::{self as geometry, Axis, Bar},
    scrollbar_input as input,
    scrollbar_presentation::{Interaction, Resolved},
};
use gpui::{prelude::*, *};
use gpui_base::{ElementExt as _, ScrollbarHandle};
use gpuio_protocol::scrollbar::Config;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::Arc,
};

pub type Shared = Rc<RefCell<State>>;
pub type RecordFocus = Rc<dyn Fn(Axis, &FocusHandle, Bounds<Pixels>, &Window)>;
/// Native host integration only; neither hook crosses the OCaml bridge.
pub struct Hooks {
    pub allowed: Rc<dyn Fn(bool) -> bool>,
    pub record_focus: RecordFocus,
}
#[derive(Clone, Copy, Debug)]
pub struct Policy {
    pub visible: bool,
    pub enabled: bool,
    pub pointer: bool,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            visible: true,
            enabled: true,
            pointer: true,
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Viewport {
    Handle,
    Layout,
}
#[derive(Clone, Copy, Debug)]
pub enum Error {
    InvalidConfig,
    InvalidGeometry,
}
#[derive(Clone, Copy)]
struct PaintedBar {
    thumb: Bounds<Pixels>,
    hover_thumb: Option<Bounds<Pixels>>,
    track: Bounds<Pixels>,
    clip: Bounds<Pixels>,
    translation: Point<Pixels>,
}
pub struct State {
    id: ElementId,
    config: Arc<Config>,
    input: input::State,
    clocks: [clock::Owner; 2],
    focus: [FocusHandle; 2],
    policy: Policy,
    viewport: Viewport,
    bounds: Option<Bounds<Pixels>>,
    measured: Option<input::Measurement>,
    painted_bars: [Option<PaintedBar>; 2],
    last_offsets: [Option<f64>; 2],
    capture: Option<HitboxId>,
    active: bool,
    closed: bool,
    painted: bool,
    activation: Option<Subscription>,
    hooks: Option<Hooks>,
}
fn ix(axis: Axis) -> usize {
    match axis {
        Axis::Horizontal => 0,
        Axis::Vertical => 1,
    }
}
fn axes() -> [Axis; 2] {
    [Axis::Horizontal, Axis::Vertical]
}
fn get_bar(m: input::Measurement, axis: Axis) -> Option<Bar> {
    match axis {
        Axis::Horizontal => m.bars.horizontal,
        Axis::Vertical => m.bars.vertical,
    }
}
fn absolute(rect: geometry::Rect, viewport: Bounds<Pixels>) -> Bounds<Pixels> {
    Bounds {
        origin: viewport.origin + point(px(rect.origin[0] as f32), px(rect.origin[1] as f32)),
        size: size(px(rect.size[0] as f32), px(rect.size[1] as f32)),
    }
}
fn usable(bounds: Bounds<Pixels>) -> bool {
    bounds.size.width > px(0.) && bounds.size.height > px(0.)
}
/// Hover styling must not repeatedly move its own activation boundary. Grabbing
/// still uses the exact painted thumb; only hover uses this stable state union.
fn hover_thumbs(input: &input::State, resolved: &Resolved) -> [Option<Bounds<Pixels>>; 2] {
    let mut thumbs: [Option<Bounds<Pixels>>; 2] = [None; 2];
    for interaction in [
        Interaction::Rest,
        Interaction::TrackHover,
        Interaction::ThumbHover,
        Interaction::Pressed,
    ] {
        if let Ok(m) = input.preview([resolved.parts(interaction).geometry; 2]) {
            for axis in axes() {
                if let Some(bar) = get_bar(m, axis) {
                    let thumb = absolute(bar.thumb, m.viewport);
                    if usable(thumb) {
                        let previous = &mut thumbs[ix(axis)];
                        *previous = Some(previous.map_or(thumb, |p| p.union(&thumb)));
                    }
                }
            }
        }
    }
    thumbs
}
impl State {
    pub fn set_corner_peer(&mut self, peer: Option<Rc<dyn ScrollbarHandle>>) {
        self.input.set_corner_peer(peer);
    }
    pub fn new<T: 'static>(
        id: ElementId,
        config: Arc<Config>,
        handle: Rc<dyn ScrollbarHandle>,
        viewport: Viewport,
        window: &mut Window,
        cx: &mut Context<T>,
    ) -> Result<Shared, Error> {
        let parts = Resolved::new(&config, 0x000000ff).map_err(|_| Error::InvalidConfig)?;
        let input = input::State::new(
            handle,
            config.axis,
            [parts.parts(Interaction::Rest).geometry; 2],
        )
        .map_err(|_| Error::InvalidGeometry)?;
        let shared = Rc::new(RefCell::new(Self {
            id,
            input,
            clocks: [
                clock::Owner::new(config.mode, config.motion, cx)
                    .map_err(|_| Error::InvalidConfig)?,
                clock::Owner::new(config.mode, config.motion, cx)
                    .map_err(|_| Error::InvalidConfig)?,
            ],
            focus: [
                cx.focus_handle().tab_stop(false),
                cx.focus_handle().tab_stop(false),
            ],
            config,
            policy: Policy::default(),
            viewport,
            bounds: None,
            measured: None,
            painted_bars: [None; 2],
            last_offsets: [None; 2],
            capture: None,
            active: window.is_window_active(),
            closed: false,
            painted: false,
            activation: None,
            hooks: None,
        }));
        let weak = Rc::downgrade(&shared);
        shared.borrow_mut().activation =
            Some(cx.observe_window_activation(window, move |_, window, cx| {
                if let Some(shared) = weak.upgrade() {
                    let mut s = shared.borrow_mut();
                    s.active = window.is_window_active();
                    if !s.active {
                        s.suspend(window, cx);
                    }
                    window.refresh();
                }
            }));
        Ok(shared)
    }
    fn allowed(&self, pointer: bool) -> bool {
        !self.closed
            && self.active
            && self.policy.visible
            && self.policy.enabled
            && (!pointer || self.policy.pointer)
            && self.hooks.as_ref().is_none_or(|h| (h.allowed)(pointer))
    }
    pub fn set_hooks(&mut self, hooks: Hooks) {
        self.hooks = Some(hooks);
    }
    fn axis_allowed(&self, axis: Axis) -> bool {
        use gpuio_protocol::scrollbar::Axis as Selection;
        matches!(
            (self.config.axis, axis),
            (Selection::Both, _)
                | (Selection::Horizontal, Axis::Horizontal)
                | (Selection::Vertical, Axis::Vertical)
        )
    }
    pub fn reconcile(
        &mut self,
        config: Arc<Config>,
        policy: Policy,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<(), Error> {
        if !config.is_valid() {
            return Err(Error::InvalidConfig);
        }
        if self.config.axis != config.axis {
            self.cancel(window, cx);
        }
        for clock in &self.clocks {
            clock
                .set_policy(config.mode, config.motion)
                .map_err(|_| Error::InvalidConfig)?;
        }
        self.config = config;
        self.policy = policy;
        for axis in axes() {
            if !self.axis_allowed(axis) {
                self.painted_bars[ix(axis)] = None;
                self.clocks[ix(axis)].set_eligible(false);
                self.focus[ix(axis)].clone().tab_stop(false);
                if self.focus[ix(axis)].is_focused(window) {
                    window.blur(cx);
                }
            }
        }
        if !self.allowed(false) {
            self.suspend(window, cx);
        } else if !self.allowed(true) {
            self.cancel(window, cx);
            for i in 0..2 {
                self.clocks[i].set_interaction(
                    Interaction::Rest,
                    self.focus[i].is_focused(window),
                    cx,
                );
            }
        }
        Ok(())
    }
    pub fn begin_frame(&mut self) {
        self.painted = false;
        for clock in &self.clocks {
            clock.prepare_frame();
        }
    }
    pub fn finish_frame(&mut self, window: &mut Window, cx: &mut App) {
        if !self.painted {
            self.suspend(window, cx);
        }
        for i in 0..2 {
            self.clocks[i].finish_frame();
            if !self.clocks[i].is_eligible() {
                self.painted_bars[i] = None;
                self.focus[i].clone().tab_stop(false);
                if self.focus[i].is_focused(window) {
                    window.blur(cx);
                }
            }
        }
        if self
            .input
            .captured_axis()
            .is_some_and(|axis| !self.clocks[ix(axis)].accepts_pointer())
        {
            self.cancel(window, cx);
        }
    }
    pub fn focus(&self, axis: Axis) -> &FocusHandle {
        &self.focus[ix(axis)]
    }
    pub fn focused(&self, window: &Window) -> bool {
        self.focus.iter().any(|focus| focus.is_focused(window))
    }
    pub fn is_dragging(&self) -> bool {
        self.input.is_dragging()
    }
    pub fn cancel(&mut self, window: &mut Window, cx: &mut App) {
        let mut changed = self.input.cancel() || self.capture.is_some();
        for i in 0..2 {
            if self.clocks[i].interaction() == Interaction::Pressed {
                self.clocks[i].set_interaction(
                    Interaction::Rest,
                    self.focus[i].is_focused(window),
                    cx,
                );
                changed = true;
            }
        }
        if let Some(hitbox) = self.capture.take()
            && window.captured_hitbox() == Some(hitbox)
        {
            window.release_pointer();
        }
        if changed {
            window.refresh();
        }
    }
    fn suspend(&mut self, window: &mut Window, cx: &mut App) {
        self.cancel(window, cx);
        self.measured = None;
        self.painted_bars = [None; 2];
        self.last_offsets = [None; 2];
        for i in 0..2 {
            self.clocks[i].set_eligible(false);
            self.focus[i].clone().tab_stop(false);
            if self.focus[i].is_focused(window) {
                window.blur(cx);
            }
        }
    }
    pub fn close(&mut self, window: &mut Window, cx: &mut App) {
        self.suspend(window, cx);
        self.closed = true;
        self.activation = None;
        self.hooks = None;
        self.input.close();
        for c in &self.clocks {
            c.close();
        }
    }
    fn adjust(
        &mut self,
        axis: Axis,
        adjustment: input::Adjustment,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        if !self.allowed(false)
            || !self.axis_allowed(axis)
            || !self.clocks[ix(axis)].is_eligible()
            || self.measured.and_then(|m| get_bar(m, axis)).is_none()
        {
            return false;
        }
        let changed = self.input.adjust(axis, adjustment);
        if !self.input.is_dragging() {
            self.cancel(window, cx);
        }
        if changed {
            self.clocks[ix(axis)].activity(cx);
            window.refresh();
        }
        changed
    }
}

/// The foreground supplies themed defaults; explicit config colors remain exact.
/// Call begin_frame/finish_frame around the caller's complete deferred frame.
pub fn element(shared: &Shared, foreground: u32) -> AnyElement {
    build_element(shared, Some(foreground), 1.)
}
/// For a bar painted as an ordinary child of its styled viewport.
pub fn inherited_element(shared: &Shared) -> AnyElement {
    build_element(shared, None, 1.)
}
/// For an overlay outside the viewport's scroll transform and style scope.
pub fn overlay(shared: &Shared, foreground: u32, opacity: f32) -> AnyElement {
    build_element(shared, Some(foreground), opacity)
}
fn build_element(shared: &Shared, foreground: Option<u32>, opacity: f32) -> AnyElement {
    let weak = Rc::downgrade(shared);
    let bounds_state = weak.clone();
    let id = shared.borrow().id.clone();
    div()
        .id(id)
        .opacity(opacity)
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .overflow_hidden()
        .on_prepaint(move |bounds, _, _| {
            if let Some(s) = bounds_state.upgrade() {
                s.borrow_mut().bounds = Some(bounds);
            }
        })
        .child(container_query(move |_, window, cx| {
            let mut root = div().relative().size_full();
            let Some(shared) = weak.upgrade() else {
                return root;
            };
            let mut s = shared.borrow_mut();
            let Some(bounds) = s.bounds else {
                return root;
            };
            if !s.allowed(false) || !usable(bounds.intersect(&window.content_mask().bounds)) {
                s.suspend(window, cx);
                return root;
            }
            let foreground =
                foreground.unwrap_or_else(|| u32::from(window.text_style().color.to_rgb()));
            let resolved =
                Resolved::new(&s.config, foreground).expect("validated scrollbar config");
            let mut styles =
                axes().map(|axis| resolved.parts(s.clocks[ix(axis)].interaction()).geometry);
            let viewport = match s.viewport {
                Viewport::Handle => None,
                Viewport::Layout => Some(bounds),
            };
            let policy = input::Policy {
                enabled: s.allowed(false),
                pointer: s.allowed(true),
            };
            let selection = s.config.axis;
            if s.input
                .reconcile(selection, styles, viewport, policy)
                .is_err()
            {
                s.suspend(window, cx);
                return root;
            }
            let Ok(measured) = s.input.measurement() else {
                s.suspend(window, cx);
                return root;
            };
            if !s.input.is_dragging() {
                s.cancel(window, cx);
            }
            let mut frames = [None, None];
            for axis in axes() {
                let i = ix(axis);
                let eligible = get_bar(measured, axis).is_some_and(|bar| {
                    usable(
                        absolute(bar.envelope, measured.viewport)
                            .intersect(&bounds)
                            .intersect(&window.content_mask().bounds),
                    )
                });
                s.clocks[i].set_eligible(eligible);
                s.focus[i].clone().tab_stop(eligible);
                if !eligible {
                    if s.input.captured_axis() == Some(axis) {
                        s.cancel(window, cx);
                    }
                    s.painted_bars[i] = None;
                    if s.focus[i].is_focused(window) {
                        window.blur(cx);
                    }
                    continue;
                }
                let offset = get_bar(measured, axis).unwrap().offset;
                if s.last_offsets[i].is_some_and(|old| old != offset) {
                    s.clocks[i].activity(cx);
                }
                s.last_offsets[i] = Some(offset);
                s.clocks[i].set_interaction(
                    s.clocks[i].interaction(),
                    s.focus[i].is_focused(window),
                    cx,
                );
                let driver = s.clocks[i].driver();
                if let Ok(Some(frame)) =
                    driver.sample(styles[i].track_width, styles[i].thumb_width, cx)
                {
                    styles[i].track_width = frame.visual().track_width;
                    styles[i].thumb_width = frame.visual().thumb_width;
                    frames[i] = Some((driver, frame));
                }
            }
            if s.input
                .reconcile(selection, styles, viewport, policy)
                .is_err()
            {
                s.suspend(window, cx);
                return root;
            }
            let Ok(measured) = s.input.measurement() else {
                s.suspend(window, cx);
                return root;
            };
            s.measured = Some(measured);
            let hover_thumbs = hover_thumbs(&s.input, &resolved);
            if !s.input.is_dragging() {
                s.cancel(window, cx);
            }
            for axis in axes() {
                if let (Some(bar), Some((driver, frame))) =
                    (get_bar(measured, axis), frames[ix(axis)].take())
                {
                    let parts = resolved.parts(s.clocks[ix(axis)].interaction()).clone();
                    root = root.child(range(
                        &shared,
                        &s,
                        axis,
                        bar,
                        hover_thumbs[ix(axis)],
                        measured.viewport,
                        bounds,
                        parts,
                        driver,
                        frame,
                        window,
                    ));
                }
            }
            root
        }))
        .into_any_element()
}

#[expect(
    clippy::too_many_arguments,
    reason = "one measured range with native paint and owner context"
)]
fn range(
    shared: &Shared,
    s: &State,
    axis: Axis,
    bar: Bar,
    hover_thumb: Option<Bounds<Pixels>>,
    viewport: Bounds<Pixels>,
    root: Bounds<Pixels>,
    parts: crate::scrollbar_presentation::Parts,
    driver: clock::Driver,
    frame: crate::scrollbar_lifecycle::Frame,
    window: &Window,
) -> AnyElement {
    let envelope = absolute(bar.envelope, viewport)
        .intersect(&root)
        .intersect(&window.content_mask().bounds);
    let local = envelope.origin - root.origin;
    let focus = s.focus[ix(axis)].clone();
    let label = format!(
        "{} — {}",
        s.config.label,
        match axis {
            Axis::Horizontal => "horizontal",
            Axis::Vertical => "vertical",
        }
    );
    let mut target = div()
        .id(("scrollbar-range", ix(axis)))
        .absolute()
        .left(local.x)
        .top(local.y)
        .w(envelope.size.width)
        .h(envelope.size.height)
        .track_focus(&focus)
        .tab_index(0)
        .role(Role::ScrollBar)
        .aria_label(label)
        .aria_numeric_value(bar.offset)
        .aria_min_numeric_value(0.)
        .aria_max_numeric_value(bar.max_offset)
        .aria_numeric_value_step(f64::from(f32::from(window.line_height())))
        .aria_orientation(match axis {
            Axis::Horizontal => accesskit::Orientation::Horizontal,
            Axis::Vertical => accesskit::Orientation::Vertical,
        });
    let weak = Rc::downgrade(shared);
    let key_focus = focus.clone();
    target = target.on_key_down(move |event, window, cx| {
        if !key_focus.is_focused(window) || event.keystroke.modifiers.modified() {
            return;
        }
        let Some(shared) = weak.upgrade() else {
            return;
        };
        let mut s = shared.borrow_mut();
        if !s.allowed(false) {
            return;
        }
        if event.keystroke.key == "escape" && s.input.is_dragging() {
            s.cancel(window, cx);
            window.refresh();
            cx.stop_propagation();
            return;
        }
        use input::{Adjustment as A, Direction as D};
        let command = match (axis, event.keystroke.key.as_str()) {
            (_, "home") => A::First,
            (_, "end") => A::Last,
            (_, "pageup") => A::Page(D::Backward),
            (_, "pagedown") => A::Page(D::Forward),
            (Axis::Horizontal, "left") | (Axis::Vertical, "up") => A::Line {
                direction: D::Backward,
                height: f64::from(f32::from(window.line_height())),
            },
            (Axis::Horizontal, "right") | (Axis::Vertical, "down") => A::Line {
                direction: D::Forward,
                height: f64::from(f32::from(window.line_height())),
            },
            _ => return,
        };
        s.adjust(axis, command, window, cx);
        cx.stop_propagation();
    });
    for action in [
        AccessibleAction::Focus,
        AccessibleAction::Increment,
        AccessibleAction::Decrement,
        AccessibleAction::SetValue,
    ] {
        let weak = Rc::downgrade(shared);
        let focus = focus.clone();
        target = target.on_a11y_action(action, move |data, window, cx| {
            let Some(shared) = weak.upgrade() else {
                return;
            };
            let mut s = shared.borrow_mut();
            if !s.allowed(false)
                || !s.axis_allowed(axis)
                || !s.clocks[ix(axis)].is_eligible()
                || s.measured.and_then(|m| get_bar(m, axis)).is_none()
            {
                return;
            }
            if action == AccessibleAction::Focus {
                window.focus(&focus, cx);
                window.refresh();
                return;
            }
            let command = match action {
                AccessibleAction::Increment => Some(input::Adjustment::Line {
                    direction: input::Direction::Forward,
                    height: f64::from(f32::from(window.line_height())),
                }),
                AccessibleAction::Decrement => Some(input::Adjustment::Line {
                    direction: input::Direction::Backward,
                    height: f64::from(f32::from(window.line_height())),
                }),
                AccessibleAction::SetValue => {
                    if let Some(accesskit::ActionData::NumericValue(value)) = data {
                        Some(input::Adjustment::Set(*value))
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(command) = command {
                s.adjust(axis, command, window, cx);
            }
        });
    }
    let weak = Rc::downgrade(shared);
    target = target.child(
        canvas(
            |_, _, _| (),
            move |_, _, window, cx| {
                let Some(shared) = weak.upgrade() else {
                    return;
                };
                if !shared.borrow().allowed(false) {
                    return;
                }
                let visual = frame.visual();
                let translation = match axis {
                    Axis::Horizontal => {
                        point(px(0.), px((visual.slide * bar.envelope.size[1]) as f32))
                    }
                    Axis::Vertical => {
                        point(px((visual.slide * bar.envelope.size[0]) as f32), px(0.))
                    }
                };
                let track = absolute(bar.track, viewport) + translation;
                let thumb = absolute(bar.thumb, viewport) + translation;
                let opacity = visual.opacity as f32;
                let clip = window.content_mask().bounds.intersect(&viewport);
                window.with_content_mask(Some(ContentMask { bounds: viewport }), |window| {
                    if usable(track) {
                        window
                            .paint_quad(fill(track, rgba(parts.track_background).opacity(opacity)));
                        if parts.track_border & 255 != 0 {
                            window.paint_quad(outline(
                                track,
                                rgba(parts.track_border).opacity(opacity),
                                BorderStyle::default(),
                            ));
                        }
                    }
                    if usable(thumb) {
                        let mut style = StyleRefinement::default();
                        crate::style::refine(
                            &mut style,
                            &[gpuio_protocol::v1::Field::Background(
                                parts.thumb_background,
                            )],
                        );
                        let Fill::Color(background) =
                            style.background.expect("resolved scrollbar fill");
                        let background = background.opacity(opacity);
                        window.paint_quad(quad(
                            thumb,
                            px(bar.radius as f32),
                            background,
                            px(0.),
                            transparent_black(),
                            BorderStyle::default(),
                        ));
                    }
                    if shared.borrow().focus[ix(axis)].is_focused(window) {
                        window.paint_quad(outline(
                            absolute(bar.envelope, viewport),
                            window.text_style().color.opacity(opacity),
                            BorderStyle::default(),
                        ));
                    }
                });
                if driver.painted(&frame, window, cx) {
                    let mut s = shared.borrow_mut();
                    s.painted = true;
                    s.painted_bars[ix(axis)] = Some(PaintedBar {
                        thumb,
                        hover_thumb,
                        track,
                        clip,
                        translation,
                    });
                    let record = s.hooks.as_ref().map(|h| h.record_focus.clone());
                    let focus = s.focus[ix(axis)].clone();
                    drop(s);
                    if let Some(record) = record {
                        record(axis, &focus, envelope, window);
                    }
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full(),
    );
    Region {
        element: target,
        state: Rc::downgrade(shared),
        axis,
    }
    .into_any_element()
}

struct Region {
    element: Stateful<Div>,
    state: Weak<RefCell<State>>,
    axis: Axis,
}
impl IntoElement for Region {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Region {
    type RequestLayoutState = <Stateful<Div> as Element>::RequestLayoutState;
    type PrepaintState = Hitbox;
    fn id(&self) -> Option<ElementId> {
        Element::id(&self.element)
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.element.source_location()
    }
    fn a11y_role(&self) -> Option<accesskit::Role> {
        self.element.a11y_role()
    }
    fn write_a11y_info(&self, node: &mut accesskit::Node) {
        self.element.write_a11y_info(node);
    }
    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        w: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.element.request_layout(id, inspector, w, cx)
    }
    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        w: &mut Window,
        cx: &mut App,
    ) -> Hitbox {
        let hitbox = w.insert_hitbox(bounds, HitboxBehavior::Normal);
        if let Some(shared) = self.state.upgrade() {
            let mut s = shared.borrow_mut();
            if s.input.captured_axis() == Some(self.axis) {
                if s.capture.is_some_and(|id| w.captured_hitbox() == Some(id)) && s.allowed(true) {
                    w.capture_pointer(hitbox.id);
                    s.capture = Some(hitbox.id);
                } else {
                    s.cancel(w, cx);
                }
            }
        }
        self.element.prepaint(id, inspector, bounds, layout, w, cx);
        hitbox
    }
    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        layout: &mut Self::RequestLayoutState,
        hitbox: &mut Hitbox,
        w: &mut Window,
        cx: &mut App,
    ) {
        // This adapter owns pointer routing. Keep Div's native focus, Tab and
        // AX registration but omit its default mouse-focus listener. Preventing
        // defaults across the entire envelope would steal clicks from content
        // beneath a hidden bar or the unused portion of a narrow painted track.
        self.element
            .paint(id, inspector, bounds, layout, &mut None, w, cx);
        let weak = self.state.clone();
        let axis = self.axis;
        let hit = hitbox.clone();
        w.on_mouse_event(move |event: &MouseDownEvent, phase, w, cx| {
            if !phase.bubble() || event.button != MouseButton::Left || !hit.is_hovered(w) {
                return;
            }
            let Some(shared) = weak.upgrade() else {
                return;
            };
            let mut s = shared.borrow_mut();
            if !s.allowed(true) || !s.axis_allowed(axis) || !s.clocks[ix(axis)].accepts_pointer() {
                return;
            }
            let Some(painted) = s.painted_bars[ix(axis)] else {
                return;
            };
            if !painted.clip.contains(&event.position) {
                return;
            }
            let canonical = event.position - painted.translation;
            let position = [
                f64::from(f32::from(canonical.x)),
                f64::from(f32::from(canonical.y)),
            ];
            if usable(painted.thumb) && painted.thumb.contains(&event.position) {
                if !s.input.begin_drag(axis, position) {
                    return;
                }
                w.capture_pointer(hit.id);
                s.capture = Some(hit.id);
                s.clocks[ix(axis)].set_interaction(
                    Interaction::Pressed,
                    s.focus[ix(axis)].is_focused(w),
                    cx,
                );
            } else if usable(painted.track) && painted.track.contains(&event.position) {
                s.input.track_click(axis, position);
                s.cancel(w, cx);
            } else {
                return;
            }
            s.clocks[ix(axis)].activity(cx);
            w.prevent_default();
            w.refresh();
            cx.stop_propagation();
        });
        let weak = self.state.clone();
        let hit = hitbox.clone();
        w.on_mouse_event(move |event: &MouseMoveEvent, phase, w, cx| {
            if !phase.capture() {
                return;
            }
            let Some(shared) = weak.upgrade() else {
                return;
            };
            let mut s = shared.borrow_mut();
            if s.input.captured_axis() == Some(axis) {
                if !s.allowed(true)
                    || s.capture.is_none_or(|id| w.captured_hitbox() != Some(id))
                    || event.pressed_button != Some(MouseButton::Left)
                {
                    s.cancel(w, cx);
                    return;
                }
                if s.input.drag_to([
                    f64::from(f32::from(event.position.x)),
                    f64::from(f32::from(event.position.y)),
                ]) {
                    s.clocks[ix(axis)].activity(cx);
                    w.refresh();
                }
                if !s.input.is_dragging() {
                    s.cancel(w, cx);
                }
                cx.stop_propagation();
                return;
            }
            let hovered = s.allowed(true) && s.axis_allowed(axis) && hit.is_hovered(w);
            let thumb = hovered
                && s.painted_bars[ix(axis)].is_some_and(|p| {
                    p.hover_thumb.is_some_and(|bounds| {
                        let bounds = if s.clocks[ix(axis)].accepts_pointer() {
                            bounds + p.translation
                        } else {
                            bounds
                        };
                        bounds.contains(&event.position) && p.clip.contains(&event.position)
                    })
                });
            let interaction = if thumb {
                Interaction::ThumbHover
            } else if hovered {
                Interaction::TrackHover
            } else {
                Interaction::Rest
            };
            let before = s.clocks[ix(axis)].interaction();
            s.clocks[ix(axis)].set_interaction(interaction, s.focus[ix(axis)].is_focused(w), cx);
            if before != s.clocks[ix(axis)].interaction() {
                w.refresh();
            }
        });
        let weak = self.state.clone();
        w.on_mouse_event(move |event: &MouseUpEvent, phase, w, cx| {
            if !phase.bubble() || event.button != MouseButton::Left {
                return;
            }
            let Some(shared) = weak.upgrade() else {
                return;
            };
            let mut s = shared.borrow_mut();
            if s.input.captured_axis() != Some(axis) {
                return;
            }
            if s.allowed(true)
                && s.axis_allowed(axis)
                && s.capture.is_some_and(|id| w.captured_hitbox() == Some(id))
            {
                s.input.drag_to([
                    f64::from(f32::from(event.position.x)),
                    f64::from(f32::from(event.position.y)),
                ]);
            }
            s.cancel(w, cx);
            s.clocks[ix(axis)].set_interaction(
                Interaction::Rest,
                s.focus[ix(axis)].is_focused(w),
                cx,
            );
            w.refresh();
            cx.stop_propagation();
        });
    }
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "scrollbar_widget_test.rs"]
mod test;
