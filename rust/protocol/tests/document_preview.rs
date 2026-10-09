use binprot::BinProtWrite;
use gpuio_protocol::{NodeId, WindowId, document_preview::Config, v1::*};
fn bytes<T: BinProtWrite>(value: &T) -> Vec<u8> {
    let mut out = vec![];
    value.binprot_write(&mut out).unwrap();
    out
}
#[test]
fn preview_configuration_has_paired_bytes_and_bounded_admission() {
    let node = NodeId::from_parts(0, 1).unwrap();
    let config = Config {
        epoch: 1,
        max_lines: Some(2),
        observe: true,
    };
    assert_eq!(
        bytes(&Op::SetDocumentPreview(node, config.clone())),
        [117, 0, 1, 1, 1, 2, 1]
    );
    let message = |config| {
        Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![Op::SetDocumentPreview(node, config)],
        })
    };
    let value = message(config.clone());
    let encoded = bytes(&value);
    assert_eq!(gpuio_protocol::decode(&encoded), Ok(value));
    for length in 0..encoded.len() {
        assert!(gpuio_protocol::decode(&encoded[..length]).is_err());
    }
    for config in [
        Config {
            epoch: 0,
            ..config.clone()
        },
        Config {
            max_lines: Some(0),
            ..config.clone()
        },
        Config {
            max_lines: Some(4097),
            ..config.clone()
        },
    ] {
        assert!(gpuio_protocol::decode(&bytes(&message(config))).is_err());
    }
    let mut invalid = encoded;
    *invalid.last_mut().unwrap() = 2;
    assert!(gpuio_protocol::decode(&invalid).is_err());
    for limit in [None, Some(1), Some(4096)] {
        let value = message(Config {
            max_lines: limit,
            ..config.clone()
        });
        assert_eq!(gpuio_protocol::decode(&bytes(&value)), Ok(value));
    }
}
#[test]
fn preview_event_has_matching_ocaml_bytes() {
    use gpuio_protocol::{
        HandlerId, ResourceId,
        document_preview::{Event as PreviewEvent, State},
    };
    let event = Event::DocumentPreviewObserved(
        WindowId::from_parts(0, 1).unwrap(),
        NodeId::from_parts(0, 1).unwrap(),
        HandlerId::from_parts(0, 1).unwrap(),
        1,
        ResourceId::from_parts(0, 1).unwrap(),
        PreviewEvent {
            config_epoch: 1,
            source_revision: 1,
            source_generation: 1,
            state: State::Rich(true),
        },
    );
    assert_eq!(
        bytes(&event),
        [76, 0, 1, 0, 1, 0, 1, 1, 0, 1, 1, 1, 1, 3, 1]
    );
}
