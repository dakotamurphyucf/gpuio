# Bar-background data: stable batches and resolved brushes

This pure OCaml fixture supplies the gallery's **Charts & data → Bar backgrounds**
preview. It creates 24 categorical bars whose source-owned colors follow batch
identity through value updates and reversal. Read this before the
[Bonsai preview walkthrough](chart_backgrounds.md); no Rust knowledge is needed.

Build and run the owning application with the repository's isolated toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

See [development prerequisites](../../docs/development.md). These commands are
ordinary launch, not an acceptance test. This fixture needs no assets, credentials
or external data. macOS is the release target; Linux desktop qualification remains
separate from required build/unit/consumer checks. This guide adds no native
input, accessibility or pixel acceptance.

## Read the model, then the constructors

[chart_backgrounds_data.mli](chart_backgrounds_data.mli) exposes an abstract `t`
and pure operations. [chart_backgrounds_data.ml](chart_backgrounds_data.ml) defines
the record: `phase`, `reordered` and `patterns`. `initial` starts at phase zero,
original order and solid fills. `advance` increments modulo 13; `reorder` and
`toggle_patterns` return updated records. `[@@deriving equal]` generates typed
equality. None of these functions mutates state, starts Eio work or constructs a
Bonsai graph; the caller owns storage and publication.

`data_exn t ~theme` builds a validated immutable `Chart_data.t`. Read its steps
in this order:

1. `indices` names the original batches, zero through 23. `order` reverses them
   when requested. Both categories and points use that display order.
2. `category_id index` constructs a typed category ID from `101 + index`;
   categories have labels Batch 01 through Batch 24. Labels are presentation,
   while IDs express identity. `datum_id index` uses `240 - index * 7`, so datum
   IDs are descending and noncontiguous. `series_id` is the typed ID 7;
   `first_id` always names Batch 01, even at the opposite end after reversal.
3. Each `Categorical_point.create` connects a datum to its category and a present
   value `30 + 3 * phase + (index * 11 % 47)`. A `Categorical_series` named
   Evaluations becomes the single `Bar` layer passed to `D.categorical`.
4. Background descriptors are deliberately built from original `indices`, not
   display `order`. `D.Bar_background.create ~series ~datum` associates a brush
   by typed identity. `D.with_bar_backgrounds` validates references, canonicalizes
   descriptors and resolves color tokens against the supplied `Theme.t`.

`background t index` cycles accent, muted and foreground tokens. In solid mode it
uses `Background.solid`. Pattern mode retains a solid brush for remainder zero,
uses slash lines of width 2 and interval 4 for remainder one, and a checkerboard
of size 5 for remainder two. Transparent pattern gaps reveal the chart surface.
The original index chooses the brush, so reversal changes position without
assigning another batch's color.

The helper `ok = Or_error.ok_exn` unwraps constructors whose fixed fixture inputs
are expected to be valid. The public interface documents that `data_exn` can raise
when the supplied theme lacks the required tokens. It is demonstration data,
not an input-validation strategy for untrusted records. See the public
[data interface](../../lib/core/chart_data.mli) and
[background interface](../../lib/core/background.mli) for resource bounds and
recoverable constructor errors.

## One update through the caller

When **Reorder batches** runs, the preview reads its current `t`, calls `reorder`,
and supplies the result to `data_exn` with the last applied source theme. Categories
and points reverse together; Batch 01 keeps its datum/category IDs and value.
Background descriptors still arrive in original order and resolve to those IDs.
The preview submits the resulting dataset, then stores the new model after local
admission succeeds. Native publication and presentation happen later; this pure
module cannot acknowledge either.

To add a batch, change the `List.init 24` bound and review the ID formula, labels
and fixture bounds. The demonstration's `240 - index * 7` becomes negative at
index 35, so replace that formula before growing beyond 35 batches.
Preserve unique typed IDs and derive backgrounds from original
identity rather than the current display position. To accept missing values, use
`None` deliberately and review selection descriptions and aggregation behavior.
To change colors, change the tokens in `background` and provide a theme that
defines them. Colors resolve when constructing data; merely changing a chart's
view theme does not rewrite the stored brushes.
