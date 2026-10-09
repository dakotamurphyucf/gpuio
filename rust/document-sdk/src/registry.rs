use crate::{
    BASE_REVISION, Error, GPUI_REVISION, MAX_EVENT, MAX_PROFILES, MAX_PROPERTIES, PrepareContext,
    Profile, SDK_VERSION, contain, qualified_name,
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Descriptor {
    pub name: &'static str,
    pub version: u16,
    /// SHA-256 of the complete property/event schema, defined by the package.
    pub fingerprint: &'static str,
    pub sdk_version: u32,
    pub gpui_revision: &'static str,
    pub base_revision: &'static str,
    pub max_properties: usize,
    pub max_event: usize,
    /// Additional opaque native retention per installation; host-accounted text
    /// and highlights are reserved separately. Trusted author declaration.
    pub max_retained_bytes: usize,
}
impl Descriptor {
    pub fn validate(&self) -> Result<(), Error> {
        if !qualified_name(self.name)
            || self.version == 0
            || self.fingerprint.len() != 64
            || !self
                .fingerprint
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(Error::InvalidSchema);
        }
        if self.sdk_version != SDK_VERSION
            || self.gpui_revision != GPUI_REVISION
            || self.base_revision != BASE_REVISION
        {
            return Err(Error::IncompatibleSdk);
        }
        if !(1..=MAX_PROPERTIES).contains(&self.max_properties)
            || !(1..=MAX_EVENT).contains(&self.max_event)
            || self.max_retained_bytes > crate::MAX_RETAINED_BYTES
        {
            return Err(Error::LimitExceeded);
        }
        Ok(())
    }
}

pub trait Factory: Send + Sync + 'static {
    fn descriptor(&self) -> Descriptor;
    /// Pure bounded admission check; validate nested lengths before allocating.
    fn validate_properties(&self, properties: &[u8]) -> Result<(), Error>;
    /// Worker-only configuration. No native UI resources or synchronous OCaml calls.
    fn configure(&self, properties: &[u8], cx: &PrepareContext<'_>) -> Result<Profile, Error>;
}

pub struct Registry {
    factories: BTreeMap<&'static str, (Descriptor, Arc<dyn Factory>)>,
}
impl Registry {
    pub fn new(factories: impl IntoIterator<Item = Arc<dyn Factory>>) -> Result<Self, Error> {
        let mut registry = Self {
            factories: BTreeMap::new(),
        };
        for factory in factories {
            if registry.factories.len() >= MAX_PROFILES {
                return Err(Error::LimitExceeded);
            }
            let descriptor = contain(|| Ok(factory.descriptor()))?;
            descriptor.validate()?;
            if registry
                .factories
                .insert(descriptor.name, (descriptor, factory))
                .is_some()
            {
                return Err(Error::DuplicateProfile);
            }
        }
        Ok(registry)
    }
    pub fn descriptors(&self) -> impl Iterator<Item = Descriptor> + '_ {
        self.factories.values().map(|(d, _)| *d)
    }
    /// Resolve and validate before allocating retained property bytes. A mismatch
    /// never invokes the factory's configuration hook.
    pub fn bind(
        &self,
        name: &str,
        version: u16,
        fingerprint: &str,
        properties: &[u8],
    ) -> Result<Binding, Error> {
        let (descriptor, factory) = self.factories.get(name).ok_or(Error::UnknownProfile)?;
        if descriptor.version != version || descriptor.fingerprint != fingerprint {
            return Err(Error::IncompatibleSchema);
        }
        if properties.len() > descriptor.max_properties {
            return Err(Error::LimitExceeded);
        }
        contain(|| factory.validate_properties(properties))?;
        Ok(Binding {
            descriptor: *descriptor,
            factory: factory.clone(),
            properties: Arc::from(properties),
        })
    }
}

#[derive(Clone)]
pub struct Binding {
    descriptor: Descriptor,
    factory: Arc<dyn Factory>,
    properties: Arc<[u8]>,
}
impl Binding {
    pub fn descriptor(&self) -> Descriptor {
        self.descriptor
    }
    pub fn properties(&self) -> &[u8] {
        &self.properties
    }
    pub fn configure(&self, cx: &PrepareContext<'_>) -> Result<Profile, Error> {
        cx.check()?;
        let profile = contain(|| self.factory.configure(&self.properties, cx))?;
        cx.check()?;
        Ok(profile)
    }
}
