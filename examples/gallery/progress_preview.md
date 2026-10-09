# Progress with an editable center walkthrough

Read [progress_preview.ml](progress_preview.ml) and its [interface](progress_preview.mli). [feedback_page.ml](feedback_page.ml) mounts it on **Feedback**, as “Progress with a living center”.

`B.state (Some 0.35)` owns a reactive optional fraction: None means indeterminate. Toggles own animated transitions (true) and inert state (false). A single native editor is created outside view derivation with seed “Keep typing”. `let%arr` reads these inputs to derive two progress configurations and the editor placement. Constructing `set_fraction value` does not execute it. Native Quarter activation executes the setter, updates fraction to Some 0.25, derives new bar/circle configurations and percentage text, and reconciles them with the retained center editor.

[`Progress.Value.determinate`](../../lib/core/progress.mli) requires a finite fraction in [0,1], rejecting rather than clamping invalid input. None maps to Value.indeterminate. `Progress.Config.create` supplies separate meaningful circular/bar labels. Animated changes use a 600ms native tween; otherwise Transition.immediate. Tween durations must be positive and no more than 60 seconds, and native interpolation clamps overshooting fractions. This file supplies discrete mock values, not transfer updates or completed work.

`V.progress_circle` has 160-pixel bounds and ordinary child content: a 112-pixel column containing percentage/“Working…” and Editor.view. The rounded bar is 240 pixels wide in its parent, height 16 and radius 8. Progress changes do not replace these surviving children. Both progress roots receive `Inert inert`: this is a subtree interaction policy, not a request to pause a job or alter the progress model. Turning it on makes the editor inside the circle inert too.

Type in the center, select Tiny/Complete/Unknown and compare determinate versus indeterminate presentation without replacing the draft. Disable animation to apply changes immediately; toggle inert to exercise descendant input suppression. GPUIO owns drawing, interpolation and native editor state; Bonsai owns fraction/options and the adapter owns the editor lease. No asset registration, timer or transfer worker is started. Adapt by deriving validated fractions from real work, keeping semantic completion separate from animation, and storing drafts externally if they must survive page destruction.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

Use the repository wrapper for the isolated toolchain. These commands were not executed for this documentation change. There is no standalone executable or self-test for this component. Compilation alone does not establish native keyboard, animation, focus or platform acceptance. See [gallery instructions](README.md) and [development](../../docs/development.md).
