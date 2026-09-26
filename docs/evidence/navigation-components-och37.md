# OCH-37 implementation evidence

## Pure application models — 2026-09-25

`Navigation_stack`, `Disclosure` and `Pagination` now compile in the public Core
library. Interfaces were drafted before their implementations. No dependency,
compiler, native protocol or capability advertisement changed in this checkpoint.
These results prove pure policy behavior, not mounted UI or platform acceptance.
The full remaining ticket contract is in [the design](../design/navigation-components.md).

Ten new expect tests pass in the existing `test/view_api` suite:

- Navigation replacement preserves forward history; a new push discards it;
  pop protects the root; stale breadcrumb destinations and duplicate instance
  IDs fail explicitly. Same-ID replacement and explicit payload updates work.
  A complete 128-entry traversal preserves physical identity of caller-owned
  payloads; overflow is rejected and a branch prunes forward entries. Clearing
  does not execute a task-cancellation operation.
- Disclosure handles queued toggles, single/multiple conversion, required-single
  collapse, disabled historical selections, collection replacement/reordering,
  empty collections, invalid IDs and independent nested application models.
  A maximum 4,096-item selection contracts without retaining removed expansion.
- Pagination reduces ordered relative requests against the latest state, ignores
  stale absolute requests after shrink, handles zero pages and growth, and
  separates disabled user actions from explicit programmatic updates. Exhaustive
  totals 1..64 at every current page and all five neighborhood sizes, plus six
  positions in a billion-page domain at those sizes, yield 10,430 passing
  partitions. Every partition covers the exact domain, includes endpoints and
  current page, uses only multi-page gaps, and emits at most 13 items. Invalid
  counts, selections and neighborhood sizes are rejected.

Exact commands, local macOS arm64, isolated repository environment:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 lib/core/gpuio.cma
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @test/view_api/runtest @fmt
```

Both exited 0. No graphical windows or long-running processes were started for
this checkpoint. The existing view API expect tests also pass. Native views,
accessibility/focus/IME, hidden mount policy, timers, overlays, public Bonsai/Eio
examples, paired fixtures for subsequent bridge additions, required hosted
macOS/Linux checks and milestone merge are still pending. Linux GUI acceptance
remains deferred to OCH-17; OCH-46 remains the chat showcase follow-up.
