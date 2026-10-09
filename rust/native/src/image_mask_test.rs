//! Decoded mask scene/cache contract on TestPlatform, not GPU pixel acceptance.
use gpui::{
    AtlasKey, AtlasTextureKind, Bounds, Context, IntoElement, Pixels, Render, RenderImage,
    RenderImageParams, TestAppContext, TransformationMatrix, Window, canvas, div, point,
    prelude::*, px, rgba, size,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

struct Fixture {
    image: Arc<RenderImage>,
    bounds: Bounds<Pixels>,
    transform: TransformationMatrix,
    color: gpui::Hsla,
    opacity: f32,
    frame: usize,
    also_color: bool,
    result: Rc<RefCell<Result<(), String>>>,
}
impl Render for Fixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let image = self.image.clone();
        let bounds = self.bounds;
        let transform = self.transform;
        let color = self.color;
        let frame = self.frame;
        let also_color = self.also_color;
        let result = self.result.clone();
        div().size_full().opacity(self.opacity).child(
            canvas(
                |_, _, _| (),
                move |_, _, window, _| {
                    *result.borrow_mut() = window
                        .paint_image_mask(bounds, image.clone(), frame, transform, color)
                        .map_err(|error| error.to_string());
                    if also_color {
                        window
                            .paint_image(
                                bounds,
                                bounds,
                                Default::default(),
                                image.clone(),
                                0,
                                false,
                            )
                            .unwrap();
                    }
                },
            )
            .size_full(),
        )
    }
}

#[test]
fn decoded_masks_reuse_tiles_apply_transform_and_opacity_and_drop_both_representations() {
    let image = Arc::new(RenderImage::new(vec![image::Frame::new(
        image::RgbaImage::from_raw(2, 1, vec![255, 0, 0, 64, 0, 0, 255, 255]).unwrap(),
    )]));
    let params = RenderImageParams {
        image_id: image.id,
        frame_index: 0,
    };
    assert!(AtlasKey::Image(params.clone()) != AtlasKey::ImageMask(params.clone()));
    assert!(AtlasKey::ImageMask(params).texture_kind() == AtlasTextureKind::Monochrome);

    let result = Rc::new(RefCell::new(Ok(())));
    let mut app = TestAppContext::single();
    let (view, cx) = app.add_window_view(|_, _| Fixture {
        image: image.clone(),
        bounds: Bounds::new(point(px(10.), px(20.)), size(px(40.), px(20.))),
        transform: TransformationMatrix::unit(),
        color: rgba(0x44aa88cc).into(),
        opacity: 0.5,
        frame: 0,
        also_color: true,
        result: result.clone(),
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(result.borrow().is_ok());
    let tile = cx.update(|window, _| {
        assert!(window.has_image_atlas_entry(&image));
        assert!(window.has_image_mask_atlas_entry(&image, 0));
        let painted = window.painted_monochrome_sprites();
        assert_eq!(painted.len(), 1);
        assert!((painted[0].color.a - 0.4).abs() < 0.0001);
        assert!(painted[0].tile.texture_id.kind == AtlasTextureKind::Monochrome);
        painted[0].tile.tile_id
    });

    // The nominal quad is completely outside this viewport. Translation must
    // bring it into the scene; culling before transformation would lose it.
    view.update(cx, |view, cx| {
        view.bounds.origin.x = px(-1000.);
        view.color = rgba(0xaa4488ff).into();
        view.also_color = false;
        cx.notify();
    });
    cx.update(|window, cx| {
        view.update(cx, |view, _| {
            view.transform = TransformationMatrix::unit()
                .translate(point(px(1010.), px(0.)).scale(window.scale_factor()));
        });
        window.draw(cx).clear(cx);
        let painted = window.painted_monochrome_sprites();
        assert_eq!(
            painted.len(),
            1,
            "transformed mask entering clip must be painted"
        );
        assert_eq!(
            painted[0].tile.tile_id, tile,
            "color/placement do not create new uploads"
        );
        assert!((painted[0].color.a - 0.5).abs() < 0.0001);
    });
    assert!(result.borrow().is_ok());

    // A quarter turn around a visible center is a shader transform of the same
    // uploaded alpha data, not a separately rasterized image.
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.bounds.origin.x = px(10.);
            let center = view.bounds.center().scale(window.scale_factor());
            view.transform = TransformationMatrix::unit()
                .translate(center)
                .rotate(gpui::radians(std::f32::consts::FRAC_PI_2))
                .translate(view.bounds.center().scale(-window.scale_factor()));
            cx.notify();
        });
        window.draw(cx).clear(cx);
        let painted = window.painted_monochrome_sprites();
        assert_eq!(painted.len(), 1);
        assert_eq!(painted[0].tile.tile_id, tile);
        assert!(painted[0].transformation != TransformationMatrix::unit());
        window.drop_image(image.clone()).unwrap();
        assert!(!window.has_image_atlas_entry(&image));
        assert!(!window.has_image_mask_atlas_entry(&image, 0));
    });

    for case in 0..6 {
        cx.update(|window, cx| {
            view.update(cx, |view, cx| {
                view.bounds = Bounds::new(point(px(10.), px(20.)), size(px(40.), px(20.)));
                view.transform = TransformationMatrix::unit();
                view.frame = 0;
                view.color = rgba(0xaa4488ff).into();
                match case {
                    0 => view.color.a = 0.,
                    1 => view.bounds.size.width = px(0.),
                    2 => {
                        view.transform = TransformationMatrix::unit()
                            .translate(point(px(-1000.), px(0.)).scale(window.scale_factor()))
                    }
                    3 => view.frame = 1,
                    4 => view.transform.translation[0] = f32::NAN,
                    5 => view.transform.rotation_scale[0][0] = f32::MAX,
                    _ => unreachable!(),
                }
                cx.notify();
            });
            window.draw(cx).clear(cx);
            assert!(
                window.painted_monochrome_sprites().is_empty(),
                "case={case}"
            );
            assert!(
                !window.has_image_mask_atlas_entry(&image, 0),
                "case={case} must not allocate"
            );
        });
        assert_eq!(result.borrow().is_err(), case >= 3, "case={case}");
    }
}

#[test]
fn transformed_mask_and_subpixel_sprites_are_culled_at_their_painted_position() {
    use gpui::{
        AtlasTextureId, AtlasTile, ContentMask, DevicePixels, MonochromeSprite, ScaledPixels,
        Scene, SubpixelSprite, TileId,
    };
    let bounds = |x, y, w, h| {
        Bounds::new(
            point(ScaledPixels(x), ScaledPixels(y)),
            size(ScaledPixels(w), ScaledPixels(h)),
        )
    };
    for subpixel in [false, true] {
        for (transform, visible) in [
            (TransformationMatrix::unit(), false),
            (
                TransformationMatrix::unit().translate(point(ScaledPixels(20.), ScaledPixels(0.))),
                true,
            ),
            (
                TransformationMatrix::unit().translate(point(ScaledPixels(40.), ScaledPixels(0.))),
                false,
            ),
        ] {
            let mut scene = Scene::default();
            let original = bounds(-20., 0., 10., 10.);
            let content_mask = ContentMask {
                bounds: bounds(0., 0., 10., 10.),
            };
            let tile = AtlasTile {
                texture_id: AtlasTextureId {
                    index: 0,
                    kind: if subpixel {
                        AtlasTextureKind::Subpixel
                    } else {
                        AtlasTextureKind::Monochrome
                    },
                },
                tile_id: TileId(0),
                padding: 0,
                bounds: Bounds::new(
                    point(DevicePixels(0), DevicePixels(0)),
                    size(DevicePixels(10), DevicePixels(10)),
                ),
            };
            if subpixel {
                scene.insert_primitive(SubpixelSprite {
                    order: 0,
                    pad: 0,
                    bounds: original,
                    content_mask,
                    color: rgba(0xffffffff).into(),
                    tile,
                    transformation: transform,
                });
                assert_eq!(scene.subpixel_sprites.len(), usize::from(visible));
            } else {
                scene.insert_primitive(MonochromeSprite {
                    order: 0,
                    pad: 0,
                    bounds: original,
                    content_mask,
                    color: rgba(0xffffffff).into(),
                    tile,
                    transformation: transform,
                });
                assert_eq!(scene.monochrome_sprites.len(), usize::from(visible));
            }
        }
    }
}
