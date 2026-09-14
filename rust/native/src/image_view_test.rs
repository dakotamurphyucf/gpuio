//! Real background window rendering through the production declarative View.
use super::*;
#[path = "image_svg_view_test.rs"]
mod svg;
use crate::{session::Session, transport::Transport};
use gpui::{AppContext, Bounds, WindowBounds, WindowHandle, WindowOptions, px, size};
use gpuio_protocol::{ResourceId, WindowId, asset::Format};
use std::{
    cell::RefCell,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    rc::Rc,
    sync::Arc,
    time::Duration,
};
fn node() -> NodeId {
    NodeId::from_parts(0, 1).unwrap()
}
fn window_id() -> WindowId {
    WindowId::from_parts(0, 1).unwrap()
}
fn handler(generation: i64) -> HandlerId {
    HandlerId::from_parts(0, generation).unwrap()
}
fn upload(session: &mut Session, bytes: &[u8]) -> ResourceId {
    let store = session.assets().unwrap();
    let id = store.begin(Format::Pnm, bytes.len()).unwrap();
    store.append(id, 0, bytes).unwrap();
    store.finish(id).unwrap();
    id
}
fn source(session: &mut Session, color: [u8; 3]) -> ResourceId {
    let mut bytes = b"P6\n4 4\n255\n".to_vec();
    for _ in 0..16 {
        bytes.extend(color);
    }
    upload(session, &bytes)
}
fn config(source: ImageSource) -> ImageConfig {
    ImageConfig {
        source,
        fit: ImageFit::Contain,
        label: Some("Preview".into()),
    }
}
fn apply(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, operations: Vec<Op>) {
    window
        .update(cx, |view, window, cx| {
            let base = view.session.borrow().tree(view.id).unwrap().revision();
            let applied = view
                .session
                .borrow_mut()
                .apply(&Transaction {
                    window: view.id,
                    base,
                    revision: base + 1,
                    operations,
                })
                .unwrap();
            view.update_editors(&applied.dirty, window, cx);
            cx.notify();
        })
        .unwrap();
}
async fn pause(cx: &mut gpui::AsyncApp) {
    cx.background_executor()
        .timer(Duration::from_millis(25))
        .await;
}
async fn observed(cx: &mut gpui::AsyncApp, window: WindowHandle<View>, expected: ImageState) {
    for _ in 0..200 {
        let state = window
            .update(cx, |view, _, _| {
                view.images
                    .get(&node())
                    .and_then(|state| state.emitted.map(|(_, state)| state))
            })
            .unwrap();
        if state == Some(expected) {
            pause(cx).await;
            pause(cx).await;
            return;
        }
        pause(cx).await;
    }
    panic!("image view did not reach {expected:?}");
}
fn pixels(cx: &mut gpui::AsyncApp, handle: WindowHandle<View>, expected: [u8; 4]) {
    handle
        .update(cx, |_, window, _| {
            let image = window.render_to_image().expect("GPU readback");
            assert_eq!(
                image.get_pixel(image.width() / 2, image.height() / 2).0,
                expected
            );
        })
        .unwrap();
}
pub(crate) fn run() {
    let failure = Rc::new(RefCell::new(None));
    let task_failure = failure.clone();
    let mut fds = [0; 2];
    assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
    let _reader = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let writer = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    let transport = Arc::new(Transport::new(writer.as_raw_fd()).unwrap());
    gpui_platform::application().run(move |cx| {
        cx.set_quit_mode(gpui::QuitMode::Explicit);
        image_host::init(cx);
        let session = Rc::new(RefCell::new(Session::default()));
        session.borrow_mut().hello(VERSION, CAPABILITIES).unwrap();
        session.borrow_mut().open(1, window_id(), "Image view test", 96., 96.).unwrap();
        let red = source(&mut session.borrow_mut(), [250, 10, 20]);
        let window = cx.open_window(WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, size(px(96.),px(96.)),cx))),
            focus: false, ..Default::default()
        }, |_, cx| cx.new(|_| View::new(window_id(), session.clone(), transport.clone()))).unwrap();
        cx.spawn(async move |cx| {
            let result = crate::host::native_test::protect(async {
                apply(cx, window, vec![
                    Op::Create(node(), Kind::Image, "".into(), Some(handler(1))),
                    Op::SetImage(node(), config(ImageSource::Reference(red))),
                    Op::SetStyle(node(), vec![Style::Fields(vec![Field::Width(Length::Px(96.)), Field::Height(Length::Px(96.))])]),
                    Op::SetRoot(Some(node())),
                ]);
                // Accepted mounts acquire before any later release, even when
                // the first frame/decode has not happened yet.
                session.borrow_mut().assets().unwrap().release(red).unwrap();
                let ready = ImageState::Ready(ImageMetadata { width_px: 4, height_px: 4, frames: 1 });
                observed(cx, window, ready).await;
                pixels(cx, window, [250,10,20,255]);
                #[cfg(target_os = "macos")]
                {
                    let _ = crate::host::control_test::accessible_role(cx, window, "Preview");
                    pause(cx).await;
                    assert_eq!(crate::host::control_test::accessible_role(cx, window, "Preview").as_deref(), Some("AXImage"));
                }

                let events = transport.mailbox.lock().unwrap().drain(128);
                assert!(events.iter().any(|event| matches!(event, Event::ImageState(_, _, _, _, state) if *state == ready)));
                let mut restyled = config(ImageSource::Reference(red));
                restyled.fit = ImageFit::Cover;
                restyled.label = Some("Restyled preview".into());
                apply(cx, window, vec![Op::SetImage(node(), restyled)]);
                pause(cx).await; pause(cx).await;
                pixels(cx, window, [250,10,20,255]);
                assert!(!transport.mailbox.lock().unwrap().drain(128).iter().any(|e| matches!(e, Event::ImageState(..))), "restyling must not reacquire or duplicate state");

                #[cfg(target_os = "macos")]
                assert_eq!(crate::host::control_test::accessible_role(cx, window, "Restyled preview").as_deref(), Some("AXImage"));
                let mut decorative = config(ImageSource::Reference(red));
                decorative.label = None;
                apply(cx, window, vec![Op::SetImage(node(), decorative)]);
                pause(cx).await; pause(cx).await;
                #[cfg(target_os = "macos")]
                assert!(crate::host::control_test::accessible_role(cx, window, "Restyled preview").is_none());
                let blue = source(&mut session.borrow_mut(), [10,20,250]);
                apply(cx, window, vec![Op::Bind(node(), Some(handler(2))), Op::SetImage(node(), config(ImageSource::Reference(blue)))]);
                session.borrow_mut().assets().unwrap().release(blue).unwrap();
                observed(cx, window, ready).await;
                pixels(cx, window, [10,20,250,255]);
                let invalid = upload(&mut session.borrow_mut(), b"malformed image");
                apply(cx, window, vec![Op::SetImage(node(), config(ImageSource::Reference(invalid)))]);
                observed(cx, window, ImageState::Failed(ImageError::InvalidData)).await;
                session.borrow_mut().assets().unwrap().release(invalid).unwrap();
                apply(cx, window, vec![Op::SetImage(node(), config(ImageSource::Reference(red)))]);
                observed(cx, window, ImageState::Failed(ImageError::Released)).await;
                apply(cx, window, vec![Op::SetImage(node(), config(ImageSource::Unavailable(ImageError::WrongApplication)))]);
                observed(cx, window, ImageState::Failed(ImageError::WrongApplication)).await;
                apply(cx, window, vec![Op::SetRoot(None), Op::Remove(node())]);
                window.update(cx, |view, _, _| assert!(view.images.is_empty())).unwrap();
                assert_eq!(session.borrow_mut().assets().unwrap().stats().retired, 0);
                svg::exercise(cx, window, &session).await;
                image_host::shutdown(cx).await;
                window.update(cx, |_, window, _| window.remove_window()).unwrap();
                eprintln!("GPUIO_NATIVE_IMAGE_VIEWS_OK: actual pixels, accepted mount retirement, restyle, replacement, local errors, state events and disposal");
            }).await;
            *task_failure.borrow_mut() = result.err();
            cx.update(crate::host::stop_application);
        }).detach();
    });
    if let Some(error) = failure.borrow_mut().take() {
        std::panic::resume_unwind(error);
    }
}
