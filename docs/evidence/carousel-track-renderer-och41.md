# Measured carousel renderer — OCH-41

2026-10-02, local macOS arm64, base `83eb87e` plus uncommitted milestone work.
**Component acceptance remains open.** This checkpoint renders measured tracks
through the native GPUIO host; motion, native gesture/keyboard routing, automatic
clock tasks, complete focus/AX behavior and the gallery example are unfinished.
No physical desktop application window was opened.

## Implemented behavior

The retained tree now has a native CarouselTrack renderer. It reads GPUI's fresh
ScrollHandle bounds and padding-inclusive scroll limit before prepainting any
card content. It publishes a checked layout observation before exposing geometry
for interaction, then translates each retained card before that card registers
hitboxes. Selection and resize therefore affect paint and pointer coordinates in
the same frame. Application selection remains controlled by the OCaml model.

The viewport clips content; the inner track uses the model's axis, a single flex
line and the viewport's allocated size. Track padding/gaps and per-card extents
are measured. The track's own generic scroll offset stays zero, so the adapter
owns positioning without a competing generic wheel scroller. Animation is not
implemented yet: every accepted selection currently settles immediately.

Continuous-loop *eligible geometry* uses one-copy periodic placement at settled
positions, preserving native owner identity. Geometries that require duplicate
fragments report Jump; shorter tracks report Finite. The test covers changing
between these modes on resize, not animated movement through a wrap. A fully
clipped or zero-size viewport reports unavailable geometry. Partially clipped
viewports retain their full logical geometry rather than changing stops as an
ancestor scrolls. Empty collections publish a checked empty stop map.

Mounted state follows current node/handler identity. Synchronization visits dirty
owners and existing track states; it does not add a full-tree scan to every edit.
Removing a track disposes its publication state. The native renderer does not
currently schedule automatic timers or animation frames.

## Integration findings

The host's test-only diagnostic canvas was an extra direct child of the track.
With padding, that child expanded ScrollHandle's measured content span. It is now
omitted on the structural track container; viewport/card/control diagnostics remain
available. Track measurement requires the exact nonempty retained-child count.
Admission also rejects implicit text on the structural track container.

GPUI retains old ScrollHandle child bounds when a Div becomes empty. The adapter
explicitly handles the checked empty collection and does not reinterpret cached
bounds as live cards. The padding and nonempty-to-empty cases are regression tests.

## Evidence scope

`carousel_track_view_test` admits a real GPUIO tree, renders with GPUI TestPlatform,
and checks both axes. It verifies unequal extents and gaps, clicks a partially
visible nonselected button, changes selection without replacing its focus owner,
checks matching same-frame painted-quad and AX bounds, resizes with the same canonical stop map, adds padding,
changes loop mode, collapses/restores viewport size, empties and unmounts the track.
Three native actions remain three actions after loop placement; owners are not
cloned. Native retained-tree bytes return to zero after removal.

These are actual adapter layout/routing checks on a simulated platform, not
physical keyboard, IME, screen reader, GPU or packaged application acceptance.
The native test constructs bridge operations directly; a full public-OCaml
renderer/driver replay remains part of component integration.


## Validation

All commands use the isolated repository environment with `GPUIO_JOBS=2`:

```sh
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --features native-image-tests,native-canvas-tests --lib
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native --test carousel_track
./scripts/gpuio exec cargo clippy --offline --locked -j2 -p gpuio-native \
  -p gpuio-protocol --all-targets --features native-image-tests,native-canvas-tests \
  -- -D warnings
./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
python3 scripts/audit_component_catalog.py
git diff --check
```

Passed: **674 native library tests with two existing private-D-Bus skips**, three
transport tests, strict lint and the full OCaml tests/format/gallery build. After
that suite, an additional painted-quad assertion was added to the renderer test;
the focused renderer test passed again on both axes. The structural catalog audit
and whitespace checks also pass. No upstream pins or unrelated switches changed.
The installed-package API checks from the bridge checkpoint remain historical
package evidence; this renderer was validated through the local native host.

The first full-suite run exposed a default-stack overflow in the existing deeply
nested sidebar test. New large builder temporaries in recursive traversal caused
the regression. Track/viewport style preparation now runs in the existing
nonrecursive `node_style` helper. The focused sidebar test and full native suite
pass on the ordinary stack; no stack-size override was retained.
