use binprot::BinProtWrite;
use gpuio_protocol::{
    HandlerId, NodeId, ResourceId, WindowId,
    document_profile::{Config, Error, Event, Instance, Signal, Stage},
    extension::{MAX_MESSAGE, MAX_PROPERTIES, Payload, Schema},
    v1::{Event as WireEvent, Message, Op, Transaction},
};
fn bytes(value: &impl BinProtWrite) -> Vec<u8> {
    let mut bytes = vec![];
    value.binprot_write(&mut bytes).unwrap();
    bytes
}
fn hex(value: &impl BinProtWrite) -> String {
    bytes(value).iter().map(|b| format!("{b:02x}")).collect()
}
fn config() -> Config {
    Config {
        epoch: 1,
        instance: Some(Instance {
            schema: Schema {
                name: "test.reader".into(),
                version: 1,
                fingerprint: "a".repeat(64),
            },
            generation: 2,
            properties: Payload(vec![0, 255, 128]),
        }),
    }
}
#[test]
fn profile_config_has_paired_bytes_and_strict_bounded_decode() {
    let node = NodeId::from_parts(0, 1).unwrap();
    let make = |config| {
        Message::Apply(Transaction {
            window: WindowId::from_parts(0, 1).unwrap(),
            base: 0,
            revision: 1,
            operations: vec![Op::SetDocumentProfile(node, config)],
        })
    };
    assert_eq!(
        hex(&Op::SetDocumentProfile(node, config())),
        format!(
            "79000101010b746573742e7265616465720140{}020300ff80",
            "61".repeat(64)
        )
    );
    let message = make(config());
    let packet = bytes(&message);
    assert_eq!(gpuio_protocol::decode(&packet), Ok(message));
    for end in 0..packet.len() {
        assert!(gpuio_protocol::decode(&packet[..end]).is_err());
    }
    let mut trailing = packet.clone();
    trailing.push(0);
    assert!(gpuio_protocol::decode(&trailing).is_err());
    // The option discriminant is strict (2 must not mean Some).
    let mut option = packet.clone();
    option[packet.len() - bytes(&Op::SetDocumentProfile(node, config())).len() + 4] = 2;
    assert!(gpuio_protocol::decode(&option).is_err());
    let mut invalid = vec![];
    let mut c = config();
    c.epoch = 0;
    invalid.push(c);
    for change in 0..6 {
        let mut c = config();
        let instance = c.instance.as_mut().unwrap();
        match change {
            0 => instance.generation = 0,
            1 => instance.schema.name = "invalid".into(),
            2 => instance.schema.name = "a".repeat(129),
            3 => instance.schema.version = 65536,
            4 => instance.schema.fingerprint = "A".repeat(64),
            _ => instance.properties = Payload(vec![0; MAX_PROPERTIES + 1]),
        }
        invalid.push(c);
    }
    for c in invalid {
        assert!(!c.is_valid());
        assert!(gpuio_protocol::decode(&bytes(&make(c))).is_err());
    }
    let mut boundary = config();
    boundary.instance.as_mut().unwrap().properties = Payload(vec![255; MAX_PROPERTIES]);
    assert!(boundary.is_valid());
    assert!(boundary.retained_bytes() >= bytes(&boundary).len());
    assert_eq!(
        gpuio_protocol::decode(&bytes(&make(boundary.clone()))),
        Ok(make(boundary))
    );
    let clear = Config {
        epoch: 2,
        instance: None,
    };
    assert_eq!(
        hex(&Op::SetDocumentProfile(node, clear.clone())),
        "7900010200"
    );
    assert_eq!(
        gpuio_protocol::decode(&bytes(&make(clear.clone()))),
        Ok(make(clear))
    );
}
#[test]
fn profile_event_tags_and_payload_bounds_match_ocaml() {
    let event = Event {
        config_epoch: 1,
        instance_generation: 2,
        source_revision: 7,
        source_generation: 3,
        signal: Signal::Data(Payload(vec![0, 255, 128])),
    };
    let envelope = |event| {
        WireEvent::DocumentProfileEvent(
            WindowId::from_parts(0, 1).unwrap(),
            NodeId::from_parts(0, 1).unwrap(),
            HandlerId::from_parts(0, 1).unwrap(),
            4,
            ResourceId::from_parts(0, 1).unwrap(),
            event,
        )
    };
    assert_eq!(
        hex(&vec![envelope(event.clone())]),
        "014e00010001000104000101020703000300ff80"
    );
    let mut failure = event.clone();
    failure.signal = Signal::Failed(Stage::Input, Error::InvalidEvent);
    assert_eq!(
        hex(&vec![envelope(failure)]),
        "014e00010001000104000101020703010411"
    );
    assert!(event.is_valid());
    for field in 0..4 {
        let mut bad = event.clone();
        match field {
            0 => bad.config_epoch = 0,
            1 => bad.instance_generation = 0,
            2 => bad.source_revision = 0,
            _ => bad.source_generation = 0,
        }
        assert!(!bad.is_valid());
    }
    let mut boundary = event;
    boundary.signal = Signal::Data(Payload(vec![255; MAX_MESSAGE]));
    assert!(boundary.is_valid());
    assert!(boundary.payload_bytes() >= bytes(&envelope(boundary.clone())).len());
    boundary.signal = Signal::Data(Payload(vec![0; MAX_MESSAGE + 1]));
    assert!(!boundary.is_valid());
}
