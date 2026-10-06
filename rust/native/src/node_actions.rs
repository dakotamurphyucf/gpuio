//! Nonrecursive event/focus decoration. Keeping its temporary GPUI builders out
//! of the recursive element traversal reduces debug-stack use for rich content.
use super::*;

pub(super) struct Render<'a> {
    pub node: &'a crate::tree::Node,
    pub action_lifetime: Option<action_lifetime::Lease>,
    pub revision: i64,
    pub command: Option<command::Route>,
    pub interaction: Interaction,
    pub disabled: bool,
    pub preserve_focus: bool,
    pub button_loading: bool,
    pub button_order: Option<gpuio_protocol::checkable::TabOrder>,
}
impl View {
    pub(super) fn node_actions(
        &mut self,
        render: Render<'_>,
        mut element: gpui::Stateful<gpui::Div>,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let Render {
            node,
            action_lifetime,
            command,
            interaction,
            disabled,
            preserve_focus,
            button_loading,
            button_order,
            ..
        } = render;
        let id = node.id;
        let pointer_lifetime = action_lifetime.clone();
        if let Some(route) = command.filter(|_| !disabled && !button_loading) {
            let accessible = route.clone();
            let accessible_lifetime = action_lifetime.clone();
            let owner = cx.weak_entity();
            element =
                element.on_a11y_action(gpui::AccessibleAction::Click, move |_, window, cx| {
                    if accessible_lifetime
                        .as_ref()
                        .is_none_or(action_lifetime::Lease::is_live)
                    {
                        let _ = owner
                            .update(cx, |view, cx| view.invoke_command(&accessible, window, cx));
                    }
                    cx.stop_propagation();
                });
            if !interaction.pointer {
                element = element.on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                    window.prevent_default()
                });
            }
            let lifetime = action_lifetime.clone();
            element = element.on_click(cx.listener(move |view, event, window, cx| {
                if lifetime.as_ref().is_some_and(|lease| !lease.is_live()) {
                    cx.stop_propagation();
                    return;
                }
                if interaction.pointer || matches!(event, gpui::ClickEvent::Keyboard(_)) {
                    view.invoke_command(&route, window, cx);
                    cx.stop_propagation();
                }
            }));
        }
        if let Some(handler) = node.handler
            && node.commands.is_none()
            && node.editor.is_none()
            && node.choice.is_none()
            && node.rating.is_none()
            && node.slider.is_none()
            && node.number_input.is_none()
            && node.otp_input.is_none()
            && node.calendar.is_none()
            && node.color_input.is_none()
            && node.overlay.is_none()
            && node.pointer.is_none()
            && node.input_region.is_none()
            && node.highlight_scope.is_none()
            && node.command_binding.is_none()
            && node.image.is_none()
            && node.animation.is_none()
            && node.animation_program.is_none()
            && node.split.is_none()
            && node.carousel_track.is_none()
            && !button_loading
            && !disabled
        {
            let window = self.id;
            let revision = render.revision;
            let session = self.session.clone();
            let transport = self.transport.clone();
            let gate = self.focus.clone();
            let accessible_gate = gate.clone();
            let accessible_session = session.clone();
            let accessible_transport = transport.clone();
            let accessible_lifetime = action_lifetime.clone();
            // GPUI's fallback accessibility Click synthesizes pointer input.
            // Route the semantic action directly so pointer policy and pointer
            // occlusion do not suppress assistive activation.
            element = element.on_a11y_action(gpui::AccessibleAction::Click, move |_, _, cx| {
                if accessible_lifetime
                    .as_ref()
                    .is_none_or(action_lifetime::Lease::is_live)
                {
                    emit_press(
                        &accessible_session,
                        &accessible_gate,
                        &accessible_transport,
                        window,
                        id,
                        handler,
                        revision,
                    );
                }
                cx.stop_propagation();
            });
            if !interaction.pointer {
                element = element.on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                    window.prevent_default()
                });
            }
            // GPUI synthesizes one click for native Enter/Space and accessibility
            // activation. Registering a second key handler duplicates activation.
            let owner = cx.weak_entity();
            element = element.on_click(move |event, native_window, cx| {
                if action_lifetime
                    .as_ref()
                    .is_some_and(|lease| !lease.is_live())
                {
                    cx.stop_propagation();
                    return;
                }
                if !interaction.pointer && !matches!(event, gpui::ClickEvent::Keyboard(_)) {
                    return;
                }
                emit_press(&session, &gate, &transport, window, id, handler, revision);
                if !event.is_keyboard() {
                    let _ = owner.update(cx, |view, cx| {
                        view.focus_track_control_after_pointer(
                            id,
                            handler,
                            revision,
                            native_window,
                            cx,
                        )
                    });
                }
                cx.stop_propagation();
            });
        }
        if interaction.clip_controls
            // These outer native Regions must receive this event before
            // applying their own capture/propagation policy.
            && node.pointer.is_none()
            && node.slider.is_none()
            && node.input_region.is_none()
            && (node.handler.is_some() || node.command_ref.is_some() || node.editor.is_some())
        {
            // Keep the control's default focus/selection behavior, but do not
            // arm the enclosing table column or chart's drag on a child's mouse down.
            element = element.on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            });
        }
        if preserve_focus {
            element = element.on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                window.prevent_default();
            });
        }
        if button_loading {
            // A busy owner consumes activation instead of invoking an ancestor.
            element = element
                .on_click(|_, _, cx| cx.stop_propagation())
                .on_a11y_action(gpui::AccessibleAction::Click, |_, _, cx| {
                    cx.stop_propagation()
                });
        }
        if disabled && matches!(node.kind, Kind::Button | Kind::CommandButton) {
            // A queued AX Click can arrive after disabling the trigger. Consume
            // it rather than letting GPUI synthesize a pointer click that could
            // dismiss a surrounding popup. Disabled AX output clears actions.
            element = element.on_a11y_action(gpui::AccessibleAction::Click, |_, _, cx| {
                cx.stop_propagation()
            });
        }
        let gate = self.focus.clone();
        element = element.capture_any_mouse_down(move |_, window, cx| {
            if pointer_lifetime
                .as_ref()
                .is_some_and(|lease| !lease.is_live())
                || gate.borrow().blocks_pointer(id)
            {
                window.prevent_default();
                cx.stop_propagation();
            }
        });
        let handle = self
            .editors
            .get(&id)
            .map(|editor| editor.focus_handle(cx))
            .or_else(|| self.numbers.get(&id).map(|number| number.focus_handle(cx)))
            .or_else(|| self.otps.get(&id).map(|otp| otp.focus_handle(cx)))
            .or_else(|| {
                self.calendars
                    .get(&id)
                    .map(|calendar| calendar.focus_handle(cx))
            })
            .or_else(|| self.buttons.get(&id).map(|button| button.focus.clone()))
            .or_else(|| {
                self.input_regions
                    .get(&id)
                    .filter(|_| {
                        node.input_region.as_ref().is_some_and(|config| {
                            config.focus != gpuio_protocol::input::Focus::None
                        })
                    })
                    .map(|state| state.borrow().focus.clone())
            })
            .or_else(|| {
                self.selections
                    .get(&id)
                    .map(|state| state.borrow().focus.clone())
            })
            .or_else(|| self.focus.borrow().handle(id));
        if let Some(handle) =
            handle.filter(|_| !disabled && !preserve_focus && node.input_region.is_none())
        {
            let tab_stop = node.kind != Kind::FocusScope
                && button_order
                    .or(node.tab_order)
                    .is_none_or(|config| config.tab_stop)
                && node.link.as_ref().is_none_or(|config| config.tab_stop)
                && node
                    .input_region
                    .as_ref()
                    .is_none_or(|config| config.focus == gpuio_protocol::input::Focus::Tab);
            let manager = self.focus.clone();
            element = element.child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        if bounds.size.width > px(0.) && bounds.size.height > px(0.) {
                            let focused = handle.is_focused(window);
                            manager
                                .borrow_mut()
                                .record(id, handle, tab_stop, focused, bounds);
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        }
        #[cfg(feature = "native-tests")]
        if !node
            .parent
            .is_some_and(|parent| self.carousel_tracks.contains_key(&parent))
        {
            // A diagnostic layout child changes ScrollHandle's content span,
            // particularly with padding. Track metrics already expose bounds;
            // keep its direct children exactly the retained application items.
            let probes = self.probes.clone();
            element = element.child(
                canvas(
                    |bounds, _, _| bounds,
                    move |_, bounds, window, _| {
                        probes.borrow_mut().insert(
                            id,
                            native_test::Probe {
                                bounds,
                                color: window.text_style().color,
                            },
                        );
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            );
        }
        element
    }
}
