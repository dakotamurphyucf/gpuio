# A validated inclusive score interval

[score_range.ml](score_range.ml) and [score_range.mli](score_range.mli) define a
small pure application value for inclusive integer percentages. The interface
hides the record so callers cannot bypass validation. There is no Bonsai graph,
native widget, Eio task or I/O in this module.

From the repository root with [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open Settings → Generation, change the Result score interval and Apply score
interval, then inspect Results. Findings are simulated; no provider query occurs.
macOS is the v1 target; Linux GUI checks remain
[informational](../../../docs/platform-release-policy.md).

Read `create`, `all`, the accessors, `contains` and `describe`. `create ~lower
~upper` accepts only 0 ≤ lower ≤ upper ≤ 100, returning `Or_error` otherwise.
The implementation's three checks (lower below zero, upper above 100, reversed
endpoints) imply all bounds, including rejecting a negative upper or a lower
above 100. `all` is the valid constant 0–100. `contains t score` uses inclusive
comparisons at both ends; it does not clamp a supplied score. `describe` produces
labels such as 20–80%. Typed equality and S-expression printing are derived;
there is no generated decoder that silently admits invalid records.

In [settings.ml](settings.ml), the native range slider uses a Numeric.Domain
0–100 step 1, separate Minimum result score/Maximum result score labels and
initial values from these accessors. Drag_started/Preview snapshots populate
`score_preview`; a Committed snapshot calls `create` and stores the accepted
range, clearing preview. Cancelled clears preview without changing accepted
bounds. This caller relies on the validated integer native domain when using
`Float.to_int`; arbitrary numeric input would need validation before conversion.
Bonsai derives Ready interval/Preview labels from the current settings model.

Apply score interval is disabled while a preview is active. Its deferred effect
reads the latest epoch-guarded settings model, calls
[Results.filter_scores](results.md), then updates notice. `Result_data.Query`
stores `Filter.Between range`, and [Result_data.rows](result_data.md) uses
`contains` before complete-query sorting/paging. Thus selecting 20–80 includes
scores exactly 20/80, updates the entire accepted query and leaves native row
mounting as a separate table concern. Slider motion itself never filters on every
frame; committing bounds and applying them are distinct actions.

For a small adaptation, add a preset created with `create ~lower:80 ~upper:100`
and apply it through Results.filter_scores. Keep score units integer percentages
and inclusive semantics in labels; do not reuse this type for probabilities
0–1 without an explicit conversion. If importing stored ranges, validate through
`create` rather than exposing the record. Public
[slider](../../../lib/core/slider.mli) and
[numeric domain](../../../lib/core/numeric.mli) contracts explain native bounds;
[README verification](../README.md) separates existing native settings checks
from this source review, which performs no GUI run.
