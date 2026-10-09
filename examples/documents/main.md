# Streaming source ownership and native document views

[main.ml](main.ml) shows streamed Markdown beside static OCaml code and diff. Read
component, startup registration helper, byte-stream loop and optional test.
[README](README.md) contains exact commands; [dune](dune) links Core/GPUIO/protocol/
Bonsai/Eio with PPX. Use [isolated setup](../../docs/development.md) and current
[platform scope](../../docs/platform-release-policy.md). All content is embedded; no
service, file asset or credential is needed. The Markdown link is source text, not an
application network request.

`component` has no `B.state` or state machine. An external `B.Expert.Var` holds
`None` then `Some (markdown, code, diff)` handles. `let%arr` reads current handles
and derives a view; graph is explicitly unused. This is reactive derivation, not I/O.
`None` renders registering text. Local document helper validates
`Description.Config.create` with source, mode, label and `Viewport 220.`, then
`View.document` mounts native reader. Markdown, Code OCaml and Diff select native
parsing/presentation; application owns canonical source. `View.column`/row composes
them; no custom Rust implementation or editor controller exists here. See
[Document](../../lib/core/document.mli).

`App.run` owns GPUI OS thread and OCaml Eio UI domain, opening a 900 × 640 window. An
application-scoped producer with explicit clock/20-second timeout registers three
Document resources. on_ui bridges an enqueued owning-UI-domain effect through
`Scope.Expert.enqueue`/`E.Expert.handle` plus `Eio.Promise`; create unwraps typed
errors. Markdown starts `Source.empty_stream ()`; code/diff use validated static
strings. Setting the external handle var triggers Bonsai view derivation,
independently of whether bytes are fully streamed. This producer runs on UI domain,
not a worker domain; document operations require that ownership.

The answer includes Unicode, fenced code, table and link. `String.iter` deliberately
pushes **one byte** at a time: `Document.push_bytes` can buffer up to three
incomplete UTF-8 bytes, accepts complete scalars and rejects malformed chunks
atomically. Ordinary launch sleeps 12 ms after each byte; self-test skips delay.
Finish publishes exact terminal Complete content. Local published waits for latest
desired snapshot native acceptance, checking errors, then asserts exact source,
Complete status and zero pending bytes. These APIs distinguish accepted source from
parser completion, rendering and physical display; see
[adapter](../../lib/eio/document.mli) and
[Text_source](../../lib/core/text_source.mli).

A chunk trace: producer pushes a partial emoji byte, adapter retains incomplete
suffix; later bytes complete scalar, canonical source revision advances and coalesced
native publication carries new snapshot. Rust parses/layouts visible content and
renders. There is no Bonsai text setter on every byte or native callback asking OCaml
to shape/layout text. Terminal publication must preserve exact final content. Native
views borrow source handles; handles do not extend scope lifetime.

`--self-test` requests native render callback through a promise after terminal
publication, prints GPUIO_DOCUMENT_PUBLIC_OK, explicitly releases all resources and
marks completion. Completion effect checks result and `App.shutdown` force-cleans
app. Ordinary task completes but leaves window/resources active until close. Scope
cancellation suppresses late delivery and retires resources; cancellation of a source
via `Document.cancel` is a separate operation that can publish accepted prefix as
Cancelled. Test here does not exercise cancel/error/file/network paths, actual
keyboard selection, IME, VoiceOver, GPU pixels or Linux GUI acceptance. See
[Scope](../../lib/eio/scope.mli) and [App](../../lib/eio/app.mli).

To stream real output, acquire bytes through explicit Eio capabilities in a
conversation/data scope chosen independently of native visibility. Call
`Document.push_bytes` for arbitrary byte chunks, call `Document.finish` only after
successful source completion, and cancel intentionally on interrupted stream. Handle
validation/publication errors without converting external cancellation to normal
completion. Do not put I/O in `let%arr` or assume `Viewport 220.` loads all content
into layout. Use resource release and application persistence deliberately if source
must outlive one window.