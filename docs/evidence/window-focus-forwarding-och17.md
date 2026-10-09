# macOS application focus forwarding — OCH-17 / OCH-41

2026-10-08, physical macOS 14.5 arm64, base `4c2c06c6`. A strict carousel keyboard
walkthrough exposed a platform accessibility defect: after an actual Tab reached
Card draft, that editor reported `AXFocused=true`, but querying the application's
`AXFocusedUIElement` returned its `AXWindow`. The first run failed at exact focus
identity; the second added the leaf-state diagnostic and confirmed the mismatch.
Both failed attempts are retained. This was not a failed native keyboard move.

## Repair

GPUI uses its native window as the first responder. The pinned AccessKit adapter
implements `accessibilityFocusedUIElement` on the content view, and provides a
window-forwarding helper for windowing libraries with this responder arrangement.
GPUIO's host had installed its window hit-test forwarder but no focus forwarder.

The maintained GPUI Base adapter now provides `install_window_focus_forwarder`.
The macOS host calls it alongside `install_window_hit_test_forwarder` when opening
a window. It installs the window selector on the concrete Objective-C class and
forwards to the current content view. It neither caches a focused node nor calls
OCaml synchronously. Repeated window creation leaves the installed class method
intact. Existing hit-test forwarding and the OCaml API are unchanged.

The vendored source, reproducible patch and patch checksum change together. All
244 tracked GPUI Base files reconstruct byte-for-byte from the pinned local
upstream archive. An initial whole-directory comparison also saw an ignored local
`Cargo.lock`; the final comparison explicitly covers tracked source files.

## Actual desktop evidence

The original strict application-level focus assertion is preserved. The repaired
repository gallery and a **freshly staged independent consumer** both pass twelve
horizontal/vertical × Dark/Light × Comfortable/Large/Compact cases:

- Real Tab and Shift-Tab visit exact native editor/button identities in the visible
  cards, followed by the corresponding scoped Next/Previous control. Fully clipped
  cards are absent from the exposed AX tree and do not add hidden Tab stops.
- At Capture, horizontal compact geometry exposes the draft and Explore; vertical
  compact geometry exposes only the draft. At Publish, the horizontal case exposes
  Refine/Publish and the vertical case only Publish. The fixture asserts these
  expected visible sets rather than deriving the expected order from the AX tree.
- Home/End selection uses the viewport's native keys. Returning to Capture and
  expanding the viewport preserve the edited draft and selected card. Movement is
  immediate so these checks do not infer animation quality from sampled geometry.

`test_macos_focused_input.py` now also requires application-level focused identity,
separately from the leaf's `AXFocused` state. Its five actual desktop scenarios
pass: draft selection and native undo, masked metadata without reading its value,
read-only behavior, second-window isolation/close/return, and page remount. Focus
identity is checked before and after the native metadata-inspection shortcut.

Both carousel applications and the input application exit normally. These checks
establish keyboard routing and accessibility-query identity, not VoiceOver speech
or navigation, hardware trackpad behavior, smooth looping, process/GPU resource
budgets or Linux GUI acceptance. The earlier picker evidence's diagnostic-only
application query remains historical; this checkpoint adds a strict assertion.

## Reproduction

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe
python3 scripts/test_gallery.py --section carousel-focus --images <fresh-output>
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace <fresh-workspace>
python3 scripts/test_gallery.py --section carousel-focus --executable <workspace>/consumer/_build/default/main.exe --images <fresh-output>
python3 scripts/test_macos_focused_input.py --output <fresh-output>
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests,native-canvas-tests --lib --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native --features native-image-tests,native-canvas-tests --lib --tests -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @fmt
```

Local carousel runs use the 180-second exception/cleanup wrapper; Foundation adds
a separate three-minute focus step. The independent consumer build passes and the
native library suite passes **1,187 tests with two existing skips**. Strict Clippy
and formatting pass. Python syntax,
targeted lint, example/catalog audits, actionlint and whitespace checks pass.
Exact build/test output and terminal status are retained in the accompanying
archive; native TestPlatform results alone are not the focus-forwarding proof.

The [raw archive](window-focus-forwarding-och17/reports.tar.gz),
[verified manifest](window-focus-forwarding-och17/manifest.json) and
[summary](window-focus-forwarding-och17/summary.json) retain the failed and repaired
runs, executable hashes, focused-input report, fixtures and validation logs.
No milestone ticket is closed by this scoped repair. Final-source hosted checks,
full accessibility and remaining release gates stay open.
