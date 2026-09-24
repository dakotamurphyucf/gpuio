use binprot::BinProtWrite;
use gpuio_protocol::{HandlerId, NodeId, ResourceId, WindowId, decode, v1::*};
fn bytes(value: impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn unhex(text: &str) -> Vec<u8> {
    text.trim()
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
#[test]
fn image_wire_matches_ocaml_and_bounds_decode() {
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let asset = ResourceId::from_parts(2, 3).unwrap();
    let errors = [
        ImageError::WrongApplication,
        ImageError::Released,
        ImageError::InvalidData,
        ImageError::Unsupported,
        ImageError::ResourceLimit,
        ImageError::NativeFailure,
    ];
    let configs = [
        ImageFit::Fill,
        ImageFit::Contain,
        ImageFit::Cover,
        ImageFit::ScaleDown,
        ImageFit::None,
    ]
    .into_iter()
    .map(|fit| ImageConfig {
        source: ImageSource::Reference(asset),
        fit,
        label: Some("Preview".into()),
    })
    .chain(errors.into_iter().map(|error| ImageConfig {
        source: ImageSource::Unavailable(error),
        fit: ImageFit::Contain,
        label: None,
    }));
    let mut operations = vec![Op::Create(node, Kind::Image, "".into(), Some(handler))];
    operations.extend(configs.map(|config| Op::SetImage(node, config)));
    operations.push(Op::SetRoot(Some(node)));
    let message = Message::Apply(Transaction {
        window,
        base: 0,
        revision: 1,
        operations,
    });
    let fixture = unhex(include_str!("../../../test/fixtures/images-v1-request.hex"));
    assert_eq!(bytes(message.clone()), fixture);
    assert_eq!(decode(&fixture).unwrap(), message);
    let Message::Apply(mut icon) = message else {
        unreachable!()
    };
    icon.operations[0] = Op::Create(node, Kind::Icon, "".into(), Some(handler));
    let icon = Message::Apply(icon);
    let icon_fixture = unhex(include_str!("../../../test/fixtures/icons-v1-request.hex"));
    assert_eq!(bytes(icon.clone()), icon_fixture);
    assert_eq!(decode(&icon_fixture).unwrap(), icon);
    for length in 0..icon_fixture.len() {
        assert!(decode(&icon_fixture[..length]).is_err());
    }
    for length in 0..fixture.len() {
        assert!(decode(&fixture[..length]).is_err());
    }
    let mut trailing = fixture;
    trailing.push(0);
    assert!(decode(&trailing).is_err());
    let states = [
        ImageState::Loading,
        ImageState::Ready(ImageMetadata {
            width_px: 4,
            height_px: 4,
            frames: 1,
        }),
    ]
    .into_iter()
    .chain(errors.into_iter().map(ImageState::Failed));
    let events: Vec<_> = states
        .map(|state| Event::ImageState(window, node, handler, 1, state))
        .collect();
    assert_eq!(
        bytes(events),
        unhex(include_str!("../../../test/fixtures/images-v1-events.hex"))
    );
    for label in ["", " ", "nul\0label"] {
        assert!(
            !ImageConfig {
                source: ImageSource::Reference(asset),
                fit: ImageFit::Contain,
                label: Some(label.into())
            }
            .is_valid()
        );
    }
    for (width_px, height_px, frames) in [(0, 1, 1), (1, 1, 121), (4096, 4096, 2), (i64::MAX, 1, 1)]
    {
        assert!(
            !ImageMetadata {
                width_px,
                height_px,
                frames
            }
            .is_valid()
        );
    }
}
