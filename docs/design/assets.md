# Images, SVG and asset ownership

OCH-11 implementation in progress. Encoded registration, scoped public ownership,
bounded image scheduling, raster/SVG views and foreground-tinted icons are connected.
`CAP_ASSETS` (2097152) means encoded registration; `CAP_IMAGES` (4194304) adds image
views and state events; `CAP_SVG` (8388608) adds SVG and icon rendering. The aggregate
mask is 16777215. This is not completion of OCH-11. Common icon/control composition still requires
acceptance checks.

## Interface direction

Keep file/network acquisition in explicit Eio operations. Pure view constructors
must not fetch URLs, read files or decode images during Bonsai stabilization.
A source declares one of the pinned GPUI format families: PNG, JPEG, WebP, GIF,
SVG, BMP, TIFF, ICO or PNM. Declaring a format is not successful content validation;
malformed, unsupported codec variants and decoded-size limits need typed native
errors. `Source.of_bytes ~format data` accepts binary data, including NUL.

`Gpuio_eio.Asset.register app ~scope source` registers a source with the
application runtime under an explicit scope and returns an encoded registration.
`Gpuio_eio.Asset.handle registration` returns an immutable `Gpuio.Asset.Handle.t`.
Image views refer to that handle, so unrelated state changes do not resend megabytes of encoded data.
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
separate size/theme-dependent rasterization path described below.

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
The `image_host` adapter below connects it to the application host pump. Consumers must acquire a
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
and join/drain its outstanding tasks during teardown. The adapter below implements these host requirements and adds native GPU readback
evidence. Public image views now bind to it. SVG extends cache identity with
physical viewport size, device density, fit and optional icon tint.

## GPUI host scheduling and atlas ownership

`image_host` is initialized with the native application. A mounted Rust binding
requests an encoded lease once and retains its cache handle. The adapter launches
admitted Work on GPUI's background executor, receives bounded completions, and
refreshes registered windows. A shared binding observed while still loading
registers its destination window before returning, so it receives the completion
refresh. Loading/ready/typed error observation is available
to native views without blocking paint. There is no idle decode timer. Public
OCaml image/icon constructors and their protocol/view integration are still next.

The host accounts for each image once per window before returning pixels to
`img`/`paint_image`. It limits ownership to 32 windows, 1024 image/window copies,
4096 animation-frame keys and 256 MiB of logical uploaded pixel footprint. It
reserves an animation's full footprint, including frames that GPUI uploads later.
These are application image bounds, not a claim to measure GPU allocator pages,
fragmentation, command buffers, fonts or other native framework allocations.

Cache eviction removes each image from every window that admitted it and releases
its accounting references. Closing a window drops that window's records; its
renderer owns the destroyed atlas. The pump retries work after performing requested
LRU evictions. It does not hold the service RefCell borrow while updating windows.
Window tracking and deferred/completion callbacks do not own window entities.

Each worker publishes its completion before closing a completion fence. Foreground
processing waits for the fence before admitting replacement work. The service
owns a result waiting for that fence, so shutdown can reclaim it without needing
the foreground receiver task to run. Ordinary bridge shutdown awaits workers
asynchronously. Unconditional GPUI quit drains independent worker jobs synchronously
before GPUI destroys windows, then performs atlas cleanup. Closing the result
channel prevents a late worker from waiting for UI delivery. Cache identities and
cancelled handles continue to reject late publication.

The optional `native-image-tests` feature enables pinned GPUI platform test support
for GPU readback. It adds test-only dependency packages without changing existing
locked package versions/sources; production features remain unchanged. CI is
configured to compile/lint the target on both platforms and execute its GPU test
on macOS. Hosted results are still deferred until the full local OCH-11 scope.

## Declarative image contract

```ocaml
let config =
  Gpuio.Image.Config.create
    ~asset:(Gpuio_eio.Asset.handle registered)
    ~description:(Gpuio.Image.Description.label "Attachment preview" |> Or_error.ok_exn)
    ~fit:Contain
    ()
in
Gpuio_bonsai.View.image
  ~style:(Gpuio.Style.create_exn
    [ Width (Gpuio.Length.px_exn 240.); Height (Gpuio.Length.px_exn 180.) ])
  ~on_change:(fun state -> (* observe asynchronously *) on_image_state state)
  config
```

The pure configuration retains a small handle containing application allocation
identity, native slot/generation and declared format. It does not retain the
registration, scope, runtime or encoded bytes. Equality includes application
identity. The owning registry passes that identity through the window driver to
the reconciler; a foreign handle becomes an `Unavailable Wrong_application` wire
source, without sending its numeric ID into another application's store.

Descriptions are mandatory: `Description.decorative` deliberately omits the image
role; `Description.label` supplies bounded, nonblank UTF-8. A meaningful image uses
the native Image role, keeps its accessible identity while loading/failed, and has
no click/focus action merely because an `on_change` observer is installed. Use a
button or pointer region around an image for explicit interaction.

`Fit` maps to GPUI Fill, Contain (default), Cover, Scale_down and None. Ordinary
styles control logical layout and native state styling; when size is unspecified,
the decoded pixel dimensions provide GPUI's natural raster size. Prefer explicit
logical dimensions for application layout. Animated GIF playback remains native;
the GPUI image-element identity includes the decoded image ID so replacement starts
its own animation state. There is no OCaml frame-by-frame image transport.

Native image bindings acquire their encoded lease during accepted tree application,
before a later release request can arrive, including before their first paint.
Restyling or changing a description keeps that lease when the source is unchanged.
Changing source or remounting requires a fresh store acquisition; retired sources
fail locally with `Released`. Unmount/close drops the mounted handle. A registration
handle remains an immutable value after release, but does not extend registration
lifetime. An initial admission/decode failure persists for that mount/source; a
new key can explicitly request a fresh binding where retry is appropriate.

`on_change` observes `Loading`, `Ready metadata` or `Failed error` asynchronously.
An already decoded source can become Ready without a separately delivered Loading.
Unchanged status is not emitted every render; attaching a new observer gets the
current observed state. Source replacement rotates callback identity so old queued
results cannot be delivered to the replacement. Native deferred delivery additionally
checks current node/source/handler/revision. Errors are Wrong_application, Released,
Invalid_data, Unsupported, Resource_limit and Native_failure. Metadata exposes pixel
width/height and frame count, validates 1..16384 dimensions, 1..120 frames and a
64-MiB full-frame BGRA footprint on both sides. Messages remain under the existing
transport bounds; state observers use the existing bounded event mailbox.

Wire additions are append-only Kind 22, operation 26 (`Set_image`) and event 25
(`Image_state`). Independent OCaml/Rust fixtures fix variant order, optional
accessibility labels, all fit/error variants, and metadata representation.

The runnable `examples/images` example uses scoped Eio registration and Bonsai
views; its self-test waits for a native Ready observation, restyles after retirement,
and checks that a newly keyed mount reports Released. Native GPU/AX evidence and
SVG integration are recorded in [asset evidence](../evidence/assets-och11.md).

## SVG and monochrome icons

`View.image` accepts an SVG handle and preserves its full color. `View.icon` takes
`Icon.Config.t`, whose validated constructor requires an SVG handle. Both require
an explicit meaningful/decorative `Image.Description`, support the five image fits,
and report `Image.State`. Icons tint the rendered alpha mask with the inherited
native foreground, including theme and native state styles; they do not reinterpret
the document's individual fill/stroke colors. Partial SVG and foreground alpha multiply.
The protocol adds Icon kind 23; image configuration and state layouts are shared.
Switching an image node to an icon remounts it through ordinary keyed reconciliation.

A native canvas measures logical bounds and the current window scale on paint.
The cache key includes physical viewport width/height, device density, fit and tint.
Changes schedule background rasterization; paint only uploads ready pixels. No OCaml
transaction is needed for native resize or hover tint. A pending replacement keeps
the previous bitmap visible; initial icons suppress the untinted bootstrap image.
Repeated identical requests share work. Resampling retains the mounted source lease,
including after registration retirement. New mounts still require a live registration.
Paint closures hold weak bindings, so stale frames cannot extend mounted ownership.

The first decode establishes natural dimensions at density 1. Ready metadata and
natural layout keep those dimensions when viewport variants replace the pixels.
Natural and viewport outputs obey the existing positive dimensions <=16384 and
64-MiB retained BGRA limit; an oversized natural SVG fails even when a proposed
layout would shrink it. Cover rasterizes into the viewport instead of retaining
an oversized fitted bitmap. Fit semantics match pinned GPUI, including top-left
placement for None and centered Scale_down. Device density must be finite in (0,16].
Resampling failures use the local image error channel and may leave old pixels visible.

The decoder uses pinned resvg/usvg 0.46 with custom XML and resource resolution:

- SVGZ expansion and aggregate nested XML are limited to 32 MiB. DTDs are disabled;
  XML is limited to 100,000 nodes and depth 128, with expanded output trees also checked.
- Embedded images are limited to 32, nesting 8, aggregate decoded pixels 64 MiB and
  normalized PNG bytes 64 MiB. PNG/JPEG/GIF/WebP are validated with the raster decoder
  and embedded as a static first frame; nested SVGs use the same resource policy.
- Document image references never open files or URLs. External references fail
  Unsupported; invalid embedded codec data fails the enclosing image. Applications
  must acquire bytes through Eio and embed/register them explicitly.
- SVG text lazily discovers native system fonts, separately from pixel-cache
  accounting. A missing font database produces Unsupported. App-bundled GPUI fonts
  are not yet synchronized into this decoder, and system font discovery is cached.
  This is native rendering work; the pure OCaml view API still performs no I/O.

These limits bound input, retained output and job concurrency, not process RSS.
Parser, font discovery, filters, intermediate render layers and codec working buffers
have additional allocations. The worker shutdown contract requires no UI callbacks
or waits for UI progress; it does not assume workers perform only CPU operations.
SVG scripting/animation and browser DOM/CSS behavior are not provided by resvg.
Use the separate declarative motion API for native view animation (OCH-12).

### Native image corner styling

The image root captures its computed GPUI corner radii at paint time, using the
native hitbox and interaction state. Raster images receive these radii through a
small Element wrapper; SVG/icon canvases pass them to `Window.paint_image`.
This is necessary because GPUI Div overflow masks are rectangular and image
sprites paint their own corners. The wrapper forwards layout, element identity and
accessibility; raster images still use GPUI's existing animation/frame lifecycle.
The shared per-frame value contains only four radii, not an image/source lease.
GPUI retains its own fitting and radius clamping semantics for the visible image.
