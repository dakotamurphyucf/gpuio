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
commit `93006c09f5fa82635a0c54c9bdecf499d4be511f`, **fails**. Both windows finish
all 90 frames with one initial zero-time outcome followed by 89 presentations.
The active window has 88 animation samples, satisfying that minimum; visibility,
distinct identities, closure and settlement pass. The strict all-presented check
still fails. Only the isolated workflow invocation changed; no renderer or test
code changed. The [idle experiment archive](hosted-metal-diagnostic-och17/idle2500-reports.tar.gz)
retains ten files, verified against its
[manifest](hosted-metal-diagnostic-och17/idle2500-reports-manifest.json).

The next isolated experiment,
[run 37740193731](https://github.com/dakotamurphyucf/gpuio/actions/runs/37740193731)
at `7f648d9ad19c551b566a1f4fcdbdf0ea660df444`, records a separate active
90-frame warmup before replacing the measurement sessions and running the original
strict 90-frame phase. Its warmup reports are retained separately; the measured
phase's acceptance predicate is unchanged, including rejection of any skipped
frame and the animation minimum. This experiment also **fails**: the measured
first window presents all 90 frames, but the second has one initial zero followed
by 89 presentations. Its 88 animation samples satisfy the minimum. Both finish,
remain visible at the endpoint observations, close and settle with zero pending
callbacks and no other lost or invalid outcomes.

The separately retained warmup contains 84 presented/six zero outcomes in window
1 and 80 presented/ten zero outcomes in window 2; the latter includes a zero after
its first positive. No warmup record is silently discarded or counted as measured
success. The [warmup archive](hosted-metal-diagnostic-och17/warmup-reports.tar.gz)
retains thirteen files, including exact diagnostic source/workflow, reports and
build logs, verified against its
[manifest](hosted-metal-diagnostic-och17/warmup-reports-manifest.json).

Active warmup therefore does not establish strict steady-state acceptance under
this probe sequence. No corresponding main-branch change is adopted. This was not
a production startup repair or cold-start pass. Original failures remain failed;
required presentation checks remain unresolved. Local physical-Mac workload passes
are separate evidence.

## Continuous warm-up — three Intel trials pass, 2026-10-08

[Run 37745626692](https://github.com/dakotamurphyucf/gpuio/actions/runs/37745626692)
completed successfully at diagnostic revision
`ad7587d2acf684b070c3241f1d25c87e83f23f5b` on `macos-15-intel`.
The [22-file archive](hosted-metal-diagnostic-och17/continuous-reports.tar.gz)
and [manifest](hosted-metal-diagnostic-och17/continuous-manifest.json) preserve
all three trials, build/native logs, display/toolchain data, run metadata and
exact committed probe/driver/workflow sources. Archived bytes were read back and
verified. The sources match the independently retained experiment files, and the
measured-session `accepted` predicate matches the main probe byte-for-byte.

The previous warm-up stopped animation before starting new measurement sessions.
This variant keeps requesting animation frames while warm-up callback admission
stops and callbacks settle within the original two-second bound. It records any
render calls during that transition as **unmeasured**, drops the warm-up session,
and starts a fresh measured session without a new `cx.notify`. Each measurement
retains the original 90-frame target, 12-second phase deadline and strict
zero/loss/clock/retirement checks. All warm-up records remain visible. No renderer,
collector, clock or production behavior is changed.

The predeclared batch runs exactly three trials and stops on the first failure;
it did not retry a failed trial:

| Trial | Measured presentations, window 1 / 2 | Measured zeros | Warm-up presented/zero, window 1 / 2 | Unmeasured transition renders |
| --- | --- | --- | --- | --- |
| 001 | 90 / 90 | 0 / 0 | 80/11 / 122/19 | 1 / 2 |
| 002 | 90 / 90 | 0 / 0 | 90/0 / 138/0 | 0 / 1 |
| 003 | 90 / 90 | 0 / 0 | 90/0 / 145/0 | 0 / 0 |

Every measured session is closed, stopped and pending-zero, with 90 admitted
records and submission samples. No saturation, missing/not-submitted record,
invalid clock, duplicate, truncation or histogram overflow occurs. The active
window has 89 animation intervals in every trial; both windows are visible at
start and end. The configured hosted display is 1920×1080 at scale 1.

The physical Mac's preceding local variant also passed both measured 90-frame
windows; its warm-up and transition are separately retained in the local research
workspace. Hosted results above establish repeatability on this tested Intel
runner. They do not prove ARM timestamp support, cold-start acceptance, idle-to-
active transitions or end-to-end application performance. Trial 001's warm-up
zeros remain failures of presentation timing in that phase, not reclassified
successes. This supports the continuous-transition hypothesis for steady-state
collector qualification. The diagnostic branch remains separate; the original
Foundation probes and their failures have not been removed or waived.

## Foundation revalidation at 497236b7 — 2026-10-08

[Run 37744396036](https://github.com/dakotamurphyucf/gpuio/actions/runs/37744396036)
is complete. Linux Foundation and the separate fresh macOS extracted-app receiver
pass. macOS Foundation fails only the GPUI Metal hook and standalone API
calibration. The repaired native-controls fixture and full table-history traversal
pass. These results apply to `497236b7`, not subsequent local changes.

The hosted ARM GPUI probe finishes both 90-frame windows with zero positive
presentations and 90 zero outcomes each. Both sessions stop, close and settle
without other loss. Standalone calibration has zero timestamps in all 120
submitted frames on the Apple Paravirtual device. These remain failed gates;
the isolated Intel warm-up evidence above does not override them.

The [five-file archive](hosted-metal-diagnostic-och17/foundation-37744396036.tar.gz)
retains terminal job/step status and raw hook, calibration and display reports.
Every member was verified against its
[manifest](hosted-metal-diagnostic-och17/foundation-37744396036-manifest.json).

## ARM sequential and startup-delay follow-up — 2026-10-08

[Run 37775001974](https://github.com/dakotamurphyucf/gpuio/actions/runs/37775001974),
commit `44ee032f2acc5bc62546a6b59e98f0c9cc2e5de7`, tests two predeclared variants
on macos-15 arm64, macOS 15.7.9 (24G830), Apple Paravirtual device. This isolated
`diagnostic/metal-arm-pacing-20261008` branch replaces Foundation with a bounded
Swift-only diagnostic workflow; that workflow must not be merged into the release
branch. The main Foundation run remains independent.

The first variant uses the exact archived sequential Swift source that passed on
Intel. The second adds a fixed five-second delay after the first active/visible
observation. Both still request 120 frames, keep the original twelve-second total
deadline, and retain every submitted record. The delay consumes part of that
original deadline. The complete validation/teardown function is byte-identical to
the maintained probe; neither variant discards zero timestamps or replaces them
with completion times.

| Variant | Submitted / requested | GPU completed | Presentation callbacks received | Positive timestamps | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| Sequential | 3 / 120 | 3 | 2 | 0 | Deadline failure |
| Sequential, five-second startup delay | 2 / 120 | 2 | 1 | 0 | Deadline failure |

Every submitted record observes an active, visible window. Received presentation
callbacks report zero; the final submitted frame in each variant has no
presentation callback before the deadline, despite completed GPU work. Sequential
admission therefore stops before all 120 frames can be submitted. These outcomes
reject these two changes as sufficient remedies on this runner. They do not prove
that ARM presentation callbacks are universally unsupported, explain the missing
callbacks, or qualify GPUI workload performance. The earlier Intel success does
not transfer to this ARM environment.

The job succeeds only at collecting both outcomes; both strict probes exit 1.
The [raw archive](hosted-metal-diagnostic-och17/arm-pacing-reports.tar.gz)
contains nineteen files: both exact Swift variants, full reports/build logs,
helper/workflow sources, run identity, and display/power/OS metadata. Every member
was read back and checked against its [manifest](hosted-metal-diagnostic-och17/arm-pacing-manifest.json).
Python syntax, Ruff and actionlint pass for the diagnostic helper/workflow.
No production renderer, main probe, deadline or release requirement changes.
