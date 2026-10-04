# Tooltip entry and switching evidence — OCH-41

2026-10-02, local macOS/arm64 checkout `milestone-07-gallery-release`, base
`83eb87e` plus uncommitted milestone work. GPUI TestPlatform exercises the actual
native host but does not qualify physical macOS/VoiceOver/IME/GPU behavior.
No OS windows were opened for these checks.

`Tooltip.Config.create ~motion:Enter_and_switch` adds opt-in native entry and
rapid managed switching through Op92. The source review now explicitly uses
**trigger** bounds for the row/center comparison: that is what the pinned Base
provider passes to the styled tooltip renderer. It does not compare popup bounds.
The [contract](../design/tooltip-motion.md) records the controlled/retained-content
adaptation and the differences from upstream's one-visible-tip provider.

Two motion-state tests verify first-paint start, cubic entry/slide timing,
unequal trigger widths, the strict ten-pixel row threshold and permanent
settlement after reduced motion or disabling. Five native host tests cover:

- Actual painted entry alpha and translated accessibility geometry; same-row
  switch displacement and cross-row immediate placement; pointer focus on a
  moving input; stable accessible identity and retained native editor handles.
- False-before-true replacement notifications, suppression of stale focus events,
  native show deadlines, controlled intents versus accepted swaps and explicit
  pending-deadline survival through a motion-only update.
- Reduced motion, hide/disable/unmount, idle-frame retirement and zero native
  retained-tree bytes after teardown; zero-effective-opacity paint and expired
  grace cannot seed a new switch.
- Hidden peers retain their accepted state. Replacing multiple explicitly opened
  visible managed tips batches native state changes before one focus/visibility
  synchronization.

The initial harness used sparse node IDs and compared lazy button focus owners
across hiding. It now uses the protocol's dense slot allocation and retained
native editor handles, matching the actual ownership contract. Those initial
failures were test mistakes, not reported production defects.

Review also removed recursive per-peer synchronization during replacement and
excluded hidden/modal-blocked peers. Only actual visible panel paint records a
previous trigger, including effective style opacity; frame layout alone cannot
supply the history. History owns values only, and existing show/hide timers remain
the only tooltip timers. No per-frame bridge events are added.

## Completed checks

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native \
  --offline --features native-image-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native \
  --offline --features native-image-tests --lib tooltip::motion_tests
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native -p gpuio-protocol \
  --offline --test tooltip_motion
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --offline
```

The full native suite after the final production refactor passed **644 tests with
two existing private-D-Bus skips**. The subsequently added hidden-peer/batch
regression passes with all **five** focused host tests. Full protocol passes
**329 tests, no skips**; the independent codec and atomic admission tests pass
one each. Admission rejects HoverCard/other node targets and releases its fixed
reservation on teardown. The authored physical gallery walkthrough is
syntax-checked only.

Strict Rust lint and full OCaml tests/format/gallery build also pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -j2 -p gpuio-native -p gpuio-protocol \
  --all-targets --features native-image-tests --offline -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
```

Two Core expect tests verify motion-only updates retain child IDs and do not
resubmit the native timing configuration, plus the independent Op92 bytes.
A fresh installed gallery consumer build passes:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20260929-m7-resumed/tooltip-motion-installed-gallery
```

It reports `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
Public packages are staged separately without mutating the opam switch, and the
copied gallery/backend build against those public APIs. This is build acceptance,
not a running packaged app or clean-machine distribution check. The gallery has three help triggers controlled
by **Animate opening**. Actual visual/keyboard/IME/VoiceOver/GPU/resource
acceptance, current Linux gates and release distribution/review/publication remain
separate work. This is not ticket or milestone completion.
