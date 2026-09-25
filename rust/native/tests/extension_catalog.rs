use gpuio_extension_sdk::{self as sdk, Factory as _};
use gpuio_native::session::Session;
use gpuio_protocol::{HandlerId, NodeId, WindowId, extension::*, v1::*};
use std::sync::Arc;

struct Factory;
impl sdk::Factory for Factory {
    fn descriptor(&self) -> sdk::Descriptor {
        sdk::Descriptor {
            name: "test.catalog",
            version: 1,
            fingerprint: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            sdk_version: sdk::SDK_VERSION,
            gpui_revision: sdk::GPUI_REVISION,
            max_properties: 1,
            max_command: 1,
            max_event: 1,
        }
    }
    fn validate_properties(&self, bytes: &[u8]) -> Result<(), sdk::Error> {
        assert_ne!(bytes, &[99], "contained package validation panic");
        if bytes == [7] {
            Ok(())
        } else {
            Err(sdk::Error::InvalidProperties)
        }
    }
    fn validate_command(&self, bytes: &[u8]) -> Result<(), sdk::Error> {
        if bytes == [8] {
            Ok(())
        } else {
            Err(sdk::Error::InvalidCommand)
        }
    }
    fn mount(
        &self,
        _: &[u8],
        _: &mut sdk::Context<'_>,
    ) -> Result<Box<dyn sdk::Component>, sdk::Error> {
        unreachable!("pure admission never mounts a component")
    }
}

#[test]
fn linked_schema_and_package_validation_precede_atomic_tree_publication() {
    gpuio_native::extensions::install([Arc::new(Factory) as Arc<dyn sdk::Factory>]).unwrap();
    let schema = Schema {
        name: "test.catalog".into(),
        version: 1,
        fingerprint: Factory.descriptor().fingerprint.into(),
    };
    assert_eq!(gpuio_native::extensions::catalog(), vec![schema.clone()]);
    assert_eq!(
        gpuio_native::extensions::install([]),
        Err(sdk::Error::Closed)
    );
    let mut session = Session::default();
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Catalog test", 400., 300.).unwrap();
    let config = Config {
        schema,
        generation: 1,
        label: "Count".into(),
        disabled: false,
        properties: Payload(vec![7]),
        command: None,
    };
    let transaction = |config| Transaction {
        window,
        base: 0,
        revision: 1,
        operations: vec![
            Op::Create(
                node,
                Kind::Extension,
                "".into(),
                Some(HandlerId::from_parts(0, 1).unwrap()),
            ),
            Op::SetExtension(node, config),
            Op::SetRoot(Some(node)),
        ],
    };
    let mut variants = Vec::new();
    for bytes in [vec![6], vec![99], vec![7, 7]] {
        let mut invalid = config.clone();
        invalid.properties = Payload(bytes);
        variants.push(invalid);
    }
    let mut unknown = config.clone();
    unknown.schema.name = "test.missing".into();
    variants.push(unknown);
    let mut incompatible = config.clone();
    incompatible.schema.version = 2;
    variants.push(incompatible);
    let mut command = config.clone();
    command.command = Some(Command {
        sequence: 1,
        payload: Payload(vec![9]),
    });
    variants.push(command);
    for invalid in variants {
        assert_eq!(
            session.apply(&transaction(invalid)),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(session.tree(window).unwrap().revision(), 0);
        assert!(session.tree(window).unwrap().is_empty());
    }
    session.apply(&transaction(config)).unwrap();
    assert_eq!(session.tree(window).unwrap().revision(), 1);
}
