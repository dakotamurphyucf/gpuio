//! Mounted image leases. Acquire at accepted tree application, before later
//! source release messages; paint observes pixels without acquiring new readers.
use super::{Interaction, View, image_corners};
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
    layout_error: Option<ImageError>,
    svg: bool,
    mask: bool,
}
impl Binding {
    fn new(current: Result<image_host::Handle, ImageError>, mask: bool) -> Self {
        let svg = current.as_ref().is_ok_and(|handle| handle.is_svg());
        Self {
            current,
            pending: None,
            requested: None,
            rendered: Default::default(),
            intrinsic: None,
            resize_error: None,
            layout_error: None,
            svg,
            mask,
        }
    }
    fn pixels(
        &self,
        handle: &image_host::Handle,
        window: &mut Window,
        cx: &mut gpui::App,
    ) -> Result<Option<Arc<gpui::RenderImage>>, image_host::Error> {
        if self.mask {
            image_host::image_mask(handle, window, cx)
        } else {
            image_host::image(handle, window, cx)
        }
    }
    fn observe(
        &mut self,
        window: &mut Window,
        cx: &mut gpui::App,
    ) -> (Option<Arc<gpui::RenderImage>>, ImageState) {
        let mut observed = match &self.current {
            Ok(handle) => self.pixels(handle, window, cx).map_err(error),
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
            match self.pixels(pending, window, cx) {
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
        let status = if let Some(error) = self.layout_error.or(self.resize_error) {
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
        // An invalid layout size does not poison a previously successful raster
        // request. Keep it separate from a real decode/upload failure.
        if self.layout_error.take().is_some() {
            window.refresh();
        }
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

// An avatar must know which slot it will display before laying out a rich
// fallback. Freeze its SVG observation during prepaint; the paint pass consumes
// exactly this frame instead of polling the worker again. Ordinary tinted icons
// still prepare during paint, when the current hover/pressed foreground is known.
struct VectorFrame {
    image: Option<Arc<gpui::RenderImage>>,
    status: ImageState,
    rendered: asset_svg::Request,
}

fn vector_request(
    bounds: gpui::Bounds<gpui::Pixels>,
    scale: f32,
    fitting: ImageFit,
    tint: Option<u32>,
) -> Result<asset_svg::Request, crate::asset_decode::Error> {
    let width = (f32::from(bounds.size.width) * scale).ceil();
    let height = (f32::from(bounds.size.height) * scale).ceil();
    let size = asset_svg::RasterSize::new(width as u32, height as u32)?;
    Ok(asset_svg::Request {
        size: asset_svg::Size::Exact(size),
        density: asset_svg::Density::new(scale)?,
        fit: fitting,
        tint,
    })
}

fn prepare_vector(
    binding: &mut Binding,
    desired: Result<asset_svg::Request, crate::asset_decode::Error>,
    owner: &gpui::WeakEntity<View>,
    rendered_status: ImageState,
    window: &mut Window,
    cx: &mut gpui::App,
) -> VectorFrame {
    match desired {
        Ok(request) => binding.request(request, window, cx),
        Err(error_) => {
            let error = error(image_host::Error::Decode(error_));
            if binding.layout_error != Some(error) {
                binding.layout_error = Some(error);
                window.refresh();
            }
        }
    }
    let (image, status) = binding.observe(window, cx);
    // Layout failures/recovery arise after the view has observed the source.
    // Invalidate its cached render so the normal deferred, source-checked bridge
    // emits the changed state. Slot selection itself does not wait for the bridge.
    if status != rendered_status {
        let owner = owner.clone();
        window.defer(cx, move |window, cx| {
            if owner.update(cx, |_, cx| cx.notify()).is_ok() {
                window.refresh();
            }
        });
    }
    VectorFrame {
        image,
        status,
        rendered: binding.rendered,
    }
}

fn vector(
    binding: &Rc<RefCell<Binding>>,
    fitting: ImageFit,
    icon: bool,
    corners: image_corners::Shared,
    fallback: Option<Arc<str>>,
    owner: gpui::WeakEntity<View>,
    rendered_status: ImageState,
) -> impl gpui::IntoElement {
    let weak = Rc::downgrade(binding);
    let prepaint_binding = weak.clone();
    let prepaint_owner = owner.clone();
    let avatar = fallback.is_some();
    canvas(
        move |bounds, window, cx| {
            if !avatar || bounds.size.width <= px(0.) || bounds.size.height <= px(0.) {
                return None;
            }
            let binding = prepaint_binding.upgrade()?;
            let desired = vector_request(bounds, window.scale_factor(), fitting, None);
            let frame = prepare_vector(
                &mut binding.borrow_mut(),
                desired,
                &prepaint_owner,
                rendered_status,
                window,
                cx,
            );
            Some(frame)
        },
        move |bounds, prepared, window, cx| {
            let Some(binding) = weak.upgrade() else {
                return;
            };
            let mut binding = binding.borrow_mut();
            if bounds.size.width <= px(0.) || bounds.size.height <= px(0.) {
                return;
            }
            let frame = prepared.unwrap_or_else(|| {
                let tint = icon.then(|| {
                    let color = window.text_style().color.to_rgb();
                    u32::from_be_bytes(
                        [color.r, color.g, color.b, color.a]
                            .map(|channel| (channel.clamp(0., 1.) * 255.).round() as u8),
                    )
                });
                let desired = vector_request(bounds, window.scale_factor(), fitting, tint);
                prepare_vector(&mut binding, desired, &owner, rendered_status, window, cx)
            });
            if let Some(text) = &fallback
                && (frame.image.is_none() || matches!(frame.status, ImageState::Failed(_)))
            {
                super::avatar::paint(text, bounds, window, cx);
                return;
            }
            paint_vector_frame(&mut binding, frame, bounds, fitting, icon, &corners, window);
        },
    )
    .size_full()
}
fn paint_vector_frame(
    binding: &mut Binding,
    frame: VectorFrame,
    bounds: gpui::Bounds<gpui::Pixels>,
    fitting: ImageFit,
    icon: bool,
    corners: &image_corners::Shared,
    window: &mut Window,
) {
    if let Some(image) = frame.image {
        if icon && frame.rendered.tint.is_none() {
            return;
        }
        let image_bounds = match frame.rendered.size {
            asset_svg::Size::Intrinsic => fit(fitting).get_bounds(bounds, image.size(0)),
            asset_svg::Size::Exact(_) => bounds,
        };
        // Both color SVGs and tinted masks are decoded off-thread. GPUI
        // only uploads/paints the ready bitmap at the measured bounds.
        let painted = window.paint_image(bounds, image_bounds, corners.get(), image, 0, false);
        if painted.is_err() && binding.resize_error != Some(ImageError::NativeFailure) {
            binding.resize_error = Some(ImageError::NativeFailure);
            window.refresh();
        }
    }
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
        self.install_spinner_cleanup(window, cx);
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
                    if (node.kind == Kind::Icon || node.spinner.is_some())
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
                    binding: Rc::new(RefCell::new(Binding::new(handle, node.spinner.is_some()))),
                    emitted: None,
                },
            );
        }
    }

    fn observe_image(
        &mut self,
        tree: &Tree,
        node: &Node,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(Option<Arc<gpui::RenderImage>>, ImageState)> {
        let state = self.images.get_mut(&node.id)?;
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
        Some((observed, status))
    }

    pub(super) fn image_element(
        &mut self,
        tree: &Tree,
        node: &Node,
        interaction: Interaction,
        mut element: Stateful<Div>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Stateful<Div>, image_corners::Shared) {
        let config = node.image.as_ref().expect("image element configuration");
        let corners = image_corners::Shared::default();
        if !self.images.contains_key(&node.id) {
            return (element, corners);
        }
        let (observed, status) = self
            .observe_image(tree, node, window, cx)
            .expect("mounted image");
        let state = &self.images[&node.id];
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
        if node.avatar.is_some() && !node.children.is_empty() {
            drop(binding);
            return (
                element.child(self.avatar_slot(
                    tree,
                    node,
                    corners.clone(),
                    Some(status),
                    interaction.passive_disabled,
                    cx,
                )),
                corners,
            );
        }
        if binding.svg {
            element = element.child(vector(
                &state.binding,
                config.fit,
                node.kind == Kind::Icon,
                corners.clone(),
                node.avatar.as_ref().map(|c| c.fallback.clone().into()),
                cx.entity().downgrade(),
                status,
            ));
        } else if let Some(avatar) = &node.avatar
            && (observed.is_none() || matches!(status, ImageState::Failed(_)))
        {
            element = element.child(super::avatar::fallback(avatar.fallback.clone().into()));
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

impl View {
    /// Resolve the candidate at the rich avatar's assigned size, before building
    /// either slot. Raster images keep GPUI Img's animated-frame lifecycle.
    pub(super) fn avatar_primary(
        &self,
        node: &Node,
        size: gpui::Size<gpui::Pixels>,
        corners: image_corners::Shared,
        rendered_status: Option<ImageState>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let state = self.images.get(&node.id)?;
        let config = node.image.as_ref()?;
        let mut binding = state.binding.borrow_mut();
        let owner = cx.entity().downgrade();
        if binding.svg {
            let frame = prepare_vector(
                &mut binding,
                vector_request(
                    gpui::Bounds::new(gpui::point(px(0.), px(0.)), size),
                    window.scale_factor(),
                    config.fit,
                    None,
                ),
                &owner,
                rendered_status.unwrap_or(ImageState::Loading),
                window,
                cx,
            );
            if frame.image.is_none() || matches!(frame.status, ImageState::Failed(_)) {
                return None;
            }
            let binding = Rc::downgrade(&state.binding);
            let fitting = config.fit;
            return Some(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        if let Some(binding) = binding.upgrade() {
                            paint_vector_frame(
                                &mut binding.borrow_mut(),
                                frame,
                                bounds,
                                fitting,
                                false,
                                &corners,
                                window,
                            );
                        }
                    },
                )
                .size_full()
                .into_any_element(),
            );
        }
        let (image, status) = binding.observe(window, cx);
        if Some(status) != rendered_status {
            window.defer(cx, move |window, cx| {
                if owner.update(cx, |_, cx| cx.notify()).is_ok() {
                    window.refresh();
                }
            });
        }
        if matches!(status, ImageState::Failed(_)) {
            return None;
        }
        image.map(|image| {
            image_corners::Rounded::apply(
                img(image.clone())
                    .id(("image-pixels", image.id.0 as u64))
                    .size_full()
                    .object_fit(fit(config.fit)),
                corners,
            )
            .into_any_element()
        })
    }
}

#[cfg(feature = "native-image-tests")]
#[path = "image_view_test.rs"]
pub(crate) mod test;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "image_prepaint_test.rs"]
mod prepaint_test;

impl View {
    fn install_spinner_cleanup(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.spinner_close.is_none() {
            let window_id = window.window_handle().window_id();
            let view = cx.entity().downgrade();
            self.spinner_close = Some(cx.on_window_closed(move |cx, closed| {
                if closed == window_id {
                    let _ = view.update(cx, |view, _| {
                        view.images.retain(|_, state| !state.binding.borrow().mask);
                        view.spinners.clear();
                    });
                }
            }));
        }
    }

    pub(super) fn sync_spinners(&mut self) {
        let session = self.session.borrow();
        self.spinners.retain(|id, owner| {
            let Some(config) = session
                .tree(self.id)
                .and_then(|tree| tree.get(*id))
                .and_then(|node| node.spinner.as_ref())
            else {
                return false;
            };
            owner
                .update(config.clone())
                .expect("admitted spinner config");
            owner.prepare_frame();
            true
        });
    }

    pub(super) fn spinner_element(
        &mut self,
        tree: &Tree,
        node: &Node,
        inert: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let status = self
            .observe_image(tree, node, window, cx)
            .map_or(ImageState::Loading, |(_, status)| status);
        let binding = self
            .images
            .get(&node.id)
            .map(|state| Rc::downgrade(&state.binding));
        let owner = self.spinners.entry(node.id).or_insert_with(|| {
            crate::spinner_clock::Owner::new(
                node.spinner.as_ref().unwrap().clone(),
                self.spinner_clock.clone(),
            )
            .expect("admitted spinner")
        });
        let driver = owner.driver();
        let view = cx.entity().downgrade();
        canvas(
            |_, _, _| (),
            move |bounds, _, window, cx| {
                // Avoid size requests and image polling for an invisible paint.
                let clip = bounds.intersect(&window.content_mask().bounds);
                if clip.size.width <= px(0.)
                    || clip.size.height <= px(0.)
                    || window.element_opacity() <= 0.
                    || window.text_style().color.a <= 0.
                {
                    driver.suspend();
                    return;
                }
                let binding = binding.and_then(|binding| binding.upgrade());
                let pixels = binding.as_ref().and_then(|binding| {
                    let frame = prepare_vector(
                        &mut binding.borrow_mut(),
                        vector_request(bounds, window.scale_factor(), ImageFit::Contain, None),
                        &view,
                        status,
                        window,
                        cx,
                    );
                    if matches!(frame.status, ImageState::Failed(_)) {
                        None
                    } else {
                        frame.image
                    }
                });
                let report = crate::spinner_paint::paint(
                    &driver,
                    bounds,
                    pixels.as_ref(),
                    inert,
                    window,
                    cx,
                );
                if matches!(
                    report,
                    crate::spinner_paint::Report::Fallback {
                        mask_failed: true,
                        ..
                    }
                ) && let Some(binding) = binding
                    && binding.borrow().resize_error != Some(ImageError::NativeFailure)
                {
                    binding.borrow_mut().resize_error = Some(ImageError::NativeFailure);
                    let view = view.clone();
                    window.defer(cx, move |window, cx| {
                        let _ = view.update(cx, |_, cx| cx.notify());
                        window.refresh();
                    });
                }
            },
        )
        .size_full()
        .into_any_element()
    }
}

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "spinner_view_test.rs"]
mod spinner_tests;

#[cfg(all(test, feature = "native-image-tests"))]
#[path = "tab_menu_icons_test.rs"]
mod tab_menu_icons_tests;
