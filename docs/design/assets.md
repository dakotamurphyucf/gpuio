# Images, SVG and asset ownership

OCH-11 implementation in progress. `Gpuio.Asset.Format` and `Asset.Source` plus
matching Rust source descriptors are implemented. They preserve opaque encoded
bytes, reject empty or greater-than-16-MiB inputs, and keep diagnostics bounded.
They do not register, decode or display assets yet. No asset capability bit is
advertised by this checkpoint. Core expect tests, Rust source-bound tests, protocol
Clippy and full Dune build/tests/format pass locally on macOS.

## Interface direction

Keep file/network acquisition in explicit Eio operations. Pure view constructors
must not fetch URLs, read files or decode images during Bonsai stabilization.
A source declares one of the pinned GPUI format families: PNG, JPEG, WebP, GIF,
SVG, BMP, TIFF, ICO or PNM. Declaring a format is not successful content validation;
malformed, unsupported codec variants and decoded-size limits need typed native
errors. `Source.of_bytes ~format data` accepts binary data, including NUL.

The next layer should register a source with the application runtime under an
explicit scope and return a generational asset handle. Image/icon views refer to
that handle, so unrelated state changes do not resend megabytes of encoded data.
Support deliberate release and scope cleanup; define what happens to mounted
views and later uses of a released handle before implementing the public handle
interface. Keep loading/ready/failure states explicit, including cancellation and
late-result suppression. A window must remain usable if an image fails to decode.

Image views need meaningful or explicitly decorative accessibility semantics,
contain/cover/fill-style fitting, clipping and the existing layout/style vocabulary.
Keep full-color SVG images distinct from monochrome icons tinted by the current
foreground/theme color. Size and display-scale changes must select appropriately
rasterized SVG output without forcing OCaml commits on each native paint.

## Transport and bounds

The existing bridge envelope is limited to 1 MiB. Asset uploads therefore need
bounded chunks, not a larger inline image field on every view transaction. Preserve
that envelope limit. Register/upload/finish/abort operations must validate exact
asset generation, byte counts and chunk ordering; expose an asset to rendering
only after a complete upload. Cancellation, malformed uploads, window/app disposal
and resource-budget rejection must reclaim staging memory. Incomplete data must
never be treated as a valid prefix image.

The 16-MiB encoded-source bound is separate from aggregate upload/registered-source,
decoded-pixel, animation-frame, worker and GPU-cache budgets. Specify concrete
limits before native decoding. Reserve decoded memory before allocation; use the
pinned decoder's limits and frame iteration bounds, rather than accepting a small
encoded file as proof of a small decoded image. Bound queued/running jobs as well
as completed cache entries. A cancelled synchronous decoder can finish in its
worker, but its late result must not attach to a disposed or replaced owner.

## Pinned-source findings

Inspected GPUI revision `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b` locally:

- `platform.rs`: `ImageFormat` contains the nine families above. `Image::from_bytes`
  constructs an encoded value; `to_image_data` performs decoding. The GIF path
  iterates all frames, so it is not itself an aggregate animation-memory budget.
- `elements/img.rs`: image sources can be resources, encoded `Image`, decoded
  `RenderImage`, or a custom loader. Passing file/URL resources would bypass the
  explicit Eio acquisition contract. The custom/decoded paths allow native ownership
  without adding a GPUI fork.
- `elements/svg.rs`: `.data` copies and hashes bytes for a path-keyed monochrome
  SVG raster in the sprite atlas. Calling it repeatedly is not an asset-lifetime
  strategy, and the inspected public window API has no corresponding SVG eviction
  method.
- `window.rs`: `drop_image(Arc<RenderImage>)` explicitly removes every image frame
  from the window's atlas. Prefer decoded image ownership with tracked per-window
  uploads and explicit eviction/disposal for bounded caching.
- `svg_renderer.rs`: GPUI exposes parsing and rasterization separately, but its
  options include automatic font/resource resolution. Audit SVG embedded/external
  resource resolution before adopting that parser directly. SVG content must not
  silently open application files or fetch network resources outside Eio.
- Locked dependencies already include `image` 0.25.10 and `resvg`/`usvg` 0.46.0.
  Reuse those exact versions if direct decoder control is needed; do not introduce
  a second graphics/codec dependency closure.

Decoded caches should reuse immutable source data, have explicit byte/entry limits,
and evict native atlas entries as well as CPU buffers. A content hash alone is not
proof of equality or ownership. Reusing asset slots must not allow late uploads or
worker completions to affect a new generation. Keep the exact upload/handle/cache
interfaces provisional until the resource-lifetime contract is implemented and
validated.

## Required next evidence

Implement and validate chunked registration, scope cancellation, generational
handles, native decode limits and errors, image/icon views and accessibility,
shared/cache lifetime cleanup, theme/scale rerasterization and animation behavior.
Exercise actual macOS rendering, scale/theme changes, repeated replacement/unmount,
late completion after disposal, malformed/oversized data, and bounded cache eviction.
Verify independent OCaml/Rust protocol fixtures and Linux builds/unit tests before
merging the completed OCH-11 scope. Pure source-descriptor tests are not evidence
of image rendering or a finished asset subsystem.
