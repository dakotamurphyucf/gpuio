use super::*;

impl ColorInput {
    fn channel_enabled(&self, channel: c::Channel) -> bool {
        self.shows_panel(Panel::Channels)
            && !self.model.config().disabled
            && self.access(false) == Access::Allowed
            && !(channel == c::Channel::Alpha
                && self.model.config().alpha_policy == AlphaPolicy::OpaqueOnly)
    }
    fn channel_value(&self, index: usize, position: Point<Pixels>) -> f64 {
        let bounds = self.tracks[index];
        let fraction = if bounds.size.width > px(0.) {
            f64::from(f32::from(position.x - bounds.left()))
                / f64::from(f32::from(bounds.size.width))
        } else {
            0.
        };
        fraction.clamp(0., 1.) * CHANNELS[index].maximum()
    }
    fn begin_channel(
        &mut self,
        index: usize,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let channel = CHANNELS[index];
        if !self.channel_enabled(channel)
            || self.model.config().read_only
            || self.access(true) == Access::Blocked
            || window.captured_hitbox().is_some()
            || window.default_prevented()
        {
            return;
        }
        let Some(hitbox) = self.hitboxes[index] else {
            return;
        };
        let channels = channel
            .set(
                self.model.snapshot().channels,
                self.channel_value(index, position),
            )
            .expect("bounded pointer fraction");
        if !self.model.config().allows_channels(channels) {
            return;
        }
        if self.model.snapshot().interaction.is_some() {
            self.cancel(c::CancelReason::Interrupted, window, cx);
        }
        let access = self.access(true);
        let result = self.model.begin_drag(channel, access).and_then(|started| {
            let id = started.snapshot().interaction.unwrap().id;
            let mut events = vec![started];
            if let Some(preview) =
                self.model
                    .preview_drag(id, self.channel_value(index, position), access)?
            {
                events.push(preview);
            }
            Ok(events)
        });
        if result.is_ok() {
            window.capture_pointer(hitbox);
            self.capture = Some((index, hitbox));
            window.focus(&self.channel_focus[index], cx);
            window.prevent_default();
            cx.stop_propagation();
        }
        self.publish(result, window, cx);
    }
    fn drag_channel(
        &mut self,
        index: usize,
        position: Point<Pixels>,
        finish: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(interaction) = self.model.snapshot().interaction else {
            self.release(window);
            return;
        };
        if interaction.kind != c::InteractionKind::Drag(CHANNELS[index]) {
            self.release(window);
            return;
        }
        let access = self.access(true);
        let result = self
            .model
            .preview_drag(interaction.id, self.channel_value(index, position), access)
            .and_then(|preview| {
                let mut events: Vec<_> = preview.into_iter().collect();
                if finish {
                    events.push(self.model.finish(interaction.id, access)?);
                }
                Ok(events)
            });
        if finish {
            self.release(window);
        }
        self.publish(result, window, cx);
    }
    fn adjust_channel(
        &mut self,
        index: usize,
        value: f64,
        source: c::Source,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.channel_enabled(CHANNELS[index]) {
            return;
        }
        let result = self
            .model
            .set_channel(CHANNELS[index], value, source, self.access(false));
        if result.is_ok() {
            self.release(window);
        }
        self.publish(result, window, cx);
    }
    pub(super) fn channel(
        &self,
        index: usize,
        channel: c::Channel,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let labels = &self.model.config().labels;
        let label = match channel {
            c::Channel::Hue => &labels.hue,
            c::Channel::Saturation => &labels.saturation,
            c::Channel::Lightness => &labels.lightness,
            c::Channel::Alpha => &labels.alpha,
        };
        let enabled = self.channel_enabled(channel);
        let read_only = self.model.config().read_only;
        let focus = self.channel_focus[index].clone().tab_stop(enabled);
        let channels = self.model.snapshot().channels;
        let value = channel.read(channels);
        let focused = focus.is_focused(window);
        let geometry = cx.weak_entity();
        let painter = cx.weak_entity();
        let mut track = div()
            .id(("channel", index))
            .relative()
            .h(px(self.presentation.channel_height as f32))
            .w_full()
            .role(Role::Slider)
            .aria_label(label.clone())
            .aria_numeric_value(value)
            .aria_min_numeric_value(0.)
            .aria_max_numeric_value(channel.maximum())
            .aria_numeric_value_step(1.)
            .aria_orientation(accesskit::Orientation::Horizontal)
            .when(enabled, |this| this.track_focus(&focus))
            .on_key_down(cx.listener(move |s, e: &KeyDownEvent, w, cx| {
                if e.keystroke.modifiers.modified() {
                    return;
                }
                let value = channel.read(s.model.snapshot().channels);
                let next = match e.keystroke.key.as_str() {
                    "up" | "right" => value + 1.,
                    "down" | "left" => value - 1.,
                    "pageup" => value + 10.,
                    "pagedown" => value - 10.,
                    "home" => 0.,
                    "end" => channel.maximum(),
                    _ => return,
                };
                s.adjust_channel(
                    index,
                    next.clamp(0., channel.maximum()),
                    c::Source::Keyboard,
                    w,
                    cx,
                );
                cx.stop_propagation();
            }));
        let weak = cx.weak_entity();
        let focus_action = focus.clone();
        track = track.on_a11y_action(AccessibleAction::Focus, move |_, w, cx| {
            let _ = weak.update(cx, |s, cx| {
                if s.channel_enabled(channel) {
                    w.focus(&focus_action, cx);
                }
            });
        });
        if enabled && !read_only {
            for (action, delta) in [
                (AccessibleAction::Increment, 1.),
                (AccessibleAction::Decrement, -1.),
            ] {
                let weak = cx.weak_entity();
                track = track.on_a11y_action(action, move |_, w, cx| {
                    let _ = weak.update(cx, |s, cx| {
                        let value = (channel.read(s.model.snapshot().channels) + delta)
                            .clamp(0., channel.maximum());
                        s.adjust_channel(index, value, c::Source::Accessibility, w, cx);
                    });
                });
            }
            let weak = cx.weak_entity();
            track = track.on_a11y_action(AccessibleAction::SetValue, move |data, w, cx| {
                if let Some(accesskit::ActionData::NumericValue(value)) = data {
                    let _ = weak.update(cx, |s, cx| {
                        s.adjust_channel(index, *value, c::Source::Accessibility, w, cx)
                    });
                }
            });
        }
        track = track
            .child(
                canvas(
                    move |bounds, w, cx| {
                        let hitbox = w.insert_hitbox(bounds, HitboxBehavior::BlockMouse);
                        let _ = geometry.update(cx, |s, cx| {
                            if let Some((owner, old)) = s.capture
                                && owner == index
                            {
                                if s.tracks[index] == bounds
                                    && w.captured_hitbox() == Some(old)
                                    && s.access(true) == Access::Allowed
                                {
                                    w.capture_pointer(hitbox.id);
                                    s.capture = Some((index, hitbox.id));
                                } else {
                                    s.cancel(c::CancelReason::Interrupted, w, cx);
                                }
                            }
                            s.tracks[index] = bounds;
                            s.hitboxes[index] = Some(hitbox.id);
                        });
                        hitbox
                    },
                    move |bounds, hitbox, w, _| {
                        let down = painter.clone();
                        w.on_mouse_event(move |e: &MouseDownEvent, phase, w, cx| {
                            if phase.bubble()
                                && e.button == MouseButton::Left
                                && hitbox.is_hovered(w)
                            {
                                let _ = down
                                    .update(cx, |s, cx| s.begin_channel(index, e.position, w, cx));
                            }
                        });
                        // Encoded-sRGB HSL is piecewise linear across six hue
                        // sectors and the two lightness halves. GPU gradients
                        // keep the ramp smooth with at most six bounded quads.
                        let bar = Bounds::new(
                            point(bounds.left(), bounds.center().y - px(4.)),
                            size(bounds.size.width, px(8.)),
                        );
                        w.paint_quad(fill(bar, rgba(0x808080ff)));
                        let segments = match channel {
                            c::Channel::Hue => 6,
                            c::Channel::Lightness => 2,
                            _ => 1,
                        };
                        for n in 0..segments {
                            let sample = |index| {
                                let sample = channel
                                    .set(
                                        channels,
                                        channel.maximum() * f64::from(index) / f64::from(segments),
                                    )
                                    .unwrap();
                                rgba(Rgba::of_hsla(sample).packed() as u32)
                            };
                            let rect = Bounds::new(
                                point(
                                    bar.left() + bar.size.width * (n as f32 / segments as f32),
                                    bar.top(),
                                ),
                                size(bar.size.width / segments as f32, bar.size.height),
                            );
                            w.paint_quad(fill(
                                rect,
                                linear_gradient(
                                    90.,
                                    linear_color_stop(sample(n), 0.),
                                    linear_color_stop(sample(n + 1), 1.),
                                ),
                            ));
                        }
                        let center = point(
                            bounds.left() + bounds.size.width * (value / channel.maximum()) as f32,
                            bounds.center().y,
                        );
                        let thumb_height = (bounds.size.height - px(6.)).clamp(px(1.), px(16.));
                        let thumb = Bounds::new(
                            center - point(px(5.), thumb_height / 2.),
                            size(px(10.), thumb_height),
                        );
                        let color = w.text_style().color;
                        let mut quad = fill(thumb, color);
                        quad.corner_radii = px(3.).into();
                        w.paint_quad(quad);
                        if focused {
                            let mut ring = outline(thumb.dilate(px(3.)), color, BorderStyle::Solid);
                            ring.corner_radii = px(5.).into();
                            w.paint_quad(ring);
                        }
                        let moved = painter.clone();
                        w.on_mouse_event(move |e: &MouseMoveEvent, phase, w, cx| {
                            if !phase.capture() {
                                return;
                            }
                            let _ = moved.update(cx, |s, cx| {
                                let Some((owner, hitbox)) = s.capture else {
                                    return;
                                };
                                if owner != index {
                                    return;
                                }
                                if w.captured_hitbox() != Some(hitbox)
                                    || e.pressed_button != Some(MouseButton::Left)
                                    || s.access(true) == Access::Blocked
                                {
                                    s.cancel(c::CancelReason::Interrupted, w, cx);
                                } else {
                                    s.drag_channel(index, e.position, false, w, cx);
                                    cx.stop_propagation();
                                }
                            });
                        });
                        let up = painter.clone();
                        w.on_mouse_event(move |e: &MouseUpEvent, phase, w, cx| {
                            if !phase.bubble() {
                                return;
                            }
                            let _ = up.update(cx, |s, cx| {
                                let Some((owner, hitbox)) = s.capture else {
                                    return;
                                };
                                if owner != index {
                                    return;
                                }
                                if w.captured_hitbox() != Some(hitbox)
                                    || e.button != MouseButton::Left
                                    || s.access(true) == Access::Blocked
                                {
                                    s.cancel(c::CancelReason::Interrupted, w, cx);
                                } else {
                                    s.drag_channel(index, e.position, true, w, cx);
                                    w.prevent_default();
                                    cx.stop_propagation();
                                }
                            });
                        });
                    },
                )
                .absolute()
                .left(px(8.))
                .right(px(8.))
                .top_0()
                .bottom_0(),
            )
            .child(self.record(&focus, index as u16, enabled, false));
        div()
            .w_full()
            .min_w(px(0.))
            .flex()
            .flex_col()
            .gap(px(2.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.))
                    .child(
                        div()
                            .min_w(px(0.))
                            .flex_1()
                            .text_ellipsis()
                            .child(label.clone()),
                    )
                    .child(
                        div()
                            .w(relative(0.6))
                            .min_w(px(0.))
                            .flex_shrink_0()
                            .child(self.editor_element(index + 1, window)),
                    ),
            )
            .child(crate::semantics::State {
                identity: None,
                busy: false,
                element: track,
                metadata: None,
                live: None,
                hidden: false,
                disabled: !enabled,
                read_only,
                modal: false,
            })
    }
}
