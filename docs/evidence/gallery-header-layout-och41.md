# Gallery header reflow — OCH-41

Local macOS arm64, 2026-10-07, based on `708a013b`. The Large-scale Markdown
page pushed New window **25 logical pixels outside** its 1120-pixel window.
The original actual-window check fails against that bound; its screenshot and
log are retained.

## Repair

The example header is a wrapping row. Its heading column has a 320-pixel flex
basis, can grow, and can shrink below its text's intrinsic width. The appearance,
scale and window controls form their own wrapping row, capped to the available
width. When both groups no longer fit side by side, the toolbar moves below the
heading. This is ordinary public OCaml `View`/`Style` composition, with no Rust
change, breakpoint model or resize callback. The adjacent
[implementation walkthrough](../../examples/gallery/shell.md) explains the
properties and event ownership.

The native-menu gallery test also now discovers the newly opened window by its
actual AX identity/title set. The application increments window serials, so a
hardcoded “Studio 2” cannot compose with earlier tests that already opened a
window. Two consecutive complete menu-bar exercises pass in one gallery process.
This is a test-driver correction; application window naming is unchanged.

## Local verification

The focused `--section header-layout` driver passes **59 cases**: all 24 page
headings at Large scale at widths 900 and 1120, additional Markdown cases across
Dark/Light and Compact/Comfortable/Large, and the original reproduction case.
It checks actual window/control rectangles and overlap with the heading. Twelve
representative screenshots are retained; the Dark/Large narrow render was
visually reviewed with its complete heading, description and toolbar visible.
A real pointer click on New window opens the second window. Both windows close
normally and the child exits zero.

The repository gallery executable SHA-256 is
`05ce5f270878537dad7a19133b101af9b5b8a7bd900e2cc075cf6fa52a33b1ae`.
Dune build and Jane Street formatting pass. Documentation and catalog audits are
structural checks, distinct from the actual layout evidence. No new native
library test count is claimed for this OCaml-only layout change.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe
python3 scripts/test_gallery.py --section header-layout --images scratch/header
```

The layout driver is deliberately a standalone section: it resizes the window,
changes appearance and opens another window. Its scope is shell geometry and
New window interaction, not all page content, all possible window widths,
VoiceOver, Linux desktop behavior, performance or complete catalog acceptance.
OCH-41 and OCH-17 remain In Progress.

## Independently installed consumer

A fresh consumer built against an isolated staged prefix also passes both the
59-case header walkthrough and the complete native menu-bar section. The build
includes the document-profile expect-test alias and a no-window catalog check
for the counter and document-profile backends. It does not install into an opam
switch or change the developer's defaults.

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/bar-header-consumer
python3 scripts/test_gallery.py \
  --executable scratch/bar-header-consumer/consumer/_build/default/main.exe \
  --section native-bar --images scratch/installed-bar
python3 scripts/test_gallery.py \
  --executable scratch/bar-header-consumer/consumer/_build/default/main.exe \
  --section header-layout --images scratch/installed-header
```

Installed consumer executable SHA-256:
`4af26501dbc837a3dfa463a21e15bb71752b70d8ce8fcb9c71f21b87cf34c192`.
The two drivers exit zero and reap their windows/processes. These runs do not
operate VoiceOver or invoke clipboard commands. They qualify these public API
paths, not the entire installed gallery, clean-machine distribution or Linux.

[Logs, source and captures](gallery-header-layout-och41/reports.tar.gz) are indexed
by a [SHA-256 manifest](gallery-header-layout-och41/manifest.json). The original
failed bounds check is retained with the passing local and installed results.
