# Measured-track bridge and publication state — OCH-41

2026-10-02, local macOS arm64, `milestone-07-gallery-release`, base `83eb87e`
with uncommitted milestone changes. **This is an implementation checkpoint, not a
working track widget or OCH-41/OCH-17 completion.** No physical application window
was opened. Linux desktop, physical macOS input/AX/VoiceOver/GPU, release packaging
and performance/resource qualification are not established by these checks.

## Implemented contract

Core and Bonsai now construct a bounded, keyed track tree through
`View.carousel_track`. Viewport, track, per-item and control styles are separate;
every item remains mounted. Controls reduce against the model's latest measured
stops. The actual native presenter/input adapter is still absent, and public
interface comments explicitly identify that limitation.

The paired protocol appends Kind54, Op96 and Event72 without moving existing tags.
Independent hand-written operation/event vectors agree across OCaml and Rust.
Decoders reject truncated, trailing and malformed payloads. Native admission
requires one Container track and matching retained Panel items, accounts for
retained metadata, and rejects invalid revisions, lineages and tree updates
atomically. A track-only update also revalidates its unchanged root owner.

Session and reconciler reject invalid windows, retired nodes/handlers, impossible
transaction revisions and stale collection lineages before delivering requests.
Observations remain available while disabled; user navigation does not. Unmount,
remount and shutdown tests prove old sources cannot replace the current model's
geometry. The mailbox preserves layout/intent ordering, includes stop-map bytes in
its budget, counts window output, rejects overflow and disallows late shutdown
input. These are bridge tests, not native interaction evidence.

The pure native publication/deadline state adds these guarantees for the upcoming
presenter:

- Measurements become available for input only after successful observation enqueue.
- A layout-only model update does not need to echo a new config revision.
- Stable geometry does not repeatedly publish or postpone an automatic deadline.
- Resize retires pending proposals even when canonical stops are unchanged.
- A changed retained item owner requires fresh measurement, including when its
  logical ID and stop map are unchanged.
- Unavailable layout, pause, changed revisions/lineage, replacement sources and
  disposal reject stale clock tickets. Resume starts a full interval.
- Automatic successors skip duplicate measured stops and allow only one pending
  proposal until acknowledged or invalidated.
- Invalid updates and epoch exhaustion preserve the previous valid state.

The state does not own a native task or render a component yet. Its integration
must still supply source replacement, visibility/focus/gesture eligibility,
same-frame measurement, clipped interactive neighbors and deterministic teardown.
See the [design](../design/carousel-track.md).

## Local validation

All commands use the repository's isolated toolchain, with `GPUIO_JOBS=2`.

```sh
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-protocol
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native --test carousel_track
./scripts/gpuio exec cargo clippy --offline --locked -j2 \
  -p gpuio-native -p gpuio-protocol --all-targets \
  --features native-image-tests,native-canvas-tests -- -D warnings
./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
python3 scripts/audit_component_catalog.py
git diff --check
```

Passed: **338 protocol tests**, **673 native library tests with two existing
private-D-Bus skips**, three new native transport tests, the full OCaml test/format
and gallery build, strict lint and the structural catalog audit. Five of the
native library tests cover the new publication/deadline state. Two new Core expect
tests cover paired outer envelopes and actual view/reconciler lifetime behavior.
The full native suite includes GPUI TestPlatform tests; none substitutes for
physical desktop acceptance. No upstream pins or unrelated switches changed.

A fresh independent installed gallery consumer also builds successfully:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20260929-m7-resumed/carousel-track-bridge-installed-gallery
```

It reports `run=False`; this is package/build coverage. A separate executable in
that consumer imports the installed Core and Bonsai constructors, mounts the Core
view through Reconciler, dispatches a synthetic layout observation, reduces Next
and rejects events after close. It reports `GPUIO_INSTALLED_TRACK_BRIDGE_PASS`.
It does not run a native track or claim GUI coverage. Its exact command sets
`OCAMLPATH` to the staged `installed/lib` and runs `./bridge_probe/probe.exe` with
`dune exec --root` set to that independent consumer. Scratch contains the full
command, probe and logs; it is not a build dependency.

The actual measured renderer, motion API, loop rebasing, native gesture/keyboard
routing, focus/AX, gallery demonstration and component/release qualification remain
open. Rich tabs and flat split groups also remain within OCH-41's catalog scope.
