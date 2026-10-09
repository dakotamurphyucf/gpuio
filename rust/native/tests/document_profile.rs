use gpuio_document_sdk as sdk;
use gpuio_native::{document_profiles, registrations, session::Session};
use gpuio_protocol::{
    HandlerId, NodeId, WindowId,
    document::{Config as Document, Layout, Mode},
    document_profile::{Config, Instance},
    extension::{Payload, Schema},
    v1::*,
};
use std::sync::Arc;
const DESCRIPTOR: sdk::Descriptor = sdk::Descriptor {
    name: "test.reader",
    version: 1,
    fingerprint: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    sdk_version: sdk::SDK_VERSION,
    gpui_revision: sdk::GPUI_REVISION,
    base_revision: sdk::BASE_REVISION,
    max_properties: 1,
    max_event: 4,
    max_retained_bytes: 1024,
};
struct Factory;
impl sdk::Factory for Factory {
    fn descriptor(&self) -> sdk::Descriptor {
        DESCRIPTOR
    }
    fn validate_properties(&self, bytes: &[u8]) -> Result<(), sdk::Error> {
        assert_ne!(bytes, &[99], "contained admission panic");
        if bytes == [7] {
            Ok(())
        } else {
            Err(sdk::Error::InvalidProperties)
        }
    }
    fn configure(&self, _: &[u8], _: &sdk::PrepareContext<'_>) -> Result<sdk::Profile, sdk::Error> {
        panic!("atomic tree admission must never configure a profile")
    }
}
fn document(mode: Mode) -> Document {
    Document {
        source: None,
        mode,
        dark: false,
        layout: Layout::Flow,
        label: "Reader".into(),
        path: None,
        line_numbers: false,
        initially_collapsed: false,
        search: String::new(),
        images: vec![],
    }
}
#[test]
fn failed_catalog_registration_can_retry_and_profile_admission_is_atomic() {
    // A bad second category must not freeze an empty first category.
    let factory: Arc<dyn sdk::Factory> = Arc::new(Factory);
    assert_eq!(
        registrations::install([], [factory.clone(), factory.clone()]),
        Err(registrations::Error::Document(sdk::Error::DuplicateProfile))
    );
    registrations::install([], [factory]).unwrap();
    let schema = Schema {
        name: DESCRIPTOR.name.into(),
        version: 1,
        fingerprint: DESCRIPTOR.fingerprint.into(),
    };
    assert_eq!(document_profiles::catalog(), vec![schema.clone()]);
    assert!(gpuio_native::extensions::catalog().is_empty());
    assert_eq!(document_profiles::install([]), Err(sdk::Error::Closed));
    assert_eq!(
        registrations::install([], []),
        Err(registrations::Error::Closed)
    );
    assert_eq!(
        gpuio_native::extensions::install([]),
        Err(gpuio_extension_sdk::Error::Closed)
    );
    let instance = Instance {
        schema,
        generation: 2,
        properties: Payload(vec![7]),
    };
    let config = Config {
        epoch: 1,
        instance: Some(instance.clone()),
    };
    let window = WindowId::from_parts(0, 1).unwrap();
    let node = NodeId::from_parts(0, 1).unwrap();
    let handler = HandlerId::from_parts(0, 1).unwrap();
    let tx = |base, operations| Transaction {
        window,
        base,
        revision: base + 1,
        operations,
    };
    let mount = |config, kind, mode, handler| {
        tx(
            0,
            vec![
                Op::Create(node, kind, String::new(), handler),
                Op::SetDocumentProfile(node, config),
                Op::SetDocument(node, document(mode)),
                Op::SetRoot(Some(node)),
            ],
        )
    };
    let mut session = Session::default();
    session.hello(VERSION, CAPABILITIES).unwrap();
    session.open(1, window, "Profiles", 400., 300.).unwrap();
    let mut variants = vec![];
    for change in 0..6 {
        let mut bad = instance.clone();
        match change {
            0 => bad.properties = Payload(vec![6]),
            1 => bad.properties = Payload(vec![99]),
            2 => bad.properties = Payload(vec![7, 7]),
            3 => bad.schema.name = "test.missing".into(),
            4 => bad.schema.version = 2,
            _ => bad.schema.fingerprint = "b".repeat(64),
        }
        variants.push(mount(
            Config {
                epoch: 1,
                instance: Some(bad),
            },
            Kind::DocumentView,
            Mode::Markdown,
            Some(handler),
        ));
    }
    for (kind, mode, handler) in [
        (Kind::Container, Mode::Markdown, Some(handler)),
        (Kind::DocumentView, Mode::Markdown, None),
        (Kind::DocumentView, Mode::Code("txt".into()), Some(handler)),
    ] {
        variants.push(mount(config.clone(), kind, mode, handler));
    }
    for invalid in variants {
        assert_eq!(session.apply(&invalid), Err(ErrorCode::InvalidTree));
        assert_eq!(session.tree(window).unwrap().revision(), 0);
        assert!(session.tree(window).unwrap().is_empty());
    }
    session
        .apply(&mount(
            config.clone(),
            Kind::DocumentView,
            Mode::Markdown,
            Some(handler),
        ))
        .unwrap();
    let bytes_with_profile = session.tree(window).unwrap().retained_bytes();
    let mut regression = instance.clone();
    regression.generation = 1;
    for operations in [
        vec![Op::SetDocumentProfile(node, config)],
        vec![Op::SetDocumentProfile(
            node,
            Config {
                epoch: 2,
                instance: Some(regression.clone()),
            },
        )],
        vec![Op::SetDocument(node, document(Mode::Diff))],
        vec![Op::Bind(node, None)],
    ] {
        assert_eq!(
            session.apply(&tx(1, operations)),
            Err(ErrorCode::InvalidTree)
        );
        assert_eq!(session.tree(window).unwrap().revision(), 1);
        assert_eq!(
            session.tree(window).unwrap().retained_bytes(),
            bytes_with_profile
        );
    }
    session
        .apply(&tx(
            1,
            vec![Op::SetDocumentProfile(
                node,
                Config {
                    epoch: 2,
                    instance: None,
                },
            )],
        ))
        .unwrap();
    assert!(session.tree(window).unwrap().retained_bytes() < bytes_with_profile);
    // Clear resets application generation, but cannot reset the host epoch.
    assert_eq!(
        session.apply(&tx(
            2,
            vec![Op::SetDocumentProfile(
                node,
                Config {
                    epoch: 1,
                    instance: Some(instance)
                }
            )]
        )),
        Err(ErrorCode::InvalidTree)
    );
    session
        .apply(&tx(
            2,
            vec![Op::SetDocumentProfile(
                node,
                Config {
                    epoch: 3,
                    instance: Some(regression),
                },
            )],
        ))
        .unwrap();
    session
        .apply(&tx(3, vec![Op::SetRoot(None), Op::Remove(node)]))
        .unwrap();
    assert_eq!(session.tree(window).unwrap().retained_bytes(), 0);
}
