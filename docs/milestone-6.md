# Milestone 6 implementation plan

Started 2026-09-27 from milestone 5 merge `936fb7d`. Milestone 5's required
macOS/Linux checks passed on `473407c`; all 12 tickets are Done. Its implementation
and evidence remain in [the milestone 5 handoff](milestone-5.md).

## Scope and order

| Ticket | Deliverable | Current state |
| -- | -- | -- |
| OCH-27 | Application identity, deep-link packaging/startup/live delivery, activation, file reveal/open, document-window metadata | In progress: public routing, packaged macOS links and document metadata pass native/public checks; registration, file services and Linux forwarding pending |
| OCH-28 | OS notifications, permissions/capabilities, replacement/dismissal and stale-safe action routing | Pending OCH-27 application identity/routing |
| OCH-40 | Line, area, bar, pie, radar, candlestick and Sankey components with typed bounded datasets, native interaction and accessible alternatives | Pending native adapter evaluation; existing pins retained |
| OCH-29 | Focused graphics application consuming public canvas and an independently packaged native component; desktop workflows and one chart | Pending integration of preceding capabilities |

The goal includes all four tickets, local functional acceptance, required hosted
macOS/Linux gates, merge, and final versioned/Linear handoff. Starting a ticket or
passing pure tests is not completion. Optional platform packages and milestone 7
release validation are separate; do not silently add them to this scope.

## Implementation rules

Read [desktop services](design/desktop-services.md) for pinned-backend findings and
the first contracts. Follow the existing [engineering standards](design/engineering-standards.md),
[expanded scope](design/expanded-v1.md) and live Linear acceptance criteria.
Rust owns native calls/layout/paint/input; Core/Bonsai own application routing and
data; Eio owns application I/O and scoped work. Typed unsupported/unavailable
results must reflect backend behavior, not merely the presence of a GPUI method.

Use the repository's isolated toolchains. Local builders use `GPUIO_JOBS=2` and
GUI checks run sequentially with child/window cleanup. Prefer background checks
when possible; local foreground OS invocation/focus/input tests are authorized.
Complete feature work locally before consolidated hosted checks. Linux build/unit
checks remain required; record real X11/Wayland evidence separately and retain
full Linux graphical release validation under OCH-17.

## Current evidence

`Deep_link` expect tests pass valid/malformed/unsupported schemes, encoded Unicode,
exact byte preservation, empty components and size boundaries. `Desktop_inbox`
tests pass pre-readiness ordering, duplicates, entry/byte backpressure, permanent
close and 10,000 fill/drain cycles. These are pure semantic checks, not OS delivery.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 \
  @test/desktop/runtest @test/view_api/runtest @fmt
```

Identity, native input admission, correlated desktop requests and availability
signals now have paired OCaml/Rust fixtures and native state/mailbox unit tests.
These cover pre-configuration input, preserved malformed values for app rejection,
overflow reporting, shutdown, independent input-lane saturation and bounded
response batches. `App.run ~desktop` queues identity before application windows.
The Eio bridge compiles with the targeted expect/format checks passing.

The public application-scoped receiver now passes deterministic readiness,
coalescing, FIFO/rejection/overflow, effect backpressure, close/late-response and
explicit retry tests. Real macOS Launch Services cold/warm invocation passes in a
disposable application bundle. Accessibility observes the native window closing
and reopening, and document text changing without another process or duplicate
delivery. Receiver replacement preserves the new subscription. The test briefly
activates its own process for AppKit accessibility and cleans up on every path.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/desktop/main.exe \
  @test/desktop/runtest @test/runtime/runtest @test/lifecycle/runtest \
  @test/view_api/runtest @test/protocol/runtest @fmt
python3 scripts/test_desktop_links_macos.py
```

Document metadata now passes the native macOS window suite (represented non-UTF8
file bytes, independent edited state and clearing, existing close/quit/reopen
decisions) and the packaged OCaml example (setting/clearing and stale-window
rejection). Linux explicitly returns `Unsupported` for document metadata.
Independent OCaml/Rust command and response fixtures cover the schema extension;
mailbox checks include large represented paths in the response byte budget.

No full desktop capability bit is advertised yet. Next: remaining OS services,
Linux launch/instance forwarding and packaging, then
consolidated hosted validation. No M6 ticket is complete yet.
