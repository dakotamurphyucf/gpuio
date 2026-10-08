# Pagination and breadcrumb macOS qualification — OCH-41

2026-10-08, macOS 14.5 arm64, Apple M1 Max; source based on `09a2cc7c`.
The public gallery now has a focused `--section pagination` walkthrough. It
passes six Light/Dark × Comfortable/Large/Compact application-size combinations
in both the repository gallery and a freshly staged installed consumer. These
are application text/layout sizes, not physical monitor-scale transitions.
OCH-41 and OCH-17 remain open.

## Interaction and lifetime evidence

Each case exercises the actual public OCaml/Bonsai/Eio composition:

- Return on the Workspace breadcrumb changes the route; the current member
  becomes passive. The passive-workspace setting removes its link, while Studio
  remains actionable. The test restores the route through public buttons.
- The two gap anchors expose independent expanded state. Semantic activation,
  native numeric focus, Cancel focus return and typed page42 work. Enter normalizes
  the field without navigating; explicit Go requests page42 through the controller.
- Escape in the numeric field restores its committed value without closing the
  chooser. Escape after focusing Cancel dismisses the popover and restores its
  trigger. A new opening gets a different native AX field identity.
- Shrinking120 pages to3 closes an open chooser, clamps the current page, and
  disables boundary actions. Empty clears the current page; growth starts at1.
  Disabling while open removes the chooser. Compact likewise closes it, removes
  gaps, and keeps Previous/Next functional.
- At one billion pages, the range3..999999999 exposes seven expected shortcut
  buttons and **14 AX nodes** in the chooser, in every final root/installed case.
  Typing1000000000 and pressing Enter clamps to999999999 without navigation;
  Go selects that page and Next reaches the final boundary.
- Leaving Navigation with the chooser open removes its field and deactivates
  the opening. On return, the Bonsai page/count model is retained and the chooser
  is closed. The test resets the model through public actions between cases.
  This distinguishes application-state retention from native-editor retirement.

US input source is verified without changing it. Keyboard checks use foreground
OS event delivery; settings and some activations use AX actions. Neither driver
uses the clipboard. Both final applications close and are reaped normally. No
VoiceOver operation, signing or OS setting mutation occurs.

## Visual defect and repair

The first complete root/installed input runs pass, but screenshot review reveals
pale text on a pale mint button background in Dark. This is not visual acceptance.
The managed chooser used ordinary button defaults, pairing the theme's accent
background with its ordinary foreground. The gallery's light dark-theme accent
makes that pairing unsuitable.

`Pagination_component.view` now supplies the theme's paired background/foreground
colors to its seven shortcuts, Cancel and Go, retaining accent borders. The
navigation row already uses the same surface/text pairing. No public API, wire
operation, IDs, handlers or focus rules change. The walkthrough now samples actual
painted surface and text pixels for a shortcut, Cancel and Go in every theme/size
case. All18 button samples per final run pass. Light/Comfortable and Dark/Large
captures were inspected; before/after Dark captures are retained.

The paired OCaml/native transaction fixtures change only the nine background
colors for each opening (fixtures1,3,4). Under `Theme.default`, the color bytes
change from386ac8 to172136; message lengths and all remaining bytes are unchanged.
The initial fixture failure is retained. A temporary generator emitted proposed
bytes for review, then the original equality test was restored. No expectation
was promoted. The archive contains the exact byte-difference review.

## Commands and artifacts

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @lib/eio/runtest @fmt examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-image-tests --lib --locked -j 2 public_pagination_popup_routes_keyboard_and_restores_gap_focus_on_close
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace scratch/pagination-consumer
python3 scripts/test_gallery.py --section pagination --images scratch/pagination-root
python3 scripts/test_gallery.py --section pagination --executable <installed-main.exe> --images scratch/pagination-installed
```

The Eio suite, formatting and native paired-fixture regression pass. Both fresh
consumer builds pass the independent counter/document catalog admission checks.
Final native runs use four-minute watchdog wrappers; output is retained as
`pagination-native-006` and `pagination-installed-002`. The latter includes the
color repair. Executable SHA256:

- Repository: `536c59ff4feef369ec722dca8b8e24d14a9a6a2fd83fdb3e056c5a6286e034ad`.
- Installed: `4082c88141ab319bbd6dee136a9233983fcea7046d3c0c35114dbc0433bc5487`.

Python compilation, Ruff on the new driver, actionlint, catalog/example inventories
and diff checks pass. The legacy `test_gallery.py` has244 pre-existing Ruff
findings; comparison against HEAD confirms no new findings from its selector/
dispatch addition. This is not a claim that the entire old driver is lint-clean.

[Verified28-artifact archive](pagination-macos-och41/reports.tar.gz) and
[SHA256/size manifest](pagination-macos-och41/manifest.json) contain reports,
captures, command logs, fixture review and lint comparison. Initial driver failures
are retained: a syntax error, incorrectly expecting numeric Escape to dismiss,
selecting Disclosure's identically named Disabled button, and incorrectly expecting
Bonsai's model to reset after page deactivation. Source/native inspection corrected
those test assumptions. The production repair addresses the observed colors.

The four-minute focused case is added to macOS Foundation. Existing run37772649589
predates it. This provides bounded controls, native input, focus and scoped visual
qualification, not a measured memory/FPS result, VoiceOver qualification, Linux
compositor acceptance, or whole-catalog/release completion.
