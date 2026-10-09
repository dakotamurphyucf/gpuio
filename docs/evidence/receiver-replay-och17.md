# Exact packaged receiver replay — OCH-17

2026-10-08 follow-up to the Agent Workspace Python SIGTRAP in
[Foundation37772649589](milestone-07-ci.md). The failure remains unexplained;
these successful replays do not erase it or establish a repair.

Downloaded `macos-runtime-qualification-inputs` from that run and verified the
transfer manifest, all three package/archive/executable hashes, and packaging
revision `b13984c1960b3be8c1dde7241c12ac60b0555250`. This is the recorded packaging
checkout revision, not a newly inferred binary build attestation. Archives are
internal ad-hoc packages with incomplete notice qualification, not release artifacts.

## Local replay

The exact hosted Agent Workspace binary
`af319e8dabaa1dd7c4bf3638bae43a7119148dd377e1f2deaed0053ec1ca1555`
passes the full extracted-runtime walkthrough on macOS14.5 arm64. Attachment
selection, Eio read/render, streaming/error/retry, retained tabs, independent
windows, theme/commands and close policy pass; `App.run` returns. Development
paths remain denied by the child sandbox. Clipboard restoration is verified.

The current runtime harness enables fatal stack reporting, including SIGTRAP.
An outer supervisor owns a separate process group and clipboard preservation,
so a fatal harness crash cannot leave its test application running or skip
restoration. This run exits zero, closes its applications and is reaped normally.
It is a different OS release from the failing hosted runner, not a reproduction.

## Fresh hosted replay

Diagnostic branch `diagnostic/receiver-trap-20261008` at `00b702fe` replaces its
Foundation workflow solely to rerun the existing three-app receiver sequence.
**Never merge that replacement workflow.** No compiler/dependency bootstrap occurs;
it downloads the original run's archives and uses the new fatal-stack/native
crash-report diagnostics from `2d429f2a`.

The first diagnostic run37790615165 stops before GUI work because ordinary
transfer admission correctly rejects artifacts from another CI run. The isolated
replay then explicitly verifies the producing run37772649589/attempt1, packaging
revision and all hashes. The normal Foundation workflow still verifies its own
run/attempt and is unchanged.

[Replay37790823149](https://github.com/dakotamurphyucf/gpuio/actions/runs/37790823149)
passes on macOS15.7.9 arm64. All three receiver reports have `complete: true`,
clipboard restored, matching original executable hashes, and denied access to
existing development roots:

| Application | Exact original executable SHA256 |
| --- | --- |
| Gallery | `788c17d8e2ec66107f2b7db45040324087488a097ad98c8e5b08f234a31b9755` |
| Agent Workspace | `af319e8dabaa1dd7c4bf3638bae43a7119148dd377e1f2deaed0053ec1ca1555` |
| Signal Studio | `25435dabb8868b34b4ae19d730710e40df54afa93371f15c0502a70a5e60552a` |

No native crash report is collected in that successful run. The original
attachment-time trap therefore remains an intermittent, unattributed harness
failure. Do not assign it to GPUI, file loading, AX array shape, or desktop
occlusion without a stack or other causal evidence. Keep the newly added
fatal-stack and bounded `.ips` capture on subsequent runs. No repeated run is
being used to silently turn the earlier failed release check into a pass.

[Twenty-one-artifact archive](receiver-replay-och17/reports.tar.gz) and
[verified SHA256/size manifest](receiver-replay-och17/manifest.json) retain local
and hosted reports/logs, verification, desktop metadata, the failed diagnostic
admission and the exact diagnostic workflow. Local and hosted apps are closed;
the temporary diagnostic worktree is removed after archiving. The remote branch
is evidence only. Main Foundation37789987337 continues independently on the
newer source and remains the relevant updated-source CI run.

No signing/notarization, full license review, VoiceOver, performance timing or
current-source release acceptance is established here. Milestone07 remains active.
