//! Native GPU/AX checks for the retained, passive avatar slot. This is exercised
//! by native_images, not the TestPlatform unit suite; compiling is not acceptance.
use super::*;
use gpuio_protocol::{accessibility, avatar};

const BACKDROP: [u8; 4] = [16, 24, 32, 255];
const FALLBACK: [u8; 4] = [196, 58, 245, 255];
const GREEN: [u8; 4] = [10, 220, 30, 255];
const BLUE: [u8; 4] = [30, 70, 240, 255];
const RED: [u8; 4] = [240, 30, 70, 255];

fn id(offset: i64) -> NodeId {
    // Earlier fixtures allocate slots 0..=12. Tree admission requires sequential
    // new slots even after earlier owners have been removed.
    NodeId::from_parts(13 + offset, 1).unwrap()
}
fn callback() -> HandlerId {
    HandlerId::from_parts(500, 1).unwrap()
}
fn config(source: Option<ResourceId>) -> avatar::Config {
    avatar::Config {
        source: source.map(ImageSource::Reference),
        fit: ImageFit::Cover,
        label: Some("Rich avatar owner".into()),
        fallback: "Unused initials".into(),
    }
}
fn fill(color: [u8; 4]) -> Field {
    Field::Background(Fill::Solid(Color::Rgba(u32::from_be_bytes(color).into())))
}
fn fallback_style(size: f64, radius: f64) -> Vec<Style> {
    vec![Style::Fields(vec![
        Field::Width(Length::Px(size)),
        Field::Height(Length::Px(size)),
        Field::Shrink(0.),
        Field::TopLeftRadius(radius),
        fill(FALLBACK),
    ])]
}
fn upload_format(session: &mut Session, format: Format, bytes: &[u8]) -> ResourceId {
    let store = session.assets().unwrap();
    let source = store.begin(format, bytes.len()).unwrap();
    store.append(source, 0, bytes).unwrap();
    store.finish(source).unwrap();
    source
}
fn animated(session: &mut Session) -> ResourceId {
    let mut bytes = Vec::new();
    {
        let mut encoder = image::codecs::gif::GifEncoder::new(&mut bytes);
        encoder
            .set_repeat(image::codecs::gif::Repeat::Infinite)
            .unwrap();
        for color in [RED, BLUE] {
            encoder
                .encode_frame(image::Frame::from_parts(
                    image::RgbaImage::from_pixel(4, 4, image::Rgba(color)),
                    0,
                    0,
                    image::Delay::from_numer_denom_ms(150, 1),
                ))
                .unwrap();
        }
    }
    upload_format(session, Format::Gif, &bytes)
}
async fn painted(cx: &mut gpui::AsyncApp) {
    pause(cx).await;
    pause(cx).await;
}
async fn settled(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, expected: ImageState) {
    for _ in 0..200 {
        let actual = window
            .update(cx, |view, _, _| {
                view.images
                    .get(&id(1))
                    .and_then(|state| state.emitted.map(|(_, state)| state))
            })
            .unwrap();
        if actual == Some(expected) {
            painted(cx).await;
            return;
        }
        pause(cx).await;
    }
    panic!("rich avatar did not reach {expected:?}");
}
// Logical coordinates relative to the avatar, converted using the actual native
// scale. Negative coordinates deliberately sample outside its clip rectangle.
fn samples(cx: &mut gpui::AsyncApp, window: WindowHandle<View>) -> [[u8; 4]; 5] {
    window
        .update(cx, |view, window, _| {
            let bounds = view.probes.borrow()[&id(1)].bounds;
            assert_eq!(bounds.size, size(px(64.), px(64.)));
            let image = window.render_to_image().expect("rich avatar GPU readback");
            let scale = window.scale_factor();
            [(32., 32.), (4., 4.), (60., 4.), (-4., 32.), (68., 32.)].map(|(x, y)| {
                let x = ((f32::from(bounds.origin.x) + x) * scale).floor();
                let y = ((f32::from(bounds.origin.y) + y) * scale).floor();
                assert!(x >= 0. && y >= 0.);
                assert!(x < image.width() as f32 && y < image.height() as f32);
                image.get_pixel(x as u32, y as u32).0
            })
        })
        .unwrap()
}
#[cfg(target_os = "macos")]
async fn semantics(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, labelled: bool) {
    use crate::host::control_test::accessible_role;
    let _ = accessible_role(cx, window, "Rich avatar owner");
    painted(cx).await;
    assert_eq!(
        accessible_role(cx, window, "Rich avatar owner").as_deref(),
        labelled.then_some("AXImage")
    );
    for label in ["Hidden rich label", "Hidden rich image", "Unused initials"] {
        assert!(
            accessible_role(cx, window, label).is_none(),
            "passive fallback must not add an AX owner: {label}"
        );
    }
}

pub(super) async fn exercise(
    cx: &mut gpui::AsyncApp,
    window: WindowHandle<View>,
    session: &Rc<RefCell<Session>>,
    transport: &Arc<Transport>,
) {
    apply(
        cx,
        window,
        vec![
            Op::Create(id(0), Kind::Container, "".into(), None),
            Op::SetStyle(
                id(0),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(96.)),
                    Field::Height(Length::Px(96.)),
                    fill(BACKDROP),
                ])],
            ),
            Op::Create(id(1), Kind::Avatar, "".into(), None),
            Op::SetAvatar(id(1), config(None)),
            Op::SetStyle(
                id(1),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(64.)),
                    Field::Height(Length::Px(64.)),
                    Field::MarginLeft(Length::Px(16.)),
                    Field::MarginTop(Length::Px(16.)),
                    Field::TopLeftRadius(32.),
                    Field::TopRightRadius(32.),
                    Field::BottomLeftRadius(32.),
                    Field::BottomRightRadius(32.),
                ])],
            ),
            Op::Bind(id(1), Some(callback())),
            Op::Create(id(2), Kind::Container, "".into(), None),
            Op::SetStyle(id(2), fallback_style(96., 0.)),
            Op::SetAccessibility(
                id(2),
                Some(accessibility::Config {
                    role: Some(accessibility::Role::Label),
                    label: Some("Hidden rich label".into()),
                    description: None,
                    live: accessibility::Live::Off,
                    field: None,
                    current: None,
                }),
            ),
            Op::Splice(id(1), 0, 0, vec![id(2)]),
            Op::Splice(id(0), 0, 0, vec![id(1)]),
            Op::SetRoot(Some(id(0))),
        ],
    );
    painted(cx).await;
    assert_eq!(
        samples(cx, window),
        [FALLBACK, FALLBACK, FALLBACK, BACKDROP, BACKDROP],
        "oversized rich child is rectangularly clipped, not rounded by its parent"
    );
    #[cfg(target_os = "macos")]
    semantics(cx, window, true).await;

    crate::host::native_test::move_mouse(cx, window, gpui::point(px(-10.), px(-10.)), false);
    let mut rounded = fallback_style(64., 32.);
    rounded.push(Style::State(
        2,
        vec![Field::TopLeftRadius(0.), Field::TopRightRadius(32.)],
    ));
    apply(cx, window, vec![Op::SetStyle(id(2), rounded)]);
    painted(cx).await;
    assert_eq!(
        samples(cx, window),
        [FALLBACK, BACKDROP, FALLBACK, BACKDROP, BACKDROP],
        "full-bleed fallback owns its individual corner styles"
    );
    // Synthetic GPUI mouse routing verifies native state refinement, without
    // claiming physical pointer delivery or activating the background window.
    crate::host::native_test::move_mouse(cx, window, gpui::point(px(48.), px(48.)), false);
    painted(cx).await;
    assert_eq!(
        samples(cx, window),
        [FALLBACK, FALLBACK, BACKDROP, BACKDROP, BACKDROP],
        "hover replaces the child's corner policy"
    );
    crate::host::native_test::move_mouse(cx, window, gpui::point(px(-10.), px(-10.)), false);
    painted(cx).await;
    assert_eq!(
        samples(cx, window),
        [FALLBACK, BACKDROP, FALLBACK, BACKDROP, BACKDROP],
        "leaving hover restores the child's corners"
    );
    apply(
        cx,
        window,
        vec![Op::SetAvatar(
            id(1),
            avatar::Config {
                label: None,
                ..config(None)
            },
        )],
    );
    painted(cx).await;
    #[cfg(target_os = "macos")]
    semantics(cx, window, false).await;

    // A child image keeps its lease across primary selection, even after the
    // application releases the encoded source handle.
    let decorative = source(&mut session.borrow_mut(), [30, 70, 240]);
    apply(
        cx,
        window,
        vec![
            Op::SetStyle(id(2), fallback_style(64., 0.)),
            Op::Create(id(3), Kind::Image, "".into(), None),
            Op::SetImage(
                id(3),
                ImageConfig {
                    source: ImageSource::Reference(decorative),
                    fit: ImageFit::Cover,
                    label: Some("Hidden rich image".into()),
                },
            ),
            Op::SetStyle(
                id(3),
                vec![Style::Fields(vec![
                    Field::Width(Length::Px(64.)),
                    Field::Height(Length::Px(64.)),
                ])],
            ),
            Op::Splice(id(2), 0, 0, vec![id(3)]),
            Op::SetAvatar(id(1), config(None)),
        ],
    );
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(decorative)
        .unwrap();
    let mut child_painted = false;
    for _ in 0..200 {
        painted(cx).await;
        if samples(cx, window)[0] == BLUE {
            child_painted = true;
            break;
        }
    }
    assert!(
        child_painted,
        "rich fallback image did not decode and paint"
    );
    let child_owner = window
        .update(cx, |view, _, _| Rc::downgrade(&view.images[&id(3)].binding))
        .unwrap();
    #[cfg(target_os = "macos")]
    semantics(cx, window, true).await;

    let raster = source(&mut session.borrow_mut(), [10, 220, 30]);
    let vector = upload_format(
        &mut session.borrow_mut(),
        Format::Svg,
        br##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><rect width="4" height="4" fill="#0adc1e"/></svg>"##,
    );
    let ready = ImageState::Ready(ImageMetadata {
        width_px: 4,
        height_px: 4,
        frames: 1,
    });
    for primary in [raster, vector] {
        apply(
            cx,
            window,
            vec![Op::SetAvatar(id(1), config(Some(primary)))],
        );
        session
            .borrow_mut()
            .assets()
            .unwrap()
            .release(primary)
            .unwrap();
        settled(cx, window, ready).await;
        assert_eq!(
            samples(cx, window),
            [GREEN, BACKDROP, BACKDROP, BACKDROP, BACKDROP]
        );
        assert!(
            child_owner.upgrade().is_some(),
            "hidden fallback lost its image lease"
        );
        #[cfg(target_os = "macos")]
        semantics(cx, window, true).await;
        apply(cx, window, vec![Op::SetAvatar(id(1), config(None))]);
        painted(cx).await;
        assert_eq!(
            samples(cx, window)[0],
            BLUE,
            "fallback recovers its retained image"
        );
    }

    let broken = upload(&mut session.borrow_mut(), b"invalid PNM");
    apply(cx, window, vec![Op::SetAvatar(id(1), config(Some(broken)))]);
    session
        .borrow_mut()
        .assets()
        .unwrap()
        .release(broken)
        .unwrap();
    settled(cx, window, ImageState::Failed(ImageError::InvalidData)).await;
    assert_eq!(
        samples(cx, window)[0],
        BLUE,
        "decode failure selects custom content"
    );

    let gif = animated(&mut session.borrow_mut());
    apply(cx, window, vec![Op::SetAvatar(id(1), config(Some(gif)))]);
    session.borrow_mut().assets().unwrap().release(gif).unwrap();
    settled(
        cx,
        window,
        ImageState::Ready(ImageMetadata {
            width_px: 4,
            height_px: 4,
            frames: 2,
        }),
    )
    .await;
    let mut seen = [false; 2];
    for _ in 0..200 {
        let center = samples(cx, window)[0];
        seen[0] |= center == RED;
        seen[1] |= center == BLUE;
        if seen == [true, true] {
            break;
        }
        pause(cx).await;
    }
    assert_eq!(
        seen,
        [true, true],
        "animated primary must paint both decoded frames"
    );
    let primary_owner = window
        .update(cx, |view, _, _| Rc::downgrade(&view.images[&id(1)].binding))
        .unwrap();
    let statuses: Vec<_> = transport
        .mailbox
        .lock()
        .unwrap()
        .drain(128)
        .into_iter()
        .filter_map(|event| match event {
            Event::ImageState(_, node, handler, _, status) if node == id(1) => {
                assert_eq!(handler, callback());
                (!matches!(status, ImageState::Loading)).then_some(status)
            }
            Event::ImageState(_, node, ..) if node == id(3) => {
                panic!("passive fallback image emitted an application observation")
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        statuses,
        [
            ready,
            ready,
            ImageState::Failed(ImageError::InvalidData),
            ImageState::Ready(ImageMetadata {
                width_px: 4,
                height_px: 4,
                frames: 2
            }),
        ],
        "native root observations must preserve source transition order"
    );
    apply(
        cx,
        window,
        vec![
            Op::SetRoot(None),
            Op::Remove(id(0)),
            Op::Remove(id(1)),
            Op::Remove(id(2)),
            Op::Remove(id(3)),
        ],
    );
    // Allow already scheduled frame callbacks to drain before checking quiescence.
    for _ in 0..16 {
        pause(cx).await;
    }
    assert!(primary_owner.upgrade().is_none());
    assert!(child_owner.upgrade().is_none());
    assert_eq!(
        session.borrow_mut().assets().unwrap().stats(),
        crate::asset_store::Stats::default(),
        "all encoded source registrations and leases must be retired"
    );
    let renders = window
        .update(cx, |view, _, _| {
            assert!(!view.images.contains_key(&id(1)));
            assert!(!view.images.contains_key(&id(3)));
            assert!(!view.avatar_fallbacks.contains_key(&id(1)));
            assert_eq!(view.session.borrow().retained_bytes(), 0);
            view.render_count
        })
        .unwrap();
    for _ in 0..12 {
        pause(cx).await;
    }
    assert_eq!(
        window.update(cx, |view, _, _| view.render_count).unwrap(),
        renders,
        "removed animated avatar must not keep requesting frames"
    );
    assert!(
        !transport
            .mailbox
            .lock()
            .unwrap()
            .drain(128)
            .iter()
            .any(|event| matches!(event, Event::ImageState(_, node, ..) if *node == id(1))),
        "removed avatar must not deliver stale image observations"
    );
    #[cfg(target_os = "macos")]
    eprintln!(
        "GPUIO_RICH_AVATAR_AX_OK: native owner label, decorative root, hidden rich descendants"
    );
    eprintln!(
        "GPUIO_RICH_AVATAR_GPU_OK: clipping, child corners, primary raster/SVG/GIF, retained fallback, decode failure, disposal and idle"
    );
}
