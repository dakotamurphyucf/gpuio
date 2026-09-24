use super::*;
use std::io::Write;
fn source(body: &str) -> Source {
    Source::new(Format::Svg, format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4" viewBox="0 0 4 4">{body}</svg>"#).into_bytes()).unwrap()
}
fn pixels(decoded: &Decoded) -> &[u8] {
    decoded.image.as_bytes(0).unwrap()
}
fn at(decoded: &Decoded, x: usize, y: usize) -> &[u8] {
    let width = u32::from(decoded.image.size(0).width) as usize;
    &pixels(decoded)[(y * width + x) * 4..(y * width + x + 1) * 4]
}
fn uri(mime: &str, data: &[u8]) -> String {
    let escaped: String = data.iter().map(|byte| format!("%{byte:02X}")).collect();
    format!("data:{mime},{escaped}")
}
#[test]
fn colors_alpha_exact_sizes_and_icon_tints() {
    let svg = source(r##"<rect width="4" height="4" fill="#ff0000" fill-opacity="0.5"/>"##);
    let natural = render(&svg, Request::default()).unwrap();
    assert_eq!(natural.pixel_bytes, 64);
    assert_eq!(natural.frame_count, 1);
    assert_eq!(at(&natural, 2, 2), [0, 0, 255, 128]);
    let sized = render(
        &svg,
        Request {
            size: Size::Exact(RasterSize::new(8, 4).unwrap()),
            tint: Some(0x12345680),
            fit: Fit::Fill,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(sized.pixel_bytes, 128);
    assert_eq!(u32::from(sized.image.size(0).width), 8);
    assert_eq!(at(&sized, 3, 2), [0x56, 0x34, 0x12, 64]);
    assert_eq!(RasterSize::new(0, 4), Err(Error::InvalidData));
    assert_eq!(RasterSize::new(16385, 1), Err(Error::ResourceLimit));
    assert_eq!(RasterSize::new(4096, 4097), Err(Error::ResourceLimit));
    assert!(RasterSize::new(4096, 4096).is_ok());
}
#[test]
fn gradient_clipping_and_internal_use_render() {
    let svg = source(
        r##"<defs><linearGradient id="paint"><stop stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient><clipPath id="clip"><rect width="4" height="2"/></clipPath><rect id="shape" width="4" height="4" fill="url(#paint)"/></defs><use href="#shape" clip-path="url(#clip)"/>"##,
    );
    let decoded = render(&svg, Request::default()).unwrap();
    assert!(at(&decoded, 0, 0)[2] > at(&decoded, 3, 0)[2]);
    assert!(at(&decoded, 0, 0)[0] < at(&decoded, 3, 0)[0]);
    assert_eq!(at(&decoded, 1, 0)[3], 255);
    assert_eq!(at(&decoded, 1, 3)[3], 0);
}
#[test]
fn embedded_rasters_and_nested_svg_are_validated_not_silently_omitted() {
    let rgba = [200u8, 100, 50, 255].repeat(16);
    let mut png = vec![];
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&rgba, 4, 4, image::ExtendedColorType::Rgba8)
        .unwrap();
    let child = source(&format!(
        r#"<image width="4" height="4" href="{}"/>"#,
        uri("image/png", &png)
    ));
    let parent = source(&format!(
        r#"<image width="4" height="4" href="{}"/>"#,
        uri("image/svg+xml", child.as_bytes())
    ));
    let decoded = render(&parent, Request::default()).unwrap();
    assert_eq!(at(&decoded, 2, 2), [50, 100, 200, 255]);
    let corrupt = source(&format!(
        r#"<image width="4" height="4" href="{}"/>"#,
        uri("image/png", b"corrupt")
    ));
    assert!(matches!(
        render(&corrupt, Request::default()),
        Err(Error::InvalidData)
    ));
    for href in [
        "/tmp/image.png",
        "relative.png",
        "https://example.invalid/image.png",
    ] {
        let svg = source(&format!(r#"<image width="4" height="4" href="{href}"/>"#));
        assert!(matches!(
            render(&svg, Request::default()),
            Err(Error::Unsupported)
        ));
    }
    let child = source(r#"<image width="4" height="4" href="relative.png"/>"#);
    let parent = source(&format!(
        r#"<image width="4" height="4" href="{}"/>"#,
        uri("image/svg+xml", child.as_bytes())
    ));
    assert!(matches!(
        render(&parent, Request::default()),
        Err(Error::Unsupported)
    ));
}
#[test]
fn svgz_and_document_limits_reject_before_raster_retention() {
    let svg = source(r#"<rect width="4" height="4" fill="blue"/>"#);
    let mut zip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    zip.write_all(svg.as_bytes()).unwrap();
    let compressed = Source::new(Format::Svg, zip.finish().unwrap()).unwrap();
    assert_eq!(
        pixels(&render(&compressed, Request::default()).unwrap()),
        pixels(&render(&svg, Request::default()).unwrap())
    );
    let mut zip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    let chunk = vec![b' '; 1024 * 1024];
    for _ in 0..33 {
        zip.write_all(&chunk).unwrap();
    }
    let oversized = Source::new(Format::Svg, zip.finish().unwrap()).unwrap();
    assert!(matches!(
        render(&oversized, Request::default()),
        Err(Error::ResourceLimit)
    ));
    let many = source(&"<rect/>".repeat(MAX_NODES as usize));
    assert!(matches!(
        render(&many, Request::default()),
        Err(Error::ResourceLimit)
    ));
    let deep = source(&format!(
        "{}<rect width=\"4\" height=\"4\"/>{}",
        "<g>".repeat(MAX_DEPTH),
        "</g>".repeat(MAX_DEPTH)
    ));
    assert!(matches!(
        render(&deep, Request::default()),
        Err(Error::ResourceLimit)
    ));
    let dtd = Source::new(Format::Svg,br#"<!DOCTYPE svg [<!ENTITY external SYSTEM "file:///tmp/font">]><svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><text>&external;</text></svg>"#.to_vec()).unwrap();
    assert!(matches!(
        render(&dtd, Request::default()),
        Err(Error::InvalidData)
    ));
    let huge = Source::new(
        Format::Svg,
        br#"<svg xmlns="http://www.w3.org/2000/svg" width="16385" height="1"/>"#.to_vec(),
    )
    .unwrap();
    assert!(matches!(
        render(&huge, Request::default()),
        Err(Error::ResourceLimit)
    ));
}
#[test]
fn text_uses_explicit_font_data_and_missing_fonts_fail() {
    let bytes = include_bytes!("../../../vendor/bonsai/examples/font_hosting/font.ttf");
    let mut db = usvg::fontdb::Database::new();
    db.load_font_data(bytes.to_vec());
    assert!(!db.is_empty());
    let svg = Source::new(Format::Svg,br#"<svg xmlns="http://www.w3.org/2000/svg" width="160" height="40"><text x="4" y="28" font-size="24">Hello</text></svg>"#.to_vec()).unwrap();
    let image = render_with_fonts(&svg, Request::default(), Some(Arc::new(db))).unwrap();
    assert!(
        pixels(&image)
            .chunks_exact(4)
            .filter(|pixel| pixel[3] > 0)
            .count()
            > 100
    );
    assert!(matches!(
        render_with_fonts(
            &svg,
            Request::default(),
            Some(Arc::new(usvg::fontdb::Database::new()))
        ),
        Err(Error::Unsupported)
    ));
}

#[test]
fn fit_respects_density_and_cover_allocates_only_the_viewport() {
    let svg = Source::new(Format::Svg,br#"<svg xmlns="http://www.w3.org/2000/svg" width="2" height="4"><rect width="2" height="4" fill="red"/></svg>"#.to_vec()).unwrap();
    for density in [1., 2.] {
        for fit in [
            Fit::Fill,
            Fit::Contain,
            Fit::Cover,
            Fit::ScaleDown,
            Fit::None,
        ] {
            let reference = match fit {
                Fit::Fill => gpui::ObjectFit::Fill,
                Fit::Contain => gpui::ObjectFit::Contain,
                Fit::Cover => gpui::ObjectFit::Cover,
                Fit::ScaleDown => gpui::ObjectFit::ScaleDown,
                Fit::None => gpui::ObjectFit::None,
            };
            let bounds = reference.get_bounds(
                gpui::Bounds::new(
                    gpui::point(gpui::px(0.), gpui::px(0.)),
                    gpui::size(gpui::px(8.), gpui::px(8.)),
                ),
                gpui::size(
                    gpui::DevicePixels::from((2. * density) as u32),
                    gpui::DevicePixels::from((4. * density) as u32),
                ),
            );
            let image = render(
                &svg,
                Request {
                    size: Size::Exact(RasterSize::new(8, 8).unwrap()),
                    density: Density::new(density).unwrap(),
                    fit,
                    tint: None,
                },
            )
            .unwrap();
            assert_eq!(image.pixel_bytes, 256);
            for y in 0..8 {
                for x in 0..8 {
                    let inside = bounds.contains(&gpui::point(
                        gpui::px(x as f32 + 0.5),
                        gpui::px(y as f32 + 0.5),
                    ));
                    assert_eq!(
                        at(&image, x, y)[3],
                        if inside { 255 } else { 0 },
                        "{fit:?} density={density}, pixel={x},{y}"
                    );
                }
            }
        }
    }
    for value in [0., -1., f32::NAN, f32::INFINITY, 17.] {
        assert!(Density::new(value).is_err());
    }
}
