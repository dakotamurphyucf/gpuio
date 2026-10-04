//! Resolve bulky style refinements before traversing native children.
//! In debug builds these temporaries would otherwise accumulate per tree level.
use super::*;

pub(super) struct Render<'a> {
    pub node: &'a crate::tree::Node,
    pub styles: &'a [Style],
    pub input_presentation: Interaction,
    pub interaction: Interaction,
    pub disabled: bool,
    pub own_disabled: bool,
    pub checked: bool,
    pub indeterminate: bool,
    pub factor: Option<f32>,
    pub track_frame: Option<&'a carousel_track::Frame>,
}

impl View {
    pub(super) fn node_style(
        &mut self,
        tree: &crate::tree::Tree,
        render: Render<'_>,
        mut element: gpui::Stateful<gpui::Div>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (gpui::Stateful<gpui::Div>, Option<gpui::StyleRefinement>) {
        let Render {
            node,
            styles,
            input_presentation,
            interaction,
            disabled,
            own_disabled,
            checked,
            indeterminate,
            factor,
            track_frame,
        } = render;
        let id = node.id;
        let (mut styled, mut states) = apply_styles(element, styles, input_presentation, disabled);
        if node.tab_viewport.is_some() {
            for state in states.iter_mut().flatten() {
                // Viewport layout ownership applies to hover/focus/pressed states
                // too; retain their colors, dimensions and other presentation.
                state.flex_direction = None;
                state.flex_wrap = None;
                state.overflow = Default::default();
            }
        }
        if node.split_button.is_some() {
            styled = styled.group(split_button_view::GROUP);
        }
        if matches!(node.kind, Kind::Button | Kind::CommandButton) {
            styled = self.coordinate_split(
                tree,
                id,
                styled,
                input_presentation.pointer && !disabled,
                window,
            );
        }
        if let Some(factor) = factor {
            let style = styled.style();
            style.opacity = Some(style.opacity.unwrap_or(1.) * factor);
            for state in states.iter_mut().flatten() {
                animation::factor_state_opacity(state, factor);
            }
        }
        if let Some(config) = &node.overlay {
            for state in states.iter_mut().flatten() {
                overlay::constrain_state_style(config.kind, state);
            }
            if let Some(frame) = sheet_geometry::resolve(
                config.kind,
                crate::window_frame::content_bounds(window).size,
                config.width,
                node.sheet_insets,
            ) {
                sheet_geometry::constrain_box(
                    styled.style(),
                    &mut states,
                    &frame,
                    window.rem_size(),
                );
            }
        }
        element = styled;
        let [
            focused,
            hovered,
            pressed,
            checked_style,
            indeterminate_style,
            disabled_style,
            selected_style,
        ] = states;
        use gpui::Refineable;
        for style in [
            checked.then_some(checked_style).flatten(),
            indeterminate.then_some(indeterminate_style).flatten(),
        ]
        .into_iter()
        .flatten()
        {
            element.style().refine(&style);
        }
        if let Some(style) = focused {
            if let Some(editor) = self.editors.get(&id) {
                if editor.focus_handle(cx).is_focused(window) {
                    element.style().refine(&style);
                }
            } else if let Some(color) = self.color_inputs.get(&id) {
                if color.focused(window, cx) {
                    element.style().refine(&style);
                }
            } else if let Some(calendar) = self.calendars.get(&id) {
                if calendar.focus_handle(cx).is_focused(window) {
                    element.style().refine(&style);
                }
            } else if let Some(otp) = self.otps.get(&id) {
                if otp.focus_handle(cx).is_focused(window) {
                    element.style().refine(&style);
                }
            } else if let Some(number) = self.numbers.get(&id) {
                if number.focus_handle(cx).is_focused(window) {
                    element.style().refine(&style);
                }
            } else if let Some(slider) = self.sliders.get(&id) {
                if slider
                    .borrow()
                    .focus
                    .iter()
                    .any(|(_, focus)| focus.is_focused(window))
                {
                    element.style().refine(&style);
                }
            } else if let Some(selection) = self.selections.get(&id) {
                // The inner selectable text owns focus; the styled outer Div
                // must follow that handle just like native editor wrappers do.
                if selection.borrow().focus.is_focused(window) {
                    element.style().refine(&style);
                }
            } else {
                element = element.focus(move |_| style);
            }
        }
        if let Some(style) = hovered {
            element = element.hover(move |_| style);
        }
        if let Some(style) = pressed {
            if node.pointer.is_some() {
                if self.pointer_capture.borrow().is_active(id) {
                    element.style().refine(&style);
                }
            } else {
                element = element.active(move |_| style);
            }
        }
        if disabled {
            element.style().mouse_cursor = None;
            // A subtree policy does not repeatedly dim every nested container.
            // Individual controls retain their ordinary disabled appearance.
            if own_disabled {
                element = element.opacity(0.5 * factor.unwrap_or(1.));
            }
            if let Some(style) = disabled_style {
                element.style().refine(&style);
            }
        }
        if !interaction.pointer {
            element.style().mouse_cursor = None;
        }
        // Keep these large builder temporaries out of recursive traversal.
        if node.tab_viewport.is_some() {
            element = tab_viewport::style(element);
        }
        if node.carousel_track.is_some() {
            element = element.overflow_hidden();
        }
        if let Some(frame) = track_frame {
            element = frame.style(element);
        }
        (element, selected_style)
    }
}
