# Public API compatibility and limits

This describes the current experimental implementation, not a stable v1 release
announcement. [Current status](status.md) records open acceptance work. Use the
[starter guide](getting-started.md) for a compiled application and installation
example; older imported architecture/API sketches are design history, not exact
call signatures. Public `.mli` files are the authoritative current interfaces
where present; the [API layer map](api-layers.md) explains generated wrappers,
inferred protocol interfaces and reachable integration helpers.

## Build compatibility

GPUIO has one repository and release train for OCaml, Rust, the native backend and
vendored dependencies. Build all parts from the same checkout and exact lockfiles.
The current baseline is stock OCaml 5.3.0, the Jane Street/Bonsai v0.17 family,
Core, Eio 1.3, Dune 3.24.2 and Rust 1.97.1. OxCaml and an Async application scheduler
are not required. The maintained Bonsai/native patches are build inputs; an
unpatched upstream package is not a tested replacement.

There is no published stable API or binary plugin ABI. Public source signatures,
labels and behavior may change during this experimental phase; recompile consumers
when updating. Do not combine an installed OCaml prefix from one revision with a
native archive from another, even when both report the same protocol epoch.

The [bridge](design/bridge-v1.md) uses encoding family v1 and an exact epoch-3
handshake. Epochs 1–2 and unknown future epochs are rejected. Epoch 3 is still
unpublished and has accumulated paired additions without allocating new capability
bits; the full legacy mask is not evidence that arbitrary epoch-3 builds interoperate.
The epoch is a wire boundary, not the library's release version or an application
persistence format. Rebuild both languages together.

Native component/profile packages use explicit names, versions and schema
fingerprints, checked SDK/source identities and one generated Cargo graph. This
catches incompatible registrations; it does not provide binary compatibility
between independently compiled Rust archives. Upgrade the package codecs, native
factory, manifest and lock together. See [components](design/extensions.md) and
[document profiles](design/document-profiles.md).

## State, units and timing

- Handles and observed snapshots are scoped to generation-checked owners. A
  remounted editor or recycled window slot is a different object. Do not persist
  native IDs or replay a previous process's snapshots/commands.
- Editor selections and source offsets count UTF-8 bytes. Use validated selection
  constructors; character counts, UTF-16 indices and grapheme counts are different.
  UI dimensions and reported geometry use logical pixels unless documented
  otherwise. Bounds are not proof of physical visibility.
- Views/configurations describe desired state. Events, correlated commands,
  source publication, layout and physical presentation are different stages.
  `Window.request_frame` observes a native render callback and can be delayed
  while occluded; it is not an application-data readiness barrier.
- Effect callbacks run on the OCaml UI domain. Native synchronous editing,
  layout, paint and input policy do not call OCaml. Use Eio scopes for asynchronous
  work and inspect each command's typed result, including stale/closed/capacity
  outcomes. Cancellation and window closure retire deliveries according to the
  owning API; they are not interchangeable with successful completion.
- The default shared Bonsai clock wakes at 60 Hz. This is a runtime timer, not a
  request to redraw an idle native window at 60 FPS. `App.run ~tick_hz` changes the
  clock resolution/cost tradeoff; motion and input remain native. See
  [runtime scheduling](design/runtime.md).

Derived `bin_io`, `sexp` or equality support does not imply a stable persistence
schema. Persist application-owned records with explicit versions/migrations;
reconstruct validated GPUIO values and live resources when loading.

## Supported scope and meaningful limits

macOS is the initial functional/release target. Linux compilation, unit/private-bus
and independent-consumer checks remain required, while real Linux desktop
qualification is OCH-47. Windows is outside v1. Read the
[platform policy](platform-release-policy.md) and each OS capability's result type.

Current bounded contracts include 32 simultaneously live windows, ordinary editor
text up to 262144 UTF-8 bytes, and an application-wide limit of 64 pending editor
requests. Those numbers are admission limits, not performance promises. Large
read-only documents use the document source/virtual reader path; a full editable
code editor/LSP is deferred. See [editing](design/native-editor.md),
[documents](design/documents.md) and the [family ledger](catalog/README.md).

Managed lists virtualize active views and native caches, not all application-owned
history or metadata. Use paging for unbounded records. Static native extensions
are trusted compiled code; panic containment is not a security sandbox. Text
projections, arbitrary custom renderers and nested scroll containers have distinct
selection/search/focus obligations. Core library functionality does not imply an
arbitrary plugin satisfies those obligations.

Hot reload, a dynamic plugin ABI, full code-editor/LSP infrastructure, editable
spreadsheet-style grids, general terminal/multimedia engines and comprehensive
docking are outside this release's required scope. Native drawing, animations,
read-only tables/trees/documents, multiple windows and app-declared controls have
implemented APIs, with their exact evidence and gaps in the catalog. Application
authors can compose or extend them without assuming deferred subsystems ship as
ready-made components.

## Resource publication and recovery

`Gpuio_eio.Document`, `Chart` and `Canvas` manage scoped native registrations.
Their `create` effects complete after the initial snapshot is accepted natively;
scope cancellation suppresses late delivery and retires the registration. A
borrowed handle does not extend that lifetime or work in another application.
Creating a resource does not mount a reader, chart or canvas widget.

For subsequent changes, `Ok ()` means local admission of desired state. It does
not promise eventual publication after cancellation or failure. Updates that have
not started uploading can coalesce; the runtime finishes an in-flight publication
before publishing the latest desired snapshot. `source`, `data` and `scene` return
the desired content, which may differ from the content currently rendered.
`is_published` reports acceptance of the latest desired content, not completed
parsing, layout or physical presentation. Inspect `error` as well.

| Resource | Native update failure | Recovery |
| -- | -- | -- |
| Document | Any upload failure retires the registration and clears its retained source. The error remains available. | Preserve content in application state and call `create` again. `reset` cannot revive the old registration. |
| Chart or canvas | A recoverable rejection preserves the prior accepted snapshot and the registration. The rejected desired value remains observable. | Inspect `error`, then explicitly call `set` or `reset` to retry; successful publication clears the error. |
| Chart or canvas | `Closed`, `Stale_handle`, `Native_failure`, or failure to abort a rejected upload retires the registration. | Check `is_released` and create a new registration from application-owned content. |

Local validation/admission errors do not themselves retire a live registration.
For documents, coalescing preserves a terminal snapshot's exact content while the
registration remains live and uploads succeed. It cannot guarantee delivery
through failure or cancellation. For charts and canvases, a successful `reset`
starts a new native resource generation while retaining the registration handle;
chart reset also fences the old selection epoch immediately, before publication.

Current OCaml-side admission limits are:

| Resource | Per-application registry limits | Per-value limits |
| -- | -- | -- |
| Encoded assets | 1,024 registrations; eight pending uploads; 64 MiB pending encoded source bytes | 16 MiB encoded source; decoding and GPU admission have separate limits |
| Documents | 1,024 registrations; 64 MiB charged desired/accepted/upload snapshots | 8 MiB canonical UTF-8 source; `push_bytes` accepts chunks of at most 256 KiB and buffers at most three incomplete scalar bytes |
| Charts | 256 registrations; four staged uploads; 128 MiB charged snapshots and encoded buffers | 16 MiB encoded dataset; additional chart-family limits apply |
| Canvases | 256 registrations; four staged uploads; 64 MiB charged snapshots and encoded buffers | 20,000 items; 4,096 resources; 65,536 path commands; 1 MiB text; 2,048 interactive items; 4 MiB encoded scene |

Document, chart and canvas registries each send one correlated request at a time.
Registry charges are conservative admission accounting, not process RSS or GPU
memory guarantees. Application-held values, decoded native data and rendered
caches have separate lifetimes and costs. Release resources through their scopes;
do not assume an offscreen widget releases application state.

`Text_source` appends share prior chunks and copy at most a 16 KiB tail. Edits and
explicit flattening are O(total bytes); holding old immutable snapshots retains
their shared content. Avoid flattening the entire document for every streamed
chunk. Rejected byte chunks preserve the previous decoder and accept no partial
prefix. An asset's encoded publication likewise does not establish successful
image decoding; observe the consuming view's result.

The authoritative contracts are in [Document](../lib/eio/document.mli),
[Chart](../lib/eio/chart.mli), [Canvas](../lib/eio/canvas.mli),
[Asset](../lib/eio/asset.mli), [Text_source](../lib/core/text_source.mli),
[Chart_data](../lib/core/chart_data.mli) and
[Canvas_scene](../lib/core/canvas_scene.mli). See the scoped
[API review evidence](evidence/api-boundaries-och17.md) for validation and limits.

## Loading and cancellation bounds

Paged collections and search belong to an application/window/conversation scope,
not transient row computations. Their `value` snapshots are read in Bonsai graphs;
create and mutate the controllers from initialization/effects on the UI domain.
Load/search closures receive explicit Eio capabilities and must not block that
domain with synchronous work.

| Controller | Concurrency and cancellation |
| -- | -- |
| `List_paging` | Two lazy reusable workers, including cancellation cleanup; at most the latest request per boundary waits. Idle workers keep their Scope task slots until closure. |
| `Table_paging` | Two lazy reusable workers with the same cleanup bound; requests capture immutable query settings. |
| `Tree_loading` | Up to four lazy reusable workers; obsolete node/generation results cannot attach children to a replacement node. |
| `List_search` | One current producer; cancelled fibers may still unwind concurrently and remain charged to the shared Scope task quota. This is not the pager worker-pool bound. |

List/table producer results from a reset query are discarded without publishing
another snapshot or invoking `on_change` merely to return the old worker slot.
A queued current request can then start; its result or admission failure still
publishes normally. A retired worker returning its slot is not itself an
application state change.

Logical cancellation immediately fences obsolete results. Physical cancellation
is cooperative: protected cleanup can delay reuse, and these APIs do not forcibly
terminate a blocking foreign call. List/table workers wait without polling. These
limits do not bound user payload bytes or automatically evict loaded history.

Ordinary paging requests do not retry Failed boundaries; use explicit retry.
Immediate list worker-admission failure returns an error and publishes Failed.
A previously admitted load can fail later, so applications must also observe
snapshots. Closing a list/table pager retains readable data and publishes
cancelled boundaries to its reactive value without invoking `on_change`. A Ready
boundary in that final snapshot is not permission to request from a closed pager.
Search instead publishes its explicit Closed status. Search retains prior results
while work is pending or failed, marked stale; disable interaction with stale
results unless the application intentionally allows it.

See [list paging](../lib/eio/list_paging.mli),
[table paging](../lib/eio/table_paging.mli),
[tree loading](../lib/eio/tree_loading.mli),
[search](../lib/eio/list_search.mli), and the
[review evidence](evidence/api-boundaries-och17.md#paging-ownership-repair--2026-10-08).

## Scope cleanup failures

`Scope.cancel` is idempotent and remains synchronous on the owning UI domain.
It retires scope membership/accounting before callbacks can reenter cancellation,
attempts every descendant and producer cancellation, then runs the remaining
registered cleanup callbacks. A child can still unregister a pending ancestor
cleanup before that ancestor claims its callback batch. Queued task/stream
results from cancelled scopes remain suppressed; unrelated scopes remain usable.

Cleanup callbacks must not raise, block or perform I/O. If a callback violates
the non-raising contract, cancellation still attempts independent remaining work
and re-raises the first exception with its original backtrace. This does not make
a blocked callback safe or force a producer's protected cleanup to finish. Task
counts stay charged until producer fibers unwind. See the
[regression evidence](evidence/scope-cancellation-och17.md).

`App.Window.close` and `App.shutdown` also attempt the remaining cleanup and
queue native close/shutdown work when cleanup raises, then propagate the first
failure. Logical closing/stopping fences reentrant calls. An opening window queues
its native close after the opening acknowledgment; returning from either API is
not a synchronous OS-destruction barrier. See the
[force-close regression evidence](evidence/force-close-cleanup-och17.md).

## Desktop service lifetimes

Desktop services run through the application UI domain. Constructing a returned
Bonsai effect does not submit its request; handling the effect does. Unmounting
the component that requested OS work does not undo work already submitted.
Use current application state or a generation token to suppress obsolete feedback.

| Operation | Ownership and admission | Completion boundary |
| -- | -- | -- |
| File picker and file-dialog capability probe | Exact window generation; one pending request per window, shared between probes/open/save | User cancellation is `Ok None`; closing/shutdown is `Closed`. A save result chooses a path without creating or reserving a file. |
| Desktop capabilities, link intake, activation, file opening/reveal, scheme registration, scrollbar preference and clipboard write | One shared application lane of 16 pending requests; no window is needed for these requests | Inspect the operation's typed result. Acceptance need not mean visible OS presentation, recipient consumption or clipboard persistence. |
| Notification operations and event intake | Separate application lane of 16 pending requests; terminal service close bypasses saturation | A successful post means OS acceptance, not visible delivery. Closing the service disables it for that application lifetime. |

Desktop link and notification receivers wait for `ready`, await each handler's
effect, and yield between events. Long-running handlers backpressure their bounded
native queues. `retry` requests intake explicitly after failure; no automatic
polling is installed. Closing a receiver drops queued delivery but does not cancel
an already-running application effect. Desktop receivers can be replaced after
close; notification service closure is terminal. These are different contracts.

Application teardown rejects new requests, detaches pending application request
maps, and completes their callbacks with `Closed`. It attempts all independent
completions even if one raises, then propagates the first cleanup failure with its
backtrace. Duplicate/late native responses cannot complete detached requests
again. This also applies to pending asset/document/chart/canvas transport requests;
scope cancellation can still suppress higher-level resource creation deliveries.
Cleanup does not convert an application exception into a successful operation or
roll back an OS side effect.

See [file dialogs](../lib/eio/file_dialog.mli),
[desktop integration](../lib/eio/desktop.mli),
[clipboard](../lib/eio/clipboard.mli),
[notifications](../lib/eio/notification.mli) and the
[teardown regression evidence](evidence/application-teardown-och17.md).

## Release qualification

A successful source/consumer build proves compilation and linking in that tested
environment. TestPlatform checks do not establish real IME, clipboard, VoiceOver,
GPU timing or clean-machine distribution. Local ad-hoc application signatures are
not Developer ID/notarization. The source-library preview gates are recorded in the
[closeout checklist](milestone-07-closeout.md). Broader accessibility and signed-app
qualification follow in [OCH-164](https://linear.app/ochat/issue/OCH-164); full Linux
desktop qualification follows in OCH-47. Those claims remain unqualified, even
when the source preview is available. [Distribution](distribution.md) describes
the separate requirements for shipping application binaries.
