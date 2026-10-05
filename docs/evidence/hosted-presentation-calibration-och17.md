# Hosted Metal calibration failure — OCH-17

[Run 37321333808](https://github.com/dakotamurphyucf/gpuio/actions/runs/37321333808)
completed with failure on 2026-10-05. Run head is `af96ab9`; this is a historical
checkpoint and does not cover later local changes. The macOS job passes its other
required steps but fails both presentation probes. Linux passes; the dependent
fresh extracted-app job is skipped. [Raw reports and job metadata](hosted-presentation-calibration-och17/reports.tar.gz)
are preserved with [archive hashes](hosted-presentation-calibration-och17/manifest.json).

Unlike the preceding run, the standalone Metal calibration executed independently
of the GPUI hook result. Both fail:

| Check | Observed result |
| --- | --- |
| GPUI hook | Two independent windows, 90 admitted frames each; all 180 callbacks return zero presentation timestamps. No missing/duplicate/saturated/truncated events; pending is zero and sessions retire. |
| Standalone Swift/Metal | All 120 frames report active and visible; all GPU command buffers complete; all 120 `presentedTime` values are zero despite presentation callbacks. |
| Device / OS | `Apple Paravirtual device`, macOS 15.7.9 (24G830). `system_profiler SPDisplaysDataType` returns an empty display list. |

The standalone program does not use GPUI, OCaml or GPUIO's collector. Its result
rules out a defect unique to the GPUI hook as a sufficient explanation for this
run. It supports an environment/API limitation hypothesis, but does not establish
that every virtual Mac is unsupported or that there is no runner configuration
that can supply timestamps. GPU completion and callback delivery are not physical
presentation evidence.

Both probes remain failures. No thresholds, workflow gates, capability policy or
release requirements were changed. Do not relabel zero timestamps as accepted
presentation, infer workload FPS from GPU completion, or treat the skipped fresh
receiver as passing. Next work is to determine a defensible hosted-capability and
physical-Mac validation path while retaining the required release measurements.
The local physical-Mac results retain their own recorded source/platform scope.
VoiceOver remains on hold and was not involved in these probes.

## Repeat on hosted run 37343201640

[Run 37343201640](https://github.com/dakotamurphyucf/gpuio/actions/runs/37343201640)
finished with failure on 2026-10-05 at branch checkpoint `470210a` (runner merge
`04dcd08`). Linux passed; the fresh extracted-app job was skipped. Three additional
macOS walkthrough failures have separate local corrections: [calendar and theme
readiness](macos-ci-readiness-och17.md) and [sidebar sampling](sidebar-ci-sampling-och17.md).

Both Metal probes again fail on the Apple Paravirtual device. The GPUI hook
finishes both 90-frame sessions with all 180 timestamps zero, no missing/duplicate/
saturated/truncated events, zero pending records and both windows closed. The
standalone calibration records 120 active, visible, GPU-completed frames, all with
zero presentation time. The display inventory is empty. These independently
reproduce the earlier environment observation; they do not identify a functioning
hosted presentation-clock configuration.

[Raw reports and terminal job metadata](hosted-presentation-calibration-och17/run-37343201640/reports.tar.gz)
are preserved with a [verified six-file manifest](hosted-presentation-calibration-och17/run-37343201640/manifest.json).
No gate or timestamp criterion changed. The owner's separate VoiceOver hold has
now been lifted; these probes still provide no VoiceOver evidence.

## Repeat on hosted run 37356882651

[Run 37356882651](https://github.com/dakotamurphyucf/gpuio/actions/runs/37356882651)
finished with failure on 2026-10-05 at branch checkpoint `e96d27e`. Linux passed;
macOS failed Settings composition, navigation resize and both presentation
probes. The fresh extracted-app receiver was skipped. The first two failures have
[locally passing readiness corrections](macos-ci-readiness-och17.md#settings-and-navigation-follow-up--run-37356882651)
in `3ea0bfc`, which this older hosted run does not qualify.

The two GPUI sessions again admit 90 frames each and return all 180 timestamps
as zero. Neither session loses, duplicates, saturates or truncates records; both
close with zero pending submissions. All 120 standalone Metal frames are active,
visible and GPU-completed, with zero presentation timestamps. The device remains
Apple Paravirtual on macOS 15.7.9 (24G830). These results preserve the same
environment limitation evidence; neither probe passes or establishes physical
presentation timing.

[Six raw reports and terminal metadata](hosted-presentation-calibration-och17/run-37356882651/reports.tar.gz)
are retained with a [verified manifest](hosted-presentation-calibration-och17/run-37356882651/manifest.json).
No threshold or gate was changed.
