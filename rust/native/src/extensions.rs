//! Application-wide, immutable build-time extension catalog.
use gpuio_extension_sdk::{Descriptor, Error, Factory, Registry, contain};
use gpuio_protocol::extension::{Config, Schema};
use std::sync::{Arc, OnceLock};

static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Install exactly once before creating a transport or reading the catalog.
/// Generated application backends call this with their compiled factories.
/// Validation is atomic; a rejected catalog does not freeze registration.
pub fn install(factories: impl IntoIterator<Item = Arc<dyn Factory>>) -> Result<(), Error> {
    let registry = Registry::new(factories)?;
    REGISTRY.set(registry).map_err(|_| Error::Closed)
}

pub(crate) fn registry() -> &'static Registry {
    REGISTRY.get_or_init(|| Registry::new([]).expect("empty registry is valid"))
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

pub(crate) fn validate(config: &Config) -> Result<(Descriptor, Arc<dyn Factory>), Error> {
    if !config.is_valid() {
        return Err(Error::InvalidProperties);
    }
    let (descriptor, factory) = registry().resolve(
        &config.schema.name,
        config.schema.version as u16,
        &config.schema.fingerprint,
    )?;
    if config.properties.0.len() > descriptor.max_properties {
        return Err(Error::LimitExceeded);
    }
    contain(|| factory.validate_properties(&config.properties.0))?;
    if let Some(command) = &config.command {
        if command.payload.0.len() > descriptor.max_command {
            return Err(Error::LimitExceeded);
        }
        contain(|| factory.validate_command(&command.payload.0))?;
    }
    Ok((descriptor, factory))
}

pub(crate) fn wire_error(error: Error) -> gpuio_protocol::extension::Error {
    use gpuio_protocol::extension::Error as Wire;
    match error {
        Error::InvalidSchema => Wire::InvalidSchema,
        Error::IncompatibleSdk => Wire::IncompatibleSdk,
        Error::DuplicateComponent => Wire::DuplicateComponent,
        Error::LimitExceeded => Wire::LimitExceeded,
        Error::UnknownComponent => Wire::UnknownComponent,
        Error::IncompatibleSchema => Wire::IncompatibleSchema,
        Error::InvalidProperties => Wire::InvalidProperties,
        Error::InvalidCommand => Wire::InvalidCommand,
        Error::Closed => Wire::Closed,
        Error::Hidden => Wire::Hidden,
        Error::Stale => Wire::Stale,
        Error::Overloaded => Wire::Overloaded,
        Error::Panicked => Wire::Panicked,
    }
}
