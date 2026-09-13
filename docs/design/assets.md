# Images, SVG and asset ownership

OCH-11 implementation in progress. `Gpuio.Asset.Format` and `Asset.Source` plus
matching Rust source descriptors are implemented. They preserve opaque encoded
bytes, reject empty or greater-than-16-MiB inputs, and keep diagnostics bounded.
The native session owns a bounded encoded registry, connected through correlated
wire requests and the raw `Gpuio_eio.App.Expert.asset` effect. Capability
`CAP_ASSETS` (2097152; aggregate mask 4194303) means encoded registration only.
Scoped public ownership is now connected through `Gpuio_eio.Asset.register`.
A native raster decoder and decoded-cache/work-ticket controller are implemented
and tested independently; host scheduling, atlas cleanup, SVG rasterization and
image/icon views remain to integrate.
Independent OCaml/Rust fixtures, Rust workspace/Clippy, full Dune checks and an
actual windowless >2-MiB FFI upload pass locally on macOS.

## Interface direction

Keep file/network acquisition in explicit Eio operations. Pure view constructors
must not fetch URLs, read files or decode images during Bonsai stabilization.
A source declares one of the pinned GPUI format families: PNG, JPEG, WebP, GIF,
SVG, BMP, TIFF, ICO or PNM. Declaring a format is not successful content validation;
malformed, unsupported codec variants and decoded-size limits need typed native
errors. `Source.of_bytes ~format data` accepts binary data, including NUL.

`Gpuio_eio.Asset.register app ~scope source` registers a source with the
application runtime under an explicit scope and returns an encoded registration.
Its native generational ID remains behind an Expert interface until the pure
image/icon view integration is implemented. Image/icon views refer to
that handle, so unrelated state changes do not resend megabytes of encoded data.
Support deliberate release and scope cleanup. The native registry now defines
release as retiring acquisition: existing leases remain readable, while new uses
of the retired handle fail. Image nodes must retain their own leases to preserve
mounted content; a new/replaced node must acquire a live registration. Keep loading/ready/failure states explicit, including cancellation and
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
worker completions to affect a new generation. The native registration and scoped adapter below are implemented. Pure view
handles and decoded-cache interfaces still require integration.

## Native registration state

`rust/native/src/asset_store.rs` is owned by `Session`, behind its negotiated/live
state check. It issues `ResourceId` slot/generation pairs and never wraps an
exhausted generation. Closing one window does not implicitly release application-
scope assets; closing the application terminates registration and cancels staging.
The wire/runtime adapter must attach registrations to explicit logical scopes.

Current encoded bounds:

| Resource | Limit |
| --- | --- |
| One encoded source | 16 MiB |
| One chunk | 256 KiB |
| Concurrent uploads | 8 |
| Slots, including retained/tombstone metadata | 1024 |
| Reserved staging + registered + retired-but-read encoded bytes | 64 MiB |

`begin` reserves the entire declared encoded length before accepting chunks.
`append` accepts nonempty, sequential chunks within both chunk and declared-size
bounds. Semantically malformed decoded chunks abort that generation and reclaim staging; stale handles
cannot abort a later generation. Structurally invalid wire requests (including
oversized chunks) are rejected before native admission and cannot mutate or abort
a registration; the caller still owns its release. `finish` publishes only an exact-length encoded
source. Finishing a prefix aborts it. Publication does not claim successful image
decoding. No view or worker can acquire a staging prefix.

`release` aborts staging or retires an available registration. Repeated cleanup of
the same generation succeeds; older-generation cleanup fails without touching a
reused slot. Existing immutable leases can still be read after retirement, and
continue to count against the encoded budget until their last reader drops them.
This includes decoder jobs and mounted views. A weak retired entry cannot itself
keep the payload alive. Collection before new registration reclaims the quota and
makes the slot reusable with a new generation.

The public adapter must preserve cleanup when a scope is cancelled while `begin`
is awaiting its reply. Discarding that reply with an ordinary suppressed Bonsai
callback would leak a native registration. Keep internal pending-request cleanup
alive until the reply is accounted for, releasing any late-created ID without
running the cancelled user's callback. Later upload cancellation can release its
known ID; FIFO processing and generation checks protect subsequent reuse.

The native store tests cover assembly of a >2-MiB binary source from bounded
chunks, prefix non-publication, malformed/incomplete upload reclamation, stale
operations after reuse, retained-reader quota, terminal shutdown, and entry/upload/
generation exhaustion. A session test covers negotiation, reuse across windows,
retirement accounting and shutdown gating. These are native state-machine tests,
not evidence of FFI uploads, scope cleanup, worker scheduling or GPU rendering.

## Required next evidence

Extend scope cancellation evidence through worker/view integration; implement
pure generational view handles, native decode limits and errors, image/icon views and accessibility,
shared/cache lifetime cleanup, theme/scale rerasterization and animation behavior.
Exercise actual macOS rendering, scale/theme changes, repeated replacement/unmount,
late completion after disposal, malformed/oversized data, and bounded cache eviction.
Verify independent OCaml/Rust protocol fixtures and Linux builds/unit tests before
merging the completed OCH-11 scope. Pure source-descriptor tests are not evidence
of image rendering or a finished asset subsystem.

See [local source/registry evidence](../evidence/assets-och11.md) for current checks
and their limits.

## Encoded upload wire integration

Message tag 8 carries a positive correlation ID and Begin/Append/Finish/Release.
Event tag 24 returns Begun/Ack/Failed under that correlation. Append encodes raw
bytes with a bounded bin_prot string length, preserving NUL and non-UTF-8 bytes;
it does not encode each byte as a bin_prot integer. Each admitted request reserves
one bounded control response. Input pressure cannot coalesce or drop its reply,
and application asset responses do not prevent window-slot reuse.

`App.Expert.asset` bounds raw pending requests to 63, leaving one of 64 lanes
reserved for the scoped adapter, and reports Not_ready before
negotiation, Resource_limit at capacity and Closed during shutdown. This raw API
has no per-scope cancellation contract: its caller must process late Begin replies
and release registrations. Application shutdown closes the native store. Use `Gpuio_eio.Asset` for automatic scoped ownership. Encoded completion does
not validate pixels.

## Scoped OCaml ownership

`Gpuio_eio.Asset.register app ~scope source` is a Bonsai effect returning a typed
encoded registration or Closed/Not_ready/Resource_limit/Invalid_scope/Native_failure.
The source is immutable; acquisition remains explicit Eio I/O. The scope must
share the application's scheduler root. After completion, `release` is idempotent,
and scope cancellation retires the registration automatically. The public result
means encoded publication only; decode errors and image view handles are separate
work. No pure view accepts the Expert native ID yet.

The UI-domain registry bounds live metadata to 1024 registrations, unfinished
uploads to eight, and source bytes held by its queued/uploading states to 64 MiB.
It releases its source reference after the last chunk acknowledgement and clears
its completion callback after delivery. Caller-retained sources/effects are
caller-owned memory. At most one scoped command is in flight; Append adds at most
one 256-KiB copied chunk in the controller, with the existing transport's bounded
encoding copies. Ready registrations hold IDs, not encoded OCaml source bytes.

Cleanup has priority over uploads. Raw Expert traffic cannot consume the reserved
scoped request lane. Native mailbox backpressure keeps commands queued rather than
dropping releases. A transient cleanup admission failure retries; impossible native
response shapes raise through the application cleanup boundary rather than silently
forgetting a potentially live allocation. Application disposal closes both owners.

Cancellation clears the user callback immediately. If Begin is in flight, its
internal entry remains until the native reply supplies the ID for Release. If a
chunk or Finish is in flight, the known ID is retained until that reply and a
subsequent Release acknowledgement. Cancelling before submission requires no
native operation. Entry metadata remains charged until retirement is acknowledged,
so repeatedly cancelling uploads cannot grow an unbounded cleanup queue.

```ocaml
Bonsai.Effect.bind (Gpuio_eio.Asset.register app ~scope source) ~f:(function
  | Error error -> report_asset_registration_error error
  | Ok asset -> remember_asset asset)
```

The last two functions are application callbacks. Release deliberately with
`Gpuio_eio.Asset.release asset`, or let the owning scope end. Registration is
asynchronous; do not perform synchronous waits during Bonsai stabilization.

## Raster decoder checkpoint

`asset_decode::raster` decodes declared PNG/JPEG/WebP/GIF/BMP/TIFF/ICO/PNM sources
in memory. It produces GPUI `RenderImage` frames in BGRA order, applies EXIF
orientation to static images, preserves GIF frame delays and rejects an entire
animation on a bad frame rather than returning a silently shortened sequence.
PNG and WebP currently use the pinned GPUI static-image behavior. SVG has a
separate size/theme-dependent rasterization path still to implement.

Current per-result limits are 16384 pixels per dimension, 64 MiB of RGBA8/BGRA8
pixels across all retained frames and 120 frames. Static native-color-depth output
is checked separately against 128 MiB before `DynamicImage` allocation; dimensions
and final RGBA8 byte counts are checked first. GIF's iterator owns a compositing
canvas and can allocate a candidate frame before the aggregate retention check.
Its working buffers must therefore be charged separately from retained frames
by the upcoming worker budget. The decoder result holds no encoded source.

The pinned image library explicitly documents `max_alloc` as best-effort, unlike
its strict dimension limits. Setting 128 MiB does not prove a hard process-RSS
ceiling or account for codec-internal metadata/allocator overhead. Do not describe
this helper alone as a globally bounded cache/worker pool. The controller below
reserves retained output before dispatch; host scheduling/atlas cleanup are not
connected yet. Per-result checks prevent
publishing excessive pixel/frame output; concurrency, retained cache ownership,
transient work and GPU atlas uploads need their own aggregate bounds.

Inspected GPUI macOS `gpui_apple::MetalRenderer::new_internal`: it creates a new
`MetalAtlas` per renderer. `Window::drop_image` removes all frame keys for that
image from the window's atlas. The pinned Linux X11/Wayland
`WgpuRenderer::new` likewise creates `WgpuAtlas::from_context` for its window.
This supports per-window eviction on both intended backends; actual atlas cleanup
validation still requires host/view integration. GPUI's SVG conversion
also unpremultiplies tiny-skia pixels before BGRA conversion; preserve that
alpha behavior in our SVG path.

## Decoded cache and transferable work ownership

`asset_cache::Cache` is a native UI-owned controller. Its handles are local `Rc`
leases; its Work/Completion tickets are transferable to the background executor.
It is not yet attached to the application's host pump. Consumers must acquire a
fresh encoded-store lease before requesting a cache handle, so warm pixels cannot
resurrect a retired registration. Mounts sharing one source share a decode/result.
Only mounted handles and running work keep encoded leases; warm pixel entries
hold weak owner references and cannot retain encoded data by themselves.

| Controller resource | Limit |
| --- | --- |
| Live/warm entries | 256 |
| Queued decode requests | 32 |
| Running jobs plus undelivered completions | 2 |
| Ready/retired pixel bytes plus reserved result output | 256 MiB |
| Reservation per admitted job | 64 MiB |
| Retired metadata during admission | 256 |

The scheduler reserves maximum retained output before dispatching a job, then
replaces that reservation with its actual pixel size when publishing the result.
Codec working buffers remain separate and best-effort limited; this table is not
a process-RSS guarantee. Shared pixel references stay charged through weak image
records after eviction, including outstanding paint and atlas-cleanup references.
The host must drain returned evictions and call `Window::drop_image` on every
window that uploaded them. Dropping only a CPU entry is insufficient.

Cleanup chooses least-recently-requested unused entries and retires only enough
to make room. Retired and live entry metadata are independently bounded. During
terminal close, live entries transfer into the existing retired set, keeping the
combined maximum at 512; no new admission is possible. Atlas eviction vectors
remain bounded by that same ownership transfer. Native pixel capacity is released
only after the final image `Arc` drops, not when eviction is merely requested.

Dropping the last mounted handle signals its work's cancellation flag immediately.
Workers check before decoding and before returning output; a synchronous decoder
already running can finish. Queued owners disappear on collection. Each request
has a non-wrapping ticket distinct from the resource ID, and completion additionally
checks the exact ticket allocation identity. Late work cannot update a replacement
request or a different cache with equal numeric IDs. Dropped jobs/completions mark
their reservation abandoned for collection; unexpected decoder panics become typed
worker failures. Closing forbids work, cancels jobs and suppresses late publication.

Host integration must drive queued work on GPUI's background executor, refresh
live consumers on completion/admission failure, drain atlas evictions before reuse,
and join/drain its outstanding tasks during teardown. These requirements remain
open; a standalone controller test is not proof of application scheduling or GPU
memory disposal. SVG will extend cache identity with raster size and icon tint.
