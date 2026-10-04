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
