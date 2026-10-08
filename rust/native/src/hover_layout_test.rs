//! A style painted after relayout must also be invalidated on the next pointer exit.
use gpui::{
    App, Context, Hsla, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    TestAppContext, Window, canvas, div, point, px, rgb,
};
use std::{cell::Cell, rc::Rc};

struct MovingHover {
    left: f32,
    group: bool,
    painted: Rc<Cell<Hsla>>,
    width: Rc<Cell<f32>>,
}
impl Render for MovingHover {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let painted = self.painted.clone();
        let width = self.width.clone();
        let leaf = div()
            .id("hover-content")
            .w(px(40.))
            .h_full()
            .text_color(rgb(0x0000ff))
            .child(canvas(
                move |bounds, _, _| width.set(f32::from(bounds.size.width)),
                move |_, _, window, _: &mut App| painted.set(window.text_style().color),
            ));
        let leaf = if self.group {
            leaf.group_hover("moving-group", |style| {
                style.w(px(80.)).text_color(rgb(0xff0000))
            })
        } else {
            leaf.hover(|style| style.w(px(80.)).text_color(rgb(0xff0000)))
        };
        div().relative().size_full().child(
            div()
                .id("moving-hover")
                .group("moving-group")
                .absolute()
                .left(px(self.left))
                .top_0()
                .size(px(40.))
                .child(leaf),
        )
    }
}

fn assert_hover_exit(initial_under_pointer: bool, group: bool, check_layout: bool) {
    let mut app = TestAppContext::single();
    let painted = Rc::new(Cell::new(Hsla::default()));
    let width = Rc::new(Cell::new(0.));
    let (view, cx) = app.add_window_view(|_, _| MovingHover {
        left: if initial_under_pointer { 0. } else { 40. },
        painted: painted.clone(),
        width: width.clone(),
        group,
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.simulate_mouse_move(point(px(100.), px(20.)), None, Default::default());
    assert_eq!(painted.get(), rgb(0x0000ff).into());
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.left = 90.;
            cx.notify();
        });
        window.draw(cx).clear(cx);
    });
    assert_eq!(
        painted.get(),
        rgb(0xff0000).into(),
        "relayout paints hovered state"
    );
    if check_layout {
        // Existing hover layout semantics apply on pointer motion within the target.
        // Paint-state bookkeeping must not suppress that layout invalidation.
        cx.simulate_mouse_move(point(px(110.), px(20.)), None, Default::default());
        assert_eq!(
            width.get(),
            80.,
            "hover layout remains responsive after relayout"
        );
    }
    cx.simulate_mouse_move(point(px(180.), px(20.)), None, Default::default());
    assert_eq!(
        painted.get(),
        rgb(0x0000ff).into(),
        "pointer exit must invalidate the painted hover state"
    );
}

#[test]
fn pointer_exit_clears_hover_painted_after_stationary_pointer_relayout() {
    assert_hover_exit(false, false, false);
}

#[test]
fn pointer_exit_clears_hover_from_initial_paint() {
    assert_hover_exit(true, false, false);
}

#[test]
fn pointer_exit_clears_group_hover_painted_after_stationary_pointer_relayout() {
    assert_hover_exit(false, true, false);
}

#[test]
fn pointer_exit_clears_group_hover_from_initial_paint() {
    assert_hover_exit(true, true, false);
}

#[test]
fn hover_layout_still_updates_on_pointer_motion_after_relayout() {
    assert_hover_exit(false, false, true);
    assert_hover_exit(false, true, true);
}
