# Pointer focus preserves the click target — OCH-17

The local Results failure exposed a runtime defect in generic focus reveal.
GPUI transfers focus on mouse-down. GPUIO then revealed the focused compound
table's bounds through its parent scroller, moving the clicked cell before
mouse-up. The original [physical diagnostic](results-column-readiness-och17.md)
measured 49.5 logical pixels of movement and no selection. Retrying the click
would conceal this behavior.

The host now records pointer-origin focus after down/up dispatch when the eligible
focused target contains the pointer. Automatic ancestor reveal does not run for
that change. Explicit reveal requests remain pending, and non-pointer focus still
reveals. No timer, polling, synchronous OCaml callback or GPUI fork change is
introduced. Native table selection semantics remain unchanged.

The Results driver also explicitly reveals the inspector page's column headers
before pointer interactions. It sends a wheel event at the owned page gutter,
with an explicit screen location, preserving the table's independent scrolling
and retained column widths/order. Existing bounded header readiness and all
single-click assertions remain in force.

## Local evidence

Validation uses stock project toolchains on macOS 14.5, arm64 / Apple M1 Max.
The new production-host TestPlatform regression draws between mouse-down and
mouse-up. Before the repair, its cell moves from y=130 to y=49 and the regression
fails. Afterward it preserves geometry and scroll position through release,
selects the original cell, and proves explicit reveal plus non-pointer focus
still move the viewport when required. TestPlatform is not a physical GPU claim.

Passing checks:

- Full native unit suite with `native-image-tests,native-canvas-tests`:
  1,067 passed, two ignored.
- Strict native all-target Clippy with `native-tests,native-canvas-tests,native-image-tests`.
- Fresh `./scripts/gpuio build examples/agent_chat/main.exe` and complete
  `scripts/test_agent_chat_results.py` physical AppKit workflow: pointer/keys,
  resize/reorder/sort, Diagram → Back retention, context actions, exact Unicode
  clipboard, query cancellation/retry, 100k rows, themes, draft and cleanup.
  The 100k snapshot stays within budget at seven rows and 32 cells.
- `native_link`: actual-window GPUI dispatch and AX focus/reveal, including
  nested XY scrollers, explicit reveal, keyboard traversal, unchanged-focus
  wheel ownership, fixed clips, oversized targets, idle and disposal.
- `native_table_host`: targeted AppKit keys, marked-text commit, grapheme
  deletion, clipboard, command focus, bounded accessibility and resource release.
- `dune build @all @runtest @fmt -j2` through the isolated wrapper passes.
- A physical held-click diagnostic pauses 300 ms after mouse-down, then releases
  at the original point. The cell center stays `(817.5, 549.0)` before, during
  and after the press, and the original row is selected. The diagnostic closes
  and reaps its child; this supplements the separate full workflow above.

[Raw logs, commands, screenshots and qualified source patch](pointer-focus-reveal-och17/reports.tar.gz)
are retained with a [verified archive manifest](pointer-focus-reveal-och17/manifest.json)
and the rebuilt chat executable's hash. The examples documentation inventory
remains 427 sources / 265 reviewed groups / zero pending.

This is scoped local focus/Results qualification. Current committed-source hosted
and Linux confirmation, physical presentation, broader performance/distribution
and remaining catalog acceptance are separate requirements. No VoiceOver or
Linux GUI acceptance is inferred. Independent Icon transform work was present in
the local build but is not qualified by these checks.
