//! Native color ownership: channel gestures mutate the color policy directly.
//! No quantized slider model or OCaml paint/layout callback sits in between.
use super::{SharedSession, View, focus, pointer_enabled};
use crate::{
    color_input_state::{Access, State},
    transport::Transport,
};
use gpui::{prelude::*, *};
use gpuio_protocol::{
    HandlerId, NodeId, WindowId, color_input as c,
    color_presentation::Panel,
    color_value::{AlphaPolicy, Rgba, Value},
};
use std::sync::Arc;

#[path = "color_input_channels.rs"]
mod channels;
#[path = "color_input_editors.rs"]
mod editors;
#[path = "color_input_palette.rs"]
mod palette;
#[path = "color_input_panels.rs"]
mod panels;
#[path = "color_input_preview.rs"]
mod preview;
const CHANNELS: [c::Channel; 4] = [
    c::Channel::Hue,
    c::Channel::Saturation,
    c::Channel::Lightness,
    c::Channel::Alpha,
];

// Fixed-size paint work, independent of the display scale and palette size.
// Empty has no fill; a transparent concrete color still shows the checkerboard.
fn swatch(value: Value, radius: f32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            if let Value::Color(color) = value {
                if color.alpha < 255 {
                    for row in 0..4 {
                        for column in 0..4 {
                            let cell = Bounds::new(
                                bounds.origin
                                    + point(
                                        bounds.size.width * (column as f32 / 4.),
                                        bounds.size.height * (row as f32 / 4.),
                                    ),
                                size(bounds.size.width / 4., bounds.size.height / 4.),
                            );
                            let mut quad = fill(
                                bounds,
                                rgba(if (row + column) % 2 == 0 {
                                    0xeeeeeeff
                                } else {
                                    0x999999ff
                                }),
                            );
                            // Clip the complete rounded swatch to each tile;
                            // rounding just the corner tile fails when the
                            // requested radius extends into adjacent tiles.
                            quad.corner_radii = px(radius).into();
                            window.with_content_mask(Some(ContentMask { bounds: cell }), |w| {
                                w.paint_quad(quad);
                            });
                        }
                    }
                }
                let mut quad = fill(bounds, rgba(color.packed() as u32));
                quad.corner_radii = px(radius).into();
                window.paint_quad(quad);
            }
        },
    )
    .absolute()
    .size_full()
}

struct Route {
    window: WindowId,
    node: NodeId,
    handler: HandlerId,
    session: SharedSession,
    gate: focus::Shared,
    transport: Arc<Transport>,
}
impl Route {
    fn current(&self, config: &c::Config) -> bool {
        let session = self.session.borrow();
        session.accepts_input(self.window)
            && session
                .tree(self.window)
                .and_then(|t| t.get(self.node))
                .is_some_and(|n| {
                    n.handler == Some(self.handler)
                        && n.color_input
                            .as_ref()
                            .is_some_and(|m| m.config.as_ref() == config)
                })
    }
    fn fault(&self) {
        if self.session.borrow_mut().overload(self.window) {
            self.transport.fault(self.window);
        }
    }
    fn emit(&self, events: Vec<c::Event>) -> bool {
        if events.is_empty() {
            return true;
        }
        let routed = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.window) else {
                return false;
            };
            events
                .into_iter()
                .map(|event| {
                    session.color_input_event(
                        self.window,
                        self.node,
                        self.handler,
                        tree.revision(),
                        event,
                    )
                })
                .collect::<Option<Vec<_>>>()
        };
        let Some(events) = routed else {
            return false;
        };
        let success = self.transport.color_batch(events);
        if !success {
            self.fault();
        }
        success
    }
}

struct ColorInput {
    model: State,
    route: Route,
    channel_focus: [FocusHandle; 4],
    palette_focus: Vec<FocusHandle>,
    clear_focus: FocusHandle,
    tracks: [Bounds<Pixels>; 4],
    hitboxes: [Option<HitboxId>; 4],
    capture: Option<(usize, HitboxId)>,
    palette_preview: Option<(usize, Rgba)>,
    presentation: Arc<gpuio_protocol::color_presentation::Presentation>,
    panel: Panel,
    tab_focus: [FocusHandle; 2],
    pointer: bool,
    closed: bool,
    metadata: Option<Arc<gpuio_protocol::accessibility::Config>>,
    editors: editors::Editors,
    #[cfg(feature = "native-tests")]
    painted_font: Pixels,
}
impl ColorInput {
    fn access(&self, pointer: bool) -> Access {
        if !self.closed
            && self.route.current(self.model.config())
            && self.route.gate.borrow().allows(self.route.node)
            && (!pointer
                || self.pointer
                    && self
                        .route
                        .session
                        .borrow()
                        .tree(self.route.window)
                        .is_some_and(|t| pointer_enabled(t, self.route.node)))
        {
            Access::Allowed
        } else {
            Access::Blocked
        }
    }
    fn publish(
        &mut self,
        result: Result<Vec<c::Event>, c::Error>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match result {
            Ok(events) => {
                if events.is_empty() {
                    return;
                }
                self.clear_palette_preview(cx);
                self.editor_events(&events);
                if !self.route.emit(events) {
                    self.model.fault();
                    self.release(window);
                }
                self.sync_editors(window, cx);
                cx.notify();
            }
            Err(c::Error::LimitExceeded | c::Error::NativeFailure) => {
                self.clear_palette_preview(cx);
                self.model.fault();
                self.release(window);
                self.route.fault();
                self.sync_editors(window, cx);
                cx.notify();
            }
            Err(_) => (),
        }
    }
    fn release(&mut self, window: &mut Window) {
        if let Some((_, hitbox)) = self.capture.take()
            && window.captured_hitbox() == Some(hitbox)
        {
            window.release_pointer();
        }
    }
    fn cancel(
        &mut self,
        reason: c::CancelReason,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.clear_palette_preview(cx);
        self.release(window);
        let result = self.model.cancel(reason);
        let changed = matches!(result, Ok(Some(_)));
        self.publish(result.map(|event| event.into_iter().collect()), window, cx);
        changed
    }
    fn focused(&self, window: &Window) -> bool {
        self.channel_focus
            .iter()
            .chain(&self.palette_focus)
            .chain(&self.tab_focus)
            .chain(std::iter::once(&self.clear_focus))
            .chain(self.editors.fields.iter().map(|editor| &editor.focus))
            .any(|f| f.is_focused(window))
    }
    fn hide(&mut self, reason: c::CancelReason, window: &mut Window, cx: &mut Context<Self>) {
        self.cancel(reason, window, cx);
        self.sync_editors(window, cx);
        if self.focused(window) {
            window.blur(cx);
        }
    }
    fn choose(
        &mut self,
        value: Value,
        source: c::Source,
        pointer: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if source == c::Source::Palette && !self.shows_panel(Panel::Palette) {
            return;
        }
        let result = self.model.choose(value, source, self.access(pointer));
        if result.is_ok() {
            self.release(window);
        }
        self.publish(result, window, cx);
    }
    fn record(
        &self,
        focus: &FocusHandle,
        part: u16,
        enabled: bool,
        editor: bool,
    ) -> impl IntoElement + use<> {
        let focus = focus.clone();
        let gate = self.route.gate.clone();
        let node = self.route.node;
        canvas(
            |_, _, _| (),
            move |bounds, _, window, _| {
                if bounds.size.width > px(0.) && bounds.size.height > px(0.) {
                    let mut gate = gate.borrow_mut();
                    if editor && focus.is_focused(window) {
                        gate.remember_command_target(node);
                    }
                    gate.record_part(
                        node,
                        part,
                        super::focus::Target {
                            handle: focus.clone(),
                            tab_stop: enabled,
                            bounds,
                        },
                        focus.is_focused(window),
                    );
                }
            },
        )
        .absolute()
        .size_full()
    }
}

impl Render for ColorInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.validate_palette_preview(cx);
        self.sync_editors(window, cx);
        let config = self.model.config();
        let snapshot = self.model.snapshot();
        let enabled = !config.disabled && self.access(false) == Access::Allowed;
        let editable = enabled && !config.read_only;
        let mut root = div()
            .id("color-input")
            .relative()
            .w_full()
            .min_w(px(0.))
            .flex()
            .flex_col()
            .gap(px(self.presentation.control_gap as f32))
            .p(px(self.presentation.padding as f32))
            .role(Role::Group)
            .aria_label(config.labels.control.clone())
            .aria_value(match snapshot.value {
                Value::Empty => String::new(),
                Value::Color(color) => color.to_hex(),
            });
        root = root.child(
            div()
                .flex()
                .items_center()
                .gap(px(8.))
                .child(
                    div()
                        .relative()
                        .w(px(30.))
                        .h(px(30.))
                        .flex_shrink_0()
                        .rounded(px(5.))
                        .overflow_hidden()
                        .border_1()
                        .border_color(rgba(0x80808080))
                        .child(swatch(self.displayed_color(), 4.)),
                )
                .child(
                    div()
                        .min_w(px(0.))
                        .flex_1()
                        .child(self.editor_element(0, window)),
                ),
        );
        // A permanently reserved caption line avoids shifting the channels or
        // palette when hover starts or ends. It is paint-only inspection, not
        // an editor draft or an accessible selected value.
        root = root.child(crate::semantics::State {
            identity: None,
            busy: false,
            element: div()
                .id("palette-preview-caption")
                .h(window.line_height())
                .overflow_hidden()
                .text_ellipsis()
                .child(
                    self.palette_preview
                        .map_or_else(String::new, |(_, c)| c.to_hex()),
                ),
            metadata: None,
            live: None,
            hidden: true,
            disabled: false,
            read_only: false,
            modal: false,
        });
        if let Some(tabs) = self.tab_bar(window, cx) {
            root = root.child(tabs);
        }
        if self.shows_panel(Panel::Channels) {
            let mut channels = div()
                .id("color-channels")
                .flex()
                .flex_col()
                .gap(px(self.presentation.control_gap as f32));
            if let Some(label) = self.panel_label(Panel::Channels) {
                channels = channels.role(Role::TabPanel).aria_label(label.to_owned());
            }
            for (index, channel) in CHANNELS.into_iter().enumerate() {
                channels = channels.child(self.channel(index, channel, window, cx));
            }
            root = root.child(channels);
        }
        if self.shows_panel(Panel::Palette) {
            let mut palette = div().id("color-palette").child(self.palette(window, cx));
            if let Some(label) = self.panel_label(Panel::Palette) {
                palette = palette.role(Role::TabPanel).aria_label(label.to_owned());
            }
            root = root.child(palette);
        }
        let clear_focus = self
            .clear_focus
            .clone()
            .tab_stop(enabled && config.allow_empty);
        let mut clear = div()
            .id("clear")
            .relative()
            .px(px(8.))
            .py(px(5.))
            .rounded(px(4.))
            .border_1()
            .border_color(rgba(0x80808080))
            .role(Role::Button)
            .aria_label(config.labels.clear.clone())
            .when(enabled && config.allow_empty, |this| {
                this.track_focus(&clear_focus)
            })
            .child(config.labels.clear.clone())
            .child(self.record(&clear_focus, 260, enabled && config.allow_empty, false));
        if editable && config.allow_empty {
            clear = clear.on_click(cx.listener(|s, event: &ClickEvent, w, cx| {
                s.choose(
                    Value::Empty,
                    c::Source::Clear,
                    !matches!(event, ClickEvent::Keyboard(_)),
                    w,
                    cx,
                );
                cx.stop_propagation();
            }));
        }
        root = root.child(crate::semantics::State {
            identity: None,
            busy: false,
            element: clear,
            metadata: None,
            live: None,
            hidden: false,
            disabled: !enabled || !config.allow_empty,
            read_only: config.read_only,
            modal: false,
        });
        #[cfg(feature = "native-tests")]
        {
            let weak = cx.weak_entity();
            root = root.child(
                canvas(
                    |_, _, _| (),
                    move |_, _, w, cx| {
                        let _ = weak.update(cx, |s, _| {
                            s.painted_font = w.text_style().font_size.to_pixels(w.rem_size())
                        });
                    },
                )
                .absolute()
                .size_full(),
            );
        }
        crate::semantics::State {
            identity: None,
            busy: false,
            element: root,
            metadata: self.metadata.clone(),
            live: None,
            hidden: false,
            disabled: config.disabled,
            read_only: config.read_only,
            modal: false,
        }
    }
}

pub(super) struct Instance {
    state: Entity<ColorInput>,
}
impl Instance {
    fn new(
        view: &View,
        node: &crate::tree::Node,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Self, c::Error> {
        let mount = node.color_input.as_ref().expect("validated color input");
        let model = State::from_retained(mount.config.clone(), mount.initial)?;
        let route = Route {
            window: view.id,
            node: node.id,
            handler: node.handler.expect("color handler"),
            session: view.session.clone(),
            gate: view.focus.clone(),
            transport: view.transport.clone(),
        };
        if !route.emit(vec![c::Event::Observed(model.snapshot())]) {
            return Err(c::Error::NativeFailure);
        }
        let state = cx.new(|cx| ColorInput {
            palette_focus: (0..model.config().palette.len())
                .map(|_| cx.focus_handle())
                .collect(),
            model,
            route,
            channel_focus: std::array::from_fn(|_| cx.focus_handle()),
            clear_focus: cx.focus_handle(),
            tracks: [Bounds::default(); 4],
            hitboxes: [None; 4],
            capture: None,
            palette_preview: None,
            presentation: node.color_presentation.clone().unwrap_or_default(),
            panel: node
                .color_presentation
                .as_ref()
                .map_or(Panel::Palette, |p| p.panels.initial()),
            tab_focus: std::array::from_fn(|_| cx.focus_handle()),
            pointer: true,
            closed: false,
            metadata: node.accessibility.clone(),
            editors: editors::Editors::default(),
            #[cfg(feature = "native-tests")]
            painted_font: px(0.),
        });
        state.update(cx, |s, cx| s.init_editors(window, cx));
        Ok(Self { state })
    }
    pub(super) fn retained(&self, window: &Window, cx: &App) -> bool {
        let state = self.state.read(cx);
        state.focused(window) || state.model.snapshot().interaction.is_some()
    }
    pub(super) fn text_focused(&self, window: &Window, cx: &App) -> bool {
        self.state
            .read(cx)
            .editors
            .fields
            .iter()
            .any(|field| field.focus.is_focused(window))
    }
    pub(super) fn focused(&self, window: &Window, cx: &App) -> bool {
        self.state.read(cx).focused(window)
    }
    pub(super) fn element(
        &self,
        base: Stateful<Div>,
        pointer: bool,
        cx: &mut App,
    ) -> Stateful<Div> {
        self.state.update(cx, |s, cx| {
            s.pointer = pointer;
            if !pointer {
                s.clear_palette_preview(cx);
            }
        });
        base.child(self.state.clone())
    }
}
impl View {
    pub(super) fn sync_color_inputs(
        &mut self,
        dirty: &[NodeId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (nodes, present) = {
            let session = self.session.borrow();
            let tree = session.tree(self.id);
            (
                dirty
                    .iter()
                    .filter_map(|id| tree.and_then(|t| t.get(*id)))
                    .filter(|n| n.color_input.is_some())
                    .cloned()
                    .collect::<Vec<_>>(),
                self.color_inputs
                    .keys()
                    .copied()
                    .filter(|id| {
                        tree.and_then(|t| t.get(*id))
                            .is_some_and(|n| n.color_input.is_some())
                    })
                    .collect::<std::collections::BTreeSet<_>>(),
            )
        };
        self.color_inputs.retain(|id, instance| {
            if present.contains(id) {
                true
            } else {
                instance.state.update(cx, |s, cx| {
                    s.hide(c::CancelReason::Unmounted, window, cx);
                    s.model.close();
                    s.closed = true;
                });
                false
            }
        });
        for node in nodes {
            if let Some(instance) = self.color_inputs.get(&node.id) {
                instance.state.update(cx, |s, cx| {
                    let changed_handler = s.route.handler != node.handler.unwrap();
                    if changed_handler
                        || s.model.config() != node.color_input.as_ref().unwrap().config.as_ref()
                    {
                        s.clear_palette_preview(cx);
                    }
                    s.route.handler = node.handler.unwrap();
                    if changed_handler {
                        s.cancel(c::CancelReason::Interrupted, window, cx);
                    }
                    s.metadata = node.accessibility.clone();
                    let result = s
                        .model
                        .configure(node.color_input.as_ref().unwrap().config.clone());
                    if s.model.snapshot().interaction.is_none() {
                        s.release(window);
                    }
                    s.publish(result, window, cx);
                    s.configure_presentation(
                        node.color_presentation.clone().unwrap_or_default(),
                        window,
                        cx,
                    );
                    let count = s.model.config().palette.len();
                    if s.palette_focus
                        .iter()
                        .skip(count)
                        .any(|f| f.is_focused(window))
                    {
                        window.blur(cx);
                    }
                    s.palette_focus.resize_with(count, || cx.focus_handle());
                    let config = s.model.config();
                    if (config.alpha_policy == AlphaPolicy::OpaqueOnly
                        && s.channel_focus[3].is_focused(window))
                        || (!config.allow_empty && s.clear_focus.is_focused(window))
                        || s.palette_focus
                            .iter()
                            .zip(&config.palette)
                            .any(|(focus, entry)| {
                                focus.is_focused(window)
                                    && !config.allows(Value::Color(entry.color))
                            })
                    {
                        window.blur(cx);
                    }
                    if s.model.config().disabled || s.access(false) == Access::Blocked {
                        s.hide(c::CancelReason::Hidden, window, cx);
                    }
                    cx.notify();
                });
            } else {
                match Instance::new(self, &node, window, cx) {
                    Ok(instance) => {
                        self.color_inputs.insert(node.id, instance);
                    }
                    Err(_) => {
                        if self.session.borrow_mut().overload(self.id) {
                            self.transport.fault(self.id);
                        }
                    }
                }
            }
        }
    }
    pub(super) fn cancel_color_inputs(
        &self,
        reason: c::CancelReason,
        window: &mut Window,
        cx: &mut App,
    ) -> bool {
        let mut changed = false;
        for instance in self.color_inputs.values() {
            changed = instance
                .state
                .update(cx, |s, cx| s.cancel(reason, window, cx))
                || changed;
        }
        changed
    }
    pub(super) fn close_color_inputs(&mut self, window: &mut Window, cx: &mut App) {
        for instance in self.color_inputs.values() {
            instance.state.update(cx, |s, cx| {
                s.hide(c::CancelReason::Unmounted, window, cx);
                s.model.close();
                s.closed = true;
            });
        }
        self.color_inputs.clear();
    }
    pub(super) fn hide_unvisited_color_inputs(&self, window: &mut Window, cx: &mut App) {
        for (id, instance) in &self.color_inputs {
            let s = instance.state.read(cx);
            let reason = if !self.visited.contains(id) || !s.route.gate.borrow().visible(*id) {
                Some(c::CancelReason::Hidden)
            } else if s.route.gate.borrow().disabled(*id) {
                Some(c::CancelReason::Disabled)
            } else if !s.route.gate.borrow().allows(*id) {
                Some(c::CancelReason::Modal)
            } else if (s.capture.is_some() || s.palette_preview.is_some())
                && s.access(true) == Access::Blocked
            {
                Some(c::CancelReason::Interrupted)
            } else {
                None
            };
            if let Some(reason) = reason
                && (s.focused(window)
                    || s.model.snapshot().interaction.is_some()
                    || s.palette_preview.is_some())
            {
                let weak = instance.state.downgrade();
                window.defer(cx, move |w, cx| {
                    let _ = weak.update(cx, |s, cx| s.hide(reason, w, cx));
                });
            }
        }
    }
}

#[cfg(feature = "native-tests")]
#[path = "color_input_view_test.rs"]
pub(crate) mod test;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "color_input_preview_test.rs"]
mod preview_test;
