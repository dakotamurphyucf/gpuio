//! Bounded first-party GPUI calendar presentation over the civil-date policy.
//! The pinned base CalendarState has different range/today/ownership semantics;
//! no upstream state or synchronous OCaml day/layout callback is used here.
use super::{SharedSession, View, focus};
use crate::{
    calendar_state::{Access, Action, State},
    transport::Transport,
};
use gpui::{prelude::*, *};
use gpuio_protocol::{HandlerId, NodeId, WindowId, calendar_input as c};
use std::sync::Arc;

struct Route {
    window: WindowId,
    node: NodeId,
    handler: HandlerId,
    session: SharedSession,
    gate: focus::Shared,
    transport: Arc<Transport>,
}
impl Route {
    fn fault(&self) {
        if self.session.borrow_mut().overload(self.window) {
            self.transport.fault(self.window);
        }
    }
    fn current(&self, config: &c::Config) -> bool {
        let session = self.session.borrow();
        session.accepts_input(self.window)
            && session
                .tree(self.window)
                .and_then(|t| t.get(self.node))
                .is_some_and(|n| {
                    n.handler == Some(self.handler)
                        && n.calendar
                            .as_ref()
                            .is_some_and(|m| m.config.as_ref() == config)
                })
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
                    session.calendar_event(
                        self.window,
                        self.node,
                        self.handler,
                        tree.revision(),
                        event,
                    )
                })
                .collect::<Option<Vec<_>>>()
        };
        let Some(mut events) = routed else {
            return false;
        };
        let success = match events.len() {
            1 => self.transport.input(events.pop().unwrap()),
            2 => self
                .transport
                .calendar_completion(events.try_into().expect("two calendar events")),
            _ => false,
        };
        if !success {
            self.fault();
        }
        success
    }
}

struct Button {
    id: String,
    text: String,
    label: String,
    action: Option<Action>,
    selected: bool,
    cursor: bool,
    today: bool,
}

struct Calendar {
    model: State,
    route: Route,
    focus: FocusHandle,
    pointer: bool,
    autofocus: bool,
    metadata: Option<Arc<gpuio_protocol::accessibility::Config>>,
    _subscriptions: Vec<Subscription>,
}
impl Calendar {
    fn access(&self) -> Access {
        if self.route.current(self.model.config())
            && self.route.gate.borrow().allows(self.route.node)
        {
            Access::Allowed
        } else {
            Access::Blocked
        }
    }
    fn publish(&mut self, result: Result<Vec<c::Event>, c::Error>, cx: &mut Context<Self>) {
        match result {
            Ok(events) if !events.is_empty() => {
                self.route.emit(events);
                cx.notify();
            }
            Err(c::Error::LimitExceeded | c::Error::NativeFailure) => self.route.fault(),
            Ok(_) | Err(_) => (),
        }
    }
    fn act(&mut self, action: Action, cx: &mut Context<Self>) {
        let result = self.model.native(action, self.access());
        self.publish(result, cx);
    }
    fn focus_native(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.access() == Access::Blocked || self.model.config().disabled {
            return false;
        }
        window.focus(&self.focus, cx);
        self.focus.is_focused(window)
    }
    fn on_focus(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let result = self.model.observe_focus(true);
        self.publish(result, cx);
    }
    fn on_blur(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        let result = self.model.observe_focus(false);
        self.publish(result, cx);
    }
    fn hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.focus.is_focused(window) {
            window.blur(cx);
        }
    }
    fn key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        if modifiers.control
            || modifiers.platform
            || modifiers.alt
            || !self.focus.is_focused(window)
        {
            return;
        }
        let s = self.model.snapshot();
        let key = event.keystroke.key.as_str();
        let action = match (s.presentation, key) {
            (c::Presentation::Days, "left") => Some(Action::MoveDays(-1)),
            (c::Presentation::Days, "right") => Some(Action::MoveDays(1)),
            (c::Presentation::Days, "up") => Some(Action::MoveDays(-7)),
            (c::Presentation::Days, "down") => Some(Action::MoveDays(7)),
            (c::Presentation::Days, "home" | "end") => {
                let offset =
                    (s.focused_date.weekday() - self.model.config().first_weekday).rem_euclid(7);
                Some(Action::MoveDays(if key == "home" {
                    -offset
                } else {
                    6 - offset
                }))
            }
            (c::Presentation::Months, "left") => Some(Action::MoveMonths(-1)),
            (c::Presentation::Months, "right") => Some(Action::MoveMonths(1)),
            (c::Presentation::Months, "up") => Some(Action::MoveMonths(-3)),
            (c::Presentation::Months, "down") => Some(Action::MoveMonths(3)),
            (c::Presentation::Years, "left") => Some(Action::MoveMonths(-12)),
            (c::Presentation::Years, "right") => Some(Action::MoveMonths(12)),
            (c::Presentation::Years, "up") => Some(Action::MoveMonths(-48)),
            (c::Presentation::Years, "down") => Some(Action::MoveMonths(48)),
            (_, "pageup" | "pagedown") => {
                let delta = match s.presentation {
                    c::Presentation::Days => {
                        if modifiers.shift {
                            12
                        } else {
                            1
                        }
                    }
                    c::Presentation::Months => 12,
                    c::Presentation::Years => 240,
                };
                Some(Action::MoveMonths(if key == "pageup" {
                    -delta
                } else {
                    delta
                }))
            }
            (c::Presentation::Days, "enter" | "space") => Some(Action::Activate(s.focused_date)),
            (c::Presentation::Months, "enter" | "space") => Some(Action::ChooseMonth(s.month)),
            (c::Presentation::Years, "enter" | "space") => Some(Action::ChooseYear(s.month.year())),
            (_, "backspace" | "delete") => Some(Action::Clear),
            (_, "m") => Some(Action::SetPresentation(c::Presentation::Months)),
            (_, "y") => Some(Action::SetPresentation(c::Presentation::Years)),
            (_, "d") => Some(Action::SetPresentation(c::Presentation::Days)),
            _ => None,
        };
        if let Some(action) = action {
            self.act(action, cx);
            cx.stop_propagation();
        }
    }
    fn button(&self, button: Button, cx: &mut Context<Self>) -> AnyElement {
        let Button {
            id,
            text,
            label,
            action,
            selected,
            cursor,
            today,
        } = button;
        let enabled = action.is_some()
            && match action {
                Some(Action::Activate(date)) => self.model.config().constraints.allows(date),
                _ => true,
            }
            && !self.model.config().disabled
            && !(self.model.config().read_only
                && matches!(action, Some(Action::Activate(_) | Action::Clear)));
        let mut element = div()
            .id(SharedString::from(id))
            .flex()
            .items_center()
            .justify_center()
            .flex_1()
            .min_w(px(0.))
            .h(px(32.))
            .overflow_hidden()
            .rounded(px(6.))
            .border_1()
            .border_color(if cursor && self.model.snapshot().focused {
                rgba(0x6688ffff)
            } else if today {
                rgba(0x6688ff90)
            } else {
                rgba(0x00000000)
            })
            .role(Role::Button)
            .aria_label(label)
            .aria_selected(selected)
            .child(text);
        if selected {
            element = element.bg(rgba(0x6688ff38));
        }
        if enabled {
            element = element.hover(|s| s.bg(rgba(0x71809624)));
            if self.pointer {
                element = element.cursor_pointer();
            }
            let action = action.unwrap();
            element = element.on_mouse_down(
                MouseButton::Left,
                cx.listener(move |s, event: &MouseDownEvent, w, cx| {
                    if s.pointer && event.button == MouseButton::Left && s.focus_native(w, cx) {
                        cx.stop_propagation();
                    }
                }),
            );
            element = element.on_click(cx.listener(move |s, _: &ClickEvent, w, cx| {
                if s.pointer && s.focus_native(w, cx) {
                    s.act(action, cx);
                    cx.stop_propagation();
                }
            }));
            let weak = cx.weak_entity();
            element = element.on_a11y_action(AccessibleAction::Click, move |_, w, cx| {
                let _ = weak.update(cx, |s, cx| {
                    if s.focus_native(w, cx) {
                        s.act(action, cx);
                    }
                });
            });
        } else {
            element = element.opacity(0.45);
        }
        // Disabled dates remain discoverable with the cursor without selecting them.
        if let Some(Action::Activate(date)) = action {
            let weak = cx.weak_entity();
            element = element.on_a11y_action(AccessibleAction::Focus, move |_, w, cx| {
                let _ = weak.update(cx, |s, cx| {
                    if s.focus_native(w, cx) {
                        s.act(Action::Reveal(date), cx);
                    }
                });
            });
        }
        crate::semantics::State {
            element,
            metadata: None,
            hidden: false,
            disabled: !enabled,
            read_only: self.model.config().read_only,
            modal: false,
            live: None,
        }
        .into_any_element()
    }
}
impl Render for Calendar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let s = self.model.snapshot();
        let config = self.model.config();
        let labels = &config.labels;
        let delta = match s.presentation {
            c::Presentation::Days => 1,
            c::Presentation::Months => 12,
            c::Presentation::Years => 240,
        };
        let mut header = div().flex().gap(px(4.)).items_center();
        header = header.child(self.button(
            Button {
                id: "previous".into(),
                text: "‹".into(),
                label: labels.previous.clone(),
                action: s.month.shift(-delta).map(Action::ShowMonth),
                selected: false,
                cursor: false,
                today: false,
            },
            cx,
        ));
        header = header.child(self.button(
            Button {
                id: "month-title".into(),
                text: labels.months[(s.month.month() - 1) as usize].clone(),
                label: labels.choose_month.clone(),
                action: Some(Action::SetPresentation(c::Presentation::Months)),
                selected: false,
                cursor: false,
                today: false,
            },
            cx,
        ));
        header = header.child(self.button(
            Button {
                id: "year-title".into(),
                text: s.month.year().to_string(),
                label: labels.choose_year.clone(),
                action: Some(Action::SetPresentation(c::Presentation::Years)),
                selected: false,
                cursor: false,
                today: false,
            },
            cx,
        ));
        header = header.child(self.button(
            Button {
                id: "next".into(),
                text: "›".into(),
                label: labels.next.clone(),
                action: s.month.shift(delta).map(Action::ShowMonth),
                selected: false,
                cursor: false,
                today: false,
            },
            cx,
        ));
        let mut body = div().flex().flex_col().gap(px(4.));
        match s.presentation {
            c::Presentation::Days => {
                let mut weekdays = div().flex().gap(px(4.));
                for i in 0..7 {
                    let index = ((config.first_weekday + i) % 7) as usize;
                    weekdays = weekdays.child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .text_center()
                            .text_size(px(11.))
                            .child(labels.short_weekdays[index].clone()),
                    );
                }
                body = body.child(weekdays);
                for (week_index, week) in s
                    .month
                    .days(config.first_weekday)
                    .expect("validated weekday")
                    .chunks(7)
                    .enumerate()
                {
                    let mut row = div().flex().gap(px(4.));
                    for (column, date) in week.iter().enumerate() {
                        if let Some(date) = date {
                            let (year, month, day) = date.ymd();
                            let selected = match s.selection {
                                c::Selection::Empty => false,
                                c::Selection::Single(d) | c::Selection::RangeStart(d) => d == *date,
                                c::Selection::Range(r) => r.contains(*date),
                            };
                            let label = format!(
                                "{} {day}, {year:04}{}",
                                labels.months[(month - 1) as usize],
                                if config.today == Some(*date) {
                                    format!(", {}", labels.today)
                                } else {
                                    String::new()
                                }
                            );
                            let action = Some(Action::Activate(*date));
                            let cell = self.button(
                                Button {
                                    id: format!("day-{}", date.ordinal()),
                                    text: day.to_string(),
                                    label,
                                    action,
                                    selected,
                                    cursor: *date == s.focused_date,
                                    today: config.today == Some(*date),
                                },
                                cx,
                            );
                            // Selection constraints are enforced in the owner, including range interiors;
                            // out-of-month dates remain usable for natural cross-month navigation.
                            row = row.child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.))
                                    .opacity(if month == s.month.month() { 1. } else { 0.5 })
                                    .child(cell),
                            );
                        } else {
                            row = row.child(
                                div()
                                    .id(("blank", (week_index * 7 + column) as u64))
                                    .flex_1()
                                    .h(px(32.)),
                            );
                        }
                    }
                    body = body.child(row);
                }
            }
            c::Presentation::Months => {
                for row_index in 0..4 {
                    let mut row = div().flex().gap(px(4.));
                    for column in 0..3 {
                        let month = row_index * 3 + column + 1;
                        let target = c::Month::new(s.month.year(), month).unwrap();
                        row = row.child(self.button(
                            Button {
                                id: format!("month-{month}"),
                                text: labels.months[(month - 1) as usize].clone(),
                                label: labels.months[(month - 1) as usize].clone(),
                                action: Some(Action::ChooseMonth(target)),
                                selected: false,
                                cursor: target == s.month,
                                today: false,
                            },
                            cx,
                        ));
                    }
                    body = body.child(row);
                }
            }
            c::Presentation::Years => {
                let start = (s.month.year() - 1) / 20 * 20 + 1;
                for row_index in 0..5 {
                    let mut row = div().flex().gap(px(4.));
                    for column in 0..4 {
                        let year = start + row_index * 4 + column;
                        if year > 9999 {
                            row = row.child(div().flex_1().h(px(32.)));
                            continue;
                        }
                        row = row.child(self.button(
                            Button {
                                id: format!("year-{year}"),
                                text: year.to_string(),
                                label: year.to_string(),
                                action: Some(Action::ChooseYear(year)),
                                selected: false,
                                cursor: year == s.month.year(),
                                today: false,
                            },
                            cx,
                        ));
                    }
                    body = body.child(row);
                }
            }
        }
        let mut footer = div().flex().gap(px(4.));
        if let Some(today) = config.today {
            footer = footer.child(self.button(
                Button {
                    id: "today".into(),
                    text: labels.today.clone(),
                    label: labels.today.clone(),
                    action: Some(Action::Reveal(today)),
                    selected: false,
                    cursor: false,
                    today: false,
                },
                cx,
            ));
        }
        footer = footer.child(self.button(
            Button {
                id: "clear".into(),
                text: labels.clear.clone(),
                label: labels.clear.clone(),
                action: Some(Action::Clear),
                selected: false,
                cursor: false,
                today: false,
            },
            cx,
        ));
        let weak = cx.weak_entity();
        let root = div()
            .id("calendar")
            .relative()
            .w_full()
            .min_w(px(0.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(8.))
            .text_size(px(13.))
            .key_context("GpuioCalendar")
            .track_focus(&self.focus)
            .role(Role::Group)
            .aria_label(config.label.clone())
            .on_key_down(cx.listener(Self::key))
            .child(header)
            .child(body)
            .child(footer)
            .child(
                canvas(
                    |_, _, _| (),
                    move |_, _, window, cx| {
                        let _ = weak.update(cx, |s, cx| {
                            if s.autofocus
                                && s.access() == Access::Allowed
                                && !s.model.config().disabled
                            {
                                s.autofocus = false;
                                let weak = cx.weak_entity();
                                window.defer(cx, move |w, cx| {
                                    let _ = weak.update(cx, |s, cx| {
                                        s.focus_native(w, cx);
                                    });
                                });
                            }
                        });
                    },
                )
                .absolute()
                .size_full(),
            );
        crate::semantics::State {
            element: root,
            metadata: self.metadata.clone(),
            hidden: false,
            disabled: config.disabled,
            read_only: config.read_only,
            modal: false,
            live: None,
        }
    }
}

pub(super) struct Instance {
    state: Entity<Calendar>,
}
impl Instance {
    fn new(
        view: &View,
        node: &crate::tree::Node,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<Self, c::Error> {
        let mount = node.calendar.as_ref().expect("validated calendar");
        let model = State::from_retained(mount.config.clone(), mount.initial, mount.initial_month)?;
        let route = Route {
            window: view.id,
            node: node.id,
            handler: node.handler.expect("calendar handler"),
            session: view.session.clone(),
            gate: view.focus.clone(),
            transport: view.transport.clone(),
        };
        if !route.emit(vec![c::Event::Observed(model.snapshot())]) {
            return Err(c::Error::NativeFailure);
        }
        let state = cx.new(|cx| {
            let focus = cx.focus_handle();
            let subscriptions = vec![
                cx.on_focus(&focus, window, Calendar::on_focus),
                cx.on_blur(&focus, window, Calendar::on_blur),
            ];
            Calendar {
                model,
                route,
                focus,
                pointer: true,
                autofocus: mount.config.auto_focus,
                metadata: node.accessibility.clone(),
                _subscriptions: subscriptions,
            }
        });
        Ok(Self { state })
    }
    pub(super) fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.read(cx).focus.clone()
    }
    pub(super) fn element(
        &self,
        base: Stateful<Div>,
        pointer: bool,
        cx: &mut App,
    ) -> Stateful<Div> {
        self.state.update(cx, |s, _| s.pointer = pointer);
        base.child(self.state.clone())
    }
}
impl View {
    pub(super) fn sync_calendars(
        &mut self,
        dirty: &[NodeId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let nodes = {
            let session = self.session.borrow();
            let Some(tree) = session.tree(self.id) else {
                self.calendars.clear();
                return;
            };
            self.calendars.retain(|id, instance| {
                if tree.get(*id).is_some() {
                    true
                } else {
                    instance.state.update(cx, |s, cx| s.hide(window, cx));
                    false
                }
            });
            dirty
                .iter()
                .filter_map(|id| tree.get(*id))
                .filter(|n| n.calendar.is_some())
                .cloned()
                .collect::<Vec<_>>()
        };
        for node in nodes {
            if let Some(instance) = self.calendars.get(&node.id) {
                instance.state.update(cx, |s, cx| {
                    s.route.handler = node.handler.expect("calendar handler");
                    s.metadata = node.accessibility.clone();
                    let result = s
                        .model
                        .configure(node.calendar.as_ref().unwrap().config.clone());
                    s.publish(result, cx);
                    if s.model.config().disabled || s.access() == Access::Blocked {
                        s.hide(window, cx);
                    }
                    cx.notify();
                });
            } else {
                match Instance::new(self, &node, window, cx) {
                    Ok(instance) => {
                        self.calendars.insert(node.id, instance);
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
    pub(super) fn hide_unvisited_calendars(&self, window: &mut Window, cx: &mut App) {
        for (id, instance) in &self.calendars {
            let s = instance.state.read(cx);
            if (!self.visited.contains(id) || s.access() == Access::Blocked)
                && s.focus.is_focused(window)
            {
                let weak = instance.state.downgrade();
                window.defer(cx, move |w, cx| {
                    let _ = weak.update(cx, |s, cx| s.hide(w, cx));
                });
            }
        }
    }
}

#[cfg(feature = "native-tests")]
#[path = "calendar_view_test.rs"]
pub(crate) mod test;
