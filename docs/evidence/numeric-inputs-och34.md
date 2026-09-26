# Numeric and OTP component evidence (OCH-34)

## Current scope

Local macOS acceptance is complete for the full OCH-34 scope: single/range
sliders, numeric editor/stepper layouts and segmented OTP inputs. Public controllers,
commands/events and examples are implemented. Capability `34359738368` advertises
the family (aggregate `68719476735`). OCH-34 remains In Progress until consolidated
hosted macOS/Linux gates and merge; Linux desktop release acceptance remains OCH-17.

The final OTP acceptance entry below closes the local gaps described in earlier
chronological checkpoints. Those earlier entries record their historical scope.

The [design and pinned-source review](../design/numeric-inputs.md) records the
candidate single/range support and bridge gaps. Public contracts were drafted in
`lib/core/numeric.mli` before implementation. No dependency or toolchain pin changed.

## Shared numeric foundation, local macOS arm64

`Numeric.Domain` validates finite bounds, a positive resolvable step and bounded
step count; supports fixed intervals and steps larger than the span; anchors the
grid at minimum and includes an irregular maximum endpoint. Normalization and
adjacent stepping are bounded operations. `Numeric.Draft` separates empty,
incomplete, malformed/nonfinite/oversized, in-range and out-of-range text without
owning or modifying an editor.

OCaml expect tests and Rust tests cover min anchoring, irregular endpoints, exact
binary ties, repeated stepping/saturation, nonfinite inputs and invalid domain
imports. Each implementation checks 7,175 samples across seven domains for bound
preservation, normalization idempotence, strict progress and inverse adjacency,
including large offsets, tiny/large magnitudes and a trillion grid intervals.
Draft tests include `-`, `.`, `1e-`, trailing decimal points, exponents, outside
bounds, overflow, invalid separators/hex/non-ASCII digits and oversized input.

An independent 24-byte little-endian fixture for min=-1.5, max=2.25, step=0.125
matches OCaml bin_prot and Rust binprot. The Rust standalone decoder rejects
truncation, trailing/oversized input and invalid/nonfinite domain fields. OCaml
validates wire imports through the public abstract domain boundary.

Cross-language testing found that Rust's `is_ascii_whitespace` excludes vertical
tab, unlike the intended six-character ASCII whitespace grammar. Both parsers
now explicitly enumerate space, tab, LF, CR, VT and FF. The regression passes.
The only OCaml expectation adjustment was reviewed UTF-8 sexp escaping for the
invalid full-width-digit input; no behavior assertion was auto-promoted.

Commands (isolated checkout, `GPUIO_JOBS=2`) all pass:

```sh
./scripts/gpuio exec dune build -j 2 @test/view_api/runtest @fmt
./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol
./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-protocol --all-targets -- -D warnings
./scripts/gpuio exec cargo fmt --all
```

This is pure Core/protocol validation, not native interaction evidence. Next:
explicit native single/range-slider ownership, revisioned commands/snapshots,
input cancellation/coalescing and independent keyboard/AX thumbs, followed by
the numeric editor/stepper and OTP acceptance in the live ticket. Consolidated
hosted macOS/Linux gates and merge remain pending.

## Slider contracts, codec and native state

The public Core `Slider` interface was drafted before implementation. It provides
validated finite single/range values, domain/axis/scale/labels/policy configuration,
abstract revisions, snapshots with distinct preview/committed values, lifecycle
events and explicit commands. Single/range identity is immutable per mounted
owner. Snapshot admission rejects mode mismatches, negative revisions, a changed
stationary thumb and inconsistent drag phases. No View/Eio controller or mounted
slider bridge is claimed at this checkpoint.

Rust `slider_state::State` owns the current/committed values and drag lifecycle.
It shares configuration through `Arc`, has no timer or retained event queue, and
returns at most two bounded events from a mutation. Tests pass for:

- Thumb collision without identity swapping; preview versus commit and Escape.
- Bound changes cancelling under the old domain before normalization/observation
  under the new domain; label-only updates preserve an active gesture.
- Disabled/read-only cancellation and allowed explicit programmatic replacement.
- Stale revision, wrong mode, malformed value and nonfinite AX-like mutation
  rejection without cancelling or mutating an active drag.
- Discrete input interrupting a drag in order; repeated native steps using the
  latest value, Page stepping, Home/End and saturation.
- Revision overflow rejecting one/two-event mutations atomically.
- Shared configuration lifetime, replacement of equivalent allocations without
  resetting state, and final-owner release.

Independent config/preview/guarded-replace fixtures agree between OCaml and Rust.
Standalone decoders bound input and reject truncation, trailing bytes, invalid
tags, malformed values/configurations and invalid event phases. All lifecycle
variants round-trip. Linear/logarithmic mapping tests cover endpoints, fixed
domains, narrow positive spans and an overflowing max/min ratio without unsafe
geometry values. Malformed raw wire values also fail mutation helpers.

Local macOS arm64 commands pass (isolated `GPUIO_JOBS=2`):

```sh
./scripts/gpuio exec dune build -j 2 @test/view_api/runtest @fmt
./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol
./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --lib
./scripts/gpuio exec cargo clippy --locked --workspace --all-targets --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests -j 2 -- -D warnings
```

The native library suite passes 147 tests, including eight slider model tests.
The final defensive raw-value guard also passes the slider protocol suite.
These are deterministic model/codec tests and compilation, not actual mouse,
keyboard, AX or GUI acceptance. Renderer ownership, transport coalescing and
current-node/handler fences, two-thumb traversal and mounted native acceptance
are the next layer. No capability or hosted result is claimed.

## Retained slider views and observation bridge

Core/Bonsai `View.slider` now carries a stable controller, configuration,
mount-only initial value and an observation callback. Public snapshots preserve
their window/node identity. Native tree admission validates complete slider
leaves, immutable single/range mode and retained configuration accounting.
Session routing rejects stale identities/handlers, invalid lifecycle data and
wrong modes, while retaining historical old-domain cancellations. A slider does
not also trigger a generic Press callback.

Independent request/event fixtures agree between OCaml and Rust, including the
appended Kind/Op/Event tags. Tests reject truncated/trailing envelopes, malformed
configuration/value data and invalid lifecycle phases. OCaml tests cover latest
callbacks, duplicate/out-of-order native revisions, invalid-event fence poisoning,
pending-update delivery, remount generations, closed windows and atomic rejection
of mode changes or duplicate controller ownership. Policy updates retain the
observation handler, including cancellation snapshots outside the new domain.

Native tests cover failed-admission rollback and accounting, replacement/release
of shared configurations, identity/phase routing and overload/close rejection.
Ten thousand adjacent drag previews retain one latest preview between the start
and final commit. Cancellation, observations, commit, start, responses and unrelated
input remain barriers. Owner/tree revisions, thumbs and committed values prevent
inappropriate coalescing. Capacity rejects a discrete commit explicitly rather
than overwriting prior input, and normal delivery resumes after draining.

This checkpoint does not implement native rendering or correlated slider
commands. Actual pointer, keyboard, independent-thumb AX/focus, hidden/modal
cancellation and mounted lifetime acceptance remain required before advertising
the component. Numeric editor/stepper and OTP scope is unchanged.

Local macOS arm64 validation passes with isolated `GPUIO_JOBS=2`:

```sh
./scripts/gpuio exec dune build -j 2 @all @test/view_api/runtest @fmt
./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol
./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol -p gpuio-native --lib --test slider --test rating --test session
./scripts/gpuio exec cargo clippy --locked --workspace --all-targets --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests -j 2 -- -D warnings
./scripts/gpuio exec cargo fmt --all
```

These commands build/link the public examples without launching GUI windows.
The four native bridge tests and five slider protocol tests pass, alongside the
native library and existing rating/session regressions. No hosted or Linux GUI
result is claimed for this checkpoint.

## Mounted native slider, initial macOS acceptance

The native adapter now mounts the shared slider model once per node generation,
retains native values across initial-value rerenders and owns at most two focus
handles. Its pointer region captures and rebinds the native hitbox across frames;
keyboard and accessibility use the same model. There is no idle timer. Dirty
configuration updates preserve state or cancel/normalize according to the
contract. Current-generation, policy, visibility, modal and overload gates apply
to callbacks, including callbacks retained by an older frame. Owner removal
releases capture and lets the old frame's references retire.

Focus registration now permits distinct subcontrols of one retained node. Range
thumbs participate separately in a trapped Tab order and retain owner-scoped
eligibility. Native testing found and fixed the host's fallback-focus check,
which initially failed to count slider thumb focus and returned focus to the root.

`native_slider` passes in an actual macOS window using GPUI-dispatched keyboard
and pointer events and actual AppKit accessibility reads/actions:

- Horizontal range: independent thumb stepping, Home/End, ordered collision,
  trapped Tab navigation, retained initial values, preview versus committed state,
  drag release, Escape and bound-change cancellation/normalization.
- Separate AppKit slider nodes with distinct labels, values, min/max and native
  increment/decrement/set-value actions. The first AX query enables GPUI's lazy
  tree; the test awaits a paint before reading it, as existing widget tests do.
- Ancestor hiding cancels dragging and removes AX exposure. A nested modal scope
  cancels the old drag and gates old-thumb AX actions. Read-only keyboard edits
  are rejected; disabled thumbs lose focus.
- Single vertical/logarithmic mode: arrow/Page/Home/End, a midpoint track click
  mapped through the logarithmic scale, dragging outside bounds, and AppKit value
  and range observations.
- Removal releases the native owner and retained tree accounting returns to zero.

The suite is wired into macOS CI and compilation on both platforms, but no hosted
result is claimed yet. Correlated commands, the public Eio controller/example,
expanded rendering/scale/style/idle/deactivation/multi-window workloads and
external public-application validation remain. This is an initial mounted
checkpoint, not full slider or OCH-34 acceptance.

Local validation commands (`GPUIO_JOBS=2`) pass:

```sh
./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-tests --test native_slider --no-run
./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-tests --test native_controls --test native_presentation --no-run
./scripts/gpuio exec cargo clippy --locked -j 2 --workspace --all-targets --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests -- -D warnings
./scripts/gpuio exec cargo fmt --all
```

The resulting slider, controls and presentation executables each ran directly
under a Python subprocess timeout (45 seconds for slider, 60 seconds for each
regression suite), exited zero, and were reaped. Slider emits
`GPUIO_SLIDER_NATIVE_OK` and `GPUIO_SLIDER_AX_OK`; the existing control/focus and
presentation/AppKit/IME regressions also pass. These regression binaries used
`native-tests`, so optional `native-image-tests` pixel assertions are not claimed
from this run. No owned test windows/processes remain.

## Correlated commands and public Bonsai/Eio controller

`Gpuio_eio.Slider` now provides create/view/snapshot, read_snapshot, replace,
replace_if_unchanged, cancel_drag and focus thumb. Native command handling shares
the mounted model and capture state. The App adapter reserves at most 64 pending
requests, correlates exact window/node generations, handles typed replies/failures
and closes pending requests with their window. Controller reply observations are
lease-checked and monotonically ordered; they do not duplicate lifecycle callbacks.

Independent command/result fixtures match between OCaml and Rust. Tests reject
nonpositive correlations, invalid guards/values, malformed result snapshots,
truncation and trailing data. Native mailbox regression verifies a reserved
slider response survives a full input lane and prevents window-slot reuse until
it is drained. The native window suite additionally passes reading a live drag,
rejecting stale/wrong-mode commands without altering it, and explicit programmatic
cancellation releasing capture.

The public `examples/numeric/main.exe --self-test` passes through the actual
OCaml/Rust bridge: initial observation, exact read, guarded replacement, stale
revision, wrong-mode/thumb failures, focus, idle cancellation, disabled replacement,
unmount, remount with fresh initial values, old-controller rejection, cross-lease
guarded replacement rejection and closed-window failure. It exits zero and emits
`GPUIO_SLIDER_PUBLIC_OK`; the native suite also exits zero. Both processes/windows
are reaped. The example remains interactive without `--self-test`.

Final local macOS arm64 checks pass with `GPUIO_JOBS=2`:

```sh
./scripts/gpuio exec dune build -j 2 @all @test/view_api/runtest @fmt
./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol
./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol -p gpuio-native --test slider
./scripts/gpuio exec cargo clippy --locked -j 2 --workspace --all-targets --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests -- -D warnings
./scripts/gpuio exec cargo fmt --all
```

The native executable was built using `native-tests`; GUI subprocesses used a
45-second timeout. The new public example is wired into the future macOS CI run.
Hosted macOS/Linux checks remain pending. Expanded slider style/theme/pixel/scale,
idle/deactivation/lifetime/workload and public external-AX coverage remain, as do
numeric editor/stepper and OTP implementation. No OCH-34 capability is advertised.

## Slider appearance and public mode coverage

The native rail/thumb artwork now resolves `Style.Foreground` during paint. The
host supplies its default accent before application refinements. Focus-state
styles apply to either focused thumb; a separate geometric ring identifies that
thumb. Disabled appearance uses the existing host opacity. This removes the
previous hardcoded child colors without changing ownership, commands or wire data.
The public numeric example now includes horizontal single/range linear controls
and vertical single/range logarithmic controls, with live observed values.

Local `native_slider` with `native-image-tests` checks actual rendered RGBA
pixels for light/dark palettes, selected/unselected rails, both thumb centers,
focus color and geometric ring, both axes and synthetic 1x/1.5x/2x density. It
also verifies disabled dimming and constrained 24/32/56px layouts without value
reset. Screenshots were inspected locally. These are component appearance
fixtures, not a claim that the polished milestone-05 chat showcase is complete.

The density harness draws synchronously immediately after setting the test scale:
an intervening AppKit bounds notification can otherwise restore physical density
between awaited frames. Early failures also corrected the fixture's focus-state
wire index and moved a ring sample off the antialiased outer edge. None required
an upstream GPUI patch. The existing native keyboard/pointer/AppKit accessibility
suite passes with the new artwork. Expanded workload/window lifecycle acceptance
and numeric editor/stepper/OTP work remain pending.

Validation at this checkpoint (local macOS):

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @test/view_api/runtest @fmt` — passed.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-image-tests --test native_slider --no-run` — passed; the resulting executable ran directly under a 45-second subprocess timeout and passed. This includes the idle render-count assertion after settling the visible slider.
- `_build/default/examples/numeric/main.exe --self-test` — passed with the expanded four-control example mounted, including the original range controller's lease/revision/close checks. Other mode controls are interactive examples; this self-test does not itself prove external keyboard/AX coverage of each one.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 --workspace --all-targets --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests -- -D warnings` — passed.

Owned GUI processes exited and were reaped. CI now explicitly prebuilds the slider
image-test variant on both platforms; hosted execution/merge remains pending.
No Linux GUI coverage or completion of OCH-34 is claimed.

## Styled geometry, public OS input and window lifetimes

A native regression exposed a real pointer bug with asymmetric borders: clicking
20% along the laid-out rail selected `1` instead of `0` in the `[-2, 8]` domain.
The pointer calculation had approximated the rail from the outer box. Native
state now records the actual rail bounds during prepaint and uses those bounds
for all pointer mapping. Changing rail geometry during capture cancels the drag
and restores the committed value even if the outer size remains unchanged.
Horizontal/vertical decorated-layout tests now pass, including late-release
suppression after reconfiguration.

`scripts/test_numeric.py` launches the public Bonsai/Eio example and uses external
macOS accessibility APIs plus OS keyboard events. It passed locally for all four
examples: independent horizontal range thumb traversal, AX values/min/max/set,
single linear and logarithmic keyboard stepping, logarithmic range actions and
non-crossing bounds, read-only input suppression, explicit replacement while
disabled, unmount/remount initialization and close. An observed native change is
also checked in the OCaml-rendered value text. The example now exposes a read-only
toggle. The script always reaps its own child; CI runs it alongside the existing
presentation automation.

Expanded native tests passed for capture loss, ancestor pointer exclusion,
minimization during a drag, no settled minimized-window renders, restoration with
late-release suppression, actual two-window deactivation, independent sliders
using the same node/handler slots in different windows, closing a window during
capture, weak-owner reclamation and input in the surviving window.

The first two-window harness stalled because it unnecessarily queued activation
of an already-active first window. GPUI's platform activation is asynchronous;
that queued request ran after the second window opened, bringing the first back
in front. The harness now requests activation only when needed and settles the
first frame before opening the second. No production focus or upstream change was
needed. Window activation and rendering were verified after this correction.

Validation (local macOS):

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @test/view_api/runtest @fmt` passed for the production geometry/public example changes.
- Native `native_slider` built with `native-image-tests` and passed under a 30-second subprocess timeout, including the previous GPU/keyboard/AX regressions plus decorated geometry, capture, minimize/restore and two-window checks.
- `python3 scripts/test_numeric.py` passed; owned child exited normally.
- Workspace/all-target Clippy with combined native image/canvas features passed; Rustfmt and diff whitespace checks passed.

All GUI children are terminal and reaped. Bounded many-owner workloads and the
remaining numeric editor/stepper/OTP families are still pending. Hosted CI, merge
and Linux desktop GUI acceptance remain outstanding; OCH-34 is still In Progress.

## Mounted slider workload

`slider_workload_test.rs`, included in `native_slider`, passed three cycles of
1,024 retained range sliders / 2,048 thumb focus handles in the actual native
window. The ordinary layout includes offscreen owners; this is not a claim that
all 1,024 controls fit onscreen. Mounts are admitted in 64-owner batches with event
drains between them, below the shared input lane's 128 discrete-event capacity.
It does not promise that one arbitrarily large atomic mount can bypass overload
policy. Fresh slots are consecutive and reused slots advance generations.

Every cycle checks all mount observations, 64 sequential keyboard actions with
native commits and no tree transactions, and a 2,000-move burst through GPUI's
native event dispatch before a frame boundary. The burst produces exactly three
queued observations: drag start, latest preview, final commit. Stationary owners
produce no events or idle renders. After removal and settled frames, all 1,024
weak owner references expire, the native owner map is empty, and session payload
accounting returns to zero. The mounted payload accounting is 142,336 bytes in
each cycle; that figure is not total process RSS or renderer/allocator memory.

One local macOS debug run measured:

| Cycle | Mount + settled frame | 64 sequential keyboard events + frame | Complete cycle |
| --- | --- | --- | --- |
| 1 | 509.973 ms | 2.857 s | 3.968 s |
| 2 | 483.037 ms | 2.900 s | 3.979 s |
| 3 | 482.791 ms | 2.900 s | 3.995 s |

That is roughly 45 ms per sequential keyboard event in this deliberately large,
unvirtualized debug tree. It is not a 60-fps or release-build latency guarantee.
Use managed lists for large scrolling control collections; final application
performance/release validation remains part of integrated showcase/OCH-17 work.
The initial stress harness issued each of 2,000 synthetic moves in a separate
outer application update. Sampling showed a full tree redraw after each one;
it exceeded the 60-second test timeout and was killed/reaped. The final test
separately measures sequential input cost and queue pressure from a burst, while
retaining the full 2,000-event coalescing assertion. No performance claim is based
on the timeout run.

The complete native image-test slider suite passes with the workload included,
covering the earlier visual/input/geometry/window-lifetime regressions as well.
All children exited normally on the passing run. Numeric editor/stepper/OTP
implementation remains pending, so no OCH-34 capability or ticket completion is
claimed. The detailed Number_input target in the design document records the
next implementation's draft/commit/IME/undo/revision/stepper contracts.

## Numeric editor contracts and standalone codecs

Added Core `Number_input` value/configuration/revision/snapshot/event/command
contracts and matching internal OCaml/Rust representations. A snapshot records
its own domain, separating historical classification from current configuration.
It preserves a native draft independently of the normalized committed value,
UTF-8 selection direction and composition, focus and numeric observation revision.
Explicit replacements reuse the public text-input selection/history policies;
finite values, owner-bound snapshot conversion and semantic boundaries are validated.
These types do not yet mount or edit a numeric widget.

Six independently constructed bin_prot fixtures cover configuration, Unicode
observation, commit, guarded draft/value replacements and a rejection response.
The fixtures use domain `[-2,8]` with step `0.5`, an observed `é1e-` draft with
backward byte selection `5..2` and composition `2..5`, and a settled Stepper
commit of `2.5`. The two replacements exercise different selection/history
policies and numeric revisions. Both encoders match the independent byte strings.
Native decoding rejects every proper fixture prefix, trailing bytes, invalid
UTF-8/tags/Booleans, non-finite or unnormalized committed values, invalid byte
boundaries, reversed composition, inconsistent commit/cancel/rejection events,
negative guards and bounded-message overflow. Maximum valid payloads also pass.

Core expect tests cover the same public invariants, owner identity, classification
of transient/invalid/out-of-range drafts, historical domains and command
conversion. The first run's only expectation difference was Sexplib's escaped
UTF-8 display (`é` becomes `\195\169`) and line layout; those diffs were reviewed
before promotion. An initial compile error required explicit conversion of the
public `Numeric.Direction` constructors to the wire type across its `.mli` boundary.
Neither fix changed numeric behavior or weakened a validation assertion.

Native number-input/stepper integration, top-level bridge envelopes, Eio
controllers, actual editing/IME/AX validation and OTP remain pending. No new
capability, native-widget completion, hosted CI or Linux GUI evidence is claimed.

Local macOS validation passed for this contract checkpoint:

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @test/view_api/runtest @fmt`
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol`
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-protocol --all-targets -- -D warnings`
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all -- --check`

No GUI tests were needed for this type/codec-only checkpoint; no GUI process was
started. Native slider evidence above is from the preceding mounted-widget runs.

## Native numeric policy owner

`number_input_state.rs` now wraps a native editor observation with numeric policy:
validated configuration, normalized committed value, revision guards, commit/
cancel/step and typed responses. It does not own a second editor/history or run
any timer. It synchronizes pending edits before commands and configuration
changes, reserves revision capacity before invoking an editor callback, validates
its final state and suppresses a later duplicate notification. Each operation
returns at most two ordered events; semantic boundaries remain distinct from
ordinary changes. Invalid native observations or inconsistent callback success
cause a sticky fault that the mounted adapter must propagate to window input.

Twelve deterministic model tests pass:

- Initial normalization and decimal round trips include signed zero, the largest
  finite magnitudes, the smallest normal/subnormal magnitudes and wide exponents.
- Pending edits make stale replacements fail; final observations are deduplicated
  while selection direction and incomplete/Unicode drafts remain intact.
- Incomplete/invalid/non-finite/disallowed-empty commits reject without editing;
  finite values clamp/round and repeated commit attempts have distinct revisions.
  Blur preserves the draft and committed value.
- Steps normalize before advancing, seed empty drafts at normalized zero without
  another step, reject invalid drafts and saturate at endpoints.
- Stubbed native undo restores a draft while retaining the committed value;
  cancel restores committed text with the correct semantic reason.
- Configuration changes preserve draft, directional selection and composition
  while normalizing the committed value. Native observations precede the new
  domain observation. Mutating commands cannot discard marked text.
- Disabled/read-only gate user mutations and all Step commands; explicit
  replacements/Commit/Cancel retain their accepted programmatic semantics.
- Text/selection/value validation occurs before the editor callback; Preserve
  selection is checked against UTF-8 boundaries in the replacement.
- Command revision exhaustion prevents native mutation/config publication;
  ordinary-observation exhaustion faults without rewinding a native edit.
- Native command failure cannot publish numeric success. Inconsistent native
  success or malformed/regressing observations fault the owner permanently.
- Allowed empty commits are semantic boundaries. Making a field required does
  not invent a number or rewrite its draft; a later empty commit rejects.
- A 256-cycle mixed edit/commit/step/cancel/domain-update sequence maintains valid
  snapshots/events, monotonically ordered revisions and bounded publication.

The editor-result stubs deliberately do not simulate a complete input widget or
history implementation. These tests verify numeric policy and adapter contracts;
actual GPUI InputState undo/redo, typing/paste/IME, focus, stepper repeat, AX and
mounted lifetime evidence still need the retained/native adapter implementation.

Local macOS checks passed:

- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo check --locked -j 2 -p gpuio-native`
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --lib` (159 tests, including 12 numeric-policy tests)
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-native --all-targets -- -D warnings`
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all -- --check`

No GUI process was started. Retained numeric views/envelopes, mounted editor and
stepper, public controller, OTP, hosted checks and ticket completion remain open.

## Retained numeric descriptions and observation bridge

Core/Bonsai `View.number_input` now reconciles a stable controller/config/initial
seed and callback through Kind 38 and Set_number_input operation 44. Numeric
observations use Event 46. These tags append to the existing protocol. Native
tree admission enforces the leaf/configuration/handler contract and accounts for
all four configuration strings; invalid updates roll back without changing the
published revision or retained bytes. Removing a node releases its config and
returns payload accounting to zero.

Native and OCaml routing reject stale generations, wrong owners/handlers,
negative/future tree revisions and malformed numeric events. They retain a
queued observation's original domain even after bounds/disabled/read-only changes.
Reconciliation preserves the latest callback and numeric revision fence across
pending updates, rejects duplicate controllers atomically, and fences unmount,
remount and close. Generic Press cannot invoke a numeric callback.

The mailbox coalesces only adjacent ordinary Changed events with matching routed
identity, domain and committed value. Tests preserve observed/committed/rejected/
cancelled and response boundaries. A 10,000-change burst retains its final change
between discrete boundaries. Capacity tests cover replacement at the event-count
limit, rejection of discrete input when full, draft-byte growth during coalescing,
retention of the old queued event after rejected growth, bounded drain batches,
and accounting recovery after drain. Numeric draft bytes count both on admission
and when estimating outgoing batches. Undrained numeric output keeps its window
slot marked in use.

The initial byte-pressure fixture used repeated editor Changed events as filler;
those events coalesced, leaving only about 258 KB rather than filling the 4 MiB
input budget. Temporary accounting diagnostics identified the fixture mistake.
The corrected test uses discrete editor Submitted events and asserts that another
small event is refused before testing numeric-event growth. Diagnostics were
removed. The passing test now exercises actual queue pressure; its expected
rejection was not weakened.

Two additional independent bin_prot fixtures cover a 93-byte retained request and
56-byte observation envelope. Both language encoders agree; decoding tests cover
all truncated prefixes, trailing data and semantic failures. The first OCaml test
compile needed the list writer for a batch (the individual event writer was the
wrong test helper). Whole-repository compilation also identified two low-level
example event matches that needed the new ignored event case; exhaustiveness
warnings remain enabled.

Passed locally on macOS:

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @test/view_api/runtest @fmt`
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol -p gpuio-native --lib --tests`
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-protocol -p gpuio-native --all-targets -- -D warnings`
- Rustfmt and diff whitespace checks.

No GUI window was opened for this bridge checkpoint. A retained description is
not a mounted numeric editor: native InputState/stepper rendering, command/result
envelopes, Eio controller and actual editing/IME/AX/lifetime acceptance remain
pending, followed by OTP. No OCH-34 capability, ticket completion, hosted gates or
Linux GUI acceptance is claimed.

## Mounted numeric editor and command/controller checkpoint

The native adapter now mounts a single InputState with the numeric policy owner.
It uses weak native callbacks, includes numeric fields in native text-command
routing, and preserves the editing session on configuration changes. Initial
autofocus is captured before the first numeric observation. Message 15 and Event
47 carry correlated commands/results; Eio caps pending requests at 64, validates
drafts before encoding, checks reply identities and resolves closed-window work.
The public `Gpuio_eio.Number_input` controller and explicit replacement/guard
helpers compile. Independent command/reply fixtures agree in OCaml and Rust.

The actual native test `native_number_input` passes locally on macOS:

- One numeric editor without a duplicate generic editor registration.
- Initial autofocus; incomplete `-` rejection; Escape restores committed text;
  Enter clamps `99` to 8; native Up/Down step and produce keyboard commits.
- Actual InputState undo/redo restores draft history while leaving the committed
  value unchanged. A text insertion invalidates an older numeric revision guard.
- GPUI-dispatched Cmd+A/C/V copies native selection and pastes incomplete `1e-`
  without parse-to-zero behavior. The harness restores the prior clipboard.
- Changing bounds/step/control presentation preserves draft and caret, normalizes
  the committed value, and emits an observation.
- On macOS, actual NSView NSTextInputClient marked text produces composition;
  explicit commit/cancel reject it. Enter preserves it; the first Escape ends
  composition without resetting the draft and a second Escape restores the value.
- Read-only stepping is blocked while explicit replacement remains allowed.
- Removal releases both weak native references, clears the instance map and
  returns retained payload accounting to zero. The window/process exits normally.

This uses native GPUI keyboard dispatch and AppKit text-client methods, not an
external OS-keyboard automation run or a human-selected input-method session.
Native compilation initially needed explicit protocol Style/Length imports to
avoid their GPUI namesakes. The OCaml envelope test initially used a nonexistent
Event.encode helper; corrected to the existing generated batch writer. No test
expectations were promoted to bypass those compile failures.

The stage does not establish pointer hold-repeat, actual numeric AX actions,
visual/pixel acceptance, the public application's end-to-end event lifecycle,
large workloads or all hide/modal/window-loss policies. Those remain open, along
with OTP and consolidated hosted checks. No ticket/capability completion or Linux
GUI acceptance is claimed.

A final integration review added focused/composing numeric owners to managed-list
row pinning. The native test now mounts its number field as a managed virtual
row, verifies the pin and rejects a stale guarded eviction without changing the
tree revision. All editing/IME/disposal scenarios also pass in that placement.
The first list fixtures omitted the separate Splice child operation when adding
and evicting the row, so strict tree validation rejected those fixtures. Both
were corrected to preserve the metadata/children invariant; the retained-row
expectation was kept.

Validation for this checkpoint (local macOS):

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @test/view_api/runtest @fmt` passed for the new API/envelopes.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol -p gpuio-native --lib --tests` passed before the final focused/composing row-pin addition.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-tests --test native_number_input --no-run` rebuilt the final native fixture. Running the resulting `native_number_input-*` executable with a 60-second process-group timeout passed normally, including the final managed-row regression; no timeout or forced termination occurred.
- Clippy with `-p gpuio-protocol -p gpuio-native --all-targets --features native-tests -- -D warnings`, Rustfmt and whitespace checks pass.

Owned test windows and subprocesses are closed/reaped. Hosted validation and
merge remain deferred to the consolidated milestone submission.

## Native numeric step-button hold-repeat

`number_input_repeat.rs` implements captured Sides/Stacked button gestures. One
immediate step is followed by a native 400 ms delay and 75 ms repeat interval;
all steps use the existing numeric commit/history/event rules. The task holds
weak references and a per-gesture lifetime token. It is dropped on termination,
including invalid/composing drafts and reaching a bound. There is no idle
numeric-repeat task; native caret blinking/Bonsai scheduling are separate.

The actual native numeric suite now also passes:

- Immediate and repeated Stepper commits; capture survives repaint/hitbox rebinding
  and label/color-only updates. Both Sides and Stacked presentations work, including
  increasing and decreasing from separate stacked buttons.
- Fifteen cancellation cases: pointer leave, capture loss, Escape (including its
  normal Cancelled event), explicit draft replacement, read-only, disabled, hidden
  step buttons, disabled pointer input, hidden field, changed button geometry,
  native text insertion, focus loss, keyboard stepping, changed numeric domain,
  and rebinding the event handler. Late releases preserve the resulting value.
- Incomplete drafts and actual macOS marked text reject a button step without
  starting a timer or discarding draft/composition. A step reaching max stops.
- A second real window deactivates the held owner; returning to the first window
  and delivering its late release does not resume or mutate the cancelled gesture.
- After release/deactivation, native tick counts stay unchanged beyond the initial
  delay plus interval, and both task and capture are absent. Removing an actively
  held owner releases its editor/owner weak references and all retained accounting.

The first repeat run exposed a mismatch between dispatched pointer positions and
GPUI's separately polled physical cursor position. The timer initially cancelled a
valid captured gesture because those positions differed. Captured mouse motion
now determines leaving, while capture/policy/geometry/window checks remain.
A subsequent blur assertion assumed immediate notification; GPUI queues focus
notifications with painting. The adapter now checks actual focus before timer
steps and during prepaint, and the test waits for acknowledged frames. Handler
rebinding also now updates the retained route rather than leaving its owner stale.

One expanded run hit the 60-second outer timeout without diagnostic output; its
process group was terminated and reaped. A traced rerun passed all stages. The
numeric suite now bounds each two-frame paint acknowledgement to six 500 ms
attempts, refreshing and reporting activation on retry; exhaustion fails through
normal cleanup. The final run passed without a frame retry. The original stall's
cause was not established; this is not a claim to fix GPUI frame scheduling.

The final native run prints `GPUIO_NUMBER_REPEAT_OK` and
`GPUIO_NUMBER_INPUT_NATIVE_OK`, and closes/reaps its windows normally. CI now
builds this numeric suite on both target platforms and includes its macOS native
execution in the consolidated workflow. Hosted checks have not run yet. Numeric
AX/visual/public-application acceptance, broader workloads and OTP remain open.

Local checkpoint validation:

- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-tests --test native_number_input --no-run`, then the resulting native executable under the 60-second process-group guard: final native suite passed.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --lib --test number_input`: 159 native unit tests and four numeric bridge tests passed.
- All-target Clippy with `-D warnings` passed for both `native-tests` and the CI `native-image-tests` configuration.
- Rustfmt and diff whitespace checks passed. No OCaml types or wire layouts changed in this checkpoint.

## Native numeric accessibility

The native numeric suite now walks the actual AppKit accessibility objects and
invokes their actions. `number_input_accessibility_test.rs` runs before the
existing repeat suite in the same bounded native test executable. Locally on
macOS it passes:

- AXTextField draft text and required/editable/enabled state, and AXIncrementor
  NSNumber value, min/max and increment/decrement support. Actual accessibility
  focus restores the editor after blur.
- Sides, Stacked and Hidden presentations: increment/decrement commits and visible
  button press actions; Hidden retains incrementor actions without button nodes.
  Events identify Accessibility as the commit source.
- Native SetValue preserves transient `-`, Unicode-invalid text, `1e999` and `99`
  without committing. Help describes each error or commit-time clamping. The
  incrementor preserves these strings rather than exposing a false numeric value.
  A step from `99` clamps/commits to max and clears automatic feedback; empty input
  remains an empty string rather than zero.
- Real NSTextInputClient marked text remains intact when accessibility edits and
  steps are attempted. Automatic syntax feedback is suppressed while composing;
  application help/errors and a changed label remain available without resetting
  composition. Application and automatic errors combine after composition ends.
- Read-only and disabled states remove advertised edit/step actions. Forced native
  requests preserve draft and committed value; read-only remains enabled. Hiding
  the field removes both native roles.

The first run passed the earlier role/action/draft/IME assertions, then rejected
an invalid test fixture that combined field metadata with top-level description.
The fixture was corrected to put its help in the field record, preserving the
existing protocol rule. Pure metadata tests now use valid application configs for
both field and plain semantics. No protocol validation was weakened.

Validation commands (isolated local macOS toolchain):

- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-tests --test native_number_input --no-run` passed.
- The resulting native executable passed under the 60-second process-group guard,
  printing `GPUIO_NUMBER_AX_OK`, `GPUIO_NUMBER_REPEAT_OK` and
  `GPUIO_NUMBER_INPUT_NATIVE_OK`. It exited normally and all owned windows/processes
  were closed/reaped.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --lib --test number_input`: 160 unit and four numeric bridge tests passed.

This establishes native AppKit object/action behavior, not external OS accessibility
automation, human VoiceOver speech or Linux desktop behavior. Numeric visual/public
application/workload acceptance and OTP remain open. No new capability, hosted
validation or milestone/ticket completion is claimed.

All-target Clippy (`--features native-image-tests -- -D warnings`), Rustfmt and
diff whitespace checks also passed for this checkpoint.

## Public numeric editor example and bridge integration

`examples/numeric/number.ml` now demonstrates Sides, Stacked and Hidden numeric
editors through public Core/Bonsai/Eio APIs. It renders draft and committed value
separately, with explicit commit/restore/reset/read and policy/lifetime controls.
The existing slider example remains a separate executable in the same directory.
No private bridge path or synchronous native-to-OCaml callback is used.

The local macOS `--self-test` passes actual correlated commands and asynchronous
observations/events for all three layouts. It checks initial values, guarded
replacement/stale revision, transient rejection/cancel, out-of-range clamping,
stepping, undo/redo, directional UTF-8 selection, invalid selection/text/length,
required-empty rejection and zero-seeding step. It also verifies an unplaced
controller returns Not_mounted, disabled/read-only policy changes, permitted
explicit replacement while disabled, stale leases after removal and remount,
initial-value restoration on remount, old-snapshot fencing, and Closed after
window closure. Rejected, Cancelled and Committed semantic events reach the
application callback through the real runtime.

The first test expected Disabled for a focus command. Inspection confirmed that
the accepted numeric model/gate returns Focus_blocked for focus denial, while
stepping returns Disabled. The test and API documentation now state that existing
contract; production behavior was not altered to accommodate the test. Failure
diagnostics include actual results and expected values.

Commands: `GPUIO_JOBS=2 ./scripts/gpuio build examples/numeric/number.exe`, then
`_build/default/examples/numeric/number.exe --self-test` under a 45-second
process-group timeout. The final run passed normally with `GPUIO_NUMBER_PUBLIC_OK`
and closed/reaped its window/process. Formatting passed. The foundation workflow
now runs the public numeric self-test on macOS; hosted execution remains pending.
This is bridge/lifecycle evidence, not external OS keyboard/AX automation or pixel
acceptance. Broader numeric visual/workload/policy acceptance and OTP remain open.

The final `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @fmt` passed,
as did workflow YAML parsing and diff whitespace checks.

## Numeric GPU appearance and external desktop acceptance

`number_input_appearance_test.rs` extends the actual native test under
`native-image-tests`. GPU readback passes supplied light/dark foreground and
background colors, focused/unfocused border pixels, native editor text ink, all
three step-control layouts and synthetic scale factors 1 and 2. Draft text stays
intact across those changes. Geometry checks also pass 80/180 px widths and
40/64 px heights with borders, positive editor dimensions, contained step-button
bounds and no overlap with editable text.

The tests found an actual stacked-layout defect: inheriting the editor's line
height made each arrow button 26 px tall and overflowed a 48 px field. Stacked
buttons now have compact glyph metrics and stretch within their column; fields
have a 40 px default minimum height. The existing repeat geometry test now changes
height explicitly as well as width: after the minimum-height fix, changing only
the right edge no longer moved the held left button. Its contract remains
cancellation when the held button's geometry changes.

An initial geometry assertion also mistook the existing content-box probe for an
outer border-box measurement. The test now accounts for the known 2 px border;
production measurements were not changed. Actual saved renders were inspected
for all three layouts and both palettes. Representative unmodified GPU outputs:
[light Sides](../images/number-light-Sides.png) and
[dark Stacked](../images/number-dark-Stacked.png). These are deliberately simple
control fixtures, not screenshots of the final chat showcase.

`python3 scripts/test_number_input.py` also passes against the public OCaml
example using external macOS AX objects and OS keyboard events. It verifies
initial values, transient invalid feedback, Return rejection/commit/clamp,
Escape restoration, arrow stepping, incrementor and visible-button actions,
application-rendered committed observations, read-only/disabled policy,
programmatic reset while disabled, removal/remount and closure in all layouts.
This supplements the in-process AppKit suite and public command self-test; it
still does not claim human VoiceOver speech, external IME candidate-panel testing
or Linux desktop acceptance.

Local validation:

- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-image-tests --test native_number_input --no-run` passed.
- The native executable passed normally under a 60-second process-group guard,
  printing AX, APPEARANCE, REPEAT and INPUT_NATIVE success markers. To save the six
  palette/layout images, set `GPUIO_NUMBER_SCREENSHOTS` to an existing directory.
  Density changes are synthetic; no physical monitor-scale transition is claimed.
- `GPUIO_JOBS=2 ./scripts/gpuio build examples/numeric/number.exe` passed; the external
  Python desktop test passed under a 100-second process-group guard and printed
  `GPUIO_NUMBER_APP_AX_OK`.
- All-target Clippy with `--features native-image-tests -- -D warnings` and Python
  syntax compilation passed. Owned GUI processes/windows closed and were reaped.

CI runs the GPU numeric suite through the existing native-image-tests lane and
now includes the external numeric editor script. Hosted execution remains pending.
Remaining OCH-34 scope includes broader workload/lifetime/policy acceptance and
OTP; no new capability or completed ticket is claimed.

Final `dune build -j 2 @all @fmt`, Rustfmt, workflow YAML parsing and diff
whitespace checks passed through the isolated toolchain.

## Numeric policy, window-close and workload acceptance

The expanded native numeric suite passes the remaining retained-state scenarios:

- Real AppKit marked text survives simultaneous domain, label, placeholder,
  step-layout and style changes with the same editor entity, draft, selection,
  composition, focus and history payload. The committed value normalizes under the
  new domain separately. Escape unmarks first; explicit cancel restores the newly
  normalized committed number.
- Hiding an ancestor during a captured step cancels the task/capture and blocks
  focus, while explicit hidden draft replacement and state reads remain usable.
  Showing the ancestor preserves both editor identity and updated draft.
- Adding a trapping focus scope cancels a held stepper, denies background focus
  and the accessibility-source action route, and prevents late pointer release
  or keyboard input from mutating the background field. Removing it preserves
  the editor and allows focus again. This particular gate assertion calls the
  adapter route directly; actual AppKit actions are covered in the earlier suite.
- A second real window mounts the same node ID with an independent value. Closing
  that window while its own stepper is held releases both owner and editor weak
  references and removes its session tree; the original window retains its value.

`number_input_workload_test.rs` passes three cycles of 256 retained numeric editors,
with 32-owner mount batches and no duplicate ordinary-editor owners. Each cycle
checks 32 keyboard commits, 1,024 alternating maximum-size (4,096-byte) explicit
drafts, the 2 MiB history payload limit, undo after cancel, reset-to-zero history,
no native-tree transaction for native input, and no idle repeat tasks. Draft bursts
retain one latest adjacent Changed observation before the cancel boundary; undo,
explicit value replacement and focus loss retain their correct subsequent event
boundaries. Stationary unfocused owners produce no renders or bridge observations
over the measured settled idle interval. Removal releases every weak owner/editor
and all retained tree accounting.

The first workload assertion allowed four events but omitted the legitimate
focus-loss Changed observation after the explicit reset's Observed boundary.
The final assertion checks the four expected event types and payloads plus only
that optional fifth focus-loss observation; it does not permit uncoalesced drafts.

Final local debug-build measurements on this macOS host:

| Cycle | Tree payload bytes | Mount 256 editors | 32 keyboard events + frame | Whole cycle |
| -- | -- | -- | -- | -- |
| 1 | 51,968 | 204 ms | 1.242 s | 3.979 s |
| 2 | 51,968 | 208 ms | 1.129 s | 3.771 s |
| 3 | 51,968 | 209 ms | 1.145 s | 3.789 s |

Payload accounting excludes native editor/history/renderer allocations; it is not
an RSS estimate. Keyboard timing includes synthetic event/update/redraw boundaries
in an unoptimized build and is not a 60 fps or production latency promise. These
measurements establish the tested workload and cleanup behavior; simultaneous
chat integration remains OCH-46.

Validation: isolated Cargo native-image-tests numeric build and the resulting
executable under a 60-second process-group timeout passed with AX, APPEARANCE,
POLICY, REPEAT, three WORKLOAD and INPUT_NATIVE success markers. No timeout or
frame retry occurred in the final run. All-target native-image-tests Clippy with
`-D warnings` passed. Owned windows/processes closed and were reaped normally.
These test modules are included in the existing numeric CI executable; hosted
execution remains deferred to consolidated milestone validation.

Numeric editor/stepper local acceptance now joins the completed slider family.
OTP remains to be implemented and validated before advertising the OCH-34
capability or closing the ticket. Consolidated hosted macOS/Linux gates and merge
also remain pending; no Linux GUI acceptance is claimed.

All-target Clippy also passed with the smaller `native-tests` feature set;
Rustfmt and diff whitespace checks passed. This checkpoint changes tests/docs,
not the public API, wire layout or production numeric behavior.

## OTP text-policy foundation

The public Core Policy/Value interface now has OCaml and Rust implementations for
1..32 cells, digit/alphanumeric policies, full-width-to-ASCII normalization,
case preservation, paste separators, UTF-8/raw-byte bounds and atomic directional
selection replacement. Empty/separator-only paste preserves selection and value;
direct empty replacement deletes the selection. Values remain policy checked and
errors contain offsets rather than code text. See [OTP design](../design/otp-inputs.md)
for the precise implemented contract and remaining native requirements.

Local Dune view API expect tests and formatting passed. Tests check invalid
lengths, full-width values, byte offsets, unexpected Unicode, overlength/invalid
UTF-8/oversized input, paste behavior, replacement/caret direction and capacity
rejection, unchanged original values and independent policy encodings. The first
compile used a nonexistent test-only Result helper; it was corrected to ordinary
Core error mapping. No expect outputs were promoted to hide failures.

`cargo test --locked -j 2 -p gpuio-protocol otp::tests` passed all three focused
Rust tests. They also exhaust the full-width digit/Latin ranges and replacement
positions for every supported code length. Other tests in that filtered invocation
were not run and are not counted as passed. OCaml/Rust policy encoding fixtures
agree on `0600` and `2001`.

This is the text foundation only. No native OTP widget, event/command lane,
segmented rendering, IME integration, controller or capability is claimed yet.

Protocol all-target Clippy with `-D warnings`, Rustfmt and diff whitespace
checks also passed. No GUI process was needed for this pure policy checkpoint.

## OTP editing model

`rust/native/src/otp_edit.rs` now provides one bounded editing session intended
for a dedicated GPUI `EntityInputHandler` adapter. The generic editor's shared
validator/history and ordinary glyph geometry are unsuitable for the segmented
IME contract without broader changes. No shared editor behavior was changed.

The model retains canonical text separately from temporary preedit. Valid IME
commits normalize atomically into one undo edit; rejected commits restore the
original code and exact directional selection without evicting undo/redo.
It supports exact UTF-16/UTF-8 conversion, explicit native replacement ranges,
selection within preedit, unmark/cancel, distinct paste normalization, cell
navigation/deletion and guarded Record/Reset programmatic replacement. History
is bounded to 128 edits across both stacks (at most 8,192 bytes of code payload).
The composing draft is independently bounded to 4,096 bytes. See the updated
[OTP contract](../design/otp-inputs.md) for event-owner responsibilities and limits.

Local macOS command:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib otp_edit::tests -j 2
```

All 11 focused tests pass. They cover normalized/rejected edits and completion
eligibility, exact rollback and one-step IME undo/redo, history preservation under
100 repeated long preedit updates, total draft bounds, control characters,
surrogate-interior rejection, partial marked replacements, navigation/deletion,
composition command denial and atomic history reset. An exhaustive selection/
insertion comparison checks every length 1..32 under both alphabets against the
protocol's atomic edit contract. UTF conversion also checks joined emoji and
combining characters. These tests call the pure model; they do not establish
native IME, candidate placement, accessibility or GUI acceptance.

All-target native Clippy with `native-tests` and `-D warnings` also passes. No GUI
process was opened. OCH-34, native OTP integration and hosted gates remain open.

## OTP public contracts and standalone codecs

The public Core API now includes configuration, nonnegative revisions, opaque
window/node-bound snapshots, Observed/Changed/Complete/Rejected events, explicit
replacement/clear/selection/focus/history/composition/read commands and typed
command errors. Snapshots distinguish accepted code from preedit and expose
directional selection and undo/redo availability. Public interfaces specify
immutable placement policy, user-only completion ordering, masking/clipboard
behavior, revision guards and mutation permissions; native execution of these
contracts is still pending. See [OTP design](../design/otp-inputs.md).

Matching OCaml/Rust wire types and standalone bounded Rust decoders now cover
configuration, events, commands and responses. Independent fixed byte fixtures
pin a masked configuration, Unicode composing snapshot, completed snapshot,
guarded replacement, clear, every command/error tag and a rejected event with a
signed bin_prot byte offset of 4,095. These codecs do not yet register native
node/operation/message/event envelope tags.

Validation on local macOS:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @test/view_api/runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-protocol -j 2
```

Both commands pass. The full Rust protocol suite includes four new OTP test
groups; the OCaml suite adds three contract expect tests. Tests cover public
construction/leases, historical policy, malformed Unicode boundaries, impossible
completion/rejection states, noncanonical values, exact selections, negative
guards, maximum payloads, every truncation point of the fixed fixtures, trailing
data and bounded declared lengths. Malformed UTF-8, negative error offsets and
invalid policy/Boolean cases include their complete remaining payload so they
cannot pass merely because a required field was missing.

An added hand-written rejection fixture initially contained one extra Boolean
byte. Both language tests caught the mismatch; the fixture was corrected against
the documented field layout. Codec behavior and expect output were not changed
to accommodate the incorrect fixture. Formatting promotions affected layout only.

Protocol all-target Clippy with `-D warnings`, Rustfmt and whitespace checks pass.
No native window was launched. GPUI platform callbacks, revision/event ownership,
retained transport envelopes, segmented rendering, public controllers/examples
and actual local native acceptance remain necessary before completing OCH-34.

## OTP native state owner

`rust/native/src/otp_input_state.rs` now executes the public command/event
contract around the bounded editing session. It owns configuration, revision and
focus observations; reserves event capacity before mutation; keeps programmatic
Observed separate from native Changed/Complete; and rejects policy changes,
stale replacements, invalid selections and disallowed edits atomically.

Native access uses current disabled/read-only policy and an adapter-supplied
visibility/modal gate. Configuration updates retain preedit, selection and
history. Unmark after editing is gated restores the original checkpoint; empty
preedit/lifecycle cancellation also works after disabling. Masked/empty copying
and cutting leave the clipboard alone, read-only selection/copy remain possible,
and a failed Rust clipboard callback cannot delete text. Focus success requires
confirmation by a Rust callback. These callbacks have no OCaml reentry.

Local validation:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native --lib otp_ -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native --all-targets --features native-tests -j 2 -- -D warnings
```

All 24 focused tests pass (13 owner tests plus the 11 editing tests); the other
160 native unit tests were filtered out, not rerun. Native all-target Clippy
passes. Owner tests cover completion pairs with consecutive revisions, no
completion for programmatic/unchanged values, user versus programmatic history,
digit/alphanumeric paste, rejection/IME rollback, scalar-boundary errors,
configuration retention, permission/focus/clipboard admission, selection policies,
stale and invalid commands, and signed-64-bit exhaustion before side effects.
A coalescing model preserves Complete/Rejected/Observed boundaries. Native
transport queues still need their own identity, batch-admission and lifetime
tests; a pure coalescing test does not establish those properties in the bridge.

No GPUI window, OS clipboard, IME or accessibility service was exercised in this
checkpoint. The owner is ready for retained/native integration; rendering,
platform callbacks, controller/event delivery and actual acceptance remain open.


## OTP retained view and observation bridge

The retained bridge now appends Kind 39, configuration operation 45 and observation
event 48 without changing previous tags. `View.otp_input` and reconciliation retain
controller identity and one-time seeds, reject policy changes and duplicate
controllers atomically, and route current typed callbacks behind window/node/
handler/tree/native-revision fences. Native tree admission enforces the same
policy lifetime and accounts configuration/seed storage through removal and
remount. Disabled/read-only cleanup observations remain deliverable.

The mailbox accounts accepted value plus draft, coalesces adjacent Changed events
only under the same route/policy, and preserves semantic/response boundaries.
Atomic Changed/Complete admission checks the entire pair and its byte/count budget
before replacing a coalescible predecessor. Quota failure cannot overwrite a prior
observation or expose half a completion.

Local macOS validation through the isolated project toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @test/view_api/runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-protocol --test otp_input -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native --test otp_input -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native --lib --tests -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native --all-targets --features native-tests -j 2 -- -D warnings
```

All pass. Two new Core expect groups and the fifth Rust protocol test pin independent
request/event fixtures, truncation/full-consumption behavior and invalid payloads.
Five native retained tests cover atomic admission/rollback, original seed retention,
weak-reference disposal, stale routes and policies, disabled cleanup, 10,000
coalescible changes, semantic/response barriers, exact count limits and byte
pressure with maximum preedit payloads. The full non-graphical native regression
run passes 270 tests across 39 test groups. Clippy compiles the native-test targets
but does not run graphical executables.

This validates retained view/observation contracts and queue admission only. No
native window was opened. The GPUI widget, actual transport publication,
command/result correlation, public mounted controllers and native OTP acceptance
remain pending. OCH-34 remains In Progress without a new capability; hosted CI,
Linux desktop acceptance and milestone completion are not claimed.

The full `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @fmt`
also passes after adding explicit OTP observation cases to the older bridge and
view examples. Exhaustiveness warnings caught those omissions; no wildcard was
introduced to hide future protocol additions.

The complete `cargo test --locked -p gpuio-protocol -j 2` regression suite
and `cargo fmt --all --check` also pass through the isolated wrapper.


## OTP native segmented adapter

`otp_input_view.rs` and `otp_input_paint.rs` now connect the retained state owner to
one GPUI EntityInputHandler. Accepted text uses segmented cells; active Unicode
preedit is shaped continuously with a marked underline. Paint, caret, pointer
selection and IME candidate positions share layout, clipping and horizontal scroll.
The host integrates native focus/commands, managed-list retention and cleanup.
Native event routing uses the atomic completion transport method; terminally
overloaded windows reject further text input. No synchronous OCaml callback or
permanent caret/animation timer is introduced.

Local macOS validation through the isolated project environment:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native --test native_otp_input --test native_number_input --features native-tests --no-run -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native --lib --tests -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native --all-targets --features native-tests -j 2 -- -D warnings
```

The built OTP and numeric native executables were run separately with a 60-second
owned-process-group timeout and TERM/KILL cleanup. Both pass and close their
windows normally. The OTP harness reports `GPUIO_OTP_NATIVE_OK` and covers:

- Stable mount/configuration, native full-width insertion and exact ordered
  Changed/Complete publication; no mount-time completion.
- GPUI keyboard dispatch for directional selection, replacement, undo/redo and
  paste distribution, plus actual system clipboard copy and read-only copying.
- Actual macOS NSTextInputClient marked/committed text, normalized commit, invalid
  joined-emoji rollback and preserved accepted state during preedit/config updates.
- UTF-16 range/text/candidate queries; a maximum 4,096-byte preedit inside an
  80-pixel field keeps its caret query in bounds. Oversized preedit leaves it intact.
- Pointer capture across repaint and selection outside the field, followed by
  verified capture release.
- Actual AppKit accessible value, SetValue normalization, enabled state and secure
  text-field subrole/redacted value. Masked copy preserves the system clipboard.
- Disable during composition, focus release, entity disposal, remount with another
  alphabet, and a saturated mailbox fault without partial completion. Further
  platform insertion cannot mutate the terminal owner.

The first fixture allocated slot 1 before slot 0 and was corrected to the valid
arena sequence. Initial accessibility queries also ran before lazy AccessKit
activation; the fixture now queries once, waits for painted frames and then
asserts the actual tree, matching existing numeric acceptance practice. Neither
failure was addressed by relaxing component validation.

The existing numeric native regression still passes its AppKit, hidden/modal,
repeat/capture/window and three 256-editor cleanup workloads. All 270 native unit
and non-graphical integration tests pass, as does all-target native Clippy.
The CI workflow now builds and runs the OTP native harness alongside the other
component checks; hosted execution has not occurred yet.

This is initial native-adapter acceptance. Public command/result correlation,
mounted Bonsai/Eio controllers/examples, external OS shortcut delivery, expanded
OTP-specific lifecycle/workload tests and inspected light/dark/scale screenshots
remain pending. No OCH-34 capability, completed ticket, Linux GUI validation or
milestone completion is claimed.

The full isolated `dune build -j 2 @all @fmt` also passes at this checkpoint,
and Ruby successfully parses the updated workflow YAML.

## OTP correlated commands and public application

The retained OTP instance now executes explicit native commands through appended
Message 16 / Event 49 envelopes. A command's Observed event precedes its response;
programmatic edits never produce Complete. The Eio request lane bounds pending
OTP requests to 64 across the application and validates reply lease, policy and
minimum revision. The public `Gpuio_eio.Otp_input` controller rejects late replies
from another placement or an older observed revision. Masked snapshots still
contain application text, as specified by the public contract.

Local macOS acceptance now includes:

- Independent OCaml/Rust request/result fixtures, invalid correlations and
  malformed value/selection/revision checks, complete consumption and truncation.
- Mailbox response boundaries and window-output fencing. A workload of 128 maximum
  composition snapshots interleaved with 128 responses drains into multiple
  encoded batches, each within the 1-MiB envelope; all 256 items are delivered.
- `examples/numeric/otp.exe --self-test`: both alphabet modes, native focus,
  revision-guarded replacement, directional/preserved selection, undo/redo/reset,
  invalid policy/length/selection, disabled/read-only rules, initial seeds,
  unplaced/stale/remounted controllers, and closure. A concurrent batch verifies
  exactly 64 accepted requests plus one Busy result, followed by a successful new
  request after draining. A read queued before close returns normally, while a
  command issued after the local close request returns Closed. Programmatic
  commands do not trigger Complete callbacks.
- Expanded `native_otp_input`: actual macOS preedit remains intact under rejected
  replace/clear/select/undo/redo commands and ReadSnapshot. CancelComposition
  restores the exact accepted selection and emits only Observed. Native IME
  commit still works after explicit cancellation and renewed composition.
- An independent native window tests command observation admission failure. The
  mutation occurs, but lost observation delivery returns NativeFailure, faults
  the window and prevents further mutation. Existing native Changed/Complete
  atomic-overload coverage remains intact. Both native windows close normally.
- `python3 scripts/test_otp_input.py` passes against the public example using
  actual macOS AX values/actions and OS keyboard events. It covers full-width
  normalization, both alphabets, directional deletion and history, atomic invalid
  input rejection, completion/rejection/count feedback rendered back through
  OCaml, secure value masking, disabled/read-only controls, remount and closure.
  It reports `GPUIO_OTP_APP_AX_OK` and reaps its owned application process group.

The native command harness reports `GPUIO_OTP_NATIVE_OK` and
`GPUIO_OTP_COMMAND_PRESSURE_OK`. The public bridge example reports
`GPUIO_OTP_PUBLIC_OK`. These are real local native/application checks; they do not
claim external IME candidate-menu selection, physical display-density changes or
Linux desktop acceptance.

All 271 native unit/non-graphical integration tests and native all-target
Clippy with `native-tests` / `-D warnings` pass through the isolated wrapper.
The first added mailbox test used a window ID where the existing API required
its slot; fixing that test call resolved compilation. Clippy identified a redundant
return binding in the native fixture helper, which was simplified. Neither issue
required changing the production contract. CI now includes both public OTP checks,
but consolidated hosted execution and merge remain pending.

Remaining OTP scope: inspected visual/theme/scale/masking coverage, expanded
hidden/modal/managed-list/window lifetimes and a retained-owner workload. OCH-34
capability advertisement and completion await that acceptance and the required
consolidated gates. Other milestone-05 components and final OCH-46 chat integration
remain in the milestone; this checkpoint does not redefine its completion.


The full local `dune build -j 2 @all @runtest @fmt` and all 127 Rust protocol tests
also pass; the final public example is rebuilt and exercised separately after
expanding its concurrency checks. The request-limit fixture originally used
Bonsai `Effect.all`, which is sequential in the pinned implementation; it now
starts requests explicitly through the effect evaluator in a single UI turn.
The close fixture originally expected all in-flight reads to be cancelled, but
native FIFO ordering allows an earlier read to complete before close. The test
and documentation now distinguish admission cutoff from actual native closure.
No production behavior was changed to accommodate these fixture corrections.

## Final local OTP acceptance and OCH-34 capability

Local macOS acceptance is complete for all OCH-34 component families. The new
`CAP_NUMERIC_INPUTS = 1 << 35` (`34359738368`) covers single/range sliders,
numeric editors/steppers and segmented OTP fields, including public commands and
events. The OCaml/Rust aggregate is `68719476735`; independently reviewed Hello
bytes are `0001fcffffffff0f000000`. Older capability bits remain present.
Consolidated hosted macOS/Linux checks and merge are still required; no Linux
GUI or milestone completion is implied.

The expanded `native_otp_input` harness adds four focused acceptance modules:

- **Appearance:** real GPU readback checks each of six occupied cells against
  the inherited foreground, background and focus border. Five light/dark cycles
  exercise both masks, focus states and synthetic 1x/2x scales. Different
  same-length accepted codes and active Unicode preedits must produce identical
  masked pixels. Toggling masking preserves the preedit and directional selection.
  Selected cells, marked underlining, caret, 40x20/80x40/180x64 constrained geometry
  and candidate/hit-test alignment are exercised. This is synthetic scale coverage,
  not a physical-monitor transition test.
- **Managed rows and visibility:** focus/composition pins the row; attempted
  eviction fails atomically without advancing the tree revision. Hiding an
  ancestor or entering a modal scope cancels composition back to its accepted
  selection and releases pointer capture. Hidden reads and explicit replacements
  remain valid; focus is denied. Late pointer release and a forced stale native
  input delegate do not edit the blocked field. Native entity identity survives.
- **Windows:** actual macOS activation loss cancels preedit. Another window in
  the same session uses the same node/handler identities without confusing events.
  Only the second window receives its completion pair. Closing it during pointer
  capture/composition disposes the owner while the first window preserves its value.
- **Workload:** three fresh generations of 256 mounted editors with maximum-length
  alphanumeric policies. Each cycle performs 32 native selection actions and
  1,024 maximum-length replacements in bounded batches. Adjacent native selection
  changes coalesce; programmatic observations remain discrete. Undo/redo together
  retain exactly 128 edits / at most 8,192 bytes of text, and Reset clears history.
  No transaction revision changes during native edits. Idle observation sees no
  redraws or events. All weak owners expire and session tree accounting returns
  to zero after each cycle.

Final workload measurements on macOS 14.5 arm64, Apple M1 Max, debug build:

| Cycle | Mounted owners | Tree accounting | Mount + frame | 32 keyboard actions + frame |
| --- | ---: | ---: | ---: | ---: |
| 1 | 256 | 41,216 bytes | 195.5 ms | 596.9 ms |
| 2 | 256 | 41,216 bytes | 93.1 ms | 557.2 ms |
| 3 | 256 | 41,216 bytes | 95.7 ms | 581.3 ms |

These are diagnostic debug timings under the native harness, not frame-rate or
input-latency guarantees. Tree accounting excludes GPUI/allocator overhead and
process RSS. The history limit counts text payload; total resource lifetime is
also checked through weak references and zero retained tree accounting. Mounts
and explicit command observations drain in batches of 32 under the existing
128-event limit; unbounded discrete output still faults rather than silently drops.

Inspected exported GPU artifacts:
[light](../images/otp-light-plain.png), [dark](../images/otp-dark-plain.png),
[light masked](../images/otp-light-masked.png), [dark masked](../images/otp-dark-masked.png),
[selection](../images/otp-dark-selection.png),
[composition](../images/otp-dark-composition.png), and
[masked composition](../images/otp-dark-composition-masked.png).
They show the native field under explicit diagnostic colors, not the final chat
showcase composition. All populated digits/bullets are readable in the reviewed
final artifacts, and masked preedit retains an underline/caret without revealing
its raw characters.

The first image-capture helper tried to draw while mutably borrowing the root
View. It was corrected to use `AsyncApp.update_window`, matching GPUI's capture
contract. An early artifact appeared to have two missing digits; that fixture
only checked aggregate glyph ink and did not verify its accepted value. The final
fixture is read-only during accepted-code captures, asserts the native value and
checks every cell across repeated transitions. The anomaly did not reproduce in
these stricter runs; no unverified claim of a GPUI rendering bug or production
rendering fix is made. Native test builds also fail on a glyph-paint error instead
of silently discarding it.

Validation uses the isolated environment:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native --test native_otp_input --features native-image-tests --no-run -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked --workspace -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native --all-targets --features native-image-tests -j 2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
```

The feature-enabled native executable runs separately under a 60-second owned
process-group timeout. Its final run passes APPEARANCE, POLICY, WINDOWS, NATIVE,
COMMAND_PRESSURE and WORKLOAD markers and closes/reaps every window/process.
The full Rust workspace passes 425 tests. Clippy with native-image tests and
warnings denied, the full Dune build/expect/format suite, and the public slider,
numeric and OTP self-tests all pass with the new capability handshake.

The final numeric example rerun exposed a fixture readiness race: it started on
the first editor's observation and captured two other controllers before their
mount observations arrived. The example now waits for all three snapshots before
starting its retained command sequence. Render acknowledgements replace fixed
100 ms sleeps for policy and remount checks. Rebuilding the example with `@fmt`
and running `number.exe --self-test` passes the three-layout, command, policy and
remount checks. `otp.exe --self-test` also passes. Each public executable runs
under an owned timeout and exits cleanly; no test windows remain open.

Required hosted macOS/Linux CI and merge remain pending under the milestone's
consolidated delivery workflow.


## OTP Tab-order follow-up

Calendar acceptance revealed that an explicitly tracked GPUI focus handle must
register its own Tab stop. A targeted external macOS reproduction confirmed the
OTP example skipped the next code field. The native handle now registers a Tab
stop while enabled and allowed by the visibility/modal gate. Read-only fields
remain navigable.

After rebuilding `examples/numeric/otp.exe`, the complete
`python3 scripts/test_otp_input.py` passes with added actual OS Tab/Shift-Tab checks
for both fields, read-only traversal, disabled-field skipping and remounting. Its
existing value/normalization, editing/history, masking and policy checks remain
passing. The image-enabled `native_otp_input` suite also passes all appearance,
policy, independent-window, native editor, command-pressure and repeated-workload
checks. All-target native-image Clippy passes with warnings denied. Owned child
windows/processes exited. This corrects a missed keyboard case in the earlier
local acceptance; it does not claim hosted or Linux graphical execution.
