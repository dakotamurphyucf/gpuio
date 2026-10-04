# Application and integration API boundaries

GPUIO currently has seven public Dune libraries. A library being installable, or
a module being reachable, does not make every member an application API. All
layers are experimental and must be built from the same revision; see
[compatibility and limits](api-compatibility.md).

| Library | Intended use | Contract source |
| --- | --- | --- |
| `gpuio` / `Gpuio` | Application values, configuration, styles, pure views and collection models | [`lib/core`](../lib/core) interfaces |
| `gpuio.bonsai` / `Gpuio_bonsai` | Effect-specialized views, managed rows, lists, trees, tables, settings and command observations | [`gpuio_bonsai.mli`](../lib/bonsai/gpuio_bonsai.mli) and its exported module interfaces |
| `gpuio.eio` / `Gpuio_eio` | App/window lifecycle, controllers, scoped producers, resource publication and desktop services | [`lib/eio`](../lib/eio) application interfaces |
| `gpuio.runtime_core` | Runner integration and deterministic driver tests | [`lib/runtime_core`](../lib/runtime_core); UI-domain ownership is mandatory |
| `gpuio.protocol` | Paired wire codecs and bridge implementation | [`lib/protocol`](../lib/protocol), [bridge contract](design/bridge-v1.md) |
| `gpuio.native` | Low-level transport and virtual backend interface | [`gpuio_native.mli`](../lib/native/gpuio_native.mli), [`backend.mli`](../lib/native/backend.mli) |
| `gpuio.native.default` | Default implementation selected when linking a native executable | [`lib/native_default/dune`](../lib/native_default/dune); no application controller API |

Use the first three layers for applications. Native component and document-profile
authors additionally use the [extension](design/extensions.md) and
[profile](design/document-profiles.md) package contracts. A generated backend
implements the virtual native library; it is not a second runtime to launch.

## Reachable implementation interfaces

`Gpuio.Reconciler`, all of `Gpuio_runtime_core`, and
`Gpuio_eio.Inbox`, `Asset_registry`, `Document_registry`, `Canvas_registry` and
`Chart_registry` serve the runner. They are currently exported for integration
and tests. Application code should use `App`, `Scope`, `Stream`, `Asset`,
`Document`, `Canvas` and `Chart` instead of driving their queues directly.
Some public types alias registry types; use the application module's spelling
when constructing signatures. This guidance does not make those reachable
modules private or remove existing symbols.

`Expert` members expose adapter operations, wire conversions or runtime identity.
Follow each member's stated invariants: the name does not grant permission to
bypass source revisions, owner generations, validation or domain checks.
Likewise, protocol records describe wire values, not necessarily valid domain
values. Decode and validate at the documented boundary. Most protocol modules
have inferred interfaces rather than handwritten `.mli` files; their source and
paired codec tests define the current bridge surface.

The Core and Eio libraries use Dune-generated wrappers; their `.mli` files alone
do not enumerate the library namespace. Dune's `private_modules` declarations
also apply. Bonsai has an explicit facade: use its exported aliases, for example
`Gpuio_bonsai.Settings` and `Gpuio_bonsai.Command_binding`, rather than assuming
every source filename is a public module.

## Choosing an operation and its owner

| Task | Application API | Ownership and completion |
| --- | --- | --- |
| Build a reactive view | `Gpuio_bonsai.View` and `Bonsai.Cont` | Graph evaluation constructs descriptions; it must not start I/O or mutate controllers. Effects execute later on the UI domain. |
| Run I/O | `Scope.start` with captured Eio capabilities | The scope owns cancellation. A finished producer may still have a queued result; cancellation suppresses that delivery. External Eio cancellation remains cancellation. |
| Batch streamed values | `Stream.create` / `push` | Single UI domain, capacity measured in values. A full batch or scheduler queue returns an error without admitting the value. Bound payload bytes separately. |
| Keep work across scrolling | A conversation/application scope and application-owned collection | Row visibility does not determine durable task lifetime. Transient row state and guarded effects follow [managed-list rules](design/managed-lists.md). |
| Edit native text | `Gpuio_eio.Text_input` | Observe the native editor and issue explicit commands against its lease/revision. A view's initial text is not a recurring controlled rewrite. |
| Publish a resource | `Asset`, `Document`, `Canvas`, `Chart` | Register under an owning scope. Submission, installed data, successful decoding/layout and physical presentation are distinct; inspect the specific result/event. |
| Close a window | `App.Window.request_close` | Runs the application close decision. `close` force-closes and cancels its scope. |
| Start an app | `App.run` or `run_desktop` | Owns GPUI on the OS main thread and creates the Eio UI domain. Do not wrap it in another `Eio_main.run`. |

`Scope.start` reports ordinary producer failures through `Or_error`; failures in
UI callbacks are not automatically converted into that producer result. They
propagate through the application runner's cleanup boundary. `Scope.on_cancel`
callbacks must not raise, block or perform I/O. Recoverable admission failures use
the result types of the individual APIs; a failed admission is not permission to
retry every OS operation automatically. For example, `run_desktop` documents
that a lost reply may occur after startup links were admitted.

This map identifies intended use and reviewed ownership boundaries. It does not
certify every exported operation or complete the final release API review.
