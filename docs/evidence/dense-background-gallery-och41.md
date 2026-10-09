# Data-owned bar backgrounds in the public gallery

OCH-41, 2026-10-06, local macOS 14.5 arm64 on Apple M1 Max, after `d8cdc50`.
The [foundation](dense-background-foundation-och41.md) qualifies the typed API,
bounded codecs, native resolution and large-source accounting. This checkpoint
adds the public OCaml example and its installed-package qualification. It does
not complete the catalog or the wider release gates.

The gallery's **Charts & data → Bar backgrounds** branch uses 24 categorical
observations with descending, noncontiguous datum IDs. A pure fixture module
constructs immutable source data; a separate Bonsai component owns control state,
effects and scoped registration. Colors resolve against a stored source theme;
**Apply preview colors** explicitly republishes them. Theme changes alone update
view styling. Sparse highlighting remains separate from source-owned fills.
Switching branches releases native resources, retains Bonsai choices and clears
selection when the source is reacquired.

## Local checks

Root and fresh installed-consumer `chart-backgrounds` walkthroughs pass:

- Home/End and Enter select original categories/values before and after updating
  and reversing the dataset. Batch 01 keeps datum identity, category 101 and the
  updated value 33; the reversed first batch has category 124 and value 51.
- Actual chart-region pixels show the initial source accent, the sparse amber
  highlight after reorder, and unchanged source colors after both view-theme
  changes. Explicit republishing paints the new light/dark accent and removes
  the previous accent from that region. These are positive/negative pixel checks,
  not merely event or ready-state assertions.
- Patterned bars retain original selection in all four directions. Mean and
  Uniform controls preserve original-data inspection; the first mean selection
  describes four categories. The native original-data browser reaches row 24.
- Leaving for the chart-family branch and returning recreates the source while
  retaining fixture step, patterns and applied-color count. Selection clears
  and can be made again. Leaving the page returns registered images/charts/canvases
  and source bytes to zero; the application shuts down normally.

The installed build stages public OCaml packages into a fresh prefix and copies
the example into a separate consumer. Counter/document-profile catalog handshakes
pass. Its five changed OCaml source/interface files match the root byte for byte.
It still uses the repository's pinned toolchain and composed Rust backend; this
is not clean-machine distribution or signing acceptance.

Full Dune `@all @runtest @fmt` passes. The two adjacent walkthroughs explain the
actual fixture, Bonsai graph/effects, public GPUIO calls, source publication and
scope lifetime. A GPT-6.1 Sol documentation agent drafted them; the parent reviewed
them against the code, clarified the fixture ID bound and added the test command.
The documentation audit reports **427 sources / 265 reviewed groups / 0 pending**.
That audit establishes structural coverage, not native behavior.

The first standalone run rejected a fully visible branch-navigation button:
the generic reveal helper assumed a 170-pixel top inset, while the new row starts
at 159. The helper now accepts an explicit inset; only branch navigation uses 150.
The full-chart regression subsequently exposed the radar helper waiting for
Ready before revealing an offscreen chart. It now reveals first, then waits for
native preparation. The next run passed radar, then exposed the inspection helper
moving the pointer after keyboard selection: the hover preview became Nova while
the committed selection remained Atlas. Pointer setup now precedes keyboard
inspection, and the standalone inspection walkthrough passes with its pixel
checks and cleanup. The subsequent combined run also showed that AX focus alone
does not reveal the full chart: the helper now explicitly reveals before focus,
geometry and keyboard assertions. All four failed runs and their screenshots
are retained.

The final complete `charts` walkthrough passes, including the new background
branch after the existing family, mark, axis, pie, rich inspection, radar,
inspection-pixel, ordinal, flow-label, stacked and categorical checks. It ends
with zero registered sources and bytes and normal shutdown.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe @fmt -j2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @all @runtest @fmt -j2
python3 scripts/test_gallery.py --section chart-backgrounds --images scratch/background-root
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace scratch/background-consumer
python3 scripts/test_gallery.py --section chart-backgrounds --executable scratch/background-consumer/consumer/_build/default/main.exe --images scratch/background-installed
python3 scripts/test_gallery.py --section charts --images scratch/background-all-charts
python3 scripts/audit_example_docs.py
```

The native driver is also included in the broader `charts`/`all` walkthrough.
Reviewed captures supplement real local keyboard/AX actions and pixel assertions;
they do not qualify physical IME, VoiceOver, 120 FPS presentation, Linux desktop
behavior or release packaging. Per-bar baselines remain separate catalog work.
Hosted run 37558498842 targets older `8a98315`, not this change.

The [raw logs and selected captures](dense-background-gallery-och41-logs.tar.gz)
and [SHA-256 manifest](dense-background-gallery-och41-manifest.json) include the
failed runs, final passing runs, exact commands/exits, source/executable hashes
and copied consumer source comparisons. Every archive member was hash-verified.
