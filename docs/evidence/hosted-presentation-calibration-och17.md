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


## Repeat on hosted run 37374125077

[Run 37374125077](https://github.com/dakotamurphyucf/gpuio/actions/runs/37374125077)
finished with failure on 2026-10-05 at branch `4c959f5`, tested merge `6e4ccda`.
The GPUI probe again records 180 admitted callbacks, all with zero presentation
timestamps, no missing callbacks, and retired sessions. Independent Metal on
Apple Paravirtual / macOS 15.7.9 again records 120 active, visible, GPU-completed
frames with zero presentation timestamps. Neither probe passes.

The separate standard-window lifecycle test passes. Custom chrome passes three
minimize/restore cycles and native title-bar movement, then fails its physical
pointer ownership check before clicking Fullscreen: expected PID 8587, actual
PID 2330 at (1321.5, 235.5). The report confirms the child was reaped. The owner
of PID 2330 is not established by these artifacts; this does not prove a native
fullscreen bug or a specific cause of occlusion. A bounded foreground/readiness
check and better occluder diagnostics are the next investigation.

Linux was cancelled before acquiring a hosted runner; no Linux checks ran in this
job. The fresh extracted-app receiver was skipped. These are not passing release
gates. [Reports](hosted-presentation-calibration-och17/run-37374125077/manifest.json)
retain hashes, branch/merge revisions and exact counters. The newer palette/menu
checkpoint `a8def93` is outside this run's coverage.

## Repeat on hosted run 37387307992

[Run 37387307992](https://github.com/dakotamurphyucf/gpuio/actions/runs/37387307992)
is terminal with failure at branch `f6e34e2`, tested merge `5e5a610`. Linux passes;
macOS fails custom-window pointer readiness and both presentation probes. The
fresh extracted-app receiver is skipped. Later palette and example-readability
changes are outside this run's coverage.

The GPUI sessions again report 180 admitted callbacks with zero presentation
timestamps, no missing callbacks and no pending submissions after retirement.
The independent Metal probe submits 120 frames on Apple Paravirtual/macOS
15.7.9 and fails its presentation-clock qualification. No performance gate passes
from these results.

The new pointer diagnostics identify **NotificationCenter** owning the Fullscreen
button's physical point (1321.5, 235.5), while the gallery remains frontmost.
The bounded readiness wait subsequently cannot locate the target and expires.
Three minimize/restore cycles and native title-bar movement pass before that
failure; the child is reaped. Standard-window checks pass. This establishes an
OS overlay interfering with the first pointer sample; it does not explain every
later accessibility lookup or establish a product fullscreen defect. The next
harness investigation should keep the tested window away from notification-banner
geometry while retaining actual pointer ownership checks, without changing user
notification preferences.

[Reports, terminal metadata and original failed-step log](hosted-presentation-calibration-och17/run-37387307992/reports.tar.gz)
have a [verified checksum manifest](hosted-presentation-calibration-och17/run-37387307992/manifest.json).
No threshold, assertion or release gate has been waived.

## Repeat on hosted run 37398392336

[Run 37398392336](https://github.com/dakotamurphyucf/gpuio/actions/runs/37398392336)
is terminal with failure at branch `c0694a2`. Both platforms fail Build because
the low-level `bridge` and `view_api` examples omit the new `Palette_observed`
event in exhaustive matches. The earlier local suite built the tests and gallery,
which did not compile these standalone examples. Linux unit/private-bus/consumer
checks and the macOS window walkthroughs were skipped; this run does not qualify
the preceding owned-window placement correction. The fresh receiver is skipped.

Both Metal probes run independently of that build failure and still fail. GPUI
admits 180 callbacks across two retired sessions, all with zero presentation
timestamps, no missing callbacks and no pending submissions. Standalone Metal
records 120 active, visible, GPU-completed frames, all with zero presentation
timestamps on Apple Paravirtual/macOS 15.7.9. This repeats the hosted clock
limitation; it provides no passing presentation evidence.

[Seven retained files](hosted-presentation-calibration-och17/run-37398392336/reports.tar.gz)
include the raw reports, terminal metadata and original build/job failure logs,
with a [verified checksum manifest](hosted-presentation-calibration-och17/run-37398392336/manifest.json).
The example correction explicitly handles palette observations and the newer
command responses; it retains exhaustiveness checking. Exact all-example build
validation is recorded separately below. No gate or threshold is waived.

### Local all-example build correction

At `282daf0` plus the archived four-line example patch,
`GPUIO_JOBS=2 ./scripts/gpuio build` and `GPUIO_JOBS=2 ./scripts/gpuio fmt`
both exit zero on the local physical arm64 Mac. `git diff --check` also passes.
`view_api` dispatches `Palette_observed` through its reconciler; the raw bridge
ignores that event. Both explicitly ignore command replies they never request.
No wildcard or warning suppression hides future event additions.

[Patch, exact validation metadata and logs](hosted-presentation-calibration-och17/run-37398392336/local-build.tar.gz)
have a [verified four-file manifest](hosted-presentation-calibration-och17/run-37398392336/local-build-manifest.json).
This proves local default-target compilation, including the standalone examples
omitted from the earlier targeted gallery build. It does not establish a Linux
pass, new native behavior coverage or hosted window-placement acceptance.
