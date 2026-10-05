# Metal presentation API calibration — OCH-17

2026-10-05, Apple M1 Max, macOS14.5 build23F79. This validates a platform
measurement mechanism, **not GPUI performance or completion of OCH-17**.
The [integration design](../design/metal-presentation-qualification.md) keeps the
remaining implementation, window/frame attribution and measured workloads explicit.

A standalone optimized Swift/AppKit/Metal probe creates one 640×360 logical-pixel
window, submits 120 clear-pass frames at requested 60Hz timer intervals, and closes
itself. It retains GPU completion and drawable presentation independently. No
VoiceOver, clipboard, input-source or preference operation is performed.

The final run passes all 120 records, with unique drawable IDs0..119, zero
pre-presentation timestamps, nonzero strictly increasing actual presentation times,
valid completed command buffers and visible-window submissions. Startup waits two
timer ticks for actual activation/visibility. The process exits0 after closing its
owned window. This is OS-reported presentation, not a photon measurement.

## Results and preceding attempts

| Run | Result | Reason / observations |
| --- | --- | --- |
| 001 | Failed | All120 callbacks and GPU records arrived, but frame0 was submitted before AppKit reported the window visible. The probe was changed to wait for actual startup activation/visibility. |
| 002 | Failed | All120 records were active/visible and correctly ordered, but a harness cast from Swift's drawable-ID value to UInt rejected the identities. The raw IDs are distinct0..119; storage/validation now uses explicit UInt64. |
| 003 | Passed | All120 records satisfy the corrected complete validation. No missing/zero/out-of-order presentation, repeated ID or failed GPU completion. |

All attempts are retained. Timing outliers were not discarded or used to choose a
performance threshold. For example, run002 includes a 123.318ms submission-before
to presentation interval at sequence7, whose cause is not established by this
probe. Final-run diagnostics are:

| Interval | Samples | Median | Maximum |
| --- | --- | --- | --- |
| Submission-before to presentation | 120 | 9.571ms | 89.513ms |
| GPU completion to presentation | 120 | 9.146ms | 89.140ms |
| Presentation to callback arrival | 120 | 0.360ms | 5.135ms |
| Consecutive presentation timestamps | 119 | 16.667ms | 70.833ms |

These small clear-pass/startup measurements are neither application frame budgets
nor Metal throughput. The distinction between GPU completion, presentation time
and callback arrival is visible in the records. A 60Hz timer request does not
establish fixed physical display cadence or GPUI FPS.

## Reproduction and source

Implementation/CI/design commit `286ba88`; final run used matching uncommitted
Swift source based on `841d889`. [Provenance](metal-presentation-calibration-och17/provenance.json)
records the final source/binary hashes and summaries of every attempt. Earlier
attempts used the preceding harness versions described above.

```sh
swiftc -O scripts/qualify_metal_presentation.swift -o scratch/agents/root-20261004-resumed/qualify-metal-presentation
scratch/agents/root-20261004-resumed/qualify-metal-presentation > scratch/agents/root-20261004-resumed/presentation-probe-003.json
```

Swift compilation emits no diagnostics. Actionlint1.7.12 and `git diff --check`
pass. A separate macOS CI calibration step is added for the next pushed revision;
current run999e531 predates it. No GPUI/library/vendor/dependency changes are
included in this checkpoint. There was no concurrent GPUIO build during the probe.

Raw reports: [001](metal-presentation-calibration-och17/run-001.json),
[002](metal-presentation-calibration-och17/run-002.json),
[003](metal-presentation-calibration-och17/run-003.json).
The proposed actual GPUI hook/collector, default-feature comparison, missing/dropped
callback fixtures, clock correspondence, full workloads and required final hosted
checks remain open. VoiceOver stays on the owner's hold.
