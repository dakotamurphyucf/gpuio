# Installed picker presets and focus recovery — OCH-41

On 2026-10-05, the installed public gallery on macOS 14.5 arm64 / M1 Max /
built-in display passes calendar-content, date-preset and popup color checks.
This is scoped component evidence, not complete picker or release acceptance.
VoiceOver remains on owner hold; no VoiceOver settings, automation or accessibility
adapter code were changed.

## Focus defect and repair

The initial baseline passes at application revision `b891868`, but an extended
walkthrough reproduces a failure twice: selecting a date preset and cancelling
preserves the saved date yet fails to return keyboard focus to the trigger.
The original logs and screenshots are retained.

A native regression isolates a nonmodal popup whose controls are initially
unavailable or become disabled while a request is pending. The shared focus
manager discards its focus context: the fallback accounts for modal traps but
not an already focused nonmodal scope. Without that context, closing the popup
cannot restore its anchor. Before the repair, the regression fails when the
focused control becomes disabled.

Commit `71bd02e44666a86945d642cb14a8f107db2869d7` retains the innermost eligible
scope that already contains focus, both when its focused child becomes ineligible
and when no eligible child paints. It does not reclaim focus moved outside the
popup. Existing external-focus and disabled/removed-anchor tests remain intact.
No public API, protocol, vendor or accessibility adapter changes are involved.

The full native suite passes **931 tests with two existing ignores**; strict
native lib/tests Clippy with `-D warnings`, Rust formatting and Python compilation
also pass. An initial invocation used a nonexistent manifest path and was corrected
before reproducing the native failure; it is not counted as a behavior result.

A fresh staged installed consumer at `71bd02e` builds and passes both catalog
checks. Executable SHA-256:
`d4ad6a43bd258ea414f02891965de3a54c54f8a333b2b9fbdc0b8e39220a3402`.
The first repaired desktop run passes preset focus recovery but stops at a driver
error: the read-only checkbox was looked up as a button. Correcting that role,
without changing application code or expectations, produces the complete passing
run. Both the preset and overall picker success markers are recorded, exit 0.
The driver closes and reaps its application.

## Actual walkthrough coverage

- Calendar event badges change from two to three events, hide and reappear while
  the native date target retains identity and checked state. The test now reads
  an actual Boolean; its previous string reader compared two absent values.
  Activating the adorned date starts the native range.
- The one-week preset selects September 21 in the draft while September 14 stays
  saved. Cancel restores the trigger and reopening restores the saved selection.
  Applying the preset updates the application value. Clearing then escaping
  preserves that value; reopening and applying Clear saves the empty selection.
  Demo day restores September 14.
- Read-only permits opening and inspection. All three presets and Apply are
  disabled, attempted AX actions preserve the date and keep the popup open,
  Cancel returns focus, and adjacent date/color clear actions recover after
  read-only is removed.
- Direct calendar selection also retains explicit Apply/Cancel behavior.
  The color popup retains its hex editor through Palette/HSLA switching; Iris
  followed by Cancel preserves the saved color, Coral followed by Apply updates
  it, and Escape preserves the committed value. Trigger expansion, focus return
  and adjacent clear actions are checked.

Actions use ordinary macOS AX control operations and actual Escape key delivery.
The target's `AXFocused` is awaited for focus assertions; diagnostic application
`AXFocusedUIElement` output is not treated as a focus assertion. Representative
read-only and committed-value captures were inspected. These captures do not
qualify a theme matrix, exact palette pixels or hover-preview behavior.

## Reproduction and remaining scope

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests,native-canvas-tests --lib --locked -j 2
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace <fresh-directory>
python3 scripts/test_gallery.py --section pickers --executable <fresh-directory>/consumer/_build/default/main.exe --images <output>
```

The [archive](installed-picker-presets-och41/local-validation.tar.gz) and
[manifest](installed-picker-presets-och41/manifest.json) preserve commands, terminal
results, original failures, final driver sources, production patch and captures.
The catalog audit passes structural coverage only.
Multi-month geometry/viewport changes, physical calendar and panel navigation,
palette hover-preview pixels, broader themes/scales, measured performance/resources
and clean-machine distribution remain separately qualified work. Required hosted
checks have not run against this final source yet; the preceding CI run remains
in progress with Linux successful. Linux GUI qualification remains deferred to
OCH-47. OCH-41 and OCH-17 remain open.
