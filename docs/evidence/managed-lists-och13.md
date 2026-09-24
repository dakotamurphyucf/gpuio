# Managed-list acceptance evidence (OCH-13)

2026-09-24, local macOS arm64, stock OCaml 5.3/Core+Bonsai v0.17, Dune 3.24.2,
Rust 1.97.1. Branch `och-13-managed-lists`; hosted checks/merge pending.

## Public application behavior

`examples/virtual_list/main.ml --self-test` uses the actual Bonsai/Eio runtime,
serialized bridge and native GPUI host. It starts with 200 rows, keeps a before
page request open, scrolls to key 200 at offset 17, streams into offscreen key
300, completes a 100-row prepend and verifies the unchanged key/pixel anchor.
The conversation scope and producer survive the viewport move. Jump to latest
resumes tail following. Active row computations stay at or below 32.

Marker: `MANAGED_LIST_PASS paging=true offscreen_stream=true anchor=true tail=true bounded_rows=true`.

## Native interaction and ownership

`cargo test -p gpuio-native --features native-tests --test native_list` uses
100,000 logical rows with sparse descriptions in the production host. Platform
frames verify initial placeholders and materialization, programmatic range
changes, exact anchor key 50001 / offset 37.5 through prepend/reorder, measured
height changes from 150 to 220 pixels and viewport resize. Wheel input pauses
following; an appended record leaves that anchor unchanged. Jump to latest
resumes following; dragging the scrollbar moves away again.

A native row editor receives marked text through macOS `NSTextInputClient`,
keeps composition/focus offscreen, and commits into the same entity. Native
admission vetoes stale virtualization eviction. Intentional logical deletion
succeeds and the editor's weak entity becomes dead. A held read-only selection
pins its row while scrolling; mouse-up clears dragging, and disposal releases
its weak selection state. The test closes its window on both success and error.

Marker: `GPUIO_NATIVE_LIST_OK`.

This tests actual native text-client calls, not a physical IME candidate-panel
session or full screen-reader navigation. Linux execution is separately gated;
macOS-specific composition assertions are explicitly conditional.

## Whole-history retention

The OCaml expect test visits and revisits 100,000 application records with 100
active rows and non-default 2-KiB row model payloads. Weak references retain no
more than the active payloads, then none after eviction. Immediate retained heap
growth is below 150,000 words above an already-loaded O(N) source/metadata
baseline. Application data survives. The production driver selects the explicit
release-after-flush action-history policy; dependent static/dynamic batches have
the same outcomes as the upstream recent-cache policy.

The native stress test visits and revisits all 100,000 logical rows using real
GPUI layout/paint APIs, independent of platform display-link cadence. Every row
actually renders with selectable text. At each step there are at most 256 native
row descriptions, mappings, focus handles and selection objects; old selection
weak references are dead. Native slots are recycled with increasing generations.
The order/index and GPUI measurement state intentionally retain O(100,000)
metadata, separately from active view state. Tree admission charges 192 units per
logical row (19,200,000 for this history), which is a quota, not measured RSS.

Weak text probes track only still-live payloads, so the probe bookkeeping itself
cannot retain every visited allocation. Cached retired payloads remain below a
fixed 512 bound; the observed traversal peak was 101. After unmount, 261 remained
in native rendering state; closing the window released every tracked payload.
This distinguishes bounded native frame caches from immediate row-object cleanup.
It does not establish an RSS or physical frame-latency bound for arbitrary content.

Markers: `LIST_HISTORY pass=2 visits=200000 active_cap=256 metadata_rows=100000`
and `GPUIO_NATIVE_LIST_HISTORY_OK`.

## Protocol and scheduling

Independent OCaml/Rust fixtures fix list operation/event tags and field order.
A 100,000-row consecutive order uses eight bin_prot bytes. A fully fragmented
100,000-row permutation fits one bounded message and decodes correctly. Counts,
duplicate IDs, overflow, active budgets and aggregate tree quotas are validated.
No staged transport is promised above those bounds.

A 100,000-source-row production-driver test verifies value-only streaming sends
only changed text/row invalidations; offscreen changes coalesced during native
acceptance are not lost. Point changes invoke fewer than 32 row-key mappings.
Lifecycle snapshots preserve the accepted result across a second window's shared
Incremental stabilization. Retention replies do not run rejected deactivations.

Paging tests cover concurrent directions, empty cursor advancement, fresh-layout
barriers after nonempty pages, failures/retry/end, reset/cancellation, queued
obsolete results and old-generation controls. See `test/runtime`,
`test/virtual_list` and `test/protocol/list_test.ml`.

## Reproduction and remaining gates

```sh
./scripts/gpuio exec dune build @all @runtest @fmt -j 2
./scripts/gpuio exec cargo test --workspace --lib --tests -j 2
./scripts/gpuio exec cargo clippy -p gpuio-native --all-targets --features native-tests -j 2 -- -D warnings
./scripts/gpuio exec cargo test -p gpuio-native --features native-tests --test native_list -j 2
./scripts/gpuio exec dune exec examples/virtual_list/main.exe -- --self-test
```

CI requires macOS functional checks and Linux builds/unit tests. The Linux GUI
runs are informational under OCH-17; compilation alone does not count as GUI
acceptance. Final consolidated checks, hosted run and merged revision will be
recorded here before closing OCH-13.
