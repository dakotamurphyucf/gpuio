# macOS CI readiness checks — OCH-17 / OCH-41

2026-10-05. Logs from the still-running
[hosted run 37343201640](https://github.com/dakotamurphyucf/gpuio/actions/runs/37343201640)
identify failures in the theme-file walkthrough and standalone calendar driver.
The runner checkout is merge `04dcd08` of branch `470210a` into `af7f6f0`;
the archived checkout excerpt preserves the complete revision. Linux has passed.
These are individual step results, not a terminal result for the macOS job.

## Calendar input ordering

The hosted driver requests focus on March 5, sends Right, verifies that March 6
is disabled, then sends Return and expects `Disabled_date`. The failure dump
instead shows selection March 5. Checking March 6's immutable disabled property
does not prove the preceding focus request has completed. AX requests and CG key
events are delivered through different paths.

The corrected driver waits for March 5's `AXFocused=True` before sending the
ordered Right/Return keys. It retains both the actual `Disabled_date` rejection
and unchanged selected-value checks. This acknowledges the intended starting
cursor rather than delaying by a guessed interval.

An intermediate local harness also tried waiting for `AXFocused` on disabled
March 6; that property did not become true. The final driver uses the enabled
starting cursor and actual rejection as its delivery evidence. It does not
qualify the disabled target's AX focus projection. No accessibility adapter or
VoiceOver work was performed.

The full corrected calendar walkthrough exits 0 locally, including leap-day
navigation, constraints, month/year modes, native keys, Tab order, ranges,
read-only/disabled policy, remount and shutdown. The application was rebuilt from
the current repository source before the run; executable SHA-256:
`ecc6cdad398b9f3cf1b83c6c862fae0c35a035595cdd4450c8db0acfba1384a2`.

## Theme page readiness

The hosted theme test passes its earlier delayed reads but times out waiting
for `Loading theme…` in the final window-close case. Immediately beforehand,
it switches away and back and sees retained `Loaded Aurora` text. The application
preserves this status while its page-scoped Eio capability can still be
reacquiring; the Reload button is disabled until that scope is Ready. The old
driver did not wait for readiness, and its back-to-back page actions also lacked
an observable departure boundary. A dropped Reload during that interval is
consistent with the failure; the log alone does not expose the exact dispatch.

The corrected driver acknowledges the Presentation page before returning and
waits for Reload to be enabled before each request. It preserves all held-FIFO,
native descriptor, generation, palette, editor/history and cancellation checks.
It does not substitute EOF for cancellation or extend any timeout.

All **12 theme cases** pass locally against the installed application at
`71bd02e`, SHA-256
`d4ad6a43bd258ea414f02891965de3a54c54f8a333b2b9fbdc0b8e39220a3402`.
The final window closes while the FIFO writer remains open, and the child is
reaped before EOF is released. The report records `complete=true` and
`child_reaped=true`.

## Reproduction and limits

Driver correction: `5070283`. Only test synchronization changes; no application,
dependency, protocol or adapter changes. Local platform: macOS 14.5 arm64 / M1 Max.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/calendar/main.exe
python3 scripts/test_calendar.py
python3 scripts/test_macos_theme_files.py --executable <installed-gallery> --output <fresh-output-directory>
```

Python compilation, structural catalog audit and whitespace checks pass.
[The archive](macos-ci-readiness-och17/local-validation.tar.gz) and
[manifest](macos-ci-readiness-och17/manifest.json) preserve hosted step/checkout
logs, run metadata, the intermediate local failure, passing local results,
theme report/captures, driver patch and executable hashes.
The corrected hosted run is still required; neither original CI failure is
waived. These local passes do not qualify the rest of milestone 07, VoiceOver,
Linux GUI or clean-machine distribution.

## Settings and navigation follow-up — run 37356882651

The later [hosted run](https://github.com/dakotamurphyucf/gpuio/actions/runs/37356882651)
at branch `e96d27e` passes Linux but reports two further macOS failures while its
long table-history step is still running. The job log was retrieved directly
from GitHub's job-log endpoint for job `111921432650`; only the relevant step
excerpts are archived here. This is not a terminal macOS result.

Settings composition fails immediately after Escape cancels a numeric dead-key
preedit. The native editor updates before its asynchronous Bonsai snapshot removes
the explanatory composing row. The test previously called the one-shot `absent`
helper at that point. It now uses the existing ten-second `wait_absent` helper
before continuing with real keyboard replacement and the unchanged unaccented
value check. No fixed sleep, product behavior or expected result changes.

The navigation workload requests a 500-point-wide window but sees the preceding
300-point child width. Waiting for GPUI frame callbacks alone does not establish
that AppKit has delivered its asynchronous resize notification. The test now polls
the actual `window.viewport_size()` for at most 200 ten-millisecond intervals,
then requests the existing frame barrier and runs the original exact child-width,
slide-offset and rendered-pixel checks. Failure to receive the requested viewport
still fails explicitly; layout is not repeatedly retried until it passes.

Local macOS 14.5 / M1 Max validation, based on `b457e0c` plus these two test edits:

- The fresh release gallery passes the complete `settings-composition` walkthrough
  and shutdown: real US-layout dead-key input, retained native owner/preedit across
  layout/theme/size changes, reset rejection, committed values, undo/redo, page
  retirement, Escape cancellation and subsequent ordinary input. No input source
  or VoiceOver setting is changed. Executable SHA-256:
  `2e09bd0fbc0aae3b31b9cfaa5fb9aa9be093f9cee4704dae17316457cace0440`.
- The native navigation executable passes its full sequence, including nested
  scopes, inert/disabled input, 128 retained page/button traversal, four editor
  owners, all three real-window resizes with GPU pixel checks, unmount and teardown.
  Executable SHA-256:
  `25f773f4d69f34867dbb7a52d00393d0007ad497494e1aceb46e4d256812147e`.
- Python compilation, Rust formatting, targeted strict Clippy and whitespace
  checks pass. No owned window overlaps another or a local compilation; the
  bounded native runner and gallery driver both exit zero and reap their children.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -p gpuio-native \
  --features native-image-tests --test native_navigation --no-run
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release -j 2 examples/gallery/main.exe
python3 scripts/test_gallery.py --section settings-composition \
  --images scratch/agents/root-20261004-resumed/ci-settings-composition-repaired
python3 -c 'import subprocess; subprocess.run(["target/debug/deps/native_navigation-58ae141ed1687c77"], check=True, timeout=90)'
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -p gpuio-native \
  --features native-image-tests --test native_navigation --no-deps -- -D warnings
```

[The follow-up archive](macos-ci-readiness-och17/settings-navigation.tar.gz) retains
both hosted failures, build/check/native logs, exact patch and test-source snapshots;
its [manifest](macos-ci-readiness-och17/settings-navigation-manifest.json) is verified
against the archive. Corrected hosted execution remains required. These local
results neither waive the earlier failures nor qualify the still-open startup,
presentation, accessibility and release requirements.
