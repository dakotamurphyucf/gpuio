//! Mounted image leases. Acquire at accepted tree application, before later
//! source release messages; paint observes pixels without acquiring new readers.
use super::{View, image_corners};
use crate::{
    asset_svg, image_host,
    tree::{Node, Tree},
};
use gpui::{Context, Div, Stateful, Window, canvas, img, prelude::*, px};
use gpuio_protocol::{HandlerId, NodeId, v1::*};
use std::{cell::RefCell, rc::Rc, sync::Arc};

pub(super) struct State {
    source: ImageSource,
    binding: Rc<RefCell<Binding>>,
    emitted: Option<(HandlerId, ImageState)>,
}
struct Binding {
    current: Result<image_host::Handle, ImageError>,
    pending: Option<(asset_svg::Request, image_host::Handle)>,
    requested: Option<asset_svg::Request>,
    rendered: asset_svg::Request,
    intrinsic: Option<ImageMetadata>,
    resize_error: Option<ImageError>,
    svg: bool,
}
impl Binding {
    fn new(current: Result<image_host::Handle, ImageError>) -> Self {
        let svg = current.as_ref().is_ok_and(|handle| handle.is_svg());
        Self {
            current,
            pending: None,
            requested: None,
            rendered: Default::default(),
            intrinsic: None,
            resize_error: None,
            svg,
        }
    }
    fn observe(
        &mut self,
        window: &mut Window,
        cx: &mut gpui::App,
    ) -> (Option<Arc<gpui::RenderImage>>, ImageState) {
        let mut observed = match &self.current {
            Ok(handle) => image_host::image(handle, window, cx).map_err(error),
            Err(error) => Err(*error),
        };
        if let Ok(Some(image)) = &observed
            && self.intrinsic.is_none()
        {
            self.intrinsic = Some(ImageMetadata {
                width_px: u32::from(image.size(0).width).into(),
                height_px: u32::from(image.size(0).height).into(),
                frames: image.frame_count() as i64,
            });
        }
        if let Some((_, pending)) = &self.pending {
            match image_host::image(pending, window, cx) {
                Ok(Some(image)) => {
                    let (request, handle) = self.pending.take().unwrap();
                    self.current = Ok(handle);
                    self.rendered = request;
                    self.resize_error = None;
                    observed = Ok(Some(image));
                }
                Err(error_) => {
                    self.resize_error = Some(error(error_));
                    self.pending = None;
                }
                Ok(None) => (),
            }
        }
        let status = if let Some(error) = self.resize_error {
            ImageState::Failed(error)
        } else {
            match &observed {
                Ok(None) => ImageState::Loading,
                Err(error) => ImageState::Failed(*error),
                Ok(Some(_)) => {
                    ImageState::Ready(self.intrinsic.expect("metadata precedes resampling"))
                }
            }
        };
        (observed.ok().flatten(), status)
    }
    fn request(&mut self, request: asset_svg::Request, window: &mut Window, cx: &mut gpui::App) {
        if self.intrinsic.is_none() || self.requested == Some(request) {
            return;
        }
        self.pending = None; // Cancel replaced work before asking for another slot.
        self.requested = Some(request);
        self.resize_error = None;
        if let Ok(handle) = &self.current {
            match image_host::rerasterize(handle, request, cx) {
                Ok(handle) => self.pending = Some((request, handle)),
                Err(error_) => self.resize_error = Some(error(error_)),
            }
        }
        window.refresh();
    }
}
fn fit(value: ImageFit) -> gpui::ObjectFit {
    match value {
        ImageFit::Fill => gpui::ObjectFit::Fill,
        ImageFit::Contain => gpui::ObjectFit::Contain,
        ImageFit::Cover => gpui::ObjectFit::Cover,
        ImageFit::ScaleDown => gpui::ObjectFit::ScaleDown,
        ImageFit::None => gpui::ObjectFit::None,
    }
}
fn vector(
    binding: &Rc<RefCell<Binding>>,
    fitting: ImageFit,
    icon: bool,
    corners: image_corners::Shared,
) -> impl gpui::IntoElement {
    let weak = Rc::downgrade(binding);
    canvas(
        |_, _, _| (),
        move |bounds, _, window, cx| {
            let Some(binding) = weak.upgrade() else {
                return;
            };
            let mut binding = binding.borrow_mut();
            if bounds.size.width <= px(0.) || bounds.size.height <= px(0.) {
                return;
            }
            let scale = window.scale_factor();
            let width = (f32::from(bounds.size.width) * scale).ceil();
            let height = (f32::from(bounds.size.height) * scale).ceil();
            let desired =
                asset_svg::RasterSize::new(width as u32, height as u32).and_then(|size| {
                    Ok(asset_svg::Request {
                        size: asset_svg::Size::Exact(size),
                        density: asset_svg::Density::new(scale)?,
                        fit: fitting,
                        tint: icon.then(|| {
                            let color = window.text_style().color.to_rgb();
                            u32::from_be_bytes(
                                [color.r, color.g, color.b, color.a]
                                    .map(|channel| (channel.clamp(0., 1.) * 255.).round() as u8),
                            )
                        }),
                    })
                });
            match desired {
                Ok(request) => binding.request(request, window, cx),
                Err(error_) => {
                    let error = error(image_host::Error::Decode(error_));
                    if binding.resize_error != Some(error) {
                        binding.resize_error = Some(error);
                        window.refresh();
                    }
                }
            }
            let (image, _) = binding.observe(window, cx);
            if let Some(image) = image {
                if icon && binding.rendered.tint.is_none() {
                    return;
                }
                let image_bounds = match binding.rendered.size {
                    asset_svg::Size::Intrinsic => fit(fitting).get_bounds(bounds, image.size(0)),
                    asset_svg::Size::Exact(_) => bounds,
                };
                // Both color SVGs and tinted masks are decoded off-thread. GPUI
                // only uploads/paints the ready bitmap at the measured bounds.
                let painted =
                    window.paint_image(bounds, image_bounds, corners.get(), image, 0, false);
                if painted.is_err() && binding.resize_error != Some(ImageError::NativeFailure) {
                    binding.resize_error = Some(ImageError::NativeFailure);
                    window.refresh();
                }
            }
        },
    )
    .size_full()
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
            let Some(node) = tree.get(*id) else {
                continue;
            };
            let Some(config) = node.image.as_ref() else {
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
                ImageSource::Reference(id) => session.acquire_image(id).and_then(|lease| {
                    if node.kind == Kind::Icon
                        && lease.source().format() != gpuio_protocol::asset::Format::Svg
                    {
                        return Err(ImageError::Unsupported);
                    }
                    image_host::request(lease, window, cx).map_err(error)
                }),
                ImageSource::Unavailable(error) => Err(error),
            };
            self.images.insert(
                *id,
                State {
                    source: config.source,
                    binding: Rc::new(RefCell::new(Binding::new(handle))),
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
    ) -> (Stateful<Div>, image_corners::Shared) {
        let corners = image_corners::Shared::default();
        let Some(state) = self.images.get_mut(&node.id) else {
            return (element, corners);
        };
        let (observed, status) = state.binding.borrow_mut().observe(window, cx);
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
        let binding = state.binding.borrow();
        if let Some(metadata) = binding.intrinsic {
            element = element
                .w(px(metadata.width_px as f32))
                .h(px(metadata.height_px as f32))
                .overflow_hidden();
        }
        if binding.svg {
            element = element.child(vector(
                &state.binding,
                config.fit,
                node.kind == Kind::Icon,
                corners.clone(),
            ));
        } else if let Some(image) = observed {
            element = element.child(image_corners::Rounded::apply(
                img(image.clone())
                    .id(("image-pixels", image.id.0 as u64))
                    .size_full()
                    .object_fit(fit(config.fit)),
                corners.clone(),
            ));
        }
        (element, corners)
    }
}

#[cfg(feature = "native-image-tests")]
#[path = "image_view_test.rs"]
pub(crate) mod test;
