# Measured carousel card visibility — OCH-41

2026-10-02, local macOS arm64, base `83eb87e` plus uncommitted milestone work.
This checkpoint connects retained-card clipping to input, focus and accessibility.
It does not close OCH-41 or qualify physical desktop behavior.

All cards remain mounted and measured. Fully clipped cards cannot receive Tab,
keyboard or accessibility activation; partially visible cards remain eligible.
Accepted navigation reveals cards; focus does not independently change selection.
Clipping the focused child returns focus to an eligible viewport or blurs when
there is no visible viewport. Foreign/modal focus is preserved. Native editor and
semantic identities survive navigation, reorder and resize.

A stable card-ID semantic ancestor hides clipped descendants. The focus manager
records clipped native handles so ancestor fallback cannot grant them focus.
Native measured viewport actions use their typed request contract, with no extra
generic Press. Disabled viewport accessibility and pointer focus are rejected.
Application-owned modal portals remain independent of visual clipping, while
anchored popovers suspend and resume with their card. A changed visibility mask
requests one further render; stable masks do not poll or request idle frames.

Five production-host TestPlatform regressions cover:

- Both axes, partial visibility, Tab skipping, stale AX Focus/Click, accepted
  selection, retained semantic/focus identity, external focus and disabled roots.
- A real native editor with simulated marked text before clipping, blocked typing
  while clipped, retained text/owner and independent nested-scroll focus reveal.
- Reordering by ID, zero-size viewport, restoration, leaf-first unmount and cleanup.
- A modal dialog remaining visible, focused and actionable after its card clips.
- Focused and unfocused anchored popovers disappearing and reappearing without
  forced frames or unrelated input; no stale click or repeated autofocus.

Tests exposed and fixed hidden-handle permission inherited from the viewport,
incorrect clipping of independent modals, and a retained popup waiting for unrelated
input to reappear. The initial popup test with forced frames missed that last bug;
the final version waits only for normal scheduled work after accepted updates.

Validation uses `GPUIO_JOBS=2` and the isolated repository environment:

```sh
./scripts/gpuio exec cargo fmt --all
./scripts/gpuio exec cargo clippy --offline --locked -j2 -p gpuio-native \
  --all-targets --features native-image-tests,native-canvas-tests -- -D warnings
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native --test carousel_track
./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
python3 scripts/audit_component_catalog.py
git diff --check
```

Final validation passed full OCaml tests/format/gallery build, catalog audit and
whitespace checks. Rust passes **705 native tests/two existing private-D-Bus skips**, five
transport/admission tests and strict all-target lint. The paired protocol has no
new changes in this checkpoint; its preceding 339-test pass remains prior evidence.

No physical OS window was opened. Simulated marked text is not proof of native
IME composition commit/cancel on blur; TestPlatform AX is not VoiceOver acceptance.
Full public gallery/driver, physical macOS, resource/distribution and required Linux
automation remain separate acceptance work.
