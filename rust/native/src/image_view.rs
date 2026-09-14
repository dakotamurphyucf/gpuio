//! Mounted image leases. Acquire at accepted tree application, before later
//! source release messages; paint observes pixels without acquiring new readers.
use super::View;
use crate::{
    image_host,
    tree::{Node, Tree},
};
use gpui::{Context, Div, Stateful, Window, img, prelude::*, px};
use gpuio_protocol::{HandlerId, NodeId, v1::*};

pub(super) struct State {
    source: ImageSource,
    handle: Result<image_host::Handle, ImageError>,
    emitted: Option<(HandlerId, ImageState)>,
}
fn error(error: image_host::Error) -> ImageError {
    use crate::{asset_cache::Error, asset_decode::Error as Decode};
    match error {
        Error::Closed => ImageError::Released,
        Error::ResourceLimit => ImageError::ResourceLimit,
        Error::WorkerFailed => ImageError::NativeFailure,
        Error::Decode(Decode::InvalidData) => ImageError::InvalidData,
        Error::Decode(Decode::Unsupported) => ImageError::Unsupported,
        Error::Decode(Decode::ResourceLimit) => ImageError::ResourceLimit,
    }
}
impl View {
    pub(super) fn sync_images(
        &mut self,
        dirty: &[NodeId],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let session = self.session.borrow();
        let Some(tree) = session.tree(self.id) else {
            self.images.clear();
            return;
        };
        self.images
            .retain(|id, _| tree.get(*id).is_some_and(|node| node.image.is_some()));
        for id in dirty {
            let Some(config) = tree.get(*id).and_then(|node| node.image.as_ref()) else {
                continue;
            };
            if self
                .images
                .get(id)
                .is_some_and(|state| state.source == config.source)
            {
                continue;
            }
            // Drop the old lease before requesting replacement work.
            self.images.remove(id);
            let handle = match config.source {
                ImageSource::Reference(id) => session
                    .acquire_image(id)
                    .and_then(|lease| image_host::request(lease, window, cx).map_err(error)),
                ImageSource::Unavailable(error) => Err(error),
            };
            self.images.insert(
                *id,
                State {
                    source: config.source,
                    handle,
                    emitted: None,
                },
            );
        }
    }

    pub(super) fn image_element(
        &mut self,
        tree: &Tree,
        node: &Node,
        config: &ImageConfig,
        mut element: Stateful<Div>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let Some(state) = self.images.get_mut(&node.id) else {
            return element;
        };
        let observed = match &state.handle {
            Ok(handle) => image_host::image(handle, window, cx).map_err(error),
            Err(error) => Err(*error),
        };
        let status = match &observed {
            Ok(None) => ImageState::Loading,
            Err(error) => ImageState::Failed(*error),
            Ok(Some(image)) => ImageState::Ready(ImageMetadata {
                width_px: u32::from(image.size(0).width).into(),
                height_px: u32::from(image.size(0).height).into(),
                frames: image.frame_count() as i64,
            }),
        };
        if let Some(handler) = node.handler {
            if state.emitted != Some((handler, status)) {
                state.emitted = Some((handler, status));
                let session = self.session.clone();
                let transport = self.transport.clone();
                let window_id = self.id;
                let node_id = node.id;
                let source = state.source;
                let revision = tree.revision();
                // Render holds an immutable session borrow. Dispatch afterward,
                // revalidating source and handler against the current tree.
                cx.defer(move |_| {
                    let event = session
                        .borrow()
                        .image_state(window_id, node_id, handler, revision, source, status);
                    if let Some(event) = event
                        && !transport.input(event)
                        && session.borrow_mut().overload(window_id)
                    {
                        transport.fault(window_id);
                    }
                });
            }
        } else {
            state.emitted = None;
        }
        if let Some(label) = &config.label {
            element = element.role(gpui::Role::Image).aria_label(label.clone());
        }
        if let Ok(Some(image)) = observed {
            let size = image.size(0);
            let fit = match config.fit {
                ImageFit::Fill => gpui::ObjectFit::Fill,
                ImageFit::Contain => gpui::ObjectFit::Contain,
                ImageFit::Cover => gpui::ObjectFit::Cover,
                ImageFit::ScaleDown => gpui::ObjectFit::ScaleDown,
                ImageFit::None => gpui::ObjectFit::None,
            };
            element = element
                .w(px(u32::from(size.width) as f32))
                .h(px(u32::from(size.height) as f32))
                .overflow_hidden()
                .child(
                    img(image.clone())
                        .id(("image-pixels", image.id.0 as u64))
                        .size_full()
                        .object_fit(fit),
                );
        }
        element
    }
}

#[cfg(feature = "native-image-tests")]
#[path = "image_view_test.rs"]
pub(crate) mod test;
