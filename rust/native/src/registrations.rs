//! One atomic, immutable build-time catalog for components and document profiles.
//! Validate both categories before freezing either; generated backends call install
//! on the OS main thread before creating a transport or querying a catalog.
use gpuio_document_sdk as documents;
use gpuio_extension_sdk as components;
use std::sync::{Arc, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Component(components::Error),
    Document(documents::Error),
    Closed,
}
pub(crate) struct Catalogs {
    pub components: components::Registry,
    pub documents: documents::Registry,
}
static CATALOGS: OnceLock<Catalogs> = OnceLock::new();
pub fn install(
    component_factories: impl IntoIterator<Item = Arc<dyn components::Factory>>,
    document_factories: impl IntoIterator<Item = Arc<dyn documents::Factory>>,
) -> Result<(), Error> {
    let components = components::Registry::new(component_factories).map_err(Error::Component)?;
    let documents = documents::Registry::new(document_factories).map_err(Error::Document)?;
    install_prepared(components, documents)
}
pub(crate) fn install_prepared(
    components: components::Registry,
    documents: documents::Registry,
) -> Result<(), Error> {
    CATALOGS
        .set(Catalogs {
            components,
            documents,
        })
        .map_err(|_| Error::Closed)
}
pub(crate) fn get() -> &'static Catalogs {
    CATALOGS.get_or_init(|| Catalogs {
        components: components::Registry::new([]).expect("empty component catalog is valid"),
        documents: {
            // This test executable statically links its fixture profile just as
            // a generated application backend links package factories. No runtime
            // override or mutable test registry is exposed in production.
            #[cfg(all(test, feature = "native-image-tests"))]
            let factories = crate::document_profile_fixture::factories();
            #[cfg(not(all(test, feature = "native-image-tests")))]
            let factories: Vec<Arc<dyn documents::Factory>> = vec![];
            documents::Registry::new(factories).expect("linked document catalog is valid")
        },
    })
}
