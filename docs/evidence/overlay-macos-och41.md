# Overlay macOS focus and geometry — OCH-41

2026-10-08, macOS 14.5 arm64, Apple M1 Max. Test/ documentation source is based
on `61b5f310`; production binaries are unchanged from the final pagination build
recorded in [its evidence](pagination-macos-och41.md). No production repair was
needed. OCH-41 and OCH-17 remain open.

## Actual desktop coverage

The new `overlay-matrix` selector first runs the existing overlay walkthrough
once: live backdrop changes preserve modal identity/semantics, animation can be
toggled, confirmation can cancel or confirm, details close with focus restoration,
and three focus-triggered tooltips replace prior help before Escape removes it.
These baseline cases are not repeated for every appearance combination.

Then six Light/Dark × Comfortable/Large/Compact cases pass in both repository and
installed galleries. Sizes are application text/layout preferences, not physical
monitor-scale transitions. Full application motion is selected for Comfortable
and Large, Reduced for Compact. This is not the full size/motion cross-product.

Each case verifies:

- Dialog exposes AXModal, wraps forward/reverse Tab between its two buttons,
  retains its native frame while changing backdrop, and restores the trigger
  after Escape.
- The same drawer frame moves through Right, Bottom, Left and Top. Toggling the
  public `Sheet.Insets` setting applies top/right/bottom/left56/16/16/16 logical
  pixels, then restores the original bounds. Right/Left lose72px height;
  Top/Bottom lose32px width. Edge-specific origin changes and native identity
  are checked within1px. Tab wraps from Close to the inset checkbox; Escape
  removes the drawer and restores Open drawer.
- Focusing Contributor opens an interactive nonmodal card. Return on View profile
  reaches the Bonsai notice, and Escape closes/restores Contributor focus.
- The retained placement panel updates all four corners relative to content
  point560,400. Right corners subtract measured width and bottom corners subtract
  measured height. This point fits the tested viewports; arbitrary clamping,
  oversized panels, different margins and resizing remain separate native tests.
- Navigating away with Details open retires it, and returning does not reopen it.

Keys use foreground OS delivery; most button/setting changes use accessibility
operations. Neither run types text, manipulates the clipboard or changes OS motion
settings. Both processes close and are reaped normally. Right-inset drawer captures
are retained for every case; the Dark/Large capture was visually inspected for
readable content and panel containment. Sampled rectangles and captures do not
establish frame-by-frame animation smoothness, physical presentation timing,
VoiceOver speech/navigation, IME, pointer-hover deadlines, full nested-surface
behavior, resource bounds or Linux compositor acceptance.

## Reproduction and identities

```sh
python3 scripts/test_gallery.py --section overlay-matrix --images scratch/overlay-root
python3 scripts/test_gallery.py --section overlay-matrix --executable <installed-main.exe> --images scratch/overlay-installed
```

The actual runs use240-second Python SIGALRM wrappers that raise through harness
cleanup. Logs are `overlay-native-001.log` and `overlay-installed-001.log`; both
end with six successful cases and `GPUIO_GALLERY_AX_OK`. The installed executable
comes from the fresh `pagination-colors-consumer` staging/build, reused because
production code has not changed. It is not a new installation performed by this
check. SHA256 identities printed by the harness:

- Repository: `536c59ff4feef369ec722dca8b8e24d14a9a6a2fd83fdb3e056c5a6286e034ad`.
- Installed: `4082c88141ab319bbd6dee136a9233983fcea7046d3c0c35114dbc0433bc5487`.

The [18-artifact archive](overlay-macos-och41/reports.tar.gz) retains5,638,823
uncompressed bytes of logs, reports and captures. Every entry was read back and
verified against the [SHA256/size manifest](overlay-macos-och41/manifest.json).

Python syntax, new-module Ruff, catalog/example audits, actionlint and diff checks
pass. The legacy driver retains244 pre-existing Ruff findings; comparison with
HEAD finds no added diagnostics. A four-minute focused Foundation step is added.
The currently running Foundation37789987337 predates this test addition; no hosted
pass is claimed for it. Full release gates remain unchanged.
