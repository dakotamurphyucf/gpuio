//! Immutable statically linked document profile catalog. Installation precedes
//! transport creation/catalog queries; profiles are not process-global UI defaults.
use gpuio_document_sdk::{Binding, Error, Factory, Registry};
use gpuio_protocol::{document_profile::Instance, extension::Schema};
use std::sync::Arc;
pub fn install(factories: impl IntoIterator<Item = Arc<dyn Factory>>) -> Result<(), Error> {
    let registry = Registry::new(factories)?;
    crate::registrations::install_prepared(
        gpuio_extension_sdk::Registry::new([]).expect("empty component catalog is valid"),
        registry,
    )
    .map_err(|_| Error::Closed)
}
pub(crate) fn registry() -> &'static Registry {
    &crate::registrations::get().documents
}
pub fn catalog() -> Vec<Schema> {
    registry()
        .descriptors()
        .map(|descriptor| Schema {
            name: descriptor.name.to_owned(),
            version: i64::from(descriptor.version),
            fingerprint: descriptor.fingerprint.to_owned(),
        })
        .collect()
}
pub(crate) fn bind(instance: &Instance) -> Result<Binding, Error> {
    if !instance.is_valid() {
        return Err(Error::InvalidProperties);
    }
    registry().bind(
        &instance.schema.name,
        instance.schema.version as u16,
        &instance.schema.fingerprint,
        &instance.properties.0,
    )
}
pub fn wire_error(error: Error) -> gpuio_protocol::document_profile::Error {
    use gpuio_protocol::document_profile::Error as W;
    match error {
        Error::InvalidSchema => W::InvalidSchema,
        Error::IncompatibleSdk => W::IncompatibleSdk,
        Error::DuplicateProfile => W::DuplicateProfile,
        Error::UnknownProfile => W::UnknownProfile,
        Error::IncompatibleSchema => W::IncompatibleSchema,
        Error::InvalidProperties => W::InvalidProperties,
        Error::InvalidPlugin => W::InvalidPlugin,
        Error::DuplicatePlugin => W::DuplicatePlugin,
        Error::InvalidHighlight => W::InvalidHighlight,
        Error::InvalidSource => W::InvalidSource,
        Error::Parse => W::Parse,
        Error::Render => W::Render,
        Error::Highlight => W::Highlight,
        Error::LimitExceeded => W::LimitExceeded,
        Error::Cancelled => W::Cancelled,
        Error::Panicked => W::Panicked,
        Error::Closed => W::Closed,
    }
}
