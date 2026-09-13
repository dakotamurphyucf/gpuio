# OCH-11 local command-palette evidence

Platform: macOS arm64, stock OCaml 5.3.0, Bonsai/Core v0.17, Eio,
Dune 3.24.2, Rust 1.97.1. GPUI remains pinned to
`a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`. No upstream/vendor patch was added
for this adapter. This is a local component checkpoint, not completion of OCH-11.

## Reproduction

Run sequentially from the repository root; Dune also invokes Cargo:

```sh
./scripts/gpuio exec dune build @runtest @all @fmt
./scripts/gpuio exec dune exec examples/palette/main.exe -- --self-test
./scripts/gpuio exec cargo test --workspace --locked
./scripts/gpuio exec cargo clippy --workspace --all-targets --features native-tests --locked -- -D warnings
./scripts/gpuio exec cargo fmt --all -- --check
./scripts/gpuio exec cargo test -p gpuio-native --features native-tests --test native_palette --locked
./scripts/gpuio exec cargo test -p gpuio-native --features native-tests --test native_controls --locked
```

The native commands open actual foreground windows. `native_palette` runs the
palette scenario alone; `native_controls` includes it after the preceding controls,
focus, overlay, tooltip, command and menu scenarios. Both passed locally. The
public Bonsai/Eio smoke separately passes mounting, registry changes, render
acknowledgements, unmount and application shutdown.

## Native evidence

- Private native query focus, disabled-result skipping, Escape restoration and
  rejection of repeat activation after native close but before OCaml removal.
- Native clipboard insertion filters the query. Copy acts on the captured document
  selection, preserves that selection, and restores document focus.
- Callback invocation is enqueued before Selected dismissal. Paste followed by
  Enter without an intervening frame selects against the current query.
- macOS editable-combo accessibility, disabled results, AX Focus/SetValue and real
  AX result activation. Marked/committed text enters through NSTextInputClient;
  Enter during composition does not invoke a command.
- A 1000-command palette paints fewer than 32 row anchors. Real wheel input changes
  the scroll offset; PageDown reveals item 992, and reordering the same selected ID
  reveals its new position. Oversized query insertion is rejected atomically;
  query history stays within its configured budget.
- Outside-dismissal permission, PointerEvents=false, hidden palette cleanup, and
  a palette above an existing modal preserve the document editing/focus contract.
- Disabling the highlighted command before paint blocks activation. Re-enabling
  it with a new generation dispatches that current generation. Removing the palette
  releases its query entity; unmounting an open palette restores focus without
  reporting a synthetic user dismissal.

## Validation and findings

OCaml expect tests cover metadata bounds, unique references, missing lexical
registries (including a physically shared child), callback-only refresh, stale
unmount dismissal, and independent Rust request/event fixtures. Rust session tests
cover atomic invalid metadata rollback, dismissal permissions, command-source
membership, stale unmount and retained-budget release. Rust workspace tests,
Clippy, full OCaml build/tests/format, and Rust formatting checks pass locally.

The hidden-palette test found a real lifecycle ordering defect: deferring native
close until after hidden paint discarded the query's focus ancestry before scope
restoration could use it. Visibility-driven dismissal now runs during tree/scope
synchronization, before painting. Native hidden and nested-modal checks pass with
that change. Existing closed palettes stay in the hidden set during synchronization
so unrelated updates do not manufacture focus-state changes.

An initial combined run failed the existing tooltip interactive-hover delayed-close
assertion, before the palette scenario. A similar intermittent failure was recorded
at the menu checkpoint. Added failure diagnostics capture requested pointer position,
actual pointer position, tooltip bounds and native hover/timer state. The subsequent
complete combined run passed. No proven tooltip diagnosis or product fix is claimed;
this remains an integration concern for final OCH-11 validation.

## Limits and remaining work

This is actual local AppKit/GPUI programmatic input coverage, not a physical IME
candidate-panel or complete screen-reader audit. The 1000-command check establishes
visible-row virtualization, not an end-to-end performance benchmark. Matching is
ordered Unicode-lowercase substring search, without fuzzy ranking or normalization.
Shared command routes retain native snapshots; aggregate route/cache lifetime and
memory stress remain part of OCH-11's final bounded-lifetime acceptance.

Required Linux build/unit checks and consolidated hosted CI remain pending until
all remaining OCH-11 implementation is complete, per the owner workflow. Full Linux
GUI acceptance belongs to OCH-17. No hosted or Linux GUI acceptance is claimed here.
OCH-11 still includes feedback controls, pointer/desktop interactions, assets,
remaining styling/transition integration and combined scrolling/lifetime checks;
OCH-12 animations follows. See the [palette contract](../design/native-controls.md#command-palette).
