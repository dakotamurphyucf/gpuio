# Radar label button and command lifetimes — OCH-41

2026-10-06, macOS 14.5 arm64, source base `90f212b` plus this repair.
The foreground chart regression reproduced an ordinary button emitting Press
when mouse-down preceded axis removal/return and mouse-up followed it, with no
paint between source transitions. Checking current eligibility alone let the
old gesture become valid again.

## Native ownership repair

Clipped interactive nodes now have a private native action lifetime. It stays
stable while its node remains eligible, including normal source publications.
Immediate visibility cleanup revokes ineligible lifetimes individually. Ordinary
button and command-button mouse/keyboard click and accessibility callbacks check
that lifetime as well as their existing routing rules; stale mouse-down is
consumed before it can arm native input.

The next rendered exposure gets a fresh GPUI element identity. This discards
GPUI's retained pending-down state and replaces the native accessibility target;
revoking only an old callback would not prevent a new frame from inheriting an
old gesture. OCaml Node IDs, retained Views and Bonsai models stay unchanged.
There are no protocol, public API, compiler or vendor changes.

The registry belongs to one native View, with one current entry per participating
rendered node. It allocates a lease when an exposure begins, reuses it across
renders, removes ineligible entries and revokes remaining leases on destruction.
It does not invalidate unrelated entries with a global visibility generation.
The existing table-header path shares this clipped-control renderer and is
included in regression validation.

## Behavioral evidence

The production foreground chart harness now tests ordinary Button and a callback
CommandButton in a CommandScope inside an ordinary InputRegion label. It passes:

- Native down/up activation and same-axis publication preserving a valid gesture.
- Hide/return rejecting the old mouse-up before another paint.
- A replacement frame rejecting an old pending mouse-down too.
- Fresh native gestures recovering after the axis returns.
- Actual AppKit `accessibilityPerformPress` on an exposed AXButton delivering
  exactly one action, for both ordinary and command buttons.
- An AX request queued before hide/return producing no action afterward. The test
  confirms it was still queued before the source transitions.
- A fresh AX target delivering actions, while a retained old AX object cannot
  activate the replacement exposure; another fresh press succeeds afterward.
- Fixture removal and the enclosing chart's release/zero-pending-work checks.

The pointer events are injected through GPUI, not physical mouse events. AX
actions use actual AppKit objects in a foreground window. They are not VoiceOver
qualification. No VoiceOver settings changed.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_input --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests,native-image-tests --lib --test chart_tree --test native_chart_view --test native_chart_input --test native_input_region --test native_menu_popup_queue --test native_table_host --locked -j2
```

The broader run passes 1,024 unit tests (two existing skips), three chart-tree
tests and every selected native harness. Table coverage includes targeted OS
keys, embedded marked-text editing, AX table/selection/stale-frame checks,
retained header/body Views and 100,000 sparse rows. Existing label pointer,
slider, InputRegion and AppKit popup regressions remain green.

Strict Clippy with both native test features, formatting and whitespace checks pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all --check
git diff --check
```

The rebuilt root public gallery passes its foreground radar walkthrough: native
editor typing/Backspace, keyboard and AX button effects, source-update focus,
ancestor disable, hide/browser draft retention, remount and source cleanup.

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
python3 scripts/test_gallery.py --section chart-radar
```

A fresh independent consumer, linked against packages staged into its own prefix,
also passes the complete radar walkthrough and zero registered source bytes.
No opam switch was installed into or mutated. Its executable SHA-256 is
`8f3c5a352f1f72427f37ea65f5b8d598c1c677866a7ef4c57069223b4affe527`.

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section chart-radar --workspace scratch/agents/root-20261004-resumed/radar-action-installed-consumer-90f212b
```

[Raw logs](radar-label-actions-och41-logs.tar.gz) and their
[verified manifest](radar-label-actions-och41-manifest.json) retain the initial
failure and all passing checks. All local build/test processes exited normally.

OCH-41/OCH-17 remain open. Specialized widget actions, clipping/resource children,
theme/font/scale and multiwindow label interaction still require qualification,
alongside broader catalog and release gates. CI run 37487278162 covers older
`c373e3b`, not this repair; current-source hosted checks remain required.
