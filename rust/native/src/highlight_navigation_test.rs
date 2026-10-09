//! Selected-route matching remains independent of outgoing animation paint.
use super::*;
use gpuio_protocol::navigation_stack::{Config as Navigation, Motion};

fn navigation(selected: Option<i64>) -> Navigation {
    Navigation {
        selected,
        retain: true,
        motion: Motion::Slide,
        duration_ms: 2000,
    }
}

fn page(color: i64) -> Vec<Style> {
    let mut style = frame_style(Length::Percent(100.), 120.);
    style.push(Style::Fields(vec![Field::Background(Fill::Solid(
        Color::Rgba(color),
    ))]));
    style
}

/// Both pages must actually paint. Every red pixel must be over the selected
/// page's horizontal extent, not the still-visible outgoing page.
fn transition_pixels(cx: &mut AsyncApp, handle: WindowHandle<View>, selected: [u8; 4]) {
    handle
        .update(cx, |_, window, _| {
            let image = window.render_to_image().unwrap();
            let scale = window.scale_factor();
            let width = (350. * scale) as u32;
            let baseline = (100. * scale) as u32;
            let mut first = 0;
            let mut second = 0;
            let mut washes = 0;
            for x in 0..width {
                let background = image.get_pixel(x, baseline).0;
                first += usize::from(background == [0, 128, 0, 255]);
                second += usize::from(background == [0, 0, 255, 255]);
                for y in 0..(30. * scale) as u32 {
                    if image.get_pixel(x, y).0 == [255, 0, 0, 255] {
                        washes += 1;
                        assert_eq!(background, selected, "outgoing page retained a wash");
                    }
                }
            }
            assert!(first > 10 && second > 10, "both animated pages must paint");
            assert!(washes > 20, "incoming page must paint its matches");
        })
        .unwrap();
}

pub(super) async fn exercise(cx: &mut AsyncApp, handle: WindowHandle<View>, transport: &Transport) {
    cx.update(|cx| cx.set_reduce_motion(false));
    let mut ops = vec![Op::Splice(id(1), 0, 1, vec![])];
    ops.extend((16..=21).rev().map(|n| Op::Remove(id(n))));
    ops.extend([
        Op::Create(id(22), Kind::NavigationStack, "Routes".into(), None),
        Op::SetNavigationStack(id(22), navigation(Some(0))),
        Op::SetStyle(id(22), frame_style(Length::Px(350.), 120.)),
        Op::Create(id(23), Kind::Panel, "First".into(), None),
        Op::SetStyle(id(23), page(0x008000ff)),
        Op::Create(id(24), Kind::Text, "aaa".into(), None),
        Op::SetStyle(id(24), row(0., false, false)),
        Op::Create(id(25), Kind::Panel, "Second".into(), None),
        Op::SetStyle(id(25), page(0x0000ffff)),
        Op::Create(id(26), Kind::Text, "aaa aaa".into(), None),
        Op::SetStyle(id(26), row(0., false, false)),
        Op::Splice(id(23), 0, 0, vec![id(24)]),
        Op::Splice(id(25), 0, 0, vec![id(26)]),
        Op::Splice(id(22), 0, 0, vec![id(23), id(25)]),
        Op::Splice(id(1), 0, 0, vec![id(22)]),
    ]);
    apply(cx, handle, ops);
    count(cx, handle, transport, 1).await;
    assert!(red_pixels(cx, handle) > 20);
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(id(22), navigation(Some(1)))],
    );
    let selected = count(cx, handle, transport, 2).await;
    cx.background_executor()
        .timer(Duration::from_millis(500))
        .await;
    draw(cx, handle);
    transition_pixels(cx, handle, [0, 0, 255, 255]);
    let retained = result(cx, handle);
    apply(cx, handle, vec![Op::SetText(id(24), "aaa aaa aaa".into())]);
    draw(cx, handle);
    pause(cx).await;
    assert!(Arc::ptr_eq(&retained, &result(cx, handle)));
    assert!(
        observations(transport).is_empty(),
        "outgoing edits do not count"
    );

    // Reverse before completion. The newly selected retained page exposes its
    // updated text, and the other page loses its wash immediately.
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(id(22), navigation(Some(0)))],
    );
    let returned = count(cx, handle, transport, 3).await;
    assert!(returned.epoch > selected.epoch);
    cx.background_executor()
        .timer(Duration::from_millis(150))
        .await;
    draw(cx, handle);
    transition_pixels(cx, handle, [0, 128, 0, 255]);

    cx.update(|cx| cx.set_reduce_motion(true));
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(id(22), navigation(Some(1)))],
    );
    count(cx, handle, transport, 2).await;
    assert_eq!(pixel(cx, handle, 4., 100.), [0, 0, 255, 255]);
    assert_eq!(pixel(cx, handle, 346., 100.), [0, 0, 255, 255]);
    cx.update(|cx| cx.set_reduce_motion(false));
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(id(22), navigation(Some(0)))],
    );
    count(cx, handle, transport, 3).await;
    apply(
        cx,
        handle,
        vec![Op::SetNavigationStack(id(22), navigation(Some(1)))],
    );
    count(cx, handle, transport, 2).await;
    let owner = handle
        .update(cx, |view, _, _| Rc::downgrade(&view.highlights[&id(1)]))
        .unwrap();
    let presenter = handle
        .update(cx, |view, _, _| Rc::downgrade(&view.navigation[&id(22)]))
        .unwrap();
    let mut ops = vec![Op::SetRoot(None)];
    ops.extend((22..=26).rev().map(|n| Op::Remove(id(n))));
    ops.extend([Op::Remove(id(1)), Op::Remove(id(0))]);
    apply(cx, handle, ops);
    for _ in 0..4 {
        draw(cx, handle);
        pause(cx).await;
    }
    assert!(
        owner.upgrade().is_none(),
        "scope released during transition"
    );
    assert!(
        presenter.upgrade().is_none(),
        "presenter released during transition"
    );
    assert!(
        observations(transport).is_empty(),
        "no observations after unmount"
    );
    assert_eq!(red_pixels(cx, handle), 0);
    eprintln!(
        "GPUIO_NATIVE_HIGHLIGHT_NAVIGATION_OK: selected-route counts, outgoing wash removal, interruption, reduced motion and disposal"
    );
}
