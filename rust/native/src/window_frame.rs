//! Window-owned client frame. Geometry stays native through compositor changes.
use gpui::{
    AnyElement, Bounds, CursorStyle, Decorations, Edges, Hsla, IntoElement, MouseButton, Pixels,
    Point, ResizeEdge, Size, Tiling, Window, div, point, prelude::*, px,
};
use gpuio_protocol::{
    v1::{Color, Field, Style},
    window::Frame,
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Layout {
    pub shadow: Edges<Pixels>,
    pub border: Edges<Pixels>,
    pub frame: Bounds<Pixels>,
    pub content: Bounds<Pixels>,
    pub tiling: Tiling,
    pub visible: bool,
}

fn axis(
    size: Pixels,
    before: bool,
    after: bool,
    shadow: Pixels,
) -> (Pixels, Pixels, Pixels, Pixels) {
    let requested = (shadow + px(1.)) * ((u8::from(before) + u8::from(after)) as f32);
    let factor = if requested > px(0.) {
        f32::from((size - px(1.)).max(px(0.))).min(f32::from(requested)) / f32::from(requested)
    } else {
        0.
    };
    let edge = |enabled| {
        if enabled {
            (shadow * factor, px(factor))
        } else {
            (px(0.), px(0.))
        }
    };
    let (s1, b1) = edge(before);
    let (s2, b2) = edge(after);
    (s1, b1, s2, b2)
}
pub(crate) fn layout(
    size: Size<Pixels>,
    tiling: Tiling,
    fullscreen: bool,
    shadow: Pixels,
) -> Layout {
    let size = gpui::size(size.width.max(px(0.)), size.height.max(px(0.)));
    let (left, bl, right, br) = axis(
        size.width,
        !fullscreen && !tiling.left,
        !fullscreen && !tiling.right,
        shadow,
    );
    let (top, bt, bottom, bb) = axis(
        size.height,
        !fullscreen && !tiling.top,
        !fullscreen && !tiling.bottom,
        shadow,
    );
    Layout {
        shadow: Edges {
            top,
            right,
            bottom,
            left,
        },
        border: Edges {
            top: bt,
            right: br,
            bottom: bb,
            left: bl,
        },
        frame: Bounds::new(
            point(left, top),
            gpui::size(
                (size.width - left - right).max(px(0.)),
                (size.height - top - bottom).max(px(0.)),
            ),
        ),
        content: Bounds::new(
            point(left + bl, top + bt),
            gpui::size(
                (size.width - left - right - bl - br).max(px(0.)),
                (size.height - top - bottom - bt - bb).max(px(0.)),
            ),
        ),
        tiling,
        visible: !fullscreen,
    }
}

/// Set before building application/popup elements. Never zero the platform inset
/// just because the visual padding disappeared on tiled/fullscreen sides.
pub(crate) fn configure(frame: Option<Frame>, window: &mut Window) {
    if let Some(frame) = frame
        && cfg!(target_os = "linux")
        && matches!(window.window_decorations(), Decorations::Client { .. })
    {
        window.set_client_inset(px(frame.shadow_size as f32));
    }
}
pub(crate) fn content_bounds(window: &Window) -> Bounds<Pixels> {
    if let Some(shadow) = window.client_inset()
        && let Decorations::Client { tiling } = window.window_decorations()
    {
        layout(
            window.viewport_size(),
            tiling,
            window.is_fullscreen(),
            shadow,
        )
        .content
    } else {
        Bounds::new(Point::default(), window.viewport_size())
    }
}
pub(crate) fn border_color(styles: &[Style]) -> Hsla {
    styles
        .iter()
        .rev()
        .find_map(|style| match style {
            Style::Fields(fields) => fields.iter().rev().find_map(|field| match field {
                Field::BorderColor(Color::Rgba(value)) => Some(gpui::rgba(*value as u32).into()),
                _ => None,
            }),
            _ => None,
        })
        .unwrap_or_else(|| gpui::rgba(0x80808060).into())
}

/// Narrow bands around the frame, never the entire transparent shadow gutter.
pub(crate) fn resize_edge(l: Layout, position: Point<Pixels>, band: Pixels) -> Option<ResizeEdge> {
    if !l.visible {
        return None;
    }
    // GPUI hitboxes are half-open. Use the exact painted rectangles and z-order,
    // including compressed windows where bands can overlap.
    resize_zones(l, band)
        .into_iter()
        .rev()
        .find_map(|(edge, enabled, origin, size)| {
            (enabled && Bounds::new(origin, size).contains(&position)).then_some(edge)
        })
}
fn cursor(edge: ResizeEdge) -> CursorStyle {
    match edge {
        ResizeEdge::Top | ResizeEdge::Bottom => CursorStyle::ResizeUpDown,
        ResizeEdge::Left | ResizeEdge::Right => CursorStyle::ResizeLeftRight,
        ResizeEdge::TopLeft | ResizeEdge::BottomRight => CursorStyle::ResizeUpLeftDownRight,
        ResizeEdge::TopRight | ResizeEdge::BottomLeft => CursorStyle::ResizeUpRightDownLeft,
    }
}
pub(crate) fn wrap(
    content: AnyElement,
    frame: Option<Frame>,
    stroke: Hsla,
    window: &Window,
) -> AnyElement {
    let Some(config) = frame else {
        return content;
    };
    if !cfg!(target_os = "linux") {
        return content;
    }
    let Decorations::Client { tiling } = window.window_decorations() else {
        return content;
    };
    let l = layout(
        window.viewport_size(),
        tiling,
        window.is_fullscreen(),
        px(config.shadow_size as f32),
    );
    render(
        content,
        l,
        config,
        stroke,
        window.is_window_active(),
        window.is_resizable(),
    )
}
fn resize_zones(l: Layout, b: Pixels) -> [(ResizeEdge, bool, Point<Pixels>, Size<Pixels>); 8] {
    // Paint priority mirrors resize_edge, including overlapping tiny-window bands.
    [
        (
            ResizeEdge::Left,
            !l.tiling.left,
            point(l.frame.left() - b, l.frame.top() - b),
            gpui::size(b * 2., l.frame.size.height + b * 2.),
        ),
        (
            ResizeEdge::Bottom,
            !l.tiling.bottom,
            point(l.frame.left() - b, l.frame.bottom() - b),
            gpui::size(l.frame.size.width + b * 2., b * 2.),
        ),
        (
            ResizeEdge::Right,
            !l.tiling.right,
            point(l.frame.right() - b, l.frame.top() - b),
            gpui::size(b * 2., l.frame.size.height + b * 2.),
        ),
        (
            ResizeEdge::Top,
            !l.tiling.top,
            point(l.frame.left() - b, l.frame.top() - b),
            gpui::size(l.frame.size.width + b * 2., b * 2.),
        ),
        (
            ResizeEdge::BottomLeft,
            !l.tiling.bottom && !l.tiling.left,
            point(l.frame.left() - b, l.frame.bottom() - b),
            gpui::size(b * 2., b * 2.),
        ),
        (
            ResizeEdge::BottomRight,
            !l.tiling.bottom && !l.tiling.right,
            point(l.frame.right() - b, l.frame.bottom() - b),
            gpui::size(b * 2., b * 2.),
        ),
        (
            ResizeEdge::TopRight,
            !l.tiling.top && !l.tiling.right,
            point(l.frame.right() - b, l.frame.top() - b),
            gpui::size(b * 2., b * 2.),
        ),
        (
            ResizeEdge::TopLeft,
            !l.tiling.top && !l.tiling.left,
            point(l.frame.left() - b, l.frame.top() - b),
            gpui::size(b * 2., b * 2.),
        ),
    ]
}
fn render(
    content: AnyElement,
    l: Layout,
    config: Frame,
    stroke: Hsla,
    active: bool,
    resizable: bool,
) -> AnyElement {
    let mut surface = div()
        .absolute()
        .left(l.frame.left())
        .top(l.frame.top())
        .w(l.frame.size.width)
        .h(l.frame.size.height)
        .border_t(l.border.top)
        .border_r(l.border.right)
        .border_b(l.border.bottom)
        .border_l(l.border.left)
        .border_color(stroke)
        .overflow_hidden()
        .child(content);
    let available = l
        .shadow
        .top
        .min(l.shadow.right)
        .min(l.shadow.bottom)
        .min(l.shadow.left);
    if l.visible && !l.tiling.is_tiled() && available > px(0.) {
        surface = surface.shadow(vec![gpui::BoxShadow {
            color: gpui::black().opacity(if active { 0.18 } else { 0.126 }),
            blur_radius: available.min(px(10.)),
            spread_radius: px(-1.),
            offset: point(px(0.), available.min(px(2.))),
            inset: false,
        }]);
    }
    let mut root = div()
        .relative()
        .size_full()
        .bg(gpui::transparent_black())
        .child(surface);
    if resizable && l.visible {
        // Eight bounded zones, with corners last. Read current geometry again
        // on down: a compositor resize may have retired this painted band.
        let b = px(config.resize_hit_size as f32);
        let zones = resize_zones(l, b);
        for (edge, enabled, origin, size) in zones {
            if !enabled {
                continue;
            }
            root = root.child(
                div()
                    .absolute()
                    .left(origin.x)
                    .top(origin.y)
                    .w(size.width)
                    .h(size.height)
                    .cursor(cursor(edge))
                    .block_mouse_except_scroll()
                    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
                        if !window.is_window_active()
                            || !window.is_resizable()
                            || window.captured_hitbox().is_some()
                        {
                            return;
                        }
                        let Decorations::Client { tiling } = window.window_decorations() else {
                            return;
                        };
                        let current = layout(
                            window.viewport_size(),
                            tiling,
                            window.is_fullscreen(),
                            px(config.shadow_size as f32),
                        );
                        if resize_edge(current, event.position, b) == Some(edge) {
                            window.start_window_resize(edge);
                            cx.stop_propagation();
                        }
                    }),
            );
        }
    }
    root.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::size;

    fn tiling(bits: u8) -> Tiling {
        Tiling {
            top: bits & 1 != 0,
            right: bits & 2 != 0,
            bottom: bits & 4 != 0,
            left: bits & 8 != 0,
        }
    }

    #[test]
    fn all_tiling_states_preserve_content_and_fullscreen_removes_the_frame() {
        for bits in 0..16 {
            let tiled = tiling(bits);
            let l = layout(size(px(800.), px(600.)), tiled, false, px(20.));
            for (edge, shadow, border) in [
                (tiled.top, l.shadow.top, l.border.top),
                (tiled.right, l.shadow.right, l.border.right),
                (tiled.bottom, l.shadow.bottom, l.border.bottom),
                (tiled.left, l.shadow.left, l.border.left),
            ] {
                assert_eq!(shadow, px(if edge { 0. } else { 20. }));
                assert_eq!(border, px(if edge { 0. } else { 1. }));
            }
            for width in [0., 0.5, 1., 2., 12., 800.] {
                for height in [0., 0.5, 1., 2., 12., 600.] {
                    for shadow in [0., 20., 128.] {
                        let viewport = size(px(width), px(height));
                        for fullscreen in [false, true] {
                            let l = layout(viewport, tiled, fullscreen, px(shadow));
                            assert!(l.content.left() >= px(0.) && l.content.top() >= px(0.));
                            assert!(l.content.right() <= px(width + 0.0001));
                            assert!(l.content.bottom() <= px(height + 0.0001));
                            assert!(
                                l.content.size.width >= px(0.) && l.content.size.height >= px(0.)
                            );
                            if fullscreen {
                                assert_eq!(l.content, Bounds::new(Point::default(), viewport));
                                assert_eq!(resize_edge(l, Point::default(), px(4.)), None);
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn resize_bands_respect_tiling_corners_gutters_and_half_open_boundaries() {
        for bits in 0..16 {
            let l = layout(size(px(800.), px(600.)), tiling(bits), false, px(20.));
            let hit = |x, y| resize_edge(l, point(x, y), px(4.));
            for (tiled, edge, position) in [
                (
                    l.tiling.top,
                    ResizeEdge::Top,
                    point(px(400.), l.frame.top()),
                ),
                (
                    l.tiling.bottom,
                    ResizeEdge::Bottom,
                    point(px(400.), l.frame.bottom()),
                ),
                (
                    l.tiling.left,
                    ResizeEdge::Left,
                    point(l.frame.left(), px(300.)),
                ),
                (
                    l.tiling.right,
                    ResizeEdge::Right,
                    point(l.frame.right(), px(300.)),
                ),
            ] {
                assert_eq!(hit(position.x, position.y), (!tiled).then_some(edge));
            }
            for (vertical, horizontal, corner, v, h, x, y) in [
                (
                    l.tiling.top,
                    l.tiling.left,
                    ResizeEdge::TopLeft,
                    ResizeEdge::Top,
                    ResizeEdge::Left,
                    l.frame.left(),
                    l.frame.top(),
                ),
                (
                    l.tiling.top,
                    l.tiling.right,
                    ResizeEdge::TopRight,
                    ResizeEdge::Top,
                    ResizeEdge::Right,
                    l.frame.right(),
                    l.frame.top(),
                ),
                (
                    l.tiling.bottom,
                    l.tiling.left,
                    ResizeEdge::BottomLeft,
                    ResizeEdge::Bottom,
                    ResizeEdge::Left,
                    l.frame.left(),
                    l.frame.bottom(),
                ),
                (
                    l.tiling.bottom,
                    l.tiling.right,
                    ResizeEdge::BottomRight,
                    ResizeEdge::Bottom,
                    ResizeEdge::Right,
                    l.frame.right(),
                    l.frame.bottom(),
                ),
            ] {
                let expected = match (vertical, horizontal) {
                    (false, false) => Some(corner),
                    (false, true) => Some(v),
                    (true, false) => Some(h),
                    (true, true) => None,
                };
                assert_eq!(hit(x, y), expected);
            }
            assert_eq!(hit(px(400.), px(300.)), None);
        }
        let l = layout(size(px(800.), px(600.)), tiling(0), false, px(20.));
        for (x, y, expected) in [
            (4., 300., None),
            (15.99, 300., None),
            (16., 300., Some(ResizeEdge::Left)),
            (23.99, 300., Some(ResizeEdge::Left)),
            (24., 300., None),
            (400., 16., Some(ResizeEdge::Top)),
            (400., 24., None),
        ] {
            assert_eq!(resize_edge(l, point(px(x), px(y)), px(4.)), expected);
        }
        let tiny = layout(size(px(1.), px(1.)), tiling(0), false, px(20.));
        assert_eq!(
            resize_edge(tiny, point(px(0.5), px(0.5)), px(4.)),
            Some(ResizeEdge::TopLeft)
        );
    }

    #[cfg(feature = "native-canvas-tests")]
    #[test]
    fn native_layout_preserves_content_focus_across_tiling_and_fullscreen() {
        use gpui::{Context, FocusHandle, Render, TestAppContext};
        struct Fixture {
            tiling: Tiling,
            fullscreen: bool,
            focus: FocusHandle,
        }
        impl Render for Fixture {
            fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
                let child = div()
                    .id("retained-content")
                    .size_full()
                    .track_focus(&self.focus)
                    .debug_selector(|| "frame-content".into())
                    .into_any_element();
                // Explicit client layout on TestPlatform, not an OS decoration test.
                render(
                    child,
                    layout(
                        window.viewport_size(),
                        self.tiling,
                        self.fullscreen,
                        px(20.),
                    ),
                    Frame::default(),
                    gpui::rgba(0x112233ff).into(),
                    true,
                    true,
                )
            }
        }
        let mut app = TestAppContext::single();
        let (owner, cx) = app.add_window_view(|window, cx| {
            let focus = cx.focus_handle();
            focus.focus(window, cx);
            Fixture {
                tiling: tiling(0),
                fullscreen: false,
                focus,
            }
        });
        for extent in [size(px(800.), px(600.)), size(px(12.), px(8.))] {
            cx.simulate_resize(extent);
            for bits in 0..16 {
                for fullscreen in [false, true] {
                    owner.update(cx, |view, cx| {
                        view.tiling = tiling(bits);
                        view.fullscreen = fullscreen;
                        cx.notify();
                    });
                    cx.run_until_parked();
                    cx.update(|window, cx| window.draw(cx).clear(cx));
                    let actual = cx.debug_bounds("frame-content").unwrap();
                    let expected = layout(extent, tiling(bits), fullscreen, px(20.)).content;
                    // Layout rounds fractional borders to device pixels.
                    assert!((actual.left() - expected.left()).abs() <= px(1.));
                    assert!((actual.top() - expected.top()).abs() <= px(1.));
                    assert!((actual.size.width - expected.size.width).abs() <= px(1.));
                    assert!((actual.size.height - expected.size.height).abs() <= px(1.));
                    cx.update(|window, cx| assert!(owner.read(cx).focus.is_focused(window)));
                }
            }
        }
    }
}
