# Hosted Metal presentation environment investigation — OCH-17

These isolated experiments investigate required hosted presentation failures.
They do not modify application rendering or waive the release checks. Jobs run on
`diagnostic/metal-presentation-och17`, without a PR. The main worktree and its
Foundation run were unaffected. A diagnostic collection job being green means
artifacts were collected; each probe's actual report determines its result.

## Observed results

| Hosted environment / experiment | Positive timestamps / 120 GPU-completed frames | Strict probe result |
| --- | ---: | --- |
| macos-15 arm64, unchanged probe | 0 | Fail |
| macos-15 arm64, owned-window capture every 10 frames | 0 | Fail |
| macos-26 arm64, unchanged probe | 0 | Fail |
| macos-26 arm64, owned-window capture every 10 frames | 0 | Fail |
| macos-15-intel, unchanged probe | 120 | Fail: frames 0 and 1 have equal presentation times |
| macos-15-intel, owned-window capture every 10 frames | 118 | Fail: zero timestamps at frames 1 and 61, plus ordering |
| macos-15-intel, one drawable in flight | 120 | Pass: every original clock/identity/completion check |

All devices identify as Apple Paravirtual. The ARM capture cases retained twelve
screenshots each; frame 110 was inspected for both OS versions and visibly shows
the probe content. That establishes captured content, not each frame's presentation
time. The capture experiment did not produce positive timestamps on these ARM
runners. This result is narrower than claiming all virtual GPUs lack presentation
timestamps: the Intel results directly contradict that claim.

The sequential Intel experiment waits for the preceding drawable's presentation
callback before sending another. It changes only probe submission pacing, retains
all 120 records and the same twelve-second deadline, and keeps every strict
validation check. It neither drops inconvenient samples nor substitutes GPU
completion/callback arrival for presentation. It qualifies that standalone API
sequence; it does not qualify GPUI animation, workloads or performance budgets.

## Exact experiments and provenance

- [ARM matrix run 37732292390](https://github.com/dakotamurphyucf/gpuio/actions/runs/37732292390),
  commit `76c18ce14f8c516a8f035e17013757bd08250c61`: unchanged and capture variants
  on macos-15 and macos-26. All four strict reports fail.
- [Intel run 37732631942](https://github.com/dakotamurphyucf/gpuio/actions/runs/37732631942),
  commit `dcc530521672a20f94f7ecf24b247f91f2da4277`: unchanged and capture variants.
  Both strict reports fail despite positive timestamps.
- [Sequential Intel run 37733107673](https://github.com/dakotamurphyucf/gpuio/actions/runs/37733107673),
  commit `9afcb61b5112f586007c0e2585c5d952ad75d0a5`: one-inflight variant passes.

Each job applies and checks the existing display setup. The helper compiles the
exact generated Swift source, invokes it once, records its exit code and preserves
stdout/stderr, executable identity, display and power metadata. The plain source
is the maintained `scripts/qualify_metal_presentation.swift`; capture adds only
owned-window screenshots. Exact workflow/helper files for each commit and generated
probe sources are in the archive, rather than relying on a later mutable branch.

[Summary](hosted-metal-diagnostic-och17/summary.json) includes each report's hash,
all reported failures and positive/zero/completion counts. The
[raw archive](hosted-metal-diagnostic-och17/reports.tar.gz) contains 90 files,
including every retained screenshot and probe report. Every member was read back
and verified against the [manifest](hosted-metal-diagnostic-och17/manifest.json).

The next discriminator is the unchanged two-window GPUI integration probe on
Intel, [run 37733380351](https://github.com/dakotamurphyucf/gpuio/actions/runs/37733380351),
commit `938eb3b00cc5c1e05415120c5b05ef25cd8a5e20`. At this checkpoint its pinned
bootstrap completed and the build/probe step is still running; no integration
pass is claimed. Existing required main-branch ARM presentation failures remain
failures until an evidence-backed qualification configuration is implemented and
validated. Local physical-Mac workload passes are separate evidence.
