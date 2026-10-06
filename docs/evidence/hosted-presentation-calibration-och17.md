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


## Repeat on hosted run 37400903839

[Run 37400903839](https://github.com/dakotamurphyucf/gpuio/actions/runs/37400903839)
is terminal with failure at branch `2e9cd54`. Linux passes; macOS fails native
navigation and both presentation probes. The fresh extracted-app receiver was
skipped by that older workflow. The newer receiver-scheduling change is outside
this run's coverage.

The native navigation workload reaches all earlier navigation/disclosure/inert
markers, but its resized button bounds remain 500 points when 400 are required
(`navigation_lifecycle_test.rs:421`). The preceding actual-viewport wait passed;
that wait and the existing frame callback barrier did not establish updated
child geometry in this hosted run. The cause remains under investigation.

The GPUI hook again records 180 admitted callbacks with zero presentation times,
no missing callbacks and zero pending submissions at retirement. Independent
Metal records 120 active, visible, GPU-completed frames, all with zero presentation
times on Apple Paravirtual. Display inventory remains empty. Neither probe passes;
no presentation threshold or release gate changed.

[Raw reports, terminal metadata and navigation failure](hosted-presentation-calibration-och17/run-37400903839/reports.tar.gz)
are retained with a [verified manifest](hosted-presentation-calibration-och17/run-37400903839/manifest.json).
Later palette changes require their own current-source hosted qualification.

## Repeat on hosted run 37410532617

[Run 37410532617](https://github.com/dakotamurphyucf/gpuio/actions/runs/37410532617)
is terminal with failure. Linux foundation and the independent extracted-app
receiver pass. The only failed macOS foundation steps are the GPUI presentation
hook and independent Metal calibration; native navigation passes this time.
The actual tested PR merge is `7c72f21e83948c9bf5c46d5837869ffdf8581c95`.
Its Git tree `728f98973fc356bc0211e1780e1f6cb3968e483f` matches branch `8cf5b5f`.
This is older than the positioned-menu and native-icon implementation.

The hook records 180 admitted zero-time callbacks (90 in each window), no missing
callbacks and no pending submissions at retirement. Standalone Metal again
submits 120 frames on Apple Paravirtual and fails clock ordering with zero
presentation timestamps. Neither probe nor the release gate is waived.

Independent receiver scheduling now demonstrably works despite those failures:
all three extracted applications pass their native runtime walkthroughs on the
separate fresh hosted runner. Transfer verification ties package, archive and
executable hashes to the tested source revision. The application-level reports
retain their narrower environment-isolation wording; machine freshness comes
from the separate job, not that wording. These remain internal ad-hoc test
packages, with notices and release distribution/signing acceptance still open.

[Reports, terminal jobs, merge tree and receiver evidence](hosted-presentation-calibration-och17/run-37410532617/reports.tar.gz)
are retained with a [verified manifest](hosted-presentation-calibration-och17/run-37410532617/manifest.json)
(30 files; 1,258,038 uncompressed bytes). Newer source needs its own checks.

## Repeat on hosted run 37421611438

[Run 37421611438](https://github.com/dakotamurphyucf/gpuio/actions/runs/37421611438)
finished with failure on 2026-10-06. Linux foundation and the independent fresh
macOS extracted-app job pass. The only failed macOS foundation steps are the GPUI
Metal presentation hook and independent Metal calibration. The tested PR merge
is `538abf39fe7f48d86b9939d2c83a38d58f22ce99`; its tree
`e83e84e33708913f30a1b170af631ec0a355724c` matches branch `23650c8`.
This predates the newer popup-controller, categorical and stacked-chart work.

Both hook windows finish and close with no pending submissions; each records
90 zero-time callbacks and no missing callbacks. Standalone Metal submits all
120 requested frames on Apple Paravirtual, with zero presentation times and
failed clock-ordering qualification. No presentation gate is waived.

The separate receiver verifies transfer hashes and passes gallery, Agent Workspace
and Signal Studio runtime checks. Its narrower application reports remain explicit
about development-directory isolation; the separate hosted job establishes machine
freshness. These are internal ad-hoc test packages, not approved release artifacts.

[Reports, terminal jobs, source identity and receiver artifacts](hosted-presentation-calibration-och17/run-37421611438/reports.tar.gz)
are retained with a [verified manifest](hosted-presentation-calibration-och17/run-37421611438/manifest.json)
(31 files; 1,261,330 uncompressed bytes). Final-source checks, notices, signing and
release distribution acceptance remain open.

## Repeat on hosted run 37433332134

[Run 37433332134](https://github.com/dakotamurphyucf/gpuio/actions/runs/37433332134)
finished with failure on 2026-10-06. Linux foundation and the independent fresh
macOS extracted-app job pass. The only failed macOS foundation steps are the
GPUI Metal presentation hook and independent Metal calibration. The tested merge
`d439c3ae427aeeaf929cef6c782e0899450e3be8` has tree
`55bdd665cab987eb7ce84a44d2f92417c86c5f35`, matching branch `746b29b`. This includes
stacked charts but predates the ordinal-color and inspection-control work.

Both hook windows finish and close, each with 90 zero-time callbacks, no missing
callbacks and no pending submissions. Standalone Metal submits 120 frames on
Apple Paravirtual and again fails clock ordering with zero presentation times.
No gate is waived, and these probes do not establish physical presentation timing.

The independent receiver passes gallery, Agent Workspace and Signal Studio
runtime checks. These remain internal ad-hoc packages, not signed release
artifacts. [Raw reports, job results and source identity](hosted-presentation-calibration-och17/run-37433332134/reports.tar.gz)
and a [verified manifest](hosted-presentation-calibration-och17/run-37433332134/manifest.json)
retain 31 files (1,262,072 uncompressed bytes). Current-source validation,
notices, signing/distribution and broader release acceptance remain open.
