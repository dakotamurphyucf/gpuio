# Describe the qualification protocol without owning a measurement loop

[gpuio_performance_probe.ml](gpuio_performance_probe.ml) and [its interface](gpuio_performance_probe.mli)
define a typed OCaml extension protocol. This module contains no Bonsai graph,
Eio producer, native timer or window ownership. `X = Gpuio.Extension` supplies
validated schemas, bounded codecs and instance descriptions. The
[native component](../rust/src/lib.md) owns collection; a workload such as
[streaming](../../performance_streaming/main.md) owns scheduling and assertions.

## Types and byte contracts

`Command.t` offers `Begin`, `Begin_idle`, `Finish`, `Document_preparation` and
`Buckets { metric; offset }`. `Event.t` distinguishes readiness (`Begun`), a
finished CPU interval, histogram pages, idle observations and document-worker
counters. `[@@deriving sexp]` generates event serialization functions; it is not
reactive syntax. `let%bind.Or_error` in `instance` propagates constructor errors,
rather than reading a Bonsai value.

The private schema has name `qualification.performance`, version `3` and an exact
fingerprint shared with Rust. Unit properties encode as one zero byte. Commands
have a 12-byte maximum: single-byte tags for phase commands, or tag `2`, metric
byte and canonical decimal offset. Metrics must be 0–4 and offsets 0–4,294,967,295.
`decode_command` re-encodes parsed bucket requests, rejecting leading zeros and
noncanonical spellings. `events` permits up to 16,384 bytes and parses the typed
S-expression; it does not itself verify workload counts or timing budgets.

`instance ~sequence command` constructs an `X.Command` and a generation-`1`
instance labelled “Qualification measurements.” Increment sequence for each new
request; repeated rendering of the same command does not start a new measurement.
This instance is a description consumed by `View.extension`, not an installed
factory or a resource registration. Its backend must already include the probe.

## Readiness, units and caller responsibilities

A caller renders the instance, returns effects from its extension event handler,
and awaits typed `Data` events. For example, the streaming graph reads a command
Var through `let%arr`; its Eio worker publishes a new sequence on the UI context
and receives queued probe events. `Command_completed` means command handling,
not delayed measurement readiness. Wait for `Begun` before workload activity,
then `Finished` before requesting all histogram pages. Validate each page's
metric, offset, total and accumulated count independently.

CPU interval durations and capture costs are nanoseconds; metric `4` buckets
count inputs per frame rather than duration. Document preparation stage fields
are cumulative application-wide elapsed worker microseconds, including discarded
work, not CPU time. Compare phase boundary counters; retain worker/reserved-byte
peaks as absolute maxima. Idle observations preserve activation and optional
visibility: `None` means unavailable, never assumed visible. Presentation-enabled
builds additionally export a separate JSON record; this OCaml protocol remains v3.

Trace: publish `Begin` → command acknowledgement → delayed `Begun` → run workload →
publish `Finish` → `Finished` → request metric pages → independently validate
content, histogram totals and cleanup. A timing event does not establish source
correctness, real keyboard input or physical pixel presentation.

## Pure tests, running and adaptation

From the root after [setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest examples/performance_probe/ocaml -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/performance_streaming/main.exe -j 2
python3 scripts/measure_streaming_typing.py --build-profile release --smoke --output scratch/probe-streaming-smoke
```

The three inline expect tests check event shapes and canonical bounded commands,
document counter units/peaks, and unknown idle visibility. They decode fixtures;
they run no native workload. The final command requires the macOS collector and
fresh output directory. No commands were executed for this guide; see
[package README](../README.md) for ownership and qualification limits.

When changing the protocol, coordinate schema/fingerprint, OCaml codecs, Rust
validation, fixtures and collectors. Do not interpret an application-wide worker
counter as a per-window latency, or promote this diagnostic package into an
ordinary application service without a separate supported contract.
