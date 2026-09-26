# Segmented OTP inputs (OCH-34)

Status: public Core/Bonsai/Eio APIs, bounded protocol, retained native editor,
commands/events and local macOS acceptance are complete. This includes native
IME/clipboard/AX, public application/OS keyboard checks, GPU appearance, managed
row retention, hidden/modal/window lifetimes, overload and retained-owner workload.
Capability `34359738368` covers numeric/range/stepper/OTP inputs; aggregate
`137438953471`. Consolidated hosted gates and merge remain pending. This document
supplements [numeric inputs](numeric-inputs.md).

## Implemented text contract

`Gpuio.Otp_input.Policy` validates length 1 through 32 and chooses Digits (default)
or Ascii_alphanumeric. Alphanumeric values preserve case. `Value` is an abstract
canonical ASCII prefix; empty prefixes are valid. Compatibility with a particular
policy is checked explicitly, since a value constructed under a wider/alphanumeric
policy need not fit a different policy. `is_complete` checks compatibility and
exact length; it never means authentication succeeded.

`Value.of_string` maps full-width U+FF10–FF19 digits and U+FF21–FF3A / U+FF41–FF5A
Latin letters to their ASCII equivalents, then checks the selected alphabet.
It does not normalize other Unicode digits, spaces, punctuation or confusables.
`Value.of_paste` additionally removes ASCII hyphens and the six ASCII whitespace
characters (space, tab, CR, LF, vertical tab, form feed). Full-width hyphens and
nonbreaking spaces remain invalid. Raw input is bounded to 4,096 bytes before
normalization, including separators; there is no unbounded separator scan.

Unexpected characters report their original UTF-8 byte offset without embedding
the code text in the error. The entire input/edit rejects on invalid UTF-8,
unexpected characters, capacity overflow or excessive raw bytes; no valid prefix
is accepted implicitly and no truncation occurs. Check raw byte bounds before
UTF-8 validity, then examine characters left to right. Raw unvalidated protocol
policies are rejected before normalization.

`Value.replace` and `Value.paste` take a validated value, policy and directional
selection. Canonical ASCII byte offsets equal cell indices. Both verify the
existing value and selection before constructing a result. A successful insertion
replaces the selected interval and collapses the caret after the normalized
inserted text. Direct empty replacement deletes the selection. Empty or
separator-only paste preserves both value and directional selection, matching
an empty clipboard operation rather than unexpectedly deleting a selected code.
The supplied immutable value is unchanged after any failure.

OCaml implementation: `lib/core/otp_input.{ml,mli}` and
`lib/protocol/otp_wire.ml`. Rust counterpart: `rust/protocol/src/otp.rs`.
The policy encoding is length followed by alphabet (Digits=0,
Ascii_alphanumeric=1). Independent tests pin six digits to `0600` and length-32
alphanumeric to `2001`. The standalone contract codecs below add configuration,
observations and commands. The retained bridge below registers a node kind,
configuration operation, observations and correlated command/result envelopes.

## Public Core and standalone wire contracts

`Gpuio.Otp_input.Config.create` takes a validated policy and a required accessible
label, with optional masked/disabled/read-only/auto-focus flags (all false by
default). Labels are nonblank single-line UTF-8 without NUL, bounded to 4,096
bytes. A policy belongs to a native placement and changing it requires remounting;
initial code is a separate one-time seed, never a controlled text property.
Masked behavior hides painted/accessibility text and disables copy
and cut. It does not redact application snapshots or promise secure storage.

Snapshots carry a nonnegative revision, immutable policy, accepted value, raw
draft, directional selection, optional nonempty marked range, focus and undo/redo
availability. Value must fit the policy; draft must equal value outside
composition. Selection/mark offsets must be UTF-8 scalar boundaries in the
bounded draft. The public snapshot also holds its originating window/node lease
behind the abstract interface. `is_complete` requires a full accepted value and
no active composition; it never asserts authentication.

The event contract is `Observed | Changed | Complete | Rejected`:

- Observed covers mount/configuration/programmatic operations. Mount may use
  revision zero; native changes and semantic events require a positive revision.
- Changed covers native value, selection, composition, focus and history changes.
- A native edit that changes accepted text to a full code emits Changed followed
  by Complete with the next revision. This includes a different full code and
  native undo/redo; initial state, unchanged values, preedit and explicit commands
  do not complete. The owner must reserve both revisions before mutation and the
  queue must preserve that ordered pair across coalescing.
- Rejected carries the post-attempt snapshot and a bounded, text-free input error.
  Invalid final IME text has already rolled back its checkpoint and ended
  composition. A rejection itself is a semantic boundary even when state did not
  change. Preedit/range admission errors that leave composition active are not
  represented as committed-input Rejected events.

Commands are Replace, Clear, Select, Focus, Undo, Redo, Cancel_composition and
Read_snapshot. Replace/Clear have optional revision guards and Record/Reset
history policy. Replace takes a canonical value plus Start/End/Preserve/Select
selection policy; Preserve clamps each previous endpoint to the new ASCII length,
while Select must fit exactly. Clear places the caret at zero. Programmatic
Replace/Clear are allowed while disabled/read-only; Undo/Redo are not. Content,
history and Select commands reject composition; explicit Cancel_composition
restores its checkpoint. Focus uses native visibility/modal/disabled gates.
Responses are Applied(snapshot) or a typed command error. Native window/node
lifetime checks remain necessary in addition to the revision guard.

`lib/protocol/otp_wire.ml`, `rust/protocol/src/otp_input.rs` and
`rust/protocol/src/decode/otp_input.rs` implement matching validators and standalone
codecs. Rust decoding bounds every string before allocation, rejects invalid
variant/Boolean tags, malformed UTF-8, negative guards, impossible snapshot/event
states, truncation and trailing bytes. Standalone encoded limits are 4,200 bytes
for configuration, 4,352 for event/response and 128 for commands. These are
payload bounds, separate from the transport-envelope accounting below.

These interfaces define the native owner contract. The state owner below now
implements sequencing and commands; the mounted adapter below connects native
rendering, AX, observation delivery and public command correlation.

## Native integration requirements

Retain one native editing session for the entire code and draw segmented cells;
never allocate one editor per cell. Keep stable native ownership, revision and
window/node lifetime fences. Ordinary observations, mask and disabled/read-only
updates must not reset the code or caret. Explicit replacement and clear commands
must validate atomically. Keyboard navigation/deletion, directional selection,
paste distribution, native IME and accessibility editing must share the same
accepted text policy. Complete is an ordered input notification, not verification
or a request to contact an authentication service.

Inspection of the pinned `vendor/gpui-base/src/otp_input.rs` confirms that its
append/backspace model lacks the required selection, paste and IME handling and
accepts unfiltered programmatic strings. It remains a reference, not the adapter.
The existing `InputBaseState` has a boolean validator, but calls it for both final
text and marked composition. Installing an ASCII-only validator would reject
legitimate IME intermediates, and that validator cannot normalize insertion text.

The OTP adapter implements GPUI's `EntityInputHandler` directly, using one
entity and one bounded editing session. This avoids changing the generic editor's
validation/history behavior and lets segmented paint, hit testing, selection and
IME candidate bounds share the same geometry. An invisible ordinary text editor
under a separately painted row would have incorrect glyph/candidate positions.
Do not repair input by feeding asynchronous observations back as replacements.

### Implemented editing model

`rust/native/src/otp_edit.rs` owns an immutable policy, canonical accepted text,
directional selection, optional preedit draft/marked range and bounded undo/redo.
Policy changes require an explicit remount; no retained value is truncated or
reinterpreted. Configuration, focus, leases, revision reservation and publication
belong to the mounted owner and are not implemented by this pure model.

The first marked insertion keeps the accepted value and selection as a checkpoint.
Subsequent preedit updates affect only the draft, including arbitrary single-line
Unicode that would be invalid as a final code. NUL/CR/LF are rejected. The entire
draft is bounded to 4,096 UTF-8 bytes; excessive or malformed preedit leaves the
previous state intact. GPUI mutation ranges use exact UTF-16 scalar boundaries;
surrogate-interior, reversed or out-of-document ranges reject. The model exposes
UTF-8 offsets for render/snapshot consumers. Selection within new preedit is
relative to its insertion, as required by GPUI's platform interface.

Final native insertion or `unmark_text` normalizes the candidate once. A valid
candidate becomes one undo edit with mapped ASCII cell selection. Invalid final
text restores the original accepted value and exact directional selection, ends
composition and reports rejection. Neither case puts intermediate preedit in
history. Escape cancellation and empty marked text restore the checkpoint too.
For a composition rejection, unexpected-character offsets refer to the complete
raw candidate draft; ordinary insertion/paste errors refer to the raw insertion.
Malformed final replacement ranges preserve the active composition unchanged.

Paste has its own normalization path and rejects while composing. Programmatic
replacement, undo/redo and cell navigation/deletion also reject active composition;
native IME navigation must be routed to the platform. Outside composition, cell
movement and forward/backward deletion handle directional selections, endpoints
and selection extension. Programmatic replacement takes canonical text, an exact
selection and Record/Reset history policy; validation precedes any history reset.
Ordinary selection changes and same-value edits do not create undo entries or
discard redo. Explicit Reset clears both stacks even for an unchanged value.

History retains at most 128 edits across undo and redo, each with before/after
values and selections. Its text payload is at most 8,192 bytes (128 × 2 × 32),
separate from the one bounded preedit draft and allocator/structure overhead.
There is no unbounded IME transaction log. A changed accepted value that reaches
full length reports completion eligibility, including a different full code or
redo; unchanged values, selection and preedit do not. The mounted event owner
must distinguish user edits from programmatic observations and preserve ordered
Changed/Complete boundaries. The model does not yet publish bridge events.

### Implemented native state owner

`rust/native/src/otp_input_state.rs` owns the editing session, immutable policy,
configuration, revision and observed focus. It executes explicit commands and
native actions separately. Programmatic effects emit only Observed; a user edit
that changes accepted code to full length emits Changed then Complete with
distinct consecutive revisions. Rejected input always produces its semantic
boundary, even if text did not change. Repeated focus/configuration observations
and no-op edits do not consume revisions.

The owner reserves the operation's maximum event count before mutation, including
before Rust focus/clipboard callbacks. Reservation is conservative near signed
64-bit exhaustion: an operation that could produce two events needs capacity for
both even when a particular input would produce fewer. Exhaustion leaves code,
selection, preedit, history, configuration and callback side effects untouched.
Read_snapshot remains available. The mounted adapter must stop/fault input on
exhaustion instead of applying unobservable native edits.

Replacement guards and validation precede mutation. Preserve clamps each
selection endpoint, explicit Select fits exactly, and Reset clears history even
for unchanged text. Policy-changing configurations reject atomically. Other
configuration changes retain the editor session, code, directional selection,
preedit and history. Actual platform focus updates are reported separately;
programmatic Focus must be confirmed by the Rust adapter before success.

Native access checks include an adapter-supplied visibility/modal gate and the
current disabled/read-only policy. Read-only selection and unmasked copying are
allowed; native content/history edits and cut are denied. Programmatic Replace/
Clear remain available under disabled/read-only. Masked copy/cut and empty
selection do not touch the clipboard. Copy/cut reject active composition. Cut
reserves capacity, calls the Rust clipboard writer, then deletes the selection;
a failed writer leaves text/selection/history/revision unchanged. These tests use
Rust callbacks, not the actual operating-system clipboard.

Platform unmark commits only when editing is still permitted. If the field has
become hidden, modal-blocked, disabled or read-only, unmark restores the preedit
checkpoint instead. Empty marked text and explicit lifecycle cancellation also
restore it even after interaction is gated; composition cannot be left stranded
because a control was disabled. Cleanup produces Changed without Complete.
Ordinary configuration updates themselves do not cancel composition.

The owner returns batches of at most two native events or one programmatic
observation before its response. Adjacent Changed observations may coalesce only
with increasing revisions under the same policy; Complete, Rejected, Observed
and responses remain boundaries. No queue or GPUI entity is established by this
owner; the retained bridge below supplies routing and queue admission separately.

### Implemented retained view and event bridge

`View.otp_input` takes a controller key, configuration, initial value and typed
event callback. Reconciliation checks that the seed fits the policy, rejects
duplicate controller identities and preserves the native placement across ordinary
rerenders. Changing the seed alone emits no operation. Configuration changes do
not change the original retained seed; changing length or alphabet requires a new
controller identity. Both OCaml reconciliation and native tree admission reject
policy changes atomically. The native tree requires a valid configuration and
handler even for disabled fields, rejects children/text/control payloads and
accounts configuration and seed bytes until disposal.

Appended wire tags are Kind 39 (`Otp_input`), operation 45 (`Set_otp_input`) and
event 48 (`Otp_input_event`). Existing tags are unchanged. The event carries
window, node, handler and tree revision in addition to the typed observation.
Native routing rejects foreign generations/handlers, future or negative tree
revisions, invalid observations and mismatched policies. Core additionally fences
native revisions, including across callback refreshes and pending reconciliations.
Disabled/read-only cleanup observations still reach the current callback; stale
events cannot revive an unmounted or closed controller.

Mailbox accounting charges both accepted value and preedit draft. Adjacent Changed
observations coalesce only under matching window/node/handler/tree revision and
policy, with increasing native revisions. Observed, Complete, Rejected and
responses preserve their boundaries. `Mailbox::otp_completion` validates and
admits the entire Changed/Complete pair under one lock. Count and byte checks
precede replacing a coalescible tail, so admission failure leaves the old queue
intact and never publishes half a completion. Complete must describe the same
full, noncomposing snapshot with exactly the next revision.

### Implemented GPUI adapter

`rust/native/src/otp_input_view.rs` owns one GPUI entity per retained node; it
holds the bounded state owner, native focus, event route and focus/activation
subscriptions. `otp_input_paint.rs` supplies the layout used by paint, caret,
selection, hit testing and UTF-16 candidate queries. Accepted ASCII is rendered as
segmented cells. During composition, the bounded Unicode draft is continuously
shaped inside the field, with its marked range underlined. This avoids splitting
combining characters into artificial code cells. Both modes clip to the field and
scroll the caret into view under constrained width. Masked modes paint bullets
and expose a protected accessibility value while application snapshots retain text.

Native key actions handle cell navigation, selection, deletion, undo/redo and
clipboard operations. Two OTP-scoped selection actions avoid changing the base
library's export surface. Native platform paste uses the same normalization path
as keyboard paste. Platform selection/range queries use exact UTF-16 boundaries;
read-only fields allow selection/copy but reject edits. AppKit SetValue uses the
same atomic normalization/validation as ordinary user insertion. Programmatic
commands use the correlated lane described below.

The host retains entities across rerenders, includes them in native command focus
routing and pins focused/composing managed-list rows. Pointer capture survives
repaint and allows selection beyond the field bounds; release, blur, hiding,
removal and window deactivation clean it up. Losing access cancels composition
through the existing checkpoint semantics. No caret polling or permanent animation
timer is installed; the current caret is steady.

Native observations pass through session generation/handler/policy validation.
Two-event completions use `Transport::otp_completion`, holding one mailbox lock
through pair admission. Failed admission faults the window; subsequent platform
input cannot mutate its terminal owner. Rendering and native delegates never call
OCaml synchronously.

The local native harness now validates actual macOS NSTextInputClient marked and
committed text, key dispatch, the system clipboard, real AppKit accessible values
and SetValue, protected values, captured selection, narrow maximum-size preedit,
configuration retention, disable/unmount cleanup and terminal overload. These
checks establish the initial native adapter; they do not establish final visual
quality, external OS shortcut delivery or the full public-application contract.

### Correlated commands and public controller

Message tag 16 (`Otp_input_command`) carries a positive correlation ID, window,
node and bounded command. Event tag 49 (`Otp_input_result`) returns the same
identity and Applied/Failed result. Prior tags are unchanged. Independent OCaml
and Rust fixtures cover both envelopes, invalid bounds and truncation. Response
accounting includes both accepted value and preedit draft; drain batches remain
within the 1-MiB protocol envelope and responses fence observation coalescing.

The native host resolves the exact window/node generation, then the retained
instance validates its current route/configuration. Commands run on the GPUI
thread. Focus checks the actual visibility/modal gate and confirms native focus.
The owner publishes any Observed event before the correlated response; commands
never emit Complete. A full observation queue faults the window and reports
Native_failure rather than Applied. That failure may follow a native mutation;
subsequent commands and user edits cannot mutate the terminal owner. Callers must
not interpret a transport/native failure as proof that no mutation occurred.

`Gpuio_eio.App.Window.Expert.otp_input_command` bounds pending OTP requests to 64
across the application and rejects wrong-window snapshots. A close request
rejects new commands immediately; admitted commands may reply before native close.
Actual closure completes any remaining requests with Closed. Applied replies must match the request's
window/node, immutable policy and minimum observed revision. Unknown or mismatched
correlations cannot complete a different request. The ordinary application API is
`Gpuio_eio.Otp_input`: `create`, `view`, `snapshot`, explicit `replace`/`clear`,
`select`, `focus`, `undo`/`redo`, `cancel_composition` and `read_snapshot` effects.
`replace_if_unchanged` fences both lease and revision. Results are typed; no callback
object crosses FFI.

The controller's Bonsai state accepts only nonregressive observations within a
lease. A command reply cannot overwrite a newer revision or another mounted
lease. An unplaced controller returns Not_mounted; a retained controller that was
unmounted may still hold a snapshot, but native commands return Stale_input.
Rerendering never writes snapshot text back into the native editor. Both alphabets
have a public example in `examples/numeric/otp.ml`; its self-test exercises the
real bridge and closes its window. `scripts/test_otp_input.py` additionally targets
the example through macOS accessibility and OS keyboard delivery.

### Acceptance and remaining delivery gates

Local macOS acceptance now covers retained public application behavior and real
OS keyboard/AX delivery; native NSTextInputClient composition; GPU light/dark,
selection, masked preedit, 1x/2x synthetic density and narrow geometry; managed-row
pins, hidden/modal cleanup, actual activation loss, independent windows and close
during capture/composition; and three 256-owner mount/edit/disposal workloads.
See the [evidence ledger](../evidence/numeric-inputs-och34.md) for limits, exact
commands, artifacts and debug timings. Native glyph paint errors fail the native
test configuration instead of passing unnoticed.

The visual fixture checks every occupied cell, verifies its accepted value and
uses read-only fields while capturing accepted codes. Masked images are compared
across different same-length values and preedit, and actual exported screenshots
are inspected. Synthetic density is not evidence of physical monitor transitions.

Consolidated required macOS/Linux CI and merge remain before ticket completion.
Full Linux desktop GUI release acceptance remains OCH-17. No new dependency,
toolchain or upstream compatibility claim is introduced by these controls.
