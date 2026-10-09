# Flat split-group desktop walkthrough — OCH-41

2026-10-08, physical macOS 14.5 arm64, base `e510ab7d`. No production code
changes. The repository gallery and installed consumer pass the same public-API
walkthrough through `--section split-group`.

## Qualified behavior

Four Dark/Light × horizontal/vertical cases exercise real pointer previews,
completed drags and Escape cancellation. A thirty-pixel preview must actually
move the exposed divider value before release/cancellation is tested. Release
commits the new boundary; Escape restores the prior committed value, including
after pointer release. The completed-size caption is checked against current
Files/Draft extents, verifying asynchronous delivery through the example's
`on_resize` callback into Bonsai, not just native geometry movement.

Each case also uses actual axis-appropriate arrows with the default sixteen-pixel
step, Home/End to feasible limits, and macOS AXIncrement/AXDecrement. The native
draft remains edited throughout. US/ABC layout is verified before OS keys; pointer
targets must belong to the owned app.

A serialled request sizes Draft to 320. A subsequent keyboard resize makes it
304; toggling passive grips and the callback-driven Bonsai render do not replay
the old request. The custom grip keeps the sixteen-pixel hit extent. Constraining
all panels to a maximum of 200 clamps Draft to 200, and a new 320 request remains
at 200. Request captions match actual boundary-derived sizes.

Reorder, Inspector hide/show, Outline insertion/removal and reset preserve the
edited draft. Retired boundaries disappear from accessibility and newly relevant
boundaries appear. Page departure removes the editor; returning recreates its
initial text. Both apps exit normally. This is native editor lifetime evidence,
not a measured resource-retirement or persistence claim.

## Commands, provenance and attempts

```sh
python3 scripts/test_gallery.py --section split-group --images scratch/split-group
python3 scripts/test_gallery.py --section split-group --executable /path/to/installed/main.exe --images scratch/split-group-installed
python3 -m py_compile scripts/gallery_split_group.py scripts/test_gallery.py
python3 scripts/audit_example_docs.py
python3 scripts/audit_component_catalog.py
git diff --check
```

The first fixture tried to find the editor before revealing its offscreen native
group and failed. The group publishes its children when it has usable visible
layout. The corrected helper reveals the outer group first using the page gutter;
it also uses the documented sixteen-pixel keyboard step instead of its initial
ten-pixel assumption. Local `002` passed. Final local `003` adds the explicit
Bonsai caption checks, and it and installed `001` pass. All children were reaped.

The independent consumer was built for the
[scroll adapter checkpoint](macos-scroll-phases-och41.md); unchanged application
and native sources permit reuse, not a new-install claim. Executable hashes in
the logs match that checkpoint. Local runs use a 180-second exception/cleanup
wrapper. A three-minute Foundation step now runs the regression. Python syntax,
focused Ruff checks, example/catalog audits, actionlint and whitespace pass.

The [archive](split-group-macos-och41/reports.tar.gz) retains all attempts,
reports and final source snapshots, verified against its
[manifest](split-group-macos-och41/manifest.json). The
[adjacent walkthrough](../../examples/gallery/split_group_preview.md) explains
the native/OCaml ownership split and the request → observation → render sequence.

Broader sizes/window geometries, capture loss/deactivation, GPU paint, full
Tab/VoiceOver, idle/resource budgets and consolidated release acceptance remain
separate. Sampled AX movement does not prove smooth presentation. Linux desktop
qualification remains deferred; required nongraphical checks remain in force.
