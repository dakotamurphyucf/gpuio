# Production bridge V1 implementation contract

Implementation contract for OCH-7. Hosted acceptance is recorded in Linear. This is separate from the private foundation smoke
protocol. No released protocol compatibility is promised before the first API
release. OCH-8 extends the initial style tags with paired
OCaml/Rust definitions and an independent full-field fixture.

## Ownership and identity

One native runtime owns its windows. Each window owns a retained tree. Window,
node, handler and resource identities are distinct `(slot, generation)` types.
Slots fit unsigned 32 bits; generations are 1 through 2^32-1, never wrapping.
New node slots are contiguous; freed slots can be reused only with the next
generation. Tombstones contain no text, child arrays or widget resources.
Events must match both the window generation and current node/handler binding.
Changing a callback closure without replacing its binding may retain the handler
ID; dispatch uses the currently committed OCaml callback registry.

## Schema and transaction semantics

Owned bin_prot messages use declaration-order variant tags. Hello requests an
exact protocol version and a required capability mask. Unsupported versions or
capabilities fail explicitly. Resources/document append/edit and namespaced
extensions are reserved capabilities, not accepted opaque arbitrary commands.
Correlated open/close/frame requests are distinct from per-window transactions.
Acceptance and rendering are distinct events; rendering does not assert physical
screen presentation. The client submits one transaction per window at a time.

Create chooses immutable node kind. Text/style/binding updates preserve identity.
Style lists replace the previous list, so absent properties reset; repeated
refinements compose in order, last property wins. Splice uses the current child
array's zero-based offset and deletion count. Remove removes exactly one node:
the same atomic batch must detach it and remove or reparent its descendants.
The final tree must have exactly one root and one parent per other node, with
no cycles, dangling edges or unreachable nodes. An empty tree has no root.

The native implementation stages changed slots in an overlay. Validation failure
publishes no mutation, generation change or revision. Successful commits advance
exactly one revision. Text/style/binding edits do not walk unrelated history;
structural changes currently validate the complete final tree iteratively.
Untouched text, style and child arrays remain shared immutable allocations.
Structural splice cost includes copying the changed parent's child array; this
is not a claim of constant-time child insertion.

## Bounds and validation

Initial limits: 1 MiB encoded message, 256 KiB text field, 4096 operations,
128 style refinements per node, 100,000 slots per tree, depth 128, and 32 windows.
Decode rejects invalid tags, malformed UTF-8, trailing bytes and non-finite
numbers. Container allocation is bounded by both the declared limit and the
remaining input. Domain validation additionally checks sizes, colors, IDs,
revisions and structural invariants. Public callers must not bypass validation
with generated deserializers. Retained variable-size payloads are limited to
64 MiB per window and 256 MiB per session. Text, styles and child arrays count
against this budget; fixed slot metadata is separately bounded by slot/window
counts. The staging overlay shares untouched payloads, and each operation checks
its prospective payload budget before the batch can publish. Peak memory also
includes the old changed payloads, bounded decoded commands and staging metadata.

The mailbox holds at most 64 commands / 4 MiB encoded command backlog, with 128
response reservations and 128 ordered input/observation events. Decoder count
limits separately bound the in-memory expansion of encoded commands. Admission
reserves a response before accepting a command; Busy means the caller retains
its desired state and retries after draining output. A window's transaction slot
is released when its Accepted/Rejected response is drained. RequestFrame retains
its reservation until a real paint callback or an explicit close failure.

Only consecutive render observations for the same window coalesce; responses
and input events form ordering barriers. Input overflow disables that window's
input/transactions and emits a reserved Overloaded event; close/reopen recovers it.
There is room for 32 overload and 32 native-close notifications, plus terminal
Stopped. Reusing a window slot waits for its old output to drain. Native close
requests and emergency abort do not depend on normal command capacity. Stopped
cancels any outstanding requests, including commands still queued when the OS
closes the last window. Shutdown and closed runtimes reject further submission.
An event envelope contains at most 256 events and preserves wire order.

The initial style subset is native-owned; OCaml will resolve public theme tokens
in OCH-8. Unresolved wire Color.Token values currently fail UnsupportedCapability
instead of substituting a color. Full style parity belongs to OCH-8.

## FFI and native host

`gpuio.native` is a native-code-only OCaml library, linked from a Dune-built Rust
archive. Its opaque handle identifies an explicit transport instance; a bounded
registry holds at most eight handles and GPUI permits one active application.
`run` owns the OS main thread, releasing the OCaml runtime lock while GPUI runs.
`submit` and `drain` run on the OCaml UI domain. Each window has an independent
retained tree and generation, and GPUI owns native element state.

Commands use a bounded asynchronous wake channel. Native callbacks enqueue owned
events and notify a duplicated nonblocking Eio pipe. No callback synchronously
enters OCaml. Encoding/decoding/tree work occurs outside the mailbox mutex.
The caller keeps the pipe reader alive through native exit and worker join, then
disposes the handle. Emergency abort wakes GPUI even when the command queue is
full. Registry admission/disposal cannot race application startup. Exported Rust
panics become OCaml exceptions; application logic must preserve its backtrace,
abort the native host on worker failure, join workers and dispose resources.

## Validation

Independent OCaml/Rust fixtures cover requests and ordered event envelopes,
including Unicode, integer width boundaries, optional handles and styles.
Truncation, unknown tags, invalid identities, allocation bombs and trailing bytes
are rejected. Tests cover structural/text rollback, cycles, duplicate parents,
invalid styles, stale generations, close/reopen, response reservation, queue
pressure, coalescing barriers, overload, terminal stop and retained cleanup.
A deterministic randomized structural reference model checks both accepted and
rejected batches, alongside a separate repeated-text-update model.

`rust/native/tests/allocation.rs` measures a single edit in an isolated test
process. One local macOS run on 10,001 nodes / 10,400,000 retained payload bytes
allocated 2,776 bytes in four allocations and took 5 microseconds. It touched one
record and scanned zero unrelated nodes. Timing is an observation, not an SLA;
the test enforces bounded allocation and touched-record behavior.

`examples/bridge/main.exe` exercises the actual production FFI/GPUI path: protocol
negotiation, two windows, 50 accepted/rendered revisions, rollback of an invalid
batch, continued use after closing one window, and panic containment without
poisoning the registry. It emits `PRODUCTION_BRIDGE_PASS` after clean shutdown.

Public views/reconciliation and Bonsai/Eio scheduling belong to OCH-8/OCH-9.
macOS native validation is the development gate; Linux builds/tests remain
required and graphical checks are informational. Hosted results and any remaining
acceptance gaps belong in the live ticket, not inferred from compilation.

## Typed style extension (OCH-8)

Original Style tags 0–18 remain in order. Tag 19 is Fields, a bounded list of
Field refinements; tag 20 is State(state, fields), where 1=focused, 2=hovered and
3=pressed. States are not recursive. Base refinements use Fields. Every Field
tag is fixed by the paired declaration order and exercised by the independent
`style-v1.hex` fixture. Fill represents solid colors or a two-stop gradient;
Shadow has color, offsets, blur, spread and inset. No unreleased schema change
is a promise of compatibility with an independently installed older host.

There are at most 128 fields per refinement and eight shadows per field. Native
semantic validation rejects invalid units, enums, non-finite values, unknown
state numbers and state-specific interaction policies. Theme tokens resolve to
RGBA in the OCaml adapter; unresolved tokens are rejected by the native host.
Nested field arrays, shadow arrays, font names and accessible labels count
toward retained payload limits, including inside state refinements. Repeated
state entries compose before creating one GPUI handler for each state.

See [typed UI](typed-ui.md) for public reset/inheritance, native interaction and
selection semantics. Owned resource handles remain reserved for the later
resource/component tickets; the pure style values contain no native pointers.
