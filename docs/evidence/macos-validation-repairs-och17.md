# macOS CI repair batch — OCH-17

Local follow-up to hosted run `37241646017` at `6d6d96f`. That run's entire Linux
job passed; macOS failed sixteen steps. The following repairs and reruns apply
to the worktree following `c0d2eaf`. They do not make the old hosted run green.

## Production numeric-input repair

Application-controlled stepping had broadened the held-gesture cancellation
predicate to every editing-config change. This regressed the existing native
contract: updating an accessibility label cancelled an ordinary held step.
`number_input_view` again distinguishes stepping policy (domain, layout, empty
acceptance, disabled/read-only) from metadata. Handler/mode changes still cancel.
Any config change still invalidates a pending application request; its waiting
hold is stopped as well, so it cannot retain a gesture without a future reply.

The actual native numeric suite passes label/color restyling during a held step,
the cancellation cases, keyboard/clipboard/native text-client checks, three
256-editor disposal cycles and bounded history/idle checks. An extended
TestPlatform regression verifies that a label change cancels an outstanding
application request and its hold, and rejects the late result. This is not real
candidate-window IME qualification.

## Fixture and public-example corrections

- **Progress:** the moving segment starts outside the track. Its clipped origin
  remains zero for the first 300 ms of the 1500 ms cycle. The old 180 ms assertion
  was invalid. The test now observes full entry within two seconds without
  requesting frames, then checks position, width, color and unchanged tree revision.
- **Slider:** native thumb rings now have interaction springs. Both functional
  and 1,024-owner workload fixtures require bounded settling (at most three
  seconds) before retaining the existing no-redraw idle assertion. Endless redraws
  still fail. All three workload disposal cycles pass.
- **Rich avatar:** a source-less avatar cannot bind an image-state callback.
  The fixture now binds only when attaching a primary source, and unbinds when
  removing it. GPUI also deliberately pauses GIF playback in inactive windows;
  that portion activates the application/window and observes activation before
  requiring both frames. Static checks remain in the background. Raster/SVG/GIF,
  clipping, fallback leases, AX ownership, decode failure, disposal and idle pass.
  The complete image suite, including spinner/progress/control pixels, passes.
- **Date picker:** confirmation resolves the current draft for its opening;
  a retained value is not itself an old native calendar command. Remount rejection
  is now tested with the captured native command. Window closure separately
  checks component `Not_open` and native command `Closed`. The full public
  self-test passes single/range, policy changes, cancellation and close.
- **Canvas:** the inspector updates when OCaml receives movement, before the
  replacement scene necessarily prepares. The old AX action raced that publication
  and was correctly rejected as `PublicationPending`. The example's native scene
  description now includes the four sample positions, useful to accessibility
  readers. The external driver waits for those coordinates in AppKit `AXHelp`,
  then invokes the prepared scene's action. Selection, physical targeted keyboard
  movement, activation, hide/show, reset and shutdown pass without weakening the
  native stale-snapshot guard.
- **Chat:** three specialized Python overrides now forward the shared AX lookup
  deadline. All ten affected walkthroughs pass; see [the helper evidence](ax-traversal-och41.md).

## Validation

Repository stock OCaml 5.3/Core/Bonsai v0.17 environment on Apple M1 Max, macOS
14.5 arm64. No unrelated toolchains were modified. Commands include:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked --offline -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked --offline -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest
GPUIO_JOBS=2 ./scripts/gpuio check-fmt
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked --offline -j2 -p gpuio-native --features native-image-tests --test native_controls --test native_slider --test native_number_input --test native_image_views
./_build/default/examples/calendar/picker.exe --self-test
python3 scripts/test_canvas.py --screenshot scratch/canvas-repair.png
```

Native unit result: **920 passed, two existing macOS private-bus skips**. Strict
lint, full OCaml expect tests and formatting pass. Native executables were built
with the shown feature set and run sequentially under bounded child cleanup.
The controls executable also passes with CI's exact `--features native-tests`
configuration (`ci-controls-exact-feature-001.log`).
Native tests use actual windows/GPU and scoped AX/native input fixtures; they do
not all inject real OS keyboard events. Python walkthroughs use external macOS AX
and targeted OS events. Some functional reruns overlapped compilation, so their
timings are diagnostics, not performance budgets.

Raw evidence under `scratch/agents/root-20261004-resumed/`: controls/number
`ci-native-*-repair-001.log`, slider `ci-native-slider-repair-002.log`, images and
chat `ci-public-*-repair-001.log`, calendar/canvas `ci-public-*-repair-002.log`,
`ci-native-repair-unit-001.log`, `ci-native-repair-clippy-001.log`,
`ci-repair-dune-tests-001.log`, and `ci-repair-format-002.log`. Earlier failed
attempts are retained. All owned GUI children are reaped. Fresh required hosted
checks and the remaining milestone acceptance are still required.

## Hosted follow-up at 1921ae3

Run37251380938 finishes with Linux passing and four macOS failures. The following
repairs are subsequent local changes; a fresh hosted run remains required.

- **Small mixed checkbox:** native GPU readback on CI finds no dash at size8.
  A TestPlatform regression reproduces this on a 1× display before the repair:
  GPUI rounds both edges of the subpixel-height quad to the same device pixel,
  removing the primitive. The renderer now preserves its center while enforcing
  a minimum one-device-pixel height. Both paint tests pass, including scales
  1, 1.25, 1.5, 2 and 3; the full native image executable passes locally with
  21 control size/state cases, clipping/alpha, AX and cleanup. The local physical
  display is Retina; the 1× regression uses TestPlatform rather than claiming
  physical 1× desktop coverage.
- **Public color picker:** `Not_mounted` occurs when the self-test treats two
  painted frames as readiness. Popup rendering and native draft observation are
  asynchronous. Reopening now waits, with a 120-frame bound, for an open picker
  with a draft before issuing commands. The complete public self-test passes.
- **Carousel:** the failed vertical-arrow step immediately followed asynchronous
  direction activation. The driver now observes the accepted vertical button
  label before focusing the region and sending Down. The physical keyboard/AX
  walkthrough passes through both axes, editor retention/unmount, modal/disabled
  routing and native auto-advance.
- **Color-input idle measurement:** CI sees three extra frames after the fixed
  100ms settling delay. The original short test passes locally, but the stronger
  check reproduces four late frames: it requires a 250ms quiet window
  within a two-second settling bound before measuring 600ms with zero redraws,
  extending the former 150ms observation beyond the caret's 500ms period.
  A deterministic Base regression identifies programmatic edits starting a blink
  timer without focus, while blur left a pending timer alive. Cursor activation
  is now explicit: focus starts it, blur/window deactivation cancels it, and
  unfocused edits cannot restart it. Existing active-cursor pause/selection
  behavior remains tested. With the repair, all three physical 64-owner cycles
  pass the stronger idle observation and owner/child disposal.
  Removing those accidental wakes also exposes a geometry consumer relying on
  a later cursor frame. Changed geometry now notifies after paint, ensuring a
  cached consumer receives it without continuous idle invalidation. All **269
  Base input-related tests** pass, including that consumer regression. This
  identifies a real local defect; the prior CI log alone cannot classify every
  extra frame it recorded.

Logs: `checkbox-scale-before-001.log` (expected failure),
`checkbox-scale-after-001.log` (two passes), `native-ci-reruns-001.log` (physical
color/image passes), `public-ci-clipboard-reruns-001.log` (picker/carousel passes),
and `clipboard-final-validation-003.log` (broader checks and the strengthened
idle failure) in the same scratch session. Cursor regression logs are
`cursor-idle-before-003.log` (expected failure), `cursor-idle-after-003.log`
(269 passes), and `cursor-native-validation-001.log` (physical idle repair).
The earlier standalone Base command attempts lacked the repository's explicit
local dependency overrides and were rejected without changing a lock or pin.
The successful Base command uses the same overrides documented in
[editor geometry evidence](editor-range-geometry-och41.md).

The cumulative `third_party/patches/gpui-base.patch` and its manifest hash include
these three-file adaptations. Reconstructing with `scripts/vendor_gpui_base.py`
and the hash-verified upstream archive reproduces all **235 files** byte-for-byte,
excluding the documented ignored standalone Cargo.lock. The pinned upstream and
toolchain versions are unchanged. None of these functional-test durations is a
performance measurement.

Final follow-up validation passes after the cursor/geometry repair:
`cursor-native-validation-001.log` records the stronger physical color test,
923 native unit tests (two existing macOS skips), strict combined-feature
all-target Clippy and full Dune expect/build/format checks. The rebuilt picker,
carousel and clipboard walkthroughs pass again in `cursor-public-validation-001.log`.
Real Japanese IME also passes after the cursor change: OS preedit/candidates,
commit, undo/redo, cancel, exact source-state restoration and shutdown
(`ime-cursor-001/`). This remains the scoped single-line/method qualification;
other IME/VoiceOver/release requirements are not inferred from it.
