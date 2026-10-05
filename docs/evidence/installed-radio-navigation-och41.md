# Installed standalone radio navigation — OCH-41

On 2026-10-05, the public gallery passes the standalone-radio walkthrough on
macOS 14.5 arm64 / M1 Max / built-in display. This reuses the independently
installed consumer from [the lifetime repairs](gallery-lifetime-repairs-och41.md),
application source `e210d3e4a7b261e6bdf40023023937fbeb096e85`, executable SHA-256
`5e96404a5df658b7dfa9af05b8d4ff8240737e8df0f751f05c58968a0e24d495`.
Only the test driver and documentation change in this checkpoint.

## Observed behavior

- The composed group exposes vertical `AXRadioGroup` semantics with three named
  `AXRadioButton` children. Queries are scoped to the group because other gallery
  cards reuse the Fast and Balanced labels. The application model and checked
  values remain exclusive; enabled radios also report matching `AXSelected`.
- Real Tab/Shift-Tab follows tree order at equal indices. Space and Return select
  choices in both Light and Dark. Activating the checked choice leaves it checked.
  Right Arrow leaves independent focus and selection unchanged. This public-state
  check does not count queued callbacks; the existing native tests cover checked
  callback suppression separately.
- Negative indices remain traversable: Deep (-2) precedes Fast (-1), with reverse
  traversal back to Deep. Ordering belongs to the whole focus scope, so the test
  does not assert a trapped three-item group or contiguous traversal through other
  index-zero controls.
- Skipping Fast removes it from forward/backward sequential traversal while a
  real pointer click on its lower descriptive label still selects and focuses it.
  Passive descriptions are not additional AX text leaves. Replacing rich labels
  with plain strings shrinks the geometry and preserves native AX identity and
  selection; restoring rich labels also preserves identity.
- Disabling the choices preserves their owners and checked state and rejects a
  stale activation intent. Re-enabling and resetting the navigation options
  restores ordinary Tab traversal. Leaving the page removes the group; reopening
  creates fresh native owners while retaining the application's selected value.
  Old references cannot select before or after remount; the new owner accepts
  Return normally. The driver closes and reaps its application.

Light and Dark captures were visually inspected: all three choices, the selected
indicator and focus outline are visible. This is not an exact pixel-color test.

## Corrected platform expectation

The first run exits 1 at the disabled-state assertion: Fast remains checked in
`AXValue` and in the application readout, but `AXSelected` is false. The pinned
macOS adapter's `isAccessibilitySelected` requires `Node::is_selectable`; pinned
`accesskit_consumer` 0.38.0 excludes disabled nodes from that predicate. This
differs from the underlying AccessKit node's selected metadata, which existing
native semantic tests correctly preserve while disabled.

The driver now explicitly checks that platform projection: checked values remain
exclusive while disabled `AXSelected` values are false. It does not drop the
selection, disabled-action or identity assertions. The corrected second run exits
0 with `GALLERY_CHECKABLE_NAVIGATION_OK` and `GPUIO_GALLERY_AX_OK`. No production
code or accessibility adapter changed. This observation does not settle future
screen-reader acceptance; **VoiceOver remains on owner hold and was untouched**.

## Reproduction and limits

```sh
python3 scripts/test_gallery.py --section checkable-navigation \
  --executable <installed-main.exe> --images <output>
```

The focused selector is also included in `all`; this checkpoint ran the focused
selector, not the entire gallery. The [archive](installed-radio-navigation-och41/local-validation.tar.gz)
retains exact commands/results, both driver versions, original failure output,
captures and the passing driver. The [manifest](installed-radio-navigation-och41/manifest.json)
contains hashes. Python compilation, the catalog audit and whitespace checks pass.

This closes the scoped installed standalone-radio/Tab-order walkthrough gap.
It does not qualify VoiceOver, Linux desktop, full-family resource/performance,
all remaining gallery scenarios, final-source hosted CI or release distribution.
Linux desktop qualification remains deferred to OCH-47.
