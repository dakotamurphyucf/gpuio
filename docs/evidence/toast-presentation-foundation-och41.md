# Layered toast geometry, motion and lifecycle foundation — OCH-41

Local checkpoint, 2026-10-03, macOS checkout, base `83eb87e` plus the milestone
working tree. This is a native foundation for the
[presentation contract](../design/toast-presentation.md). **Layered rendering,
public configuration and gallery integration remain unimplemented.** Existing
placement and ordinary toast behavior remain available.

## Implemented native foundations

`toast_geometry` validates up to 32 full-generation IDs and measured heights. It
computes variable-height expanded layout, mirrored top/bottom offsets, collapsed
peek/width reduction and the footprint of only the visible layers. Hidden deeper
cards retain descriptors without owning their child resources. Only the front
card is interactive while collapsed; ending/zero-size cards are noninteractive.
The future renderer must enforce these flags and provide the bounded scroll area.

`toast_reflow` uses four native analytic springs per retained rectangle, after the
caller composes placement and card offsets. Retargeting preserves last-painted
position and velocity, never a speculative layout sample. Generation replacement
and removal retire old records; suspend clears travel/history to avoid replay on
show. Valid layout outside the generic spring coordinate domain snaps exactly.
Opaque paint samples prevent callers from changing IDs/coordinates before commit.

`toast_lifecycle` separates Pending, Entering, Present, Ending and Closed. Entry
starts at accepted paint; deadlines survive intermediate frames. An accepted
dismissal immediately ends input eligibility, retains the first reason and exits
from the last actual paint. One terminal result is available after exit. Ordinary
motion updates do not restart a phase; hidden/reduced/immediate policy settles it;
unmount/discard releases pending results. Opaque deadline/paint tokens reject stale
callbacks and owner replacement. These models own no timer, transport or entities.

Native `Session` now also creates an opaque, consuming `AcceptedToastDismissal`
token. Acceptance validates the reason and current configuration. Completion
checks the original session identity, live window/node/handler generations,
revision and overload state, without undoing an accepted timeout after a later
persistent-setting update. Existing immediate publication delegates through the
same acceptance check. Production animated delivery is not connected yet.

## Local verification

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec` with the pinned offline toolchain:

```sh
cargo test --offline --locked -j2 -p gpuio-native \
  --test toast_delivery --test toast --test toast_geometry \
  --test toast_lifecycle --test toast_reflow
cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
```

Results: **14 focused tests pass** (12 new foundation/delivery and two existing
toast admission tests); the full native library suite passes **729 tests with two
existing private-D-Bus skips**. Geometry includes 1,000 generated collections across
expanded/collapsed and top/bottom cases. Reflow checks finite-difference velocity
continuity, speculative samples, reorder, removal, generation replacement and
large-coordinate fallback. Lifecycle tests cover first-paint timing, exact
deadlines, interrupted entry, first-reason retention, zero/invalid configuration,
suspension, teardown and stale samples. Delayed-delivery tests cover persistent
updates, cross-session rejection, handler changes, removal, overload and close.

Final `cargo clippy --offline --locked -j2 -p gpuio-native --all-targets
--features native-image-tests,native-canvas-tests -- -D warnings` passes.
`cargo fmt --all -- --check`, the catalog structural audit and `git diff --check`
pass. No wire/Core
API changed, and no new installed consumer or OCaml run is claimed for this native
foundation. Full public integration will require those checks. Local logs and
recovery notes are under `scratch/agents/root-20260929-m7-resumed/`, prefixed
`toast-foundation-` and in `OCH-41-toast-presentation.md`.

No OS windows opened. Pure state/geometry tests and TestPlatform regressions do
not establish physical GPU/input/IME/VoiceOver behavior, Linux desktop acceptance,
CI or release readiness. Remaining native rendering, public bridge, gallery and
physical/resource/distribution requirements keep OCH-41/OCH-17/milestone 07 open.
