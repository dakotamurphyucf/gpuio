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
