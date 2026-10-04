# Application-controlled numeric stepping

OCH-41 implementation contract. Native request ownership, guarded resolution,
paired codecs, opt-in input delivery, hold-repeat pause/resume and managed Eio
handlers are implemented locally. Native stepping remains the default. The gallery
uses a page-scoped Eio handler. Physical desktop and release acceptance remain
open; deterministic checks do not establish those gates.

## Application interface and ownership

`Number_input.Config.create ~step_mode:Application` intercepts user step intents from native
keys, buttons and accessibility actions before changing the draft or committed
value. The event carries a unique request token, direction, original input source
and exact numeric snapshot. OCaml may compute a next value using the current
value and direction, perform application work, or decline the request. Native
input, layout and paint never call OCaml synchronously.

The public `Number_input.Step_request.t` is opaque and retains the
window/node generations and native request token. Its accessors are
`snapshot`, `direction` and `source`; `Gpuio_eio.Number_input.resolve_step` takes the controller first,
then the request and an explicit `Apply value | Decline` decision. It fences
the request's original placement, not merely the controller's latest placement.
The Core event/command pair retains the request identity, and the Eio App bridge
rejects mismatched supplied editor snapshots before sending. Handler
exceptions and cancellation must release a matching request without applying a
value or leaving a permanently busy input.

Ordinary programmatic `step` retains its current synchronous native command
semantics; it does not recursively invoke application handlers. Only opted-in
user input is intercepted. Application computation can depend on direction,
current numeric value and its own state, covering both current-value step sizing
and custom step handling. Proposed values normalize to `Numeric.Domain`'s bounds
and grid, as existing explicit replacements do. Invalid/incomplete/nonfinite
native drafts and active composition retain current rejection behavior. An empty
draft may request a value; a proposed empty result requires `allow_empty`.

## Native request lifecycle

Exactly one request is pending per numeric owner. No unbounded queue of key or
repeat events is retained; additional requests while pending return `Busy`.
Creating a request does not edit, commit, change undo history or consume a numeric
revision. A strictly increasing positive request token prevents duplicate replies
from affecting a later request with the same snapshot revision. Token exhaustion
returns `Limit_exceeded`; tokens never wrap or reset during the owner's lifetime.
The native pending record stores only a token/revision/source stamp; the emitted
request contains the snapshot. No native editor or timer is owned by the stamp.

Before request and resolution, synchronize the actual native editor. Any new
text, selection, composition or focus state invalidates the request. A changed
inner editor revision invalidates it even when intervening edits restored the
same visible snapshot. Editing-config changes, explicit admitted mutation
commands, removal and window closure also cancel. A read or identical config
update preserves it. Presentation-only changes preserve an already-issued intent. Moving a pointer
target cancels future repeats, independently of that pending discrete intent.

Resolution matches the pending token and exact numeric revision and consumes
that request once. A stale reply cannot clear a newer pending request. The
mounted adapter must additionally check current window/node generations,
interaction eligibility and application-mode generation before entering the
model. The model rechecks disabled/read-only/composition policy. This deliberately
does not use an ordinary programmatic replacement as its authorization check:
those commands permit edits while disabled or read-only.

An accepted value records one undoable replacement with end selection and emits
`Committed` with the original user input source. Declining changes neither value
nor revision and emits no commit. Invalid proposals or a native edit failure
consume the matching request and leave editing state unchanged; ordinary bounded
error responses report the failure. Faults invalidate the pending request.

## Bridge and request delivery

Numeric event tag 5 appends `Step_requested` with positive request id, direction,
source and numeric snapshot. Numeric command tag 10 appends `Resolve_step` with
request id, expected revision and optional value (`None` declines). Earlier
numeric payload tags do not change. This remains the unpublished epoch-3 bridge;
both runtimes must be rebuilt together. Independent OCaml/Rust fixtures cover
both decisions and the request, framing errors and maximum draft size.

Core tracks the highest request token separately from the highest snapshot
revision. A request can therefore arrive after an observation with the same
revision and still dispatch exactly once. Callback replacement preserves both
counters. Requests older than the current snapshot or with invalid/duplicate
tokens are rejected without advancing history. Mailbox request events are
ordered discrete boundaries and never coalesce with draft changes.

Op83 `Set_number_step_mode` carries node identity and enum tag 0 (`Native`) or
1 (`Application`). It is numeric-only and independent of the existing editing
config payload. Mode-only updates preserve editor/config identity; changing mode
retires outstanding requests and holds without replacing the draft.

The native command dispatcher uses the guarded request-resolution path rather
than ordinary programmatic replacement. It checks current native interaction
eligibility and window activity before resolving. A mounted TestPlatform check
injects a model intent and verifies actual InputState resolution/undo; this is
separate from the newer key, mouse, AX and fake-clock hold checks below.

## Held buttons and remaining adapter work

A held button suspends its repeat timer while waiting for the application,
then resumes after successful resolution only if the same capture, geometry and
native eligibility survive. The first repeat waits 400 ms after resolution; later
repeats wait 75 ms. Waiting owns no repeat timer or duplicate requests. Additional
clicks while pending cannot focus-cancel the original intent. Mouse release,
capture loss or geometry changes stop future repeats, but an
already-issued step may still resolve once if its editor and policy remain
current. Otherwise a slow handler would lose ordinary clicks released before
the bridge round trip completes. Deactivation, removal, policy changes and loss
of editing eligibility retire both the gesture and request. Decline or failed
resolution stops the gesture. Keyboard/AX requests have no
idle timer. These transitions have fake-clock TestPlatform coverage. Physical macOS
validation remains required.

The native request model tests cover no pre-step mutation, single-flight bounds,
read preservation, source-preserving normalized commits, duplicate/late replies,
text/selection/composition/focus/config and hidden editor-revision invalidation,
invalid drafts/proposals, explicit cancellation, native edit failure and token
exhaustion. They use a deterministic editor adapter stub, not GPUI InputState.
Paired codec, mailbox, Core routing, mounted editor/hold checks and a public gallery
example now supplement these model tests. Managed Eio handler tests supplement these checks as documented below;
physical end-to-end cancellation/error acceptance remains required.
Physical macOS qualification remains a separate release gate.

## Local model verification

2026-10-02, macOS, dirty worktree based on `83eb87e`, isolated toolchain. No OS
windows were opened. Commands prefixed with `GPUIO_JOBS=2 ./scripts/gpuio exec`:

- `cargo test -j2 -p gpuio-native --features native-image-tests --lib --test number_input --offline`:
  590 library tests and six admission tests pass, with two existing skips.
- `cargo clippy -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --offline -- -D warnings`: passes.
- `cargo fmt --all -- --check`: passes.

`git diff --check` passes. No OCaml or wire payload changed in this model stage;
the preceding full OCaml/gallery checkpoint is documented in
[numeric presentation evidence](../evidence/number-presentation-och41.md).
The five new request tests are model checks using an editor stub. They do not
establish completed native-to-OCaml request delivery, repeat integration or
physical desktop behavior. The milestone and OCH-41 remain incomplete.

## Local bridge verification

The paired bridge stage on the same local worktree adds independent request,
apply and decline fixtures; Core opaque-owner conversion and same-revision token
routing tests; native mailbox ordering; the guarded command entry point; and an
actual retained InputState resolution/undo check. The last check injects a model
request directly; user-input emission and hold-repeat were pending at that stage.

With the same isolated command prefix:

- `cargo test -j2 -p gpuio-protocol --offline`: 315 pass.
- `cargo test -j2 -p gpuio-native --features native-image-tests --lib --test number_input --offline`:
  591 library tests and six admission tests pass; two existing skips.
- `cargo clippy -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --offline -- -D warnings`: passes.
- `dune build -j2 @runtest @fmt examples/gallery/main.exe`: passes.
- `cargo fmt --all -- --check`: passes.

No OS windows were opened. Required Linux automated checks, current CI, physical
macOS and resource/distribution acceptance are not established by this checkpoint.

## Local opt-in mode and native input verification

The mode/native stage on the same local worktree adds Op83, mode-only Core
reconciliation, real TestPlatform Up and AX Increment request delivery, and native
mouse/capture/fake-clock repeat checks. Before resolution the draft is unchanged;
additional requests are bounded. A delayed mouse-up reply can still apply once.
A second click cannot focus-cancel the first pending intent. Repeat owns no task
while waiting, resumes at 400/75 ms, and stops on decline. Typing, read-only,
hiding, deactivation and mode changes invalidate replies. Removal releases the
owner. The gallery demonstrates value-dependent steps through the public API.

With the same isolated prefix, the final checks pass:

- `cargo test -j2 -p gpuio-protocol --offline`: 316 tests.
- `cargo test -j2 -p gpuio-native --features native-image-tests --lib --test number_input --offline`:
  592 library tests and seven admission tests, with two existing skips.
- `cargo clippy -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --offline -- -D warnings`.
- `dune build -j2 @runtest @fmt examples/gallery/main.exe`.
- `cargo fmt --all -- --check` and `git diff --check`.

The new mode-only Core test initially found an unnecessary editing-config update;
reconciliation now compares the unchanged wire editing payload independently of
step mode. No expectations were promoted. These are local uncommitted changes,
not physical macOS, Linux, current CI or distribution acceptance. The managed Eio follow-up is documented below; physical acceptance remains open.

## Managed Eio handlers

`Gpuio_eio.Number_input.run_step controller ~scope request ~f ~on_result` starts
one child scope and returns an `Or_error` task handle through a Bonsai effect.
`f` computes `Apply value | Decline`; capture Eio capabilities explicitly when it
needs I/O. It must not access Bonsai from another domain. The caller chooses the
lifetime: the gallery uses its existing `Preview_scope` activation/deactivation
helper, so leaving the page cancels outstanding work. Hiding a view does not
implicitly cancel an unrelated application scope.

`Step_task.cancel` is idempotent. Cancelling the supplied scope or task suppresses
queued completions and late result callbacks. Cancellation before dispatch
prevents application of a proposal; after dispatch it cannot undo a native edit.
The native token, revision and placement checks remain the final editing guard.
A matching guarded decline is queued on cancellation, ordinary work failure,
resolution failure or task-start rejection. Work exceptions report `Work_failed`;
command errors report `Resolution_failed`. Start errors return without calling
`on_result`. Successful completion retires the child scope and cancellation
registration before invoking the callback. Callback exceptions follow the usual
application failure policy, rather than being misreported as work failures.

Cleanup has no completion callback and occupies no ordinary numeric pending-map
slot. It queues a normal `Resolve_step` decline with a fresh correlation and the
original owner/token/revision; its native acknowledgement is intentionally ignored.
Thus all 64 ordinary numeric command slots can be occupied without preventing
cleanup. A decline already queued remains subject to transport delivery; it is
not an acknowledgement or a synchronous native cancellation. Stale/duplicate
native requests are harmless. Closed/stopping windows need no decline because
teardown retires their owners. Correlation exhaustion requests runtime shutdown
rather than raising from cancellation cleanup. Existing Eio task/scope/cleanup
limits bound managed work; no idle polling or retry timer is introduced.

Local deterministic coverage uses real Eio fibers and Inbox delivery with an
injected resolution adapter: suspended work, queued completion, cancellation
after dispatch, parent cancellation, worker/callback exceptions, task saturation,
Inbox backpressure and normal cleanup. A separate App test uses injected host
replies to verify the saturated 64-command lane, original decline identity,
wrong-window rejection, ignored cleanup acknowledgement and window teardown.
It does not dispatch a physical native widget. The public gallery compiles with
the managed helper; an interactive cancellation/error walkthrough remains open.

Managed-handler checkpoint, 2026-10-02, same local dirty worktree and isolated
toolchain (no new native code or protocol payload):

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @check @lib/eio/runtest examples/gallery/main.exe`: passes.
- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe`: passes.
- `python3 scripts/audit_component_catalog.py` and `git diff --check`: pass.

Five new Eio handler tests and one App routing/capacity test pass. No expectations
were promoted. No OS windows were opened. The native/protocol/lint checkpoint
above remains the latest Rust verification; physical macOS, Linux automated,
current CI/review/publication and distribution acceptance remain outstanding.
