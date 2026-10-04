//! Bounded first-party GPUI calendar presentation over the civil-date policy.
//! The pinned base CalendarState has different range/today/ownership semantics;
//! no upstream state or synchronous OCaml day/layout callback is used here.
use super::{SharedSession, View, focus};
use crate::{
    calendar_state::{Access, Action, State},
    transport::Transport,
};
use gpui::{prelude::*, *};
use gpuio_protocol::calendar_viewport::{Display, Observation};
use gpuio_protocol::{HandlerId, NodeId, WindowId, calendar_content::Slot, calendar_input as c};
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
    slot: Slot,
    text: String,
    label: String,
    action: Option<Action>,
    selected: bool,
    cursor: bool,
    today: bool,
}

struct Calendar {
    host: WeakEntity<View>,
    interaction: super::Interaction,
    model: State,
    appearance: gpuio_protocol::calendar_presentation::Appearance,
    viewport: crate::calendar_viewport::Viewport,
    route: Route,
    viewport_handler: Option<HandlerId>,
    viewport_published: Option<(HandlerId, Display)>,
    viewport_sequence: Option<i64>,
    focus: FocusHandle,
    pointer: bool,
    autofocus: bool,
    metadata: Option<Arc<gpuio_protocol::accessibility::Config>>,
    _subscriptions: Vec<Subscription>,
    #[cfg(feature = "native-tests")]
    paint_probe: Option<(Bounds<Pixels>, Pixels)>,
}
impl Calendar {
    fn content(
        &self,
        slot: Slot,
        disabled: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(AnyElement, Option<String>)> {
        let shared = self.route.session.clone();
        let session = shared.borrow();
        let tree = session.tree(self.route.window)?;
        let node = tree.get(self.route.node)?;
        let content = node.calendar_content.as_ref()?;
        let index = content
            .items
            .binary_search_by_key(&slot, |item| item.slot)
            .ok()?;
        let child = *node.children.get(index)?;
        let description = content.items[index].description.clone();
        // The host returns this calendar Entity during render. GPUI renders the
        // child afterwards; weak ownership avoids a host/calendar retain cycle.
        self.host
            .update(cx, |view, cx| {
                (
                    view.control_label(tree, child, self.interaction, disabled, window, cx),
                    description,
                )
            })
            .ok()
    }

    fn caption(
        &self,
        slot: Slot,
        text: String,
        label: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if let Some((content, description)) = self.content(slot, false, window, cx) {
            let mut element = div()
                .id(SharedString::from(format!("calendar-caption-{slot:?}")))
                .role(Role::Label)
                .aria_label(label)
                .child(content);
            if let Some(description) = description {
                element = element.aria_description(description);
            }
            element.into_any_element()
        } else {
            div().child(text).into_any_element()
        }
    }

    fn access(&self) -> Access {
        if self.route.current(self.model.config())
            && self.route.gate.borrow().allows(self.route.node)
        {
            Access::Allowed
        } else {
            Access::Blocked
        }
    }
    fn viewport_display(&self) -> Display {
        let snapshot = self.model.snapshot();
        match snapshot.presentation {
            c::Presentation::Days => Display::Days {
                first_month: self.viewport.first().index(),
                months: self.appearance.months,
                first_weekday: self.model.config().first_weekday,
            },
            c::Presentation::Months => Display::Months {
                year: snapshot.month.year(),
            },
            c::Presentation::Years => {
                let first = (snapshot.month.year() - 1) / 20 * 20 + 1;
                Display::Years {
                    first,
                    last: (first + 19).min(9999),
                }
            }
        }
    }
    fn publish_viewport(&mut self) -> bool {
        let Some(handler) = self.viewport_handler else {
            self.viewport_published = None;
            return true;
        };
        let display = self.viewport_display();
        if self.viewport_published == Some((handler, display)) {
            return true;
        }
        let Some(sequence) = self.viewport_sequence else {
            self.route.fault();
            return false;
        };
        let event = {
            let session = self.route.session.borrow();
            let Some(tree) = session.tree(self.route.window) else {
                return false;
            };
            session.calendar_viewport_event(
                self.route.window,
                self.route.node,
                handler,
                tree.revision(),
                Observation { sequence, display },
            )
        };
        let Some(event) = event else {
            return false;
        };
        if !self.route.transport.input(event) {
            self.route.fault();
            return false;
        }
        self.viewport_published = Some((handler, display));
        self.viewport_sequence = sequence.checked_add(1);
        true
    }
    fn publish(&mut self, result: Result<Vec<c::Event>, c::Error>, cx: &mut Context<Self>) {
        match result {
            Ok(events) => {
                let changed = !events.is_empty();
                if self.route.emit(events) {
                    self.publish_viewport();
                }
                if changed {
                    cx.notify();
                }
            }
            Err(c::Error::LimitExceeded | c::Error::NativeFailure) => self.route.fault(),
            Err(_) => (),
        }
    }
    fn act(&mut self, action: Action, cx: &mut Context<Self>) {
        let previous_viewport = self.viewport;
        let result = self.model.native(action, self.access());
        if result.is_ok() {
            if let Action::ShowMonth(month) = action {
                self.viewport.show(month);
            }
            self.viewport
                .configure(self.appearance.months, self.model.snapshot().month);
        }
        if self.viewport != previous_viewport {
            cx.notify();
        }
        self.publish(result, cx);
    }
    fn focus_native(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.access() == Access::Blocked || self.model.config().disabled {
            return false;
        }
        window.focus(&self.focus, cx);
        self.focus.is_focused(window)
    }
    fn on_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let result = self.model.observe_focus(self.focus.is_focused(window));
        self.publish(result, cx);
    }
    fn on_blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let result = self.model.observe_focus(self.focus.is_focused(window));
        self.publish(result, cx);
    }
    fn hide(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.focus.is_focused(window) {
            window.blur(cx);
        }
        // A hidden ancestor can remove the focused element from GPUI's dispatch
        // tree before its blur subscription runs. Publish the confirmed platform
        // state here as well; a later blur callback is an idempotent no-op.
        let result = self.model.observe_focus(self.focus.is_focused(window));
        self.publish(result, cx);
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
    fn button(&self, button: Button, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Button {
            id,
            slot,
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
        let (content, description) =
            self.content(slot, !enabled, window, cx).unwrap_or_else(|| {
                (
                    div()
                        .w_full()
                        .min_w(px(0.))
                        .text_center()
                        .truncate()
                        .child(text)
                        .into_any_element(),
                    None,
                )
            });
        let mut element = div()
            .id(SharedString::from(id))
            .flex()
            .items_center()
            .justify_center()
            .flex_1()
            .min_w(px(0.))
            .h(px(self.appearance.cell_height as f32))
            .overflow_hidden()
            .rounded(px(self.appearance.cell_radius as f32))
            .border(px(self.appearance.outline_width as f32))
            .border_color(if cursor && self.model.snapshot().focused {
                rgba(self.appearance.focus_border.unwrap_or(0x6688ffff) as u32)
            } else if today {
                rgba(self.appearance.today_border.unwrap_or(0x6688ff90) as u32)
            } else {
                rgba(0x00000000)
            })
            .role(Role::Button)
            .aria_label(label)
            .child(content);
        if let Some(description) = description {
            element = element.aria_description(description);
        }
        if matches!(action, Some(Action::Activate(_))) {
            // AccessKit's macOS backend exposes button pressed state as an
            // AXToggle checkbox value; aria_selected alone is not exported for
            // buttons. Keep the action a date activation, not a text-only label.
            element = element.aria_toggled(if selected {
                gpui::accesskit::Toggled::True
            } else {
                gpui::accesskit::Toggled::False
            });
        }
        if selected {
            element = element.bg(rgba(
                self.appearance.selected_background.unwrap_or(0x6688ff38) as u32,
            ));
            if let Some(color) = self.appearance.selected_foreground {
                element = element.text_color(rgba(color as u32));
            }
        }
        // GPUI honors this only while an ancestor holds actual native focus.
        // Do not gate it on the asynchronously observed model focus flag.
        if cursor {
            element = element.aria_active_descendant();
        }
        if enabled {
            let hover = self.appearance.hover_background.unwrap_or(0x71809624) as u32;
            element = element.hover(move |s| s.bg(rgba(hover)));
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
            identity: None,
            busy: false,
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
    fn day_pane(
        &self,
        month: c::Month,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let s = self.model.snapshot();
        let config = self.model.config();
        let labels = &config.labels;
        let mut body = div()
            .id(("calendar-month", month.index() as u64))
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.))
            .gap(px(self.appearance.cell_gap as f32));
        if self.appearance.months > 1 {
            let title = format!(
                "{} {}",
                labels.months[(month.month() - 1) as usize],
                month.year()
            );
            let heading = self.caption(
                Slot::MonthHeading(month.index()),
                title.clone(),
                title,
                window,
                cx,
            );
            body = body
                .min_w(px(
                    (7. * self.appearance.cell_height + 6. * self.appearance.cell_gap) as f32,
                ))
                .child(div().text_center().child(heading));
        }
        let mut weekdays = div().flex().gap(px(self.appearance.cell_gap as f32));
        for i in 0..7 {
            let index = ((config.first_weekday + i) % 7) as usize;
            weekdays = weekdays.child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .text_center()
                    .truncate()
                    .text_size(
                        window.text_style().font_size.to_pixels(window.rem_size()) * (11. / 13.),
                    )
                    .child(self.caption(
                        Slot::Weekday(month.index(), index as i64),
                        labels.short_weekdays[index].clone(),
                        labels.weekdays[index].clone(),
                        window,
                        cx,
                    )),
            );
        }
        body = body.child(weekdays);
        for (week_index, week) in month
            .days(config.first_weekday)
            .expect("validated weekday")
            .chunks(7)
            .enumerate()
        {
            let mut row = div().flex().gap(px(self.appearance.cell_gap as f32));
            for (column, date) in week.iter().enumerate() {
                if let Some(date) = date.filter(|date| {
                    c::Month::from_date(*date) == month
                        || !self.viewport.contains(c::Month::from_date(*date))
                }) {
                    let (year, date_month, day) = date.ymd();
                    let selected = match s.selection {
                        c::Selection::Empty => false,
                        c::Selection::Single(d) | c::Selection::RangeStart(d) => d == date,
                        c::Selection::Range(r) => r.contains(date),
                    };
                    let label = format!(
                        "{} {day}, {year:04}{}",
                        labels.months[(date_month - 1) as usize],
                        if config.today == Some(date) {
                            format!(", {}", labels.today)
                        } else {
                            String::new()
                        }
                    );
                    let action = Some(Action::Activate(date));
                    let cell = self.button(
                        Button {
                            id: format!("day-{}", date.ordinal()),
                            slot: Slot::Day(date.ordinal()),
                            text: day.to_string(),
                            label,
                            action,
                            selected,
                            cursor: date == s.focused_date,
                            today: config.today == Some(date),
                        },
                        window,
                        cx,
                    );
                    // Selection constraints are enforced in the owner, including range interiors;
                    // out-of-month dates remain usable for natural cross-month navigation.
                    row = row.child(
                        div()
                            .flex_1()
                            .min_w(px(0.))
                            .when(c::Month::from_date(date) != month, |this| {
                                if let Some(color) = self.appearance.muted_foreground {
                                    this.text_color(rgba(color as u32))
                                } else {
                                    this.opacity(0.5)
                                }
                            })
                            .child(cell),
                    );
                } else {
                    row = row.child(
                        div()
                            .id(("blank", (week_index * 7 + column) as u64))
                            .flex_1()
                            .h(px(self.appearance.cell_height as f32)),
                    );
                }
            }
            body = body.child(row);
        }
        body
    }
}
impl Render for Calendar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let s = self.model.snapshot();
        self.viewport.configure(self.appearance.months, s.month);
        let config = self.model.config();
        let labels = &config.labels;
        let navigation_month = if s.presentation == c::Presentation::Days {
            self.viewport.first()
        } else {
            s.month
        };
        let delta = match s.presentation {
            c::Presentation::Days => 1,
            c::Presentation::Months => 12,
            c::Presentation::Years => 240,
        };
        let mut header = div()
            .flex()
            .gap(px(self.appearance.cell_gap as f32))
            .items_center();
        header = header.child(self.button(
            Button {
                id: "previous".into(),
                slot: Slot::Previous,
                text: "‹".into(),
                label: labels.previous.clone(),
                action: navigation_month.shift(-delta).map(Action::ShowMonth),
                selected: false,
                cursor: false,
                today: false,
            },
            window,
            cx,
        ));
        header = header.child(self.button(
            Button {
                id: "month-title".into(),
                slot: Slot::ChooseMonth,
                text: labels.months[(s.month.month() - 1) as usize].clone(),
                label: labels.choose_month.clone(),
                action: Some(Action::SetPresentation(c::Presentation::Months)),
                selected: false,
                cursor: false,
                today: false,
            },
            window,
            cx,
        ));
        header = header.child(self.button(
            Button {
                id: "year-title".into(),
                slot: Slot::ChooseYear,
                text: s.month.year().to_string(),
                label: labels.choose_year.clone(),
                action: Some(Action::SetPresentation(c::Presentation::Years)),
                selected: false,
                cursor: false,
                today: false,
            },
            window,
            cx,
        ));
        header = header.child(
            self.button(
                Button {
                    id: "next".into(),
                    slot: Slot::Next,
                    text: "›".into(),
                    label: labels.next.clone(),
                    action: if s.presentation == c::Presentation::Days
                        && self
                            .viewport
                            .months()
                            .last()
                            .and_then(|m| m.shift(1))
                            .is_none()
                    {
                        None
                    } else {
                        navigation_month.shift(delta).map(Action::ShowMonth)
                    },
                    selected: false,
                    cursor: false,
                    today: false,
                },
                window,
                cx,
            ),
        );
        let mut body = div()
            .flex()
            .flex_col()
            .gap(px(self.appearance.cell_gap as f32));
        match s.presentation {
            c::Presentation::Days => {
                let mut panes = div()
                    .flex()
                    .flex_wrap()
                    .gap(px(self.appearance.month_gap as f32));
                for month in self.viewport.months() {
                    panes = panes.child(self.day_pane(month, window, cx));
                }
                body = body.child(panes);
            }
            c::Presentation::Months => {
                for row_index in 0..4 {
                    let mut row = div().flex().gap(px(self.appearance.cell_gap as f32));
                    for column in 0..3 {
                        let month = row_index * 3 + column + 1;
                        let target = c::Month::new(s.month.year(), month).unwrap();
                        row = row.child(self.button(
                            Button {
                                id: format!("month-{month}"),
                                slot: Slot::Month(month),
                                text: labels.months[(month - 1) as usize].clone(),
                                label: labels.months[(month - 1) as usize].clone(),
                                action: Some(Action::ChooseMonth(target)),
                                selected: false,
                                cursor: target == s.month,
                                today: false,
                            },
                            window,
                            cx,
                        ));
                    }
                    body = body.child(row);
                }
            }
            c::Presentation::Years => {
                let start = (s.month.year() - 1) / 20 * 20 + 1;
                for row_index in 0..5 {
                    let mut row = div().flex().gap(px(self.appearance.cell_gap as f32));
                    for column in 0..4 {
                        let year = start + row_index * 4 + column;
                        if year > 9999 {
                            row =
                                row.child(div().flex_1().h(px(self.appearance.cell_height as f32)));
                            continue;
                        }
                        row = row.child(self.button(
                            Button {
                                id: format!("year-{year}"),
                                slot: Slot::Year(year),
                                text: year.to_string(),
                                label: year.to_string(),
                                action: Some(Action::ChooseYear(year)),
                                selected: false,
                                cursor: year == s.month.year(),
                                today: false,
                            },
                            window,
                            cx,
                        ));
                    }
                    body = body.child(row);
                }
            }
        }
        let mut footer = div().flex().gap(px(self.appearance.cell_gap as f32));
        if let Some(today) = config.today {
            footer = footer.child(self.button(
                Button {
                    id: "today".into(),
                    slot: Slot::Today,
                    text: labels.today.clone(),
                    label: labels.today.clone(),
                    action: Some(Action::Reveal(today)),
                    selected: false,
                    cursor: false,
                    today: false,
                },
                window,
                cx,
            ));
        }
        footer = footer.child(self.button(
            Button {
                id: "clear".into(),
                slot: Slot::Clear,
                text: labels.clear.clone(),
                label: labels.clear.clone(),
                action: Some(Action::Clear),
                selected: false,
                cursor: false,
                today: false,
            },
            window,
            cx,
        ));
        let weak = cx.weak_entity();
        let date_label = |date: c::Date| {
            let (year, month, day) = date.ymd();
            format!("{year:04}-{month:02}-{day:02}")
        };
        let value = match s.selection {
            c::Selection::Empty => String::new(),
            c::Selection::Single(date) => date_label(date),
            c::Selection::RangeStart(date) => format!("{} – …", date_label(date)),
            c::Selection::Range(range) => {
                format!(
                    "{} – {}",
                    date_label(range.first()),
                    date_label(range.last())
                )
            }
        };
        let root = div()
            .id("calendar")
            .relative()
            .w_full()
            .min_w(px(0.))
            .flex()
            .flex_col()
            .gap(px(8.))
            .p(px(self.appearance.padding as f32))
            .key_context("GpuioCalendar")
            .track_focus(
                &self
                    .focus
                    .clone()
                    .tab_stop(!config.disabled && self.access() == Access::Allowed),
            )
            .role(Role::Group)
            .aria_label(config.label.clone())
            .aria_value(value)
            .aria_description(format!(
                "{} {}",
                labels.months[(s.month.month() - 1) as usize],
                s.month.year()
            ))
            .on_key_down(cx.listener(Self::key))
            .child(header)
            .child(body)
            .child(footer)
            .child(
                canvas(
                    |_, _, _| (),
                    move |_bounds, _, window, cx| {
                        let _ = weak.update(cx, |s, cx| {
                            #[cfg(feature = "native-tests")]
                            {
                                s.paint_probe = Some((
                                    _bounds,
                                    window.text_style().font_size.to_pixels(window.rem_size()),
                                ));
                            }
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
            identity: None,
            busy: false,
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
        host: WeakEntity<View>,
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
            let focus = cx.focus_handle().tab_stop(true);
            let subscriptions = vec![
                cx.on_focus(&focus, window, Calendar::on_focus),
                cx.on_blur(&focus, window, Calendar::on_blur),
            ];
            Calendar {
                host,
                interaction: super::Interaction::default(),
                viewport: crate::calendar_viewport::Viewport::new(
                    model.snapshot().month,
                    node.calendar_appearance.as_deref().map_or(1, |a| a.months),
                ),
                appearance: node
                    .calendar_appearance
                    .as_deref()
                    .cloned()
                    .unwrap_or_default(),
                model,
                route,
                viewport_handler: node.calendar_viewport_handler,
                viewport_published: None,
                viewport_sequence: Some(0),
                focus,
                pointer: true,
                autofocus: mount.config.auto_focus,
                metadata: node.accessibility.clone(),
                _subscriptions: subscriptions,
                #[cfg(feature = "native-tests")]
                paint_probe: None,
            }
        });
        if !state.update(cx, |state, _| state.publish_viewport()) {
            return Err(c::Error::NativeFailure);
        }
        Ok(Self { state })
    }
    pub(super) fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.read(cx).focus.clone()
    }
    pub(super) fn command(
        &self,
        command: &c::Command,
        window: &mut Window,
        cx: &mut App,
    ) -> c::Response {
        self.state.update(cx, |state, cx| {
            if !state
                .route
                .session
                .borrow()
                .accepts_input(state.route.window)
            {
                return c::Response::Failed(c::Error::NativeFailure);
            }
            if !state.route.current(state.model.config()) {
                return c::Response::Failed(c::Error::StaleInput);
            }
            let focus = state.focus.clone();
            let gate = state.route.gate.clone();
            let node = state.route.node;
            let previous_viewport = state.viewport;
            let outcome = state.model.execute(command, || {
                if !gate.borrow().allows(node) {
                    return Err(c::Error::FocusBlocked);
                }
                window.focus(&focus, cx);
                if focus.is_focused(window) {
                    Ok(())
                } else {
                    Err(c::Error::NativeFailure)
                }
            });
            if matches!(
                outcome.response,
                c::Response::Failed(c::Error::LimitExceeded)
            ) {
                state.route.fault();
            }
            if matches!(outcome.response, c::Response::Applied(_)) {
                if let c::Command::ShowMonth(month) = command {
                    state.viewport.show(*month);
                }
                state
                    .viewport
                    .configure(state.appearance.months, state.model.snapshot().month);
            }
            let changed = !outcome.events.is_empty() || previous_viewport != state.viewport;
            if !state.route.emit(outcome.events) || !state.publish_viewport() {
                return c::Response::Failed(c::Error::NativeFailure);
            }
            if changed {
                cx.notify();
            }
            outcome.response
        })
    }
    pub(super) fn element(
        &self,
        base: Stateful<Div>,
        interaction: super::Interaction,
        cx: &mut App,
    ) -> Stateful<Div> {
        self.state.update(cx, |s, _| {
            s.pointer = interaction.pointer;
            s.interaction = interaction;
        });
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
                    s.viewport_handler = node.calendar_viewport_handler;
                    s.metadata = node.accessibility.clone();
                    s.appearance = node
                        .calendar_appearance
                        .as_deref()
                        .cloned()
                        .unwrap_or_default();
                    s.viewport
                        .configure(s.appearance.months, s.model.snapshot().month);
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
                match Instance::new(self, cx.entity().downgrade(), &node, window, cx) {
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
                && (s.focus.is_focused(window) || s.model.snapshot().focused)
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

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "calendar_presentation_test.rs"]
mod presentation_tests;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "calendar_content_test.rs"]
mod content_tests;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "calendar_viewport_test.rs"]
mod viewport_tests;
