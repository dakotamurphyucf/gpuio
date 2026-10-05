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
