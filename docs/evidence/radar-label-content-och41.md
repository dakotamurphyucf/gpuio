# Ordinary radar label Views — OCH-41

2026-10-06, macOS 14.5 arm64, source base `4b0b33c` plus this change.
The [contract](../design/radar-label-content.md) now has public OCaml construction,
paired wire metadata and a retained native adapter. This is scoped implementation
and local evidence, not completion of the chart catalog or milestone 07.

`Chart_radar_labels` validates at most 64 distinct positive axis IDs. `View.chart`
wraps each ordinary child with a stable axis key; Bonsai forwards the same API.
Unknown axes remain mounted but hidden. Native prepaint measures eligible Views
at natural size, places them with the pinned radar formula and suppresses their
plain captions. Labels retain ordinary View styles. Native buttons/editors own
input; no layout or paint callback enters OCaml. Custom labels omit the source
axis caption from the tooltip while retaining original values and series titles.

Chart-view metadata now begins with explicit version **-1** and carries ordered
axis IDs. Matching bridge revisions are required; legacy unversioned records are
rejected. Options/style/data schemas remain **7 / -2 / 1**. Independently assembled
fixtures cover empty and populated metadata. Native transactions validate slot
count/shape and IDs before committing. Source data is unchanged.

## Executed local checks

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @test/chart/runtest lib/bonsai/gpuio_bonsai.cma -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @all @runtest @fmt -j 2
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-protocol --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --lib --test chart_tree --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests --lib --tests --locked -j2 -- -D warnings
python3 scripts/test_gallery.py --section chart-radar --images scratch/agents/root-20261004-resumed/radar-content-gallery-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section chart-radar --workspace scratch/agents/root-20261004-resumed/radar-content-installed
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @test/chart/runtest @fmt -j 2
python3 scripts/audit_example_docs.py
```

The full OCaml build/expect/format run passed before the final gallery editor was
added; the final gallery then passed its targeted build and foreground walkthrough.
Native unit tests pass **649**, with two existing skips; all three chart-tree tests
pass. Strict Clippy includes dependencies. The documentation inventory remains
417 sources / 260 reviewed groups. These commands use the repository environment
and do not mutate another switch.

Core tests cover collection validation, paired bytes, keyed reorder/rename,
child-only updates without `Set_chart`, and stale removed-handler rejection.
The full Rust protocol suite passes **432** tests, including truncated/legacy/malformed metadata
and the 64/65-entry boundary. Tree tests reject malformed wrappers atomically.
Native unit tests check that custom captions change only the intended tooltip
and that one chart's hidden slots do not clear another chart/window's set.

The hidden-window production harness checks actual GPU pixels for intrinsic text
resizing, empty-series radar axes, hide/show, source-generation reset, live-axis
removal/return/reorder and unmount. Removal and reset gates are inspected in the
same native notification turn before replacement paint. The enclosing harness
also passes its pre-existing two-window managed-list streaming and teardown tests;
that does not establish two-window interaction coverage for custom labels.

The foreground gallery uses an ordinary OCaml button, two-line column and native
text input. Actual typing and Backspace work inside the label input; publication
preserves focus and draft. Accessible and Space-key button activation each run
one Bonsai effect. Ancestor disabling changes the button's enabled semantics.
Hide/show and the original-data browser preserve the editor draft; removing the
entries removes their accessible controls. Final resource counts are zero and
the test child exits normally. The custom-label screenshot was visually reviewed.
The [adjacent walkthrough](../../examples/gallery/charts_page.md) explains the
actual controller, helper, reactive syntax, effects and ownership boundaries.

The fresh installed consumer also passes. It stages public packages into its own
prefix, copies the OCaml application into an independent Dune workspace, links
the native backend and runs the real gallery. Its additional checks focus the
editor before disabling the chart, verify disabled semantics and lost focus,
send a key and confirm the draft is unchanged. Removing and re-adding labels
creates an empty native editor while the Bonsai counter remains 4. Final source
resource counts are zero. Executable SHA-256:
`dd50829715b602030dd2eed11553e5a9fdb08a0f0e72a682f6886b256c7e2596`.

## Failures retained and corrected

The first hidden-window test allocated node slots 90/91 in a tiny fresh tree;
that violated the allocator's admission contract. Consecutive slots 2/3 corrected
the fixture. A later reset assertion ran after awaiting publication, during which
GPUI could already prepare and paint the new generation. Diagnostics showed
matching live/ready generation 4. The corrected test inspects the real source
notification boundary synchronously and separately checks the new generation's
paint. It does not weaken the required hidden gate at that boundary.

The first foreground attempt scrolled upward to reveal Update before checking
editor focus. This moved the lower-axis editor outside the visible region and
legitimately removed it from accessibility. A second attempt fixed publication
but encountered the same driver scrolling at the disable/hide controls. The
passing driver invokes those accessible actions without changing the viewport,
so the assertions isolate state transitions and publication. Keyboard input is
still real foreground input; these actions are not claimed as mouse-click tests.

[Raw logs, failed attempts and screenshots](radar-label-content-och41-logs.tar.gz)
are retained with a [verified manifest](radar-label-content-och41-manifest.json).
The final focused OCaml tests/format check, Rust formatting and whitespace checks
also pass.

## Remaining qualification

Required current-source Linux/hosted checks are pending; the
older hosted run 37470525492 does not cover this schema or adapter.

[Subsequent pointer-capture evidence](radar-label-capture-och41.md) covers held
gestures across source and visibility transitions. Other custom-label acceptance
must cover icon/resource children, narrow and
oversized clipping, theme/font/scale transitions with interactive labels, source
family/replacement/release transitions, delayed stale actions across hidden/reset
and return, pointer capture and popup retirement without paint, and interaction
isolation across two charts/windows. Unit hidden-set isolation and the older
streaming harness are not substitutes for those cases. IME and VoiceOver are not
qualified by this walkthrough. All broader catalog and release gates remain open.
