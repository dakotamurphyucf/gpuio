# Modal backdrops and modal accessibility — OCH-41

2026-10-02, local macOS worktree based on `83eb87e`. No OS windows opened.
Local validation is complete for this change; it does not close OCH-41 or OCH-17.

The pinned overlay source review identified a fixed-color gap in native modal
backdrops. Public Core/Bonsai `dialog`, `sheet` and `alert_dialog` now accept a
checked `Color.t` backdrop. Theme-only updates of the same View resolve a new
wire color while retaining the focus scope and children. Omission resets the
existing half-opacity black; transparent paint keeps full modal behavior.

Op90 is append-only. Independent bytes cover a nontrivial RGBA value, transparent
and reset. Rust rejects every truncated message, an invalid option tag and trailing
bytes. Native admission rejects out-of-range RGBA, non-FocusScope targets, and
removing/changing the overlay to nonmodal while retaining backdrop metadata.
Those failures preserve revision, configuration and retained-byte accounting.
Clearing both fields together and complete teardown pass.

The production TestPlatform regression covers dialog, alert dialog and four sheet
edges. It checks full-viewport painted color/default reset/transparent artwork,
retained focus, forward/reverse Tab confinement, stale background AX and pointer
rejection, and accepted closure restoring the trigger. It also reproduced an
existing accessibility bug: `semantics::State` wrapped `SurfaceBounds` after the
panel had been erased to `AnyElement`, so it could not export the modal flag.
The renderer now attaches semantics to the concrete panel's frame first. Each
named modal now reports the flag, and a popover remains nonmodal. This is AccessKit
TestPlatform evidence, not physical AppKit/VoiceOver acceptance.

The gallery adds **Tinted backdrop** and a dialog **Change backdrop** action.
An authored extension to `test_gallery.py --section overlays` checks retained
physical AX dialog identity and `AXModal` while updating. It is syntax-checked
but unrun. The public APIs are documented in the
[contract](../design/overlay-backdrop.md) and compared to pinned behavior in the
[source review](../catalog/overlay-review.md).

## Commands and results

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native -p gpuio-protocol \
  --offline --test overlay_backdrop
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
```

These pass: one codec and one atomic-admission test, full OCaml tests/format and
the gallery build. The full native library passes **634 tests with two existing
private-D-Bus skips** using `cargo test -j2 -p gpuio-native --offline
--features native-image-tests --lib` in the repository environment. The complete protocol suite
passes **327 tests/no skips** (`cargo test -j2 -p gpuio-protocol --offline`).
Strict lint passes with `cargo clippy -j2 -p gpuio-native -p gpuio-protocol
--all-targets --features native-image-tests --offline -- -D warnings`. A fresh staged gallery consumer also passes:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20260929-m7-resumed/overlay-backdrop-installed-gallery
```

It reports `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
Public packages are staged separately without modifying the opam switch; copied
gallery sources compile against the installed interfaces and rebuilt native
backend. No application window is launched. Rust formatting, catalog source
hash/inventory audit, Python walkthrough syntax and diff whitespace checks pass.

 The first
new Core fixture omitted the standard button `accent` token; supplying a complete
test theme fixed the fixture without changing production fallback behavior.

Physical macOS input/accessibility/GPU, complete gallery resource behavior,
consumer runtime, remaining catalog features, distribution and current Linux
required checks remain open. The source review explicitly retains real
outer-dialog/sheet/managed-tooltip motion and positioning/inset gaps. It does not
infer functional sheet resizing or automatic Popover animation from unused
fields/private helpers.
