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

C1, P1, P2, D1 and R1 are complete for the focused preview scope; C2 and R2
remain open. The implementing agent owns execution; no other agent is assigned
by this plan. P1's bounded scroll assessment preserves the unconfirmed transient
visual report as follow-up, not as a claimed rendering fix.

| ID / ticket | Existing evidence to preserve | Next action | Exit condition / dependency |
| --- | --- | --- | --- |
| **C1 complete** / OCH-41 — preview coverage inventory | [Reviewed mapping](catalog/preview-coverage.md): 41 v1 families plus five accepted additions, public routes and scoped behavior evidence | No new missing-family implementation blocker found. Keep old results scoped to their actual revisions. | Inventory closed; C2/final integration and the five shared preview gates remain. |
| C2 / OCH-41 — fix preview blockers | [C1 reconciliation](catalog/preview-coverage.md) found no new independent catalog implementation blocker; latest scrollbar root/installed checks pass | Resolve any concrete current-candidate integration failure; link the existing list-scrolling issue to P1 instead of duplicating it. | Pending final integration, not another broad family audit. Full assistive/gesture permutations remain OCH-164. |
| **P1 complete** / OCH-17 — long-list timing and scrolling | [Current-source comparison](evidence/preview-list-comparison-och17.md): three valid optimized list trials, zero idle draws, complete traversal/growth/cleanup; previous failures retained | [Bounded scroll assessment](evidence/preview-scroll-assessment-och17.md) finds no reproduced ordinary-scroll blocker. Retain transient-jitter/hardware-gesture follow-up in OCH-164; no original-cause/fix claim. No repeat full timing batch after tooling/docs-only changes. | Three valid optimized trials meet retained list/input/presentation/memory/idle bounds; observable scrolling failures resolved. Needs an uninterrupted visible-window interval. Historical startup/chart findings stay documented follow-ups unless a current user-facing regression reproduces. |
| **P2 complete** / OCH-17 — rapid updates, idle and bounded resources | Three full typing passes and three 33-cycle resource trials at recorded revisions; [representative collector comparison](evidence/preview-list-comparison-och17.md) now complete | [Change-impact review](evidence/preview-performance-impact-och17.md) and current optimized streaming/resource smokes pass. Retain earlier full repetitions at their exact revisions; no second exhaustive timing matrix. | Retained streaming/input budgets, measurement validity, settled idle and cleanup bounds pass. Existing table/document timing evidence is retained; no blanket repeat of their full timing matrices. |
| **D1 complete** / OCH-17 — notices and release inputs | [499 OCaml texts verified](evidence/ocaml-notice-provenance-och17.md), existing vendor/source-equivalence research and [verified SVG attribution](../third_party/ASSET_NOTICES.md) | [Source payload inspection](evidence/preview-source-inputs-och17.md) finds no compiled binaries and repairs the retained Fira Code font notice. The assembled source artifact at `b90fb769` matches all 8,168 Git blobs/modes and retains required notices. Binary/system/SDK notice completion belongs to the separately shipped OCH-164 applications; R2 binds the final published revision. | Reviewed notices and reproducible release inputs match the actual artifact contents; unresolved classifications have a documented supported disposition. |
| **R1 complete** / OCH-17 — preview API/docs/feedback | Existing [API review](evidence/api-boundaries-och17.md), [compatibility guide](api-compatibility.md), walkthroughs and installed starter; [preview adoption/feedback guide](developer-preview.md) and GitHub report templates now prepared | [Fresh independent starter build and API/onboarding review](evidence/preview-installation-och17.md) pass. Source install, public layers, compatibility limits and feedback route are documented. Add the published revision during R2; no new full API sweep. | Installation and examples work; experimental compatibility/platform/accessibility limits and feedback route are explicit. No promise of stable API or completed signed-app distribution. |
| R2 / OCH-17 — candidate CI and preview delivery | Foundation `37862430983` at `83a9f398`: earlier point/popup/navigation repairs pass; fresh macOS extracted apps pass. Linux collector Clippy and the macOS document-cleanup snapshot fail. | [Platform condition and cleanup barrier fixes](evidence/preview-document-cleanup-och17.md) pass locally and await batched CI. Skipped consumer checks must pass on the repaired candidate. Both raw hosted timing probes agree on unavailable timestamps; no positive presentation claim. | Required preview CI/consumer checks pass; reviewed candidate tagged/published as a prerelease with source build instructions and known limits. No stable release claim. Complete all-zero hosted clocks may be unavailable; other failures remain errors. |

## Follow-up boundary and execution order

[OCH-164](https://linear.app/ochat/issue/OCH-164) carries broader real VoiceOver
qualification, additional catalog/OS permutations, stable-release hosted Metal
availability investigation, historical startup/chart diagnostics and actual
Developer ID/notarized reference-app distribution. Existing passes and failures
remain scoped evidence. A reproducible crash, data/input loss, unusable core
interaction or unbounded resource growth still blocks the preview.

C1/P1/P2/D1/R1 are closed for the preview. Next: C2/R2 final integration and
publication. Keep C2 tied to actual integration failures. Batch R2 integration throughout and publish
only after the seven preview closure items are satisfied. Do not wait for Apple
signing credentials or full screen-reader certification to publish the source
library preview; do not advertise those unqualified capabilities.

One primary task stays active at a time. Report **closed / active / next / external
prerequisite**, and update rows in place. Detailed artifacts stay in linked task
evidence. No new per-component performance matrix or automatic rerun of historical
checks is part of this plan. The earlier ~85% estimate predates this narrower
release target; do not mechanically convert it into a new percentage or deadline.
