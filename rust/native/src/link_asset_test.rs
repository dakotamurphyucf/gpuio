//! Image leases, painted content and inherited state styles inside one action.
use super::{
    content_test::{events, frame, idle},
    *,
};
use gpuio_protocol::{ResourceId, asset::Format, avatar};
use std::time::Duration;

fn upload(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, color: [u8; 3]) -> ResourceId {
    handle
        .update(cx, |view, _, _| {
            let mut bytes = b"P6\n4 4\n255\n".to_vec();
            for _ in 0..16 {
                bytes.extend(color);
            }
            let mut session = view.session.borrow_mut();
            let store = session.assets().unwrap();
            let source = store.begin(Format::Pnm, bytes.len()).unwrap();
            store.append(source, 0, &bytes).unwrap();
            store.finish(source).unwrap();
            source
        })
        .unwrap()
}
fn release(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, source: ResourceId) {
    handle
        .update(cx, |view, _, _| {
            let mut session = view.session.borrow_mut();
            let store = session.assets().unwrap();
            store.release(source).unwrap();
            assert!(
                store.acquire(source).is_err(),
                "retired source cannot issue another lease"
            );
            assert_eq!(store.stats().available, 0);
        })
        .unwrap();
}
fn image(source: ResourceId) -> ImageConfig {
    ImageConfig {
        source: ImageSource::Reference(source),
        fit: ImageFit::Cover,
        label: Some("Link preview image".into()),
    }
}
fn avatar(source: ImageSource) -> avatar::Config {
    avatar::Config {
        source: Some(source),
        fit: ImageFit::Cover,
        label: Some("Link preview avatar".into()),
        fallback: "DM".into(),
    }
}
fn styles(states: bool) -> Vec<Style> {
    let mut styles = vec![Style::Fields(vec![
        Field::Display(1),
        Field::Direction(1),
        Field::Width(Length::Px(300.)),
        Field::Height(Length::Px(190.)),
        Field::Shrink(0.),
        Field::Foreground(Color::Rgba(0x111111ff)),
        Field::Background(Fill::Solid(Color::Rgba(0x182638ff))),
    ])];
    if states {
        for (state, color) in [
            (1, 0x444444ff),
            (2, 0x222222ff),
            (3, 0x333333ff),
            (6, 0x555555ff),
        ] {
            styles.push(Style::State(
                state,
                vec![Field::Foreground(Color::Rgba(color))],
            ));
        }
    }
    styles
}
fn child_styles() -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(64.)),
        Field::Height(Length::Px(64.)),
        Field::Shrink(0.),
        Field::Foreground(Color::Rgba(0xffffffff)),
        Field::FontSize(24.),
    ])]
}
fn inherited(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, expected: u32) {
    handle
        .update(cx, |view, _, _| {
            let probes = view.probes.borrow();
            for slot in [24, 27] {
                assert_eq!(
                    probes[&id(slot)].color,
                    rgba(expected).into(),
                    "root/text style at {slot}"
                );
            }
            assert_eq!(
                probes[&id(26)].color,
                rgba(0xffffffff).into(),
                "explicit avatar foreground overrides inheritance"
            );
        })
        .unwrap();
}
async fn pixels(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, expected: [u8; 4]) {
    for _ in 0..200 {
        frame(cx, handle).await;
        let found = handle
            .update(cx, |view, window, _| {
                let pixels = window.render_to_image().unwrap();
                let scale = window.scale_factor();
                [25, 26].into_iter().all(|slot| {
                    let center = view.probes.borrow()[&id(slot)].bounds.center();
                    pixels
                        .get_pixel(
                            (f32::from(center.x) * scale) as u32,
                            (f32::from(center.y) * scale) as u32,
                        )
                        .0
                        == expected
                })
            })
            .unwrap();
        if found {
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(25))
            .await;
    }
    panic!("both link image interiors must paint {expected:?}");
}
async fn retired(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>) {
    for _ in 0..200 {
        let done = handle
            .update(cx, |view, _, _| {
                assert!(view.images.is_empty());
                view.session.borrow_mut().assets().unwrap().stats()
                    == crate::asset_store::Stats::default()
            })
            .unwrap();
        if done {
            return;
        }
        cx.background_executor()
            .timer(Duration::from_millis(25))
            .await;
    }
    panic!("retired link content must release all encoded sources");
}
pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    handle: WindowHandle<View>,
    transport: &Transport,
) {
    events(cx, handle, transport, false, None);
    let green = upload(cx, handle, [10, 220, 30]);
    apply(
        cx,
        handle,
        vec![
            Op::Create(
                id(24),
                Kind::Link,
                String::new(),
                Some(gpuio_protocol::HandlerId::from_parts(24, 1).unwrap()),
            ),
            Op::SetLink(id(24), config(24, 0)),
            Op::SetStyle(id(24), styles(true)),
            Op::Create(id(25), Kind::Image, String::new(), None),
            Op::SetImage(id(25), image(green)),
            Op::SetStyle(id(25), child_styles()),
            Op::Create(id(26), Kind::Avatar, String::new(), None),
            Op::SetAvatar(id(26), avatar(ImageSource::Reference(green))),
            Op::SetStyle(id(26), child_styles()),
            Op::Create(
                id(27),
                Kind::Text,
                "Open image collection 世界".into(),
                None,
            ),
            Op::Splice(id(24), 0, 0, vec![id(25), id(26), id(27)]),
            Op::Splice(id(0), 0, 0, vec![id(24)]),
        ],
    );
    release(cx, handle, green);
    pixels(cx, handle, [10, 220, 30, 255]).await;
    events(cx, handle, transport, true, None);
    let retained = handle
        .update(cx, |view, _, _| {
            assert_eq!(
                view.images.keys().copied().collect::<Vec<_>>(),
                vec![id(25), id(26)]
            );
            view.buttons[&id(24)].focus.clone()
        })
        .unwrap();
    for child in [25, 26] {
        click(cx, handle, child);
        focused(cx, handle, 24);
        events(cx, handle, transport, false, Some(id(24)));
    }
    let blue = upload(cx, handle, [20, 80, 240]);
    apply(
        cx,
        handle,
        vec![
            Op::SetImage(id(25), image(blue)),
            Op::SetAvatar(id(26), avatar(ImageSource::Reference(blue))),
        ],
    );
    release(cx, handle, blue);
    pixels(cx, handle, [20, 80, 240, 255]).await;
    events(cx, handle, transport, true, None);
    handle
        .update(cx, |view, _, _| {
            assert_eq!(view.buttons[&id(24)].focus, retained)
        })
        .unwrap();

    let outside = gpui::point(px(-20.), px(-20.));
    native_test::move_mouse(cx, handle, outside, false);
    focus(cx, handle, 5);
    draw(cx, handle);
    inherited(cx, handle, 0x111111ff);
    let center = handle
        .update(cx, |view, _, _| {
            view.probes.borrow()[&id(27)].bounds.center()
        })
        .unwrap();
    native_test::move_mouse(cx, handle, center, false);
    draw(cx, handle);
    inherited(cx, handle, 0x222222ff);
    native_test::mouse(cx, handle, center, true);
    draw(cx, handle);
    inherited(cx, handle, 0x333333ff);
    native_test::mouse(cx, handle, center, false);
    draw(cx, handle);
    events(cx, handle, transport, false, Some(id(24)));
    native_test::move_mouse(cx, handle, outside, false);
    draw(cx, handle);
    inherited(cx, handle, 0x444444ff);
    apply(
        cx,
        handle,
        vec![Op::SetLink(
            id(24),
            Config {
                disabled: true,
                ..config(24, 0)
            },
        )],
    );
    draw(cx, handle);
    inherited(cx, handle, 0x555555ff);
    click(cx, handle, 25);
    inherited(cx, handle, 0x555555ff);
    events(cx, handle, transport, true, None);
    apply(
        cx,
        handle,
        vec![
            Op::SetLink(id(24), config(24, 0)),
            Op::SetStyle(id(24), styles(false)),
        ],
    );
    draw(cx, handle);
    inherited(cx, handle, 0x111111ff);
    events(cx, handle, transport, true, None);
    click(cx, handle, 26);
    inherited(cx, handle, 0x111111ff);
    events(cx, handle, transport, false, Some(id(24)));

    // Accepted replacement and removal before a paint: any decode completion
    // has no surviving content owner or callback to revive.
    let red = upload(cx, handle, [240, 30, 20]);
    apply(
        cx,
        handle,
        vec![
            Op::SetImage(id(25), image(red)),
            Op::SetAvatar(id(26), avatar(ImageSource::Reference(red))),
        ],
    );
    release(cx, handle, red);
    apply(
        cx,
        handle,
        vec![
            Op::Splice(id(0), 0, 1, vec![]),
            Op::Remove(id(27)),
            Op::Remove(id(26)),
            Op::Remove(id(25)),
            Op::Remove(id(24)),
        ],
    );
    frame(cx, handle).await;
    events(cx, handle, transport, true, None);
    retired(cx, handle).await;
    idle(cx, handle).await;
    events(cx, handle, transport, false, None);
    eprintln!(
        "GPUIO_LINK_ASSET_OK: shared retired source leases, replacement pixels and retained focus, inherited native state/reset styles, pointer activation, prepaint unmount, resource retirement and idle"
    );
}
