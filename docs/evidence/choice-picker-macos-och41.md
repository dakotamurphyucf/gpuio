# Choice picker physical acceptance follow-up — OCH-41

2026-10-04, local macOS 14.5 arm64 desktop. Production baseline `984210e`, plus
the calendar wrapper repair and changes described here. This supplements native
TestPlatform evidence with actual AppKit accessibility and keyboard interaction;
it does not qualify VoiceOver, candidate IME or Linux GUI behavior.

The first walkthrough passed controlled selection and disabling, then sent Escape
immediately after requesting a controlled popup to open. The request travels
through Bonsai before native opening; the key arrived too early. The test now
waits for the trigger's actual `AXExpanded` value before continuing. The existing
popup-state helper accepts an explicit role so both date/color buttons and choice
picker popup buttons use the same bounded check. Escape still must close the
popup and remove its accessible options.

The next walkthrough reached the empty picker. Its screenshot showed “Make room
for your first idea,” but the accessibility lookup timed out. The renderer used
`control_label`, which deliberately hides decorative descendants when a control
owns their accessible name. Empty-state instructions have no option supplying that
name. The custom empty slot now renders through the ordinary element path;
passive-content admission remains enforced. A native regression checks that the
instruction exists with no hidden ancestor when open and reopened, and disappears
when closed. All twelve picker-host tests pass.

The final physical walkthrough exits successfully:

- Grouped multi-selection and clearing, followed by Escape dismissal.
- Controlled destination selection, disabled option/picker policy, and reopening
  followed by Escape after native expansion.
- Search across 4,096 options and query retention after selection/reopening.
- Accessible empty instructions, the native Create action, selection of the new
  workspace, reset, restored empty instructions and clean shutdown.

Search assignment uses the native AXValue setter. Escape is an OS keyboard event;
the walkthrough does not claim physical typing or IME validation for that search.

Commands use the isolated repository environment:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked --offline -j2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests \
  --lib choice_picker_host_test
python3 scripts/test_gallery.py --section choice-pickers \
  --images scratch/agents/root-20261004-resumed/choice-pickers-images-003
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked --offline -j2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests --lib
```

The full native suite passes **920 tests, two existing macOS private-bus skips**.
Logs: `picker-empty-native-002.log`, `gallery-choice-pickers-003.log` and
`picker-final-feature-tests-001.log` in the same ignored session directory.
Earlier gallery logs `001` and `002` retain the ordering and inaccessible-content
failures respectively. The initial new-test compile rejected a Rust 2024
reference pattern; its corrected explicit node-ID access is in the passing run.
These results do not close the broader gallery or release acceptance tickets.

Strict native Clippy also passes with both native test features and all targets:
`GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked --offline -j2
-p gpuio-native --features native-image-tests,native-canvas-tests --all-targets
-- -D warnings` (`picker-final-clippy-001.log`). The upstream `block 0.1.6`
future-incompatibility notice remains; it is not a first-party lint failure.
