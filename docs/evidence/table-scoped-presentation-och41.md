# Checked managed-table header and row presentation — OCH-41

Local implementation at the milestone-07 worktree; physical macOS and release
acceptance remain open. No Linux GUI or physical GPU/input claim follows from
these tests.

`Table_presentation.Header`/`Row` provide checked paint/typography over native
table geometry. Core's managed table and Bonsai component/paged constructors
submit them separately from rich header/cell Views. Row computations use active
managed lifetimes; Rust resolves state locally. The Collections Result table adds
**Header and row styling**, with data-dependent emphasis, header typography and
native interaction refinements. Toggle off clears metadata and restores defaults.

Op114/115 use paired OCaml/Rust fixtures and explicit empty-style reset. Decode
checks every layer and aggregate declaration limits. Admission validates values,
root/body ownership and retained-byte budgets atomically. Resolved theme changes
emit only paint operations, preserving schema/scroll/cell owners.

A production Host test first exposed a duplicate hover registration: GPUI accepts
one handler, and the table had already installed its own. The scoped adapter now
passes native hover into the final-row hook; the Host composes custom hover before
one registration. Default delegates retain native feedback. Geometry and native
selection/context outlines stay outside the application's style scope.

## Validation checkpoint

- Targeted Core view/Bonsai virtual-list expect suites and public-gallery link
  pass (`dune build -j 2 @test/view_api/runtest @test/virtual_list/runtest
  examples/gallery/main.exe`). Row style computation mounts once per active row,
  survives column changes and releases on eviction/source replacement.
- Full native library suite passes (**815**, two existing private-D-Bus skips
  on macOS), including the production TestPlatform presentation test: inherited
  row/header typography,
  row selection/focus/hover/pressed/disabled precedence, unchanged bounds, clear,
  retained native owner and zero-byte teardown. Header hover and the distinction
  between cell and whole-row selection also pass.
- Full protocol suite passes (**371**), including paired Op114/115 fixtures, every
  truncation, trailing bytes, forbidden states/fields and cross-layer quota bounds.
- Table admission tests pass (**13**), including scoped ownership, byte-budget
  rollback, malformed values/state/geometry and reset to the original retained cost.
- Full OCaml `@all @runtest @fmt` passes after repairing stale exhaustive event
  matches in the bridge and View API examples. New milestone events are explicitly
  ignored by those older demos, preserving their existing behavior. A trailing
  Dune formatting mismatch was fixed manually; no expect output was promoted.
- Strict all-target lint passes for protocol, scoped table adapter and native
  crates. A fresh independent installed-gallery build also passes, including the
  public header/row styling control outside the repository checkout.

All commands use the repository's isolated environment and two build jobs. No OS
windows were opened for this checkpoint. Real mouse/keyboard, VoiceOver, GPU
color/alpha/clip and physical resource/performance coverage remains required under
OCH-17, alongside the wider catalog and release gates.

Commands run on macOS arm64, 2026-10-03, uncommitted worktree based on `83eb87e`:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-native --test tables
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 -p gpuio-protocol -p gpuio-table-adapter -p gpuio-native --features native-image-tests,native-canvas-tests --all-targets -- -D warnings
```

All four pass. Also passed:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
python3 scripts/audit_component_catalog.py
```

The existing `block 0.1.6` future-compatibility notice remains; it is
not a new failure. The independent consumer passes:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-table-presentation-gallery-20261003
```

Result: `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`. The path
is disposable evidence, not a build dependency. It stages installed public
libraries without modifying an opam switch; it does not run a physical window.
Final `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` and `git diff --check` pass.
No hosted CI, Linux execution, commit/merge or Linear completion is claimed by
this checkpoint.
