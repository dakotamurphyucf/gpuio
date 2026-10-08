# Milestone 07 closeout

Updated 2026-10-08; implementation checkpoint `500f9b4e`. The owner now
prioritizes a **developer preview for real-user feedback**, with focused
performance qualification, over exhaustive pre-release review. This updates the
milestone 07 delivery target in [OCH-17](https://linear.app/ochat/issue/OCH-17)
and [OCH-41](https://linear.app/ochat/issue/OCH-41). It is not a stable API,
complete accessibility, Linux desktop or signed-app certification.

The preview must offer reproducible source/consumer builds, runnable documented
examples, usable required component families with honest limitations, reviewed
release notices, core performance/resource checks and no known ordinary-use
blocker such as a crash, lost input/data or unbounded growth. Existing correctness
and required Linux checks remain. Deeper qualification and signed reference-app
delivery move to [OCH-164](https://linear.app/ochat/issue/OCH-164), which does not
block the preview. Linux desktop remains OCH-47. Old failures stay recorded.

## Operating rules

1. **Name the gap before doing work.** Each task must identify a checklist ID,
   the existing requirement it satisfies, a concrete failing/missing behavior,
   and the evidence needed to close it. A historical document saying "unqualified"
   is a reason to check later evidence, not automatically to create another test.
2. **Freeze the feature scope.** Repair required v1 behavior. Record enhancements
   outside the accepted scope as follow-ups. The dated preview decision above
   identifies the deferred qualification; do not silently relabel an ordinary-use
   defect as polish. Record new blockers with their reason and impact on closure.
3. **Finish one investigation at a time.** Keep one primary implementation task
   active. After 60–90 minutes, record the hypothesis, evidence gained and next
   discriminating experiment. Change the approach if repeating it adds no evidence;
   the checkpoint does not waive a failing requirement or end useful work.
4. **Reuse evidence deliberately.** For each requirement, record its tested
   revision/binary and whether subsequent changes affect that path. Run targeted
   regressions after fixes, then the required integrated candidate checks. Do not
   repeat a successful whole matrix after unrelated documentation changes. Keep
   failed trials and prescribed repeat counts; never test until a lucky pass.
5. **Separate diagnostics from acceptance.** Covered-window timeouts and invalid
   setup cannot qualify a run. Diagnose with short targeted runs before another
   full workload. One GUI test owner at a time; announce foreground sessions,
   bound them and close/reap apps. No concurrent builds during performance runs.
6. **Batch CI and reporting.** Push coherent locally validated changes, let the
   useful run finish, and collect failures together. Continue independent work
   while CI runs. Keep one concise evidence record per completed task, linking
   existing artifacts instead of repeating history across documents and comments.
7. **Make progress countable.** Report closed checklist IDs, active work, named
   remaining gaps and external prerequisites. The earlier ~85% estimate is a
   judgment, not an audited percentage or a schedule. Do not derive a milestone
   percentage from the number of rows below: these are remaining work of unequal size.

## Remaining checklist

All rows below are open closure items, not claims that their implementation is
absent. Existing evidence reduces the work. The implementing agent owns execution;
no other agent is assigned by this plan.

| ID / ticket | Existing evidence to preserve | Next action | Exit condition / dependency |
| --- | --- | --- | --- |
| C1 / OCH-41 — preview coverage inventory | [41 v1 families](catalog/families.json), scoped gallery/consumer results; OCH-48 walkthroughs complete | Make one bounded pass mapping each required family to an example and existing behavior evidence; name ordinary-use blockers and disclosed limits. Stop opening exhaustive permutation reviews. | Each family has a usable public example and meaningful scoped evidence or a named preview blocker. Publish the finite blocker list as the next deliverable. |
| C2 / OCH-41 — fix preview blockers | Latest [managed scrollbar repair](evidence/managed-scrollbar-routing-och41.md): full OCaml suite and root/installed 18-case passes | Fix C1’s concrete blockers in user-impact order; validate affected paths. | Public examples and component families work for normal use; known limitations documented. Broader theme/gesture/assistive edge cases are OCH-164 unless they reveal an ordinary-use blocker. |
| P1 / OCH-17 — long-list timing and scrolling | [List trials](evidence/presentation-list-full-och17.md), [paint geometry](evidence/list-paint-geometry-och17.md): two valid full passes, one idle failure, subsequent interrupted diagnostics | Use existing diagnostics for a short discriminating experiment on idle/jitter; then complete the focused list qualification. | Three valid optimized trials meet retained list/input/presentation/memory/idle bounds; observable scrolling failures resolved. Needs an uninterrupted visible-window interval. Historical startup/chart findings stay documented follow-ups unless a current user-facing regression reproduces. |
| P2 / OCH-17 — rapid updates, idle and bounded resources | Three full typing passes and three 33-cycle resource trials at recorded revisions; [overhead attempts](evidence/collector-overhead-och17.md) incomplete | Review change impact, complete the representative collector comparison needed to trust the two core timing workloads, and validate affected streaming/typing/resource paths. | Retained streaming/input budgets, measurement validity, settled idle and cleanup bounds pass. Existing table/document timing evidence is retained; no blanket repeat of their full timing matrices. |
| D1 / OCH-17 — notices and release inputs | [499 OCaml texts verified](evidence/ocaml-notice-provenance-och17.md), eight reconstructed vendor roots, existing Rust/source-equivalence research | Finish unresolved Rust/embedded/asset/system/helper attribution and the assembled bundle review. Do not repeat completed archive provenance searches without a new lead. | Reviewed notices and reproducible release inputs match the actual artifact contents; unresolved classifications have a documented supported disposition. |
| R1 / OCH-17 — preview API/docs/feedback | Existing [API review](evidence/api-boundaries-och17.md), [compatibility guide](api-compatibility.md), walkthroughs and installed starter | Review the beginner install/build path and release-facing API contracts; prepare preview notes and a minimal reproduction/OS/version checklist for user reports. | Installation and examples work; experimental compatibility/platform/accessibility limits and feedback route are explicit. No promise of stable API or completed signed-app distribution. |
| R2 / OCH-17 — candidate CI and preview delivery | Foundation `37805065605`: Linux pass, macOS still running at last inspection; validated local changes await batch push | Collect that run, batch validated changes, implement/review the preview treatment of unavailable hosted Metal timing while retaining physical core measurements and hosted correctness/error/cleanup tests, then resolve required failures together. | Required preview CI/consumer checks pass; reviewed candidate tagged/published as a prerelease with source build instructions and known limits. No stable release claim. The current workflow is unchanged until that reviewed CI change is implemented; arbitrary failures cannot be suppressed as unavailable. |

## Follow-up boundary and execution order

[OCH-164](https://linear.app/ochat/issue/OCH-164) carries broader real VoiceOver
qualification, additional catalog/OS permutations, stable-release hosted Metal
availability investigation, historical startup/chart diagnostics and actual
Developer ID/notarized reference-app distribution. Existing passes and failures
remain scoped evidence. A reproducible crash, data/input loss, unusable core
interaction or unbounded resource growth still blocks the preview.

Next: C1's bounded reconciliation, yielding the exact blocker list. Then close
P1/P2 and C2; use D1/R1 during CI waits. Batch R2 integration throughout and publish
only after the seven preview closure items are satisfied. Do not wait for Apple
signing credentials or full screen-reader certification to publish the source
library preview; do not advertise those unqualified capabilities.

One primary task stays active at a time. Report **closed / active / next / external
prerequisite**, and update rows in place. Detailed artifacts stay in linked task
evidence. No new per-component performance matrix or automatic rerun of historical
checks is part of this plan. The earlier ~85% estimate predates this narrower
release target; do not mechanically convert it into a new percentage or deadline.
