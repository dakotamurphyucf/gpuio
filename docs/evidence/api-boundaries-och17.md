# Public API boundaries — OCH-17

Local source review on macOS arm64, 2026-10-04. This checkpoint covers library
visibility, the application entry points and scoped asynchronous work. It is not
a whole-surface behavior review or release approval.

## Findings and changes

- Reviewed all seven library stanzas. Core has 142 handwritten module interfaces,
  including private `List_identity`; Eio has 40, including seven private
  implementation modules. Its seven interface-less test modules are also private.
  Bonsai has ten interfaces and an explicit public facade; runtime-core has five.
  Protocol has 91 implementation files but only 14 handwritten interfaces. The
  default native implementation satisfies the virtual backend interface.
- Added the [API layer map](../api-layers.md). It distinguishes ordinary
  application entry points from reachable reconciliation, queue, registry and
  wire interfaces. No module was hidden, renamed or removed. A reachable
  registry type alias remains usable; application signatures should prefer the
  corresponding application module.
- Checked `Scope.start`, cancellation, task accounting, stream admission and
  retirement against their implementations and existing expect tests. Clarified
  scope/stream quotas, producer versus callback exceptions, and suppression of
  queued delivery. Stream close discards pending values; it does not flush them.
  These changes alter documentation only.
- Replaced stale Bonsai measured-carousel documentation with a reference to the
  detailed Core contract. Updated the carousel ledger to include existing
  [wheel](carousel-track-wheel-och41.md), grouped-control, clipping and gallery
  evidence. Full physical focus/accessibility qualification remains open.
- Corrected the compatibility guide's blanket claim that every public interface
  has a `.mli`: protocol modules commonly expose inferred signatures, and Dune
  wrappers/private-module declarations also determine visibility.

## Validation

Commands use the isolated repository environment and two jobs:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @fmt lib/bonsai/gpuio_bonsai.cmxa
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @fmt @test/runtime/runtest
python3 scripts/audit_component_catalog.py
git diff --check
```

Both build commands, runtime expect tests, catalog audit and local
documentation-link check passed. The first combined runtime/format command
reported a missing blank line in the edited scope interface; it passed after
that formatting correction. No expectations were promoted. No physical window, IME,
clipboard, accessibility or Linux qualification is supplied by this review.
Source signatures and runtime behavior are unchanged; there is no new wire epoch
or compatibility promise. The remaining whole-surface API review and release
gates stay open in [status](../status.md).

## Resource publication and recovery review — 2026-10-08

Reviewed the public Eio document/chart/canvas interfaces against their registries,
the pure source/dataset/scene contracts, and the asset registry's admission rules.
The [compatibility guide](../api-compatibility.md#resource-publication-and-recovery)
now distinguishes local admission, native publication and physical presentation,
documents resource budgets, and explains recovery without implying a process
memory bound. The desired-value getters are explicitly not accepted-state queries.

The old document interface promised that the latest terminal state was always
delivered. Native failure or scope cancellation can retire it first. Any document
upload failure clears its retained content and requires a new registration; an
application needing recovery must preserve its own content. The old chart/canvas
wording also omitted fatal failure cases: only recoverable update rejections
preserve the prior accepted snapshot for explicit retry. Closed/stale/native
failure responses and failed aborts retire those registrations.

Added deterministic expect tests in `test/runtime/document_registry_test.ml`,
`chart_registry_test.ml` and `canvas_registry_test.ml`. They exercise a failed
document upload with a queued terminal snapshot, each fatal chart/canvas response,
and a recoverable chunk rejection followed by a failed abort. They verify retired
state, rejected reset, cleanup-only requests and empty registry accounting.
The document case also checks retained error, cleared source and no second create
callback. Existing tests cover recoverable rejection and successful explicit retry.

Validation on macOS arm64, isolated repository toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/runtime/runtest @fmt
git diff --check
```

The runtime expect suite and formatting passed without promoting expectations.
These are simulated native responses in the OCaml registry tests, not OS/GPU
failure injection or physical-window validation. Production implementations and
source signatures are unchanged; this review corrects documentation and adds
coverage of existing behavior. Whole-surface API and release qualification remain
open.

## Paging ownership repair — 2026-10-08

Review of `List_paging`, `Table_paging`, `Tree_loading` and `List_search` against
their public interfaces found two concrete list-pager defects at `14775883`:

- Cancellation cleared the pending task immediately. While its protected cleanup
  was still running, new requests could start. A deterministic test holds both
  original cleanups, performs 100 resets with both boundaries requested, and
  observes **202 producers started, peak concurrency 3**. The interface promises
  at most two; only two original producers should start until cleanup releases.
- `close` cancelled requests in the pure controller but did not publish its final
  snapshot to the Bonsai variable. Direct queries reported Ready/Ready while the
  reactive value still reported Loading/Loading.

The list adapter now uses the existing table adapter's bounded-worker approach:
two lazy workers, one latest queued request per boundary, and reuse only after
producer cleanup returns through the UI inbox. The pure controller's existing
owner/generation/request predicate is exposed under `Expert` for the adapter;
applications do not need it. No polling or native bridge changes are introduced.
Closing cancels owned workers, publishes the final reactive snapshot, suppresses
callbacks and leaves unrelated tasks alone.

The repaired 100-reset test starts only generations 0 and 100 in each direction,
keeps peak concurrency at two, and publishes only the latest rows. Other regression
checks cover agreement of direct/reactive closure snapshots, no closure callback,
idempotent close, quota rejection, explicit retry, idle worker task accounting and
release to zero tasks. Existing queued-result, error, generation, scoped shutdown,
managed-list layout-demand and append-during-history tests remain unchanged.

Idle workers retain up to two Scope task slots until closure; this cost is now
explicit in the public contract and example walkthroughs. Protected cleanup must
eventually finish before its slot can be reused. The comparison with table/tree
workers and search cancellation in the [compatibility guide](../api-compatibility.md#loading-and-cancellation-bounds)
does not imply a universal payload-memory bound.

Local validation uses macOS 14.5 (23F79), arm64, and the isolated repository
OCaml 5.3/Bonsai v0.17 environment:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/runtime/runtest @fmt examples/virtual_list/main.exe examples/agent_chat/main.exe
_build/default/examples/virtual_list/main.exe --self-test
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt
python3 scripts/audit_example_docs.py
python3 scripts/audit_component_catalog.py
git diff --check
```

The runtime suite, both example builds and the native managed-conversation
self-test pass. The native test reports paging, offscreen streaming, retained
key/pixel anchor, follow-tail, bounded active rows and normal shutdown. It is a
small integration fixture, not the 10,000-row performance qualification or proof
that the separately reported visual overlap is fixed. During development, an
unchanged existing test caught spurious callbacks from obsolete worker returns;
the implementation was corrected rather than promoting its changed expectation.
A new quota test also needed sequential bindings to avoid inspecting state before
the request due to OCaml tuple evaluation order.

The full incremental Dune test and formatting aliases pass, including pure
collection and managed-list/tree/table suites. Example/catalog structural audits
and whitespace checks also pass. No expectation was promoted. The
[fourteen-file archive](list-paging-ownership-och17/reports.tar.gz) retains the
before implementation, exact repaired sources, all behavior-test attempt logs and
native output; every member was verified against the
[manifest](list-paging-ownership-och17/manifest.json). Its
[summary](list-paging-ownership-och17/summary.json) records the native executable
hash and actual environment. Hosted/Linux validation of this repair is pending;
whole-surface API and release acceptance remain open.
