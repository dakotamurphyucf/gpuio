//! Stateless tab target variants. Native focus/selection stay with the choice owner.
use gpui::{Div, Hsla, Refineable, Stateful, StyleRefinement, div, prelude::*, px};
use gpuio_protocol::tab_appearance::{Config, Variant};

pub(super) fn bar(mut base: Stateful<Div>, config: &Config, accent: Hsla) -> Stateful<Div> {
    let mut defaults = div().gap(px(config.gap as f32));
    defaults = match config.variant {
        Variant::Segmented => defaults.p(px(4.)).rounded(px(8.)).bg(accent.alpha(0.06)),
        Variant::Tab | Variant::Underline => defaults.border_b_1().border_color(accent.alpha(0.25)),
        Variant::Pill | Variant::Outline => defaults,
    };
    // Explicit root styles win over presentation defaults.
    let mut style = defaults.style().clone();
    style.refine(base.style());
    *base.style() = style;
    base
}

pub(super) fn target(
    mut target: Stateful<Div>,
    config: &Config,
    selected: bool,
    accent: Hsla,
) -> Stateful<Div> {
    target = target
        .p_0()
        .px(px(config.padding as f32))
        .h(px(config.height as f32))
        .flex_shrink_0();
    match config.variant {
        Variant::Tab => target
            .border_1()
            .border_color(if selected {
                accent.alpha(0.35)
            } else {
                accent.alpha(0.)
            })
            .rounded_t(px(6.))
            .bg(accent.alpha(if selected { 0.08 } else { 0. })),
        Variant::Outline => target.border_1().rounded(px(6.)).border_color(if selected {
            accent
        } else {
            accent.alpha(0.)
        }),
        Variant::Pill => target
            .rounded(px(config.height as f32 / 2.))
            .bg(accent.alpha(if selected { 0.15 } else { 0. })),
        Variant::Segmented => {
            target
                .rounded(px(6.))
                .bg(accent.alpha(if selected { 0.15 } else { 0. }))
        }
        Variant::Underline => {
            target
                .border_b_2()
                .border_color(if selected { accent } else { accent.alpha(0.) })
        }
    }
}

pub(super) struct State {
    pub selected: bool,
    pub scrolling: bool,
    pub focused: bool,
    pub disabled: bool,
    pub pointer: bool,
    pub accent: Hsla,
}
pub(super) fn styles(
    mut target: Stateful<Div>,
    config: &Config,
    id: &str,
    state: State,
) -> Stateful<Div> {
    let item = config
        .item_styles
        .iter()
        .find(|(key, _)| key == id)
        .map(|(_, styles)| styles.as_slice())
        .unwrap_or_default();
    let scrolling = state.scrolling;
    let refinement = |state| {
        let mut result = crate::appearance::refinement(&config.tab_style, state);
        result.refine(&crate::appearance::refinement(item, state));
        if scrolling {
            result.flex_shrink = None;
        }
        result
    };
    target.style().refine(&refinement(0));
    if state.focused {
        target.style().refine(&refinement(1));
    }
    if state.selected {
        target.style().refine(&refinement(7));
    }
    if state.disabled {
        target.style().refine(&refinement(6));
    }
    // Always register interaction styles while enabled so changing the selected
    // variant cannot leave a stale cached hover state.
    let hover = if state.pointer && !state.disabled {
        let mut style = StyleRefinement::default();
        if !state.selected {
            style.background = Some(state.accent.alpha(0.06).into());
        }
        style.refine(&refinement(2));
        style
    } else {
        StyleRefinement::default()
    };
    let pressed = if state.pointer && !state.disabled {
        refinement(3)
    } else {
        StyleRefinement::default()
    };
    target = target.hover(move |_| hover).active(move |_| pressed);
    if !state.pointer || state.disabled {
        target.style().mouse_cursor = None;
    }
    target
}

// Resolve the normal inherited foreground before applying selected declarations.
pub(super) fn normal_color(config: &Config, id: &str, focused: bool, fallback: Hsla) -> Hsla {
    let item = config
        .item_styles
        .iter()
        .find(|(key, _)| key == id)
        .map(|(_, styles)| styles.as_slice())
        .unwrap_or_default();
    let mut result = crate::appearance::refinement(&config.tab_style, 0);
    result.refine(&crate::appearance::refinement(item, 0));
    if focused {
        result.refine(&crate::appearance::refinement(&config.tab_style, 1));
        result.refine(&crate::appearance::refinement(item, 1));
    }
    result.text.color.unwrap_or(fallback)
}
