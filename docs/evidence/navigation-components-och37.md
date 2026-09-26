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

## Initial mounted disclosure and hidden-content behavior — 2026-09-26

Core and Bonsai now expose `Content_policy`, `View.panel`, `View.disclosure` and
`View.accordion`. Three additional expect tests verify independent appended-kind
bytes, retained versus unmounted child identities, latest callback delivery,
separate ordered toggle requests and skipped collapsed-content construction under
Unmount. Three Rust integration tests verify strict decoding/truncation/trailing
bytes, atomic rejection of malformed child relationships/labels, repeated
visibility updates and complete tree accounting cleanup.

The independent `disclosure-request.hex` fixture was manually assembled from
bin_prot tags and small integer/string lengths using Python bytes, without either
production codec. It contains an Accordion/Disclosure/Button/Panel tree with kinds
44/43/2/42, a Button(false) control, splices and root publication.

Actual macOS `native_navigation` tests pass:

- Up/Down/Home/End header navigation, disabled-header skipping and wrap, modified
  key exclusion, nested-group independence and exactly one Enter/AX Press event.
- Outer collapse while a nested editor is focused restores the outer trigger;
  hidden children cannot regain focus or take the next Tab. Reopening preserves
  the native editor/focus handle and text. Removing the inner editor restores its
  own trigger and drops the old native entity (verified by a weak liveness probe).
- Real NSView marked-text input enters the nested editor. Once hidden, a subsequent
  native text insertion cannot change it. This is native input-delegate evidence,
  not an IME candidate-window claim.
- A retained focus scope disappears from the active modal stack when its ancestor
  becomes style-hidden. Removal disposes editors/buttons and leaves no pending
  focus repair.
- AppKit expanded state is true for an open header and false after collapse. The
  pinned adapter initially reported false for both, exposing its missing expanded
  getter. The repository-local 0.26.3 patch fixes that behavior. No VoiceOver speech
  or external AX notification-observer acceptance is claimed here.

The existing native controls suite (including focus, overlays, tooltip, commands,
menus and pointer behavior) passes with the patched adapter. Native tabs/retained
panels also pass. Full protocol tests, feature-enabled all-target Clippy with
warnings denied, Core/Bonsai compilation, view API expect tests and formatting
pass. The default OCaml backend and separate static extension-consumer backend
both link with the local adapter; their dependency versions remain unchanged.
A source reconstruction check reapplies the recorded patch and matches every
vendored Rust source exactly; generated backend metadata resolves/tracks the same
local source. Required CI now builds `native_navigation` on both platforms and
runs its macOS functional test. Those hosted jobs have not run for this checkpoint.

Commands (isolated repository environment, local macOS arm64):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-tests --test disclosure --test native_navigation --test native_tabs
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-native --features native-tests --test native_controls
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j 2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j 2 -p gpuio-native --all-targets --features native-image-tests -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/extension_consumer/main.exe examples/color_input/picker.exe @test/view_api/runtest @fmt
```

Final post-review checks and commit are recorded in Linear. No new navigation
capability is advertised. Public component examples, broader native appearance,
motion/lifecycle/workloads and the remaining OCH-37 families are still required.
