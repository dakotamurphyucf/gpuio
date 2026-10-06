# Qualification-only performance probe

This package supplies a typed OCaml extension description and a native Rust
measurement component. Applications/workloads own their windows, producers and
collector protocols; the mounted probe owns native snapshots, bounded observations
and cancellable measurement tasks. It is not an ordinary application service.

Read the source walkthroughs:

- [OCaml commands, events, codecs and inline tests](ocaml/gpuio_performance_probe.md).
- [Native factory, measurement phases, paging and cancellation](rust/src/lib.md).
- [Optional Metal presentation session and JSON evidence](rust/src/presentation.md).

The [streaming workload guide](../performance_streaming/main.md) demonstrates
Bonsai command Vars and Eio event coordination. The
[presentation workloads](../performance_presented/README.md) statically enable the
optional feature and provide paired executable/collector commands. CPU/submission
measurements, OS presentation observations and hardware/photon latency have distinct
contracts. Pure tests and build success do not establish graphical acceptance;
smoke does not pass full optimized budgets. See the
[declared qualification plan](../../docs/design/metal-presentation-qualification.md)
for remaining acceptance work and platform limits.
