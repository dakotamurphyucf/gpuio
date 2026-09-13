//! Native progress paint. The indeterminate cycle is GPUI-owned and exists only
//! while a visible indeterminate indicator participates in rendering.
use gpui::{Animation, AnimationExt, canvas, div, prelude::*, relative};
use std::time::Duration;

#[cfg(feature = "native-tests")]
#[derive(Clone, Copy, Default)]
pub(super) struct Paint {
    pub bounds: gpui::Bounds<gpui::Pixels>,
    pub color: gpui::Hsla,
    pub count: u64,
}
#[cfg(feature = "native-tests")]
pub(super) type Probe = std::rc::Rc<std::cell::Cell<Paint>>;

pub(super) fn indicator(
    fraction: Option<f64>,
    identity: u64,
    #[cfg(feature = "native-tests")] probe: Probe,
) -> gpui::AnyElement {
    let fill = canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            #[cfg(feature = "native-tests")]
            probe.set(Paint {
                bounds,
                color: window.text_style().color,
                count: probe.get().count + 1,
            });
            window.paint_quad(gpui::fill(bounds, window.text_style().color));
        },
    )
    .size_full();
    let bar = div().absolute().top_0().h_full().child(fill);
    match fraction {
        Some(fraction) => bar.left_0().w(relative(fraction as f32)).into_any_element(),
        None => bar
            .w(relative(0.25))
            .with_animation(
                ("progress-cycle", identity),
                Animation::new(Duration::from_millis(1500)).repeat(),
                |bar, phase| bar.left(relative(phase * 1.25 - 0.25)),
            )
            .into_any_element(),
    }
}
