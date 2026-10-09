# Active presentation collector retirement — OCH-17

Status: implementation, guard tests, corrected macOS smoke and three full resource
repetitions pass on 2026-10-08. Existing
CPU/entity/Metal lifecycle passes did not activate presentation collection and
therefore did not establish retirement with that collector running.

## Implementation and boundaries

The separate resource-audit backend now offers `--presentation`. Its private
schema v3 adds one checked Boolean byte; paired OCaml/Rust codecs reject old,
malformed and trailing data. The native component starts a dedicated session at
mount, stops admission on close, and checks settlement after the existing one-second
delay. It snapshots values and drops the sole strong measurement owner before
checking entities/Metal and exporting its record. Neither a window, entity,
drawable nor extension event route is retained by that record.

The Python collector requires matching, unique session/window identities for every
cycle, actual supported drawable outcomes, closed/stopped state, zero pending
frames and consistent counts. It withholds memory sampling and continuation until
all requested application/entity/Metal/presentation records arrive, including the
final completion marker. Missing, duplicate, reused, reordered and unrequested
records fail. Disabled or unsupported collection cannot fabricate a pass.

The existing performance probe delays Begin; short lifecycle cycles can close
before it starts. This audit therefore starts its own immediate session and leaves
that probe's presentation Cargo feature off. Production backends, renderer source,
window pacing, memory limits and settlement duration are unchanged. The standalone
audit lock adds the optional native graph without replacing any existing package
identity; the composed backend changes only three dependency edges and no package
versions. See [the implementation walkthrough](../../examples/resource_audit/rust/src/presentation.md).

This is resource qualification. Settled zero-time, missing or invalid-clock
outcomes remain explicit and are not successful presentation/timing evidence.
The actual timing workloads retain their stricter acceptance rules. This audit
requires no saturation, duplicate callbacks or histogram overflow and cannot
pass with an open, accepting, pending or unused collector.

## Local checks

- Four feature-enabled Rust tests pass, including stopped-owner replacement
  rejection, actual TestPlatform window closure and rejection of unretired or
  inconsistent synthetic outcomes. TestPlatform is explicitly rejected by the
  native capture API; no simulated timestamp is reported as macOS evidence.
- Both feature-disabled Rust tests and strict feature-enabled all-target Clippy
  pass. The existing retained-entity guard still fails with a retained entity and
  passes after release.
- Fourteen Python lifecycle/record/handshake tests and the paired OCaml expect
  tests pass. Rust formatting, workflow actionlint, catalog source audit and the
  updated example inventory pass; structural audits do not prove native behavior.
- Optimized resource-audit and ordinary lifecycle executables build successfully
  using the repository toolchain. No unrelated switch or dependency default changes.

Exact commands and results are retained in the checkpoint archive, including
`validate-resource-presentation.py` and its per-step logs. CI now runs the
feature-enabled guard tests on both supported build platforms and adds collector
retirement to its existing macOS resource smoke. Those new hosted checks have not
yet been accepted.

## Corrected native smoke and retained launch mistake

The first smoke was launched prematurely while Dune was still linking. Its report
identifies the preexisting executable
`08c0b2972f2efb96ff5fd314209ccbde43d5ef2830430b08026a2520fd929c5b`, which emitted
no presentation-retirement record. The new collector correctly withheld
continuation; the child timed out with exit 2. The raw failure is preserved as a
test-launch mistake, not a failure or pass of the new implementation. Compilation
was concurrent, so it is not performance evidence either.

After the entire build pipeline exited zero, the new executable hash was verified:
`37a988883d90530d1002112579e330b522a7bd08e1596da94ed4d10f01bf3e95`.
The fresh smoke passed on M1 Max/macOS 14.5 arm64, with no concurrent local compiler
or second owned GUI. Its observation revision is `41eb7459` with the implementation
changes present; the executable source accompanies this evidence commit.

```sh
python3 scripts/measure_resource_lifecycle.py --build-profile release \
  --native-entities --presentation --metal-memory --physical-memory \
  --check-closed-surfaces --executable _build/default/examples/resource_audit/main.exe \
  --smoke --output scratch/agents/root-20261007-access-check/resource-presentation-smoke-002 \
  --timeout 180
```

All four windows have unique session/window IDs, stopped/closed state and zero
pending callbacks. Their presented/zero counts are 3/0, 3/0, 0/2 and 3/0; no other
outcomes are lost or invalid. The all-zero third cycle establishes callback
retirement, not successful frame presentation. Entity checks pass; closed Metal
allocation growth is zero and no closed checkpoint has nonzero IOSurface
accounting. The short measured RSS baseline grows 1,179,648 bytes. These four
cycles do not establish the thirty-cycle or repeated-run budgets.

The child and driver exit zero after 18.051 seconds, with `complete=true`. All
owned processes are reaped. [Checkpoint archive](presentation-retirement-och17/checkpoint.tar.gz)
retains both smoke attempts, native/build/Clippy logs, complete reports, physical
memory samples and binary/lock reviews. All 41 members were read back and verified
against the [manifest](presentation-retirement-och17/checkpoint-manifest.json).

## Three full resource repetitions

All three predeclared trials pass on clean source
`48a008cfa283a21f68e95a73a36c92ebf2715cc5`, using the same verified executable
above and the same physical Mac. Each trial includes three warmups and thirty
measured cycles. No compiler or second owned GUI ran concurrently. The batch
stopped on any failure; all three completed without replacement trials.

```sh
python3 scripts/measure_resource_lifecycle.py --build-profile release \
  --native-entities --presentation --metal-memory --physical-memory \
  --check-closed-surfaces --check-budgets \
  --executable _build/default/examples/resource_audit/main.exe --timeout 600 \
  --output scratch/agents/root-20261007-access-check/resource-presentation-full-001
```

The next trials use output suffixes `002` and `003`. The archived batch script
retains the exact order and commands.

| Trial | Wall seconds | Peak RSS bytes | Final ten-cycle RSS baseline growth bytes | Closed Metal growth bytes | Presented / zero outcomes |
| --- | ---: | ---: | ---: | ---: | ---: |
| 001 | 122.872 | 119,439,360 | 4,685,824 | 0 | 58 / 36 |
| 002 | 123.079 | 120,258,560 | 4,112,384 | 0 | 52 / 43 |
| 003 | 122.540 | 119,521,280 | 8,323,072 | 0 | 65 / 30 |

Each trial has 33 unique sessions and windows, all stopped, closed and settled
with zero pending callbacks. Counts reconcile exactly, with no missing,
not-submitted, invalid-clock, saturation, duplicate or histogram-overflow
outcomes. Application registrations, scopes, queues and native entities pass
their existing retirement checks. RSS and Metal growth satisfy the unchanged
64 MiB gates; no closed checkpoint reports nonzero IOSurface accounting.
Zero-time outcomes remain resource-only evidence, not successful presentation.

Settled physical footprint peaks are 107,367,296, 101,714,816 and 103,009,024 bytes.
Their final-window baseline growth is 17,645,568, 18,595,840 and 17,989,632 bytes.
No physical-footprint threshold was predefined: these are observations, not a
separate pass/fail claim or a complete census of GPU memory.

All drivers and children exit zero with `complete=true`; the owned keep-awake
process is reaped. The [full reports archive](presentation-retirement-och17/full-reports.tar.gz)
retains 607 files, including all 99 physical-memory checkpoints, original logs,
reports and the batch plan. Every member was read back and hash-verified against
its [manifest](presentation-retirement-och17/full-reports-manifest.json).
The [summary](presentation-retirement-och17/full-summary.json) records exact
binary/report hashes and aggregate counts.

This qualifies repeated active-collector resource retirement on the recorded
physical Mac. Collector overhead and other performance, native, accessibility,
catalog, hosted and distribution requirements remain open.
