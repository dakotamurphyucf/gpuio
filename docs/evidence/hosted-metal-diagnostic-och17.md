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

## Actual Intel GPUI integration outcome

The unchanged two-window GPUI integration probe on Intel,
[run 37733380351](https://github.com/dakotamurphyucf/gpuio/actions/runs/37733380351),
commit `938eb3b00cc5c1e05415120c5b05ef25cd8a5e20`, built successfully but failed
its strict validation. Both windows were observed visible at the start and end,
completed all 90 requested frames, had distinct identities, stopped admission,
closed and settled to zero pending callbacks. Neither had loss or invalid clocks.

Window 1 has 34 initial zero-time outcomes followed by 56 positive presentations;
window 2 has 51 initial zeros followed by 39 positive presentations. There are
no zeros after the first positive in either trace. First positive presentation
occurs 1,986.067 and 1,997.201 ms after first submission. The active second window
has 38 animation samples, below the existing 45-sample minimum. The original
all-presented and animation gates therefore fail; none is relaxed here.

The [native archive](hosted-metal-diagnostic-och17/native-hook-reports.tar.gz)
preserves eleven files: exact workflow/test sources, build logs, raw native report
and environment metadata. Its [manifest](hosted-metal-diagnostic-och17/native-hook-manifest.json)
was verified against every archive member. The unrelated successful bootstrap's
long log is available in the hosted artifact rather than copied into this bundle.

The observed initial two-second interval motivates one new diagnostic: use the
already-existing `--idle-before-frames-ms 2500` option before starting collection,
keeping all frame, animation, clock, completion and teardown checks unchanged.
[Run 37736998639](https://github.com/dakotamurphyucf/gpuio/actions/runs/37736998639),
commit `93006c09f5fa82635a0c54c9bdecf499d4be511f`, is running at this checkpoint.
Only its isolated workflow invocation changes; no renderer or test code changes.
This tests startup conditioning, not a production startup fix, and the original
failed trace remains failed. Existing required main-branch presentation failures
remain unresolved. Local physical-Mac workload passes are separate evidence.
