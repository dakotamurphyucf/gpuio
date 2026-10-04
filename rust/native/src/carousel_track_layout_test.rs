//! Measured-track feasibility on GPUI TestPlatform. This does not exercise a
//! public GPUIO track adapter yet; it verifies the geometry inputs that adapter
//! will consume, independently of the existing full-page carousel.
use crate::carousel_track_geometry::{Geometry, Item};
use gpui::{
    Context, FocusHandle, Render, ScrollHandle, TestAppContext, Window, div, point, prelude::*, px,
};

struct Track {
    vertical: bool,
    extent: f32,
    inset: f32,
    sizes: Vec<f32>,
    scroll: ScrollHandle,
    focus: Vec<FocusHandle>,
}
impl Render for Track {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut track = div()
            .id("track")
            .flex()
            .flex_nowrap()
            .gap(px(8.))
            .p(px(self.inset))
            .track_scroll(&self.scroll);
        track = if self.vertical {
            track.flex_col().w(px(80.)).h(px(self.extent))
        } else {
            track.flex_row().w(px(self.extent)).h(px(80.))
        };
        for (index, extent) in self.sizes.iter().enumerate() {
            let item = div()
                .id(index)
                .flex_none()
                .track_focus(&self.focus[index])
                .role(gpui::Role::Button)
                .aria_label(format!("Item {index}"))
                .bg(gpui::rgb(0x225588));
            track = track.child(if self.vertical {
                item.w(px(80.)).h(px(*extent))
            } else {
                item.w(px(*extent)).h(px(80.))
            });
        }
        // Custom carousel routing owns wheel behavior; no generic scroller
        // should compete with it. The outer viewport supplies clipping.
        div().overflow_hidden().child(track)
    }
}
fn geometry(track: &Track) -> Geometry {
    let viewport = track.scroll.bounds();
    let start = if track.vertical {
        viewport.origin.y
    } else {
        viewport.origin.x
    };
    let items = (0..track.sizes.len())
        .map(|index| {
            let bounds = track.scroll.bounds_for_item(index).unwrap();
            let (origin, extent) = if track.vertical {
                (bounds.origin.y, bounds.size.height)
            } else {
                (bounds.origin.x, bounds.size.width)
            };
            Item {
                start: f32::from(origin - start),
                extent: f32::from(extent),
            }
        })
        .collect();
    let limit = if track.vertical {
        track.scroll.max_offset().y
    } else {
        track.scroll.max_offset().x
    };
    Geometry::new(track.extent, f32::from(limit), items, false).unwrap()
}

#[test]
fn measured_bounds_ignore_scroll_offset_and_remeasure_resize_without_replacing_focus() {
    for vertical in [false, true] {
        let mut app = TestAppContext::single();
        let (owner, cx) = app.add_window_view(|_, cx| Track {
            vertical,
            extent: 100.,
            inset: 0.,
            sizes: vec![40., 120., 64.],
            scroll: ScrollHandle::new(),
            focus: (0..3).map(|_| cx.focus_handle().tab_stop(true)).collect(),
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let initial = owner.read_with(cx, |track, _| geometry(track));
        assert_eq!(
            initial.items(),
            [
                Item {
                    start: 0.,
                    extent: 40.
                },
                Item {
                    start: 48.,
                    extent: 120.
                },
                Item {
                    start: 176.,
                    extent: 64.
                },
            ]
        );
        assert_eq!(initial.snaps(), [0., -48., -140.]);
        let retained = cx.update(|window, cx| {
            owner.update(cx, |track, cx| {
                let focus = track.focus[1].clone();
                window.focus(&focus, cx);
                track.scroll.set_offset(if vertical {
                    point(px(0.), px(-48.))
                } else {
                    point(px(-48.), px(0.))
                });
                cx.notify();
                focus
            })
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(owner.read_with(cx, |track, _| geometry(track)), initial);
        cx.update(|window, cx| {
            owner.update(cx, |track, cx| {
                assert!(retained.is_focused(window));
                track.extent = 150.;
                track.sizes[0] = 60.;
                cx.notify();
            })
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        owner.read_with(cx, |track, _| {
            assert_eq!(track.focus[1], retained);
            let changed = geometry(track);
            assert_eq!(changed.items()[1].start, 68.);
            assert_eq!(changed.snaps(), [0., -68., -110.]);
        });
        cx.update(|window, _| assert!(retained.is_focused(window)));
        owner.update(cx, |track, cx| {
            track.extent = 100.;
            track.sizes[0] = 40.;
            track.inset = 16.;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        owner.read_with(cx, |track, _| {
            let padded = geometry(track);
            assert_eq!(padded.items()[0].start, 16.);
            assert_eq!(padded.snaps(), [-16., -64., -172.]);
        });
    }
}
