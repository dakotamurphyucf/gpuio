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
