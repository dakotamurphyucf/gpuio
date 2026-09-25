# Numeric and OTP component evidence (OCH-34)

## Current scope

OCH-34 is In Progress. Shared numeric domain/draft rules and slider contracts/codec/native state are
implemented, along with retained slider views, tree admission and observation routing.
Mounted slider rendering and initial native interaction tests also pass locally.
Correlated slider commands and the public Bonsai/Eio controller/example also pass
local integration. Expanded acceptance, numeric input/stepper and OTP integration
remain pending. No OCH-34
capability is advertised and no new native GUI acceptance is claimed.

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
