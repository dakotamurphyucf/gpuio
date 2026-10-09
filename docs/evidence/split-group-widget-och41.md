# Flat split-group native widget and handle appearance — OCH-41

Local checkpoint, 2026-10-02, macOS development checkout, base `83eb87e` plus
the uncommitted milestone tree. This extends the
[geometry/state foundation](split-group-foundation-och41.md); it does not complete
OCH-41 or milestone 07.

## Implemented

`rust/native/src/split_group_widget.rs` uses GPUI's pinned `container_query` to
lay out all visible panels and dividers from one measured size in the same frame.
It preserves keyed native focus, captures pointer movement beyond the window,
previews without emitting bridge-sized updates, commits once on release and
restores committed sizes on cancellation. Keyboard arrows/Home/End and accessible
increment/decrement/SetValue share the bounded solver. Native child buttons keep
independent input. Empty/hidden/zero/clipped groups do not schedule idle frames.

Weak frame callbacks and explicit parent lifecycle hooks cover lost capture,
deactivation, changed policy/geometry/collection, unpainted frames, replacement
and close. Clipped divider semantics are initialized from measured bounds before
GPUI collects accessibility metadata; prepaint independently checks input/focus.
Immutable configuration is shared by `Arc` rather than copying panel labels on
every frame.

Core `Split_group.Appearance` and paired wire/native records now provide separate
paint thickness and hit extent, shared/per-ID paint-only styles and bounded style
budgets. State precedence is base, hover, focus, pressed, disabled. The same checked
record drives native divider/grip paint; the earlier temporary color-only native
appearance was removed. A hand-assembled fixture independently checks the schema
in both languages, including shared and keyed state declarations. The public data
module still explicitly says the view is under construction.

## Local verification

Commands use the repository's isolated environment, two build jobs and the pinned
offline Rust dependency graph:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 \
  -p gpuio-native --features native-image-tests \
  --test split_group_widget --test split_group_appearance \
  --test split_group_geometry --test split_group_state
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 \
  -p gpuio-protocol --test split_group --test split_group_appearance
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/view_api/runtest
```

Results: **19 native tests pass**, including 2,000 generated constraint cases;
**three paired protocol tests pass**; the Core view API expect suite passes,
including four split-group tests. Eight native widget tests exercise both axes,
measured resize, preview/commit/Escape, outside-window coordinates, child input,
keyboard/AX numeric actions, reset requests, hidden/clipped/zero geometry,
per-handle paint, custom grips, disabled actions, lost capture and owner replacement.

Final strict native all-target Clippy (features `native-image-tests,native-canvas-tests`,
`-D warnings`), `dune build -j2 @test/view_api/runtest @fmt`, Rust formatting,
catalog source audit and `git diff --check` pass. The full native/protocol suites
and installed consumer were not rerun for this standalone widget checkpoint;
they remain required after production integration.

During development, a child-click assertion used an incorrect assumed button
position; measuring the actual child button fixed the test. A clipped divider was
initially exposed in the accessibility tree because metadata precedes prepaint;
the measured initial visibility fix is covered by the passing regression.

## Remaining integration and limits

These are native **TestPlatform** tests; no operating-system window opened. They
do not establish physical macOS keyboard/IME/VoiceOver/GPU behavior or Linux desktop
acceptance. The production tree operation, admission/memory accounting, Host owner,
asynchronous event fencing, passive-decoration validation, Core/Bonsai view,
gallery and installed-consumer example remain required. The widget's `Observer`
is native-only; it must enqueue a checked event, never call OCaml from layout or
paint. No public component or release completion is claimed here.
