//! Default native presentation before application styles. Keep this nonrecursive
//! preparation outside retained-tree traversal so its temporary builders do not
//! accumulate on the stack at every level of a rich label or container tree.
use super::*;

impl View {
    pub(super) fn node_presentation(
        &mut self,
        tree: &crate::tree::Tree,
        node: &crate::tree::Node,
        interaction: Interaction,
        mut element: gpui::Stateful<gpui::Div>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (gpui::Stateful<gpui::Div>, Option<image_corners::Shared>) {
        let id = node.id;
        if (node.handler.is_some()
            || node.command_ref.is_some()
            || node.control.is_some()
            || node.editor.is_some()
            || (node.kind == Kind::Text && interaction.selectable.unwrap_or(false)))
            && window_regions::within_title_bar(tree, id, window)
        {
            element = element.block_mouse_except_scroll();
        }
        // Do not enter the large event builder for ordinary nodes. In debug
        // builds its stack frame is reserved even when it returns immediately.
        if node.window_region.is_some() {
            element = self.window_region_element(element, node, window, cx);
        }
        let identity = ((id.generation() as u64) << 32) | id.slot() as u64;
        let mut image_corners = None;
        if node.kind == Kind::Text
            && !interaction.selectable.unwrap_or(false)
            && !node.text.is_empty()
        {
            element = element
                .role(gpui::Role::Label)
                .aria_label(node.text.clone());
        }
        if let Some(description) = tree.tooltip_description(id) {
            element = element.aria_description(description.to_owned());
        }
        if matches!(
            node.kind,
            Kind::Container
                | Kind::Animated
                | Kind::AnimationProgram
                | Kind::ContainerQuery
                | Kind::TabPanel
                | Kind::Panel
                | Kind::Disclosure
                | Kind::Accordion
                | Kind::NavigationStack
                | Kind::Carousel
                | Kind::CarouselTrack
                | Kind::CarouselTrackGroup
                | Kind::FocusScope
                | Kind::CommandScope
                | Kind::RadioGroup
                | Kind::Rating
                | Kind::TabBar
                | Kind::PointerArea
                | Kind::InputRegion
                | Kind::HighlightScope
                | Kind::DragSource
                | Kind::DropTarget
        ) {
            element = element.flex().flex_col();
        } else if let Some(frame) = &node.editor_frame {
            element = element.flex().items_center().gap(px(frame.gap as f32));
        } else if matches!(node.kind, Kind::Select | Kind::ChoicePicker) {
            element = element
                .flex()
                .items_center()
                .justify_between()
                .gap(px(8.))
                .p(px(8.))
                .border_1()
                .border_color(rgba(0x80808080))
                .rounded(px(4.));
        }
        if node.container_query.is_some() || node.navigation_stack.is_some() {
            element = element.size_full();
        }
        if let Some(config) = &node.drag_source {
            element = element
                .role(gpui::Role::Group)
                .aria_label(config.label().to_owned());
        }
        if let Some(config) = &node.drop_target {
            element = element
                .role(gpui::Role::Group)
                .aria_label(config.label().to_owned());
        }
        if let Some(config) = &node.pointer {
            element = element
                .role(gpui::Role::Group)
                .aria_label(config.label.clone());
        }

        if node.image.is_some() && node.spinner.is_none() {
            let (image, corners) = self.image_element(tree, node, interaction, element, window, cx);
            element = image;
            image_corners = Some(corners);
        }
        if node.rating.is_some() {
            element = element
                .flex()
                .flex_row()
                .items_center()
                .flex_nowrap()
                .border_1()
                .border_color(gpui::transparent_black())
                .rounded(px(4.))
                .text_color(rgba(0xe8ad36ff))
                .focus(|style| style.border_color(rgba(0x6688ffff)));
        }
        if node.color_input.is_some() {
            element = element
                .w(px(296.))
                .min_w(px(0.))
                .text_size(px(13.))
                .overflow_hidden();
        }
        if node.calendar.is_some() {
            element = element
                .w(px(296.))
                .min_w(px(0.))
                .text_size(px(13.))
                .overflow_hidden();
        }
        if let Some(otp) = &node.otp_input {
            let appearance = node.otp_appearance.as_deref().cloned().unwrap_or_default();
            let cell = appearance.cell_width.unwrap_or(32.) as f32;
            element = element
                .min_w(px(40.))
                .w(px(appearance.width(otp.config.policy.length(), cell)))
                .h(px(40.));
        }
        if node.number_input.is_some() {
            element = element.min_w(px(80.)).w(px(180.)).min_h(px(40.));
        }
        if let Some(slider) = &node.slider {
            element = element
                .relative()
                .min_w(px(24.))
                .min_h(px(24.))
                .text_color(rgba(0x6688ffff));
            element = match slider.config.axis {
                gpuio_protocol::slider::Axis::Horizontal => element.w(px(180.)).h(px(24.)),
                gpuio_protocol::slider::Axis::Vertical => element.w(px(24.)).h(px(180.)),
            };
        }
        if let Some(config) = &node.avatar {
            element = element
                .w(px(32.))
                .h(px(32.))
                .rounded(px(999.))
                .overflow_hidden()
                .bg(rgba(0x71809630))
                .text_color(rgba(0x718096ff))
                .text_size(px(12.));
            if let Some(label) = &config.label {
                element = element.role(gpui::Role::Image).aria_label(label.clone());
            }
            if config.source.is_none() {
                if node.children.is_empty() {
                    element = element.child(avatar::fallback(config.fallback.clone().into()));
                } else {
                    let corners = image_corners::Shared::default();
                    element = element.child(self.avatar_slot(
                        tree,
                        node,
                        corners.clone(),
                        None,
                        interaction.passive_disabled,
                        cx,
                    ));
                    image_corners = Some(corners);
                }
            }
        }
        if let Some(config) = &node.loading {
            let corners = image_corners::Shared::default();
            image_corners = Some(corners.clone());
            let spinner = config.kind == gpuio_protocol::loading::Kind::Spinner;
            element = element
                .w(px(if spinner { 20. } else { 160. }))
                .h(px(if spinner { 20. } else { 16. }))
                .rounded(px(4.))
                .overflow_hidden()
                .role(gpui::Role::ProgressIndicator)
                .aria_label(config.label.clone());
            // A standalone indicator preserves existing focus. Inside a Link,
            // let mouse-down reach the root's native focus behavior; cancelling
            // it would activate the destination while leaving keyboard focus behind.
            if !interaction.link_content {
                element = element.on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                    window.prevent_default()
                });
            }
            if node.spinner.is_some() {
                let painted = self.focus.borrow().paint_visible(tree, id);
                let inert = !self.focus.borrow().visible(id);
                if painted {
                    element = element.child(self.spinner_element(tree, node, inert, window, cx));
                }
            } else if self.focus.borrow().paint_visible(tree, id) {
                element = element.text_color(rgba(0x8b98abff));
                element = element.child(loading::indicator(
                    config,
                    identity,
                    // Inert subtrees retain their visual content, but must not
                    // restart a decorative clock behind the interaction fence.
                    cx.reduce_motion() || !self.focus.borrow().visible(id),
                    corners,
                    #[cfg(feature = "native-tests")]
                    self.loading_probes.entry(id).or_default().clone(),
                ));
            }
        }
        if let Some(config) = &node.progress {
            let circle = node.progress_presentation.as_ref().is_some_and(|config| {
                config.shape == gpuio_protocol::progress_presentation::Shape::Circle
            });
            element = element
                .text_color(rgba(0x4d8cffff))
                .role(gpui::Role::ProgressIndicator)
                .aria_label(config.label.clone())
                .aria_min_numeric_value(0.)
                .aria_max_numeric_value(100.);
            if circle {
                element = element
                    .flex()
                    .flex_col()
                    .w(px(32.))
                    .h(px(32.))
                    .items_center()
                    .justify_center();
            } else {
                element = element
                    .w(px(200.))
                    .h(px(8.))
                    .rounded(px(4.))
                    .overflow_hidden()
                    .bg(rgba(0x80808040))
                    .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                        window.prevent_default()
                    });
            }
            if let Some(fraction) = config.fraction {
                element = element.aria_numeric_value(fraction * 100.);
            }
            let corners = image_corners::Shared::default();
            image_corners = Some(corners.clone());
            if self.focus.borrow().paint_visible(tree, id) {
                let inert = !self.focus.borrow().visible(id);
                element = element.child(self.progress_element(node, corners, inert));
            }
        }
        if let Some(config) = &node.overlay {
            let available = crate::window_frame::content_bounds(window).size;
            element = element
                .w(px(config.width as f32).min((available.width - px(32.)).max(px(1.))))
                .max_h((available.height - px(32.)).max(px(1.)))
                .overflow_y_scroll()
                .p(px(16.))
                .gap(px(8.))
                .rounded(px(8.))
                .bg(rgba(0x25272aff))
                .border_1()
                .border_color(rgba(0x80808080));
        }
        (element, image_corners)
    }
}
