# Milestone 6 implementation plan

Started 2026-09-27 from milestone 5 merge `936fb7d`. Milestone 5's required
macOS/Linux checks passed on `473407c`; all 12 tickets are Done. Its implementation
and evidence remain in [the milestone 5 handoff](milestone-5.md).

## Scope and order

| Ticket | Deliverable | Current state |
| -- | -- | -- |
| OCH-27 | Application identity, deep-link packaging/startup/live delivery, activation, file reveal/open, document-window metadata | In progress: macOS OS acceptance passes; Linux portal file adapters pass local peer/worker tests; typed packaging and real bus arbitration pass locally; Linux GUI and consolidated hosted gates pending |
| OCH-28 | OS notifications, permissions/capabilities, replacement/dismissal and stale-safe action routing | In progress: Core/Eio ownership, bounded paired protocol and owned macOS adapter; public packaged macOS OS actions pass; Linux worker/private-bus routing pass locally; Linux build/display and consolidated gates pending |
| OCH-40 | Line, area, bar, pie, radar, candlestick and Sankey components with typed bounded datasets, native interaction and accessible alternatives | In progress: all-family Core data model, bounded paired codecs and expect tests; pinned Sankey algorithm compiles/tests against existing GPUI; resources, rendering and native acceptance pending |
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

The macOS services test (`python3 scripts/test_desktop_links_macos.py --services`)
checks OS-selected file delivery through a disposable native consumer, Finder
fixture visibility, actual default URL-scheme routing, and missing-file/registration
errors. Native completion tests cover bounded admission, cross-thread/reentrant
callbacks, stale tokens and shutdown races. Public APIs report submission and
native errors precisely; they do not equate success with visible presentation.

Linux file open/reveal now use XDG portal methods with owned descriptors and a
bounded worker/cleanup layer. Local private D-Bus tests pass actual descriptor
transfer, options, portal responses/version limits and cancellation. The shared
file-chooser request engine keeps its prior lifecycle regression coverage. This
is protocol/worker evidence on macOS, not Linux graphical acceptance.

Linux launch arbitration/forwarding now uses `App.run_desktop` before UI creation.
Private D-Bus peer tests cover ownership outcomes, unique-owner routing, bounded
atomic batches, typed errors, cancellation and bus loss. Native/codec tests and
the packaged macOS regression pass with explicit startup input and rejected-launch
handle cleanup. Real empty macOS reopen and public activation from behind Finder
also pass. This is not yet real Linux bus/desktop invocation evidence.

Typed packaging helpers now emit matching Info.plist and desktop entries. The
macOS invocation test consumes generated metadata. A real private D-Bus test
passes concurrent ownership/forwarding and release/reclaim locally; Linux CI now
requires it without a display. Generated Linux metadata/OS invocation has a new
early X11/Wayland smoke scenario, still awaiting a Linux run. Desktop bit41 is
advertised (aggregate4398046511103); backend-specific Unsupported results remain
explicit. OCH-27 stays In Progress pending Linux/hosted acceptance. Next: OCH-28
notifications, remaining M6 families, then consolidated hosted validation. No M6 ticket is complete yet.


OCH-28 now has typed Core/Eio notification APIs and a bounded owned macOS adapter.
Its packaged public example passes real named/default actions, readiness, same-
lifetime replacement, native/explicit dismissal, closed-window target fencing and
service cleanup on macOS 14.5 arm64. Linux now has application wiring and a
bounded worker passing local private-bus tests for actions, replacement, owner
loss and cleanup. Actual Linux build/display validation remains pending; the
notification capability bit stays unadvertised. See the [notification contract](design/os-notifications.md)
and [native evidence](evidence/os-notifications-och28.md). This is local macOS
acceptance, not completion of the cross-platform ticket or the milestone.


OCH-40 now has validated Core data for all seven chart families and mixed
Cartesian layers. Nine expect tests cover numeric/text/identity limits, 100,000
points, gaps/OHLC/radar invariants and exhaustive four-node graph classification.
The isolated upstream Sankey algorithm passes its 12 retained tests against the
unchanged GPUI pin. Native resources/rendering/interactions and public graphical
acceptance are still pending; see the [chart design](design/charts.md).

OCH-40's standalone chart payload now has bounded, versioned OCaml/Rust readers
and independent all-family byte fixtures. Aggregate point/text limits are checked
before allocation; decoded data must pass domain validation. Native resource
publication and rendered chart acceptance remain the next implementation work.
