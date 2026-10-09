# Image fixture: a small encoded PNM gradient

[image_samples.ml](image_samples.ml) and its [interface](image_samples.mli) export
one `string`, `gradient_pnm`. It contains an encoded binary RGB image, not UTF-8 text,
a file path, decoded pixels or a native image handle. This pure Core support module
creates no Bonsai state, Eio producer, window or native registration.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Images & icons**, then **Show raster gradient**, or **Markdown & code →
Image alternatives** to see the same fixture used by different readers. It has no
independent executable, external asset prerequisite or diagnostic launch flag.
Follow the [Assets](assets_page.md) and [Documents](documents_page.md) walkthroughs
for runtime/toolchain prerequisites, fit controls, ownership and platform limits.
This documentation review does not add decode, pixel or GUI acceptance evidence.

Read `gradient_pnm` from top to bottom. `Buffer.create` allocates a local builder
sized for `96 * 48 * 3` pixel bytes plus header space. `Buffer.add_string` writes:

```text
P6
96 48
255
```

P6 declares a binary RGB PNM image, 96 pixels wide by 48 high, with channel maximum
255. Nested loops emit each row's pixels: y=0–47 outside, x=0–95 inside. The three
integer channel expressions are red `80 + x * 150 / 95`, green
`120 + y * 100 / 47` and blue `210 - x * 90 / 95`. Red increases left-to-right,
green increases across rows, blue decreases left-to-right. Integer division gives
discrete steps; corners span RGB (80,120,210) to (230,220,120). All values stay within
0–255, allowing `Char.of_int_exn` to append exactly one byte per channel. `List.iter`
preserves red/green/blue order.

The 13-byte header plus 13,824 bytes of pixels produce 13,837 encoded bytes.
`Buffer.contents` yields the shared immutable string; the local mutable builder
escapes nowhere. The value is evaluated once when the module initializes, not
regenerated on every Bonsai view update. This small fixture does no file/network
I/O and requires no cleanup. NUL/non-UTF-8 bytes are valid encoded asset payloads.

## How callers acquire an image from the bytes

The [Assets page](assets_page.ml) calls `Asset.Source.of_bytes ~format:Pnm
Image_samples.gradient_pnm`, then evaluates `Gpuio_eio.Asset.register` in its page
scope. Encoded-source construction checks basic size/format metadata without
native decode. Registration publishes the bytes; `V.image` later binds its borrowed
handle and emits Loading/Ready/Failed from native decode. Toggling from SVG to
raster changes the handle in a derived view; it does not rebuild this gradient.
The page's Ready metadata should report 96×48 only if actual decode succeeds.

The [Documents page](documents_page.ml) separately registers the same bytes in its
own visit scope, mapping `asset://prism` to that registration in its Markdown/HTML
image configuration. The URL is an explicit local lookup, not a remote fetch.
Bytes can be reused between consumers; a native handle remains specific to its
owning application/scope. Source/handle contracts are in
[asset.mli](../../lib/core/asset.mli) and
[Eio asset registration](../../lib/eio/asset.mli).

Other callers are [avatar_preview.ml](avatar_preview.ml),
[attachment_preview.ml](attachment_preview.ml) and [empty_preview.ml](empty_preview.ml).
They reuse this encoded fixture as ordinary asset input; their independent component
behavior and resources remain separate documentation groups, not reviewed by this row.

To make a different-sized gradient, update header dimensions, loop endpoints,
capacity and interpolation denominators together. Keep exactly width×height×3
binary channel bytes after the header and channel values within byte bounds.
A dimension of one needs special handling to avoid division by zero. Declare Pnm
when constructing the asset source rather than assuming filename/content sniffing;
malformed pixel content can pass opaque source construction and fail natively.
