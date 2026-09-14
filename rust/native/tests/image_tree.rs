use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, ResourceId, WindowId, v1::*};
#[test]
fn image_tree_is_atomic_and_events_reject_stale_sources_handlers_and_metadata() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let source = ImageSource::Reference(ResourceId::from_parts(0, 1).unwrap());
    let config = ImageConfig {
        source,
        fit: ImageFit::Contain,
        label: Some("Image".into()),
    };
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Test", 96., 96.).unwrap();
    session
        .apply(&Transaction {
            window,
            base: 0,
            revision: 1,
            operations: vec![
                Op::Create(node, Kind::Image, "".into(), Some(handler)),
                Op::SetImage(node, config.clone()),
                Op::SetRoot(Some(node)),
            ],
        })
        .unwrap();
    assert!(
        session
            .image_state(window, node, handler, 1, source, ImageState::Loading)
            .is_some()
    );
    assert!(
        session
            .image_state(window, node, handler, 2, source, ImageState::Loading)
            .is_none()
    );
    assert!(
        session
            .image_state(
                window,
                node,
                HandlerId::from_parts(0, 2).unwrap(),
                1,
                source,
                ImageState::Loading
            )
            .is_none()
    );
    assert!(
        session
            .image_state(
                window,
                node,
                handler,
                1,
                ImageSource::Unavailable(ImageError::Released),
                ImageState::Loading
            )
            .is_none()
    );
    assert!(
        session
            .image_state(
                window,
                node,
                handler,
                1,
                source,
                ImageState::Ready(ImageMetadata {
                    width_px: 1,
                    height_px: 1,
                    frames: 121
                })
            )
            .is_none()
    );
    assert!(
        session.press(window, node, handler, 1).is_none(),
        "image state observers are not activation handlers"
    );
    let before = session.tree(window).unwrap().retained_bytes();
    assert_eq!(
        session
            .apply(&Transaction {
                window,
                base: 1,
                revision: 2,
                operations: vec![Op::SetText(node, "invalid leaf payload".into())]
            })
            .err(),
        Some(ErrorCode::InvalidTree)
    );
    assert_eq!(session.tree(window).unwrap().revision(), 1);
    assert_eq!(session.tree(window).unwrap().retained_bytes(), before);
    let replacement = ImageConfig {
        source: ImageSource::Unavailable(ImageError::Released),
        ..config
    };
    session
        .apply(&Transaction {
            window,
            base: 1,
            revision: 2,
            operations: vec![Op::SetImage(node, replacement)],
        })
        .unwrap();
    assert!(
        session
            .image_state(window, node, handler, 1, source, ImageState::Loading)
            .is_none()
    );
    session
        .apply(&Transaction {
            window,
            base: 2,
            revision: 3,
            operations: vec![Op::SetRoot(None), Op::Remove(node)],
        })
        .unwrap();
    assert!(
        session
            .image_state(
                window,
                node,
                handler,
                2,
                ImageSource::Unavailable(ImageError::Released),
                ImageState::Failed(ImageError::Released)
            )
            .is_none()
    );
}
