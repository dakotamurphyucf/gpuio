# Numeric and OTP component evidence (OCH-34)

## Current scope

OCH-34 is In Progress. Shared numeric domain/draft rules and slider contracts/codec/native state are
implemented, along with retained slider views, tree admission and observation routing.
Slider command transport/rendering, numeric input/stepper and OTP integration
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
