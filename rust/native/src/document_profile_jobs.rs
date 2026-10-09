//! Checked profile requests and complete worker output. Native UI installation
//! owns the returned profile and must retain its work charge for the same lifetime.
use crate::{document_highlight, document_jobs::Error};
use gpuio_document_sdk as sdk;
use gpuio_protocol::document_profile::{Config, Stage};
use std::sync::{Arc, atomic::AtomicBool};

#[derive(Clone)]
pub struct Request {
    config: Arc<Config>,
    binding: sdk::Binding,
    observer: Option<gpuio_protocol::HandlerId>,
}
impl Request {
    pub fn bind(config: Arc<Config>) -> Result<Self, Error> {
        if !config.is_valid() {
            return Err(Error::Profile(
                Stage::Configure,
                sdk::Error::InvalidProperties,
            ));
        }
        let instance = config.instance.as_ref().ok_or(Error::Profile(
            Stage::Configure,
            sdk::Error::InvalidProperties,
        ))?;
        let binding = crate::document_profiles::bind(instance)
            .map_err(|error| Error::Profile(Stage::Configure, error))?;
        Ok(Self {
            config,
            binding,
            observer: None,
        })
    }
    pub fn with_observer(mut self, handler: Option<gpuio_protocol::HandlerId>) -> Self {
        self.observer = handler;
        self
    }
    pub fn observer(&self) -> Option<gpuio_protocol::HandlerId> {
        self.observer
    }
    pub fn config(&self) -> &Arc<Config> {
        &self.config
    }
    pub fn descriptor(&self) -> sdk::Descriptor {
        self.binding.descriptor()
    }
    /// Conservative admission units for bounded generated strings, projection,
    /// highlight vector capacity and adapter metadata, plus declared opaque data.
    /// Ordinary source/AST work is charged separately by the document pool.
    pub fn work_units(&self) -> usize {
        self.descriptor().max_retained_bytes
            + sdk::MAX_PROPERTIES
            + sdk::MAX_GENERATED_BYTES * 4
            + sdk::MAX_CODE_BYTES * 16
            + sdk::MAX_RUNS * std::mem::size_of::<sdk::Highlight>() * 2
            + sdk::MAX_PLUGINS * 1024
    }
    pub(super) fn retained_units(&self, prepared: &sdk::Prepared) -> usize {
        self.descriptor().max_retained_bytes
            + self.binding.properties().len() * 2
            + prepared.generated_bytes * 4
            + prepared.document.displayed_text().text_bytes() * 8
            + prepared.profile.plugins().len() * 1024
            + 4096
    }
    pub(super) fn prepare(
        &self,
        text: &str,
        html: bool,
        base: gpui_base::text::MarkdownExtensions,
        parser_epoch: u64,
        dark: bool,
        cancelled: Arc<AtomicBool>,
    ) -> Result<(sdk::Prepared, std::time::Duration), Error> {
        use std::sync::atomic::Ordering;
        let check = || cancelled.load(Ordering::Acquire);
        let started = std::time::Instant::now();
        let profile = self
            .binding
            .configure(&sdk::PrepareContext::new(&check))
            .map_err(|error| Error::Profile(Stage::Configure, error))?;
        let configure_time = started.elapsed();
        let request = sdk::Preparation {
            source: text,
            base,
            parser_epoch,
            dark,
            cancelled,
        };
        let result = if html {
            sdk::prepare_html(
                Arc::new(profile),
                request,
                crate::document_markdown::html_image,
            )
        } else {
            sdk::prepare_markdown(Arc::new(profile), request)
        };
        result
            .map(|prepared| (prepared, configure_time))
            .map_err(|failure| {
                Error::Profile(
                    match failure.stage {
                        sdk::PreparationStage::Parse => Stage::Parse,
                        sdk::PreparationStage::Highlight => Stage::Highlight,
                    },
                    failure.error,
                )
            })
    }
    #[cfg(test)]
    pub(super) fn for_test(config: Arc<Config>, binding: sdk::Binding) -> Self {
        Self {
            config,
            binding,
            observer: None,
        }
    }
}

pub struct Prepared {
    pub profile: Arc<sdk::Profile>,
    pub parser_epoch: u64,
    pub descriptor: sdk::Descriptor,
    pub retained_units: usize,
}
pub enum Highlights {
    Native(Vec<document_highlight::Run>),
    Profile(Vec<sdk::Highlight>),
}
impl Highlights {
    pub fn len(&self) -> usize {
        match self {
            Self::Native(runs) => runs.len(),
            Self::Profile(runs) => runs.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn retained_units(&self) -> usize {
        match self {
            Self::Native(runs) => runs
                .capacity()
                .saturating_mul(std::mem::size_of::<document_highlight::Run>()),
            Self::Profile(runs) => runs
                .capacity()
                .saturating_mul(std::mem::size_of::<sdk::Highlight>()),
        }
    }
    pub fn styles(&self) -> Vec<(std::ops::Range<usize>, gpui::HighlightStyle)> {
        match self {
            Self::Native(runs) => runs
                .iter()
                .map(|run| (run.bytes.clone(), crate::document_editor::style(run)))
                .collect(),
            Self::Profile(runs) => runs
                .iter()
                .map(|run| {
                    let color = |[r, g, b]: [u8; 3]| {
                        gpui::rgb((r as u32) << 16 | (g as u32) << 8 | b as u32).into()
                    };
                    (
                        run.bytes.clone(),
                        gpui::HighlightStyle {
                            color: Some(color(run.foreground)),
                            background_color: run.background.map(color),
                            font_weight: Some(gpui::FontWeight(run.weight as f32)),
                            font_style: run.italic.then_some(gpui::FontStyle::Italic),
                            underline: run.underline.then_some(gpui::UnderlineStyle {
                                thickness: gpui::px(1.),
                                color: None,
                                wavy: false,
                            }),
                            strikethrough: run.strikethrough.then_some(gpui::StrikethroughStyle {
                                thickness: gpui::px(1.),
                                color: None,
                            }),
                            ..Default::default()
                        },
                    )
                })
                .collect(),
        }
    }
}
