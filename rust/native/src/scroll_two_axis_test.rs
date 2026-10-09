//! Ordinary two-axis containers, through real layout and production wheel routing.
use super::*;

fn inner_style() -> Vec<Style> {
    let mut styles = dimensions(300., 180.);
    styles.push(Style::Fields(vec![
        Field::OverflowX(3),
        Field::OverflowY(3),
    ]));
    styles
}

pub(super) async fn exercise(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) {
    apply(
        cx,
        window,
        vec![
            Op::Create(node(11), Kind::Container, "".into(), None),
            Op::SetStyle(node(11), scrolling(360., 240., false)),
            Op::Create(node(12), Kind::Container, "".into(), None),
            Op::SetStyle(node(12), inner_style()),
            Op::Create(node(13), Kind::Text, "Two-axis content\n".repeat(40), None),
            Op::SetStyle(node(13), dimensions(800., 700.)),
            Op::Create(node(14), Kind::Text, "Ancestor extent".into(), None),
            Op::SetStyle(node(14), dimensions(300., 900.)),
            Op::Splice(node(12), 0, 0, vec![node(13)]),
            Op::Splice(node(11), 0, 0, vec![node(12), node(14)]),
            Op::SetRoot(Some(node(11))),
        ],
    );
    frame(cx, window).await;
    let point = bounds(cx, window, 12).center();
    let owner = window
        .update(cx, |view, _, _| Rc::downgrade(&view.scrolls[&node(12)]))
        .unwrap();
    wheel(cx, window, point, -70., -30.).await;
    assert_eq!(
        offset(cx, window, 12),
        gpui::point(px(-70.), px(-30.)),
        "a two-axis viewport must preserve both components of a precise diagonal"
    );
    wheel_sample(
        cx,
        window,
        point,
        gpui::ScrollDelta::Pixels(gpui::point(px(-20.), px(-40.))),
        gpui::TouchPhase::Moved,
    )
    .await;
    assert_eq!(offset(cx, window, 12), gpui::point(px(-90.), px(-70.)));
    wheel(cx, window, point, 15., 10.).await;
    assert_eq!(offset(cx, window, 12), gpui::point(px(-75.), px(-60.)));
    assert_eq!(offset(cx, window, 11), Default::default());

    reset(cx, window).await;
    wheel_sample(
        cx,
        window,
        point,
        gpui::ScrollDelta::Lines(gpui::point(-2., -1.)),
        gpui::TouchPhase::Moved,
    )
    .await;
    let discrete = offset(cx, window, 12);
    assert!(discrete.x < px(0.) && discrete.y < px(0.));
    assert_eq!(discrete.x, discrete.y * 2.);
    assert_eq!(offset(cx, window, 11), Default::default());

    // A clamped X axis must not discard Y or send the consumed event upstream.
    window
        .update(cx, |view, window, _| {
            let handle = &view.scrolls[&node(12)].handle;
            handle.set_offset(gpui::point(-handle.max_offset().x, px(0.)));
            window.refresh();
        })
        .unwrap();
    frame(cx, window).await;
    let at_x_limit = offset(cx, window, 12);
    wheel(cx, window, point, -70., -30.).await;
    assert_eq!(offset(cx, window, 12), gpui::point(at_x_limit.x, px(-30.)));
    assert_eq!(offset(cx, window, 11), Default::default());

    window
        .update(cx, |view, window, _| {
            let handle = &view.scrolls[&node(12)].handle;
            handle.set_offset(-handle.max_offset());
            window.refresh();
        })
        .unwrap();
    frame(cx, window).await;
    let at_limit = offset(cx, window, 12);
    wheel(cx, window, point, -10., -40.).await;
    assert_eq!(offset(cx, window, 12), at_limit);
    assert!(
        offset(cx, window, 11).y < px(0.),
        "unconsumed boundary event reaches parent"
    );

    reset(cx, window).await;
    let mut hovered = inner_style();
    hovered.push(Style::State(2, vec![Field::OverflowX(2)]));
    apply(cx, window, vec![Op::SetStyle(node(12), hovered)]);
    frame(cx, window).await;
    wheel(cx, window, point, -10., -40.).await;
    assert_eq!(offset(cx, window, 12), gpui::point(px(0.), px(-40.)));
    apply(cx, window, vec![Op::SetStyle(node(12), inner_style())]);
    frame(cx, window).await;
    wheel(cx, window, point, -20., -30.).await;
    assert_eq!(offset(cx, window, 12), gpui::point(px(-20.), px(-70.)));
    window
        .update(cx, |view, _, _| {
            assert!(Rc::ptr_eq(
                &owner.upgrade().unwrap(),
                &view.scrolls[&node(12)]
            ));
        })
        .unwrap();
    assert_eq!(offset(cx, window, 11), Default::default());

    apply(
        cx,
        window,
        vec![
            Op::SetRoot(None),
            Op::Remove(node(14)),
            Op::Remove(node(13)),
            Op::Remove(node(12)),
            Op::Remove(node(11)),
        ],
    );
    assert!(owner.upgrade().is_none());
    frame(cx, window).await;
    window
        .update(cx, |view, _, _| assert!(view.scrolls.is_empty()))
        .unwrap();
    eprintln!(
        "GPUIO_TWO_AXIS_SCROLL_OK: precise/discrete diagonals, gesture continuation/reversal, one/both-axis limits, hovered axis replacement, owner retention/disposal"
    );
}
