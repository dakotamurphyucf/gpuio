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
