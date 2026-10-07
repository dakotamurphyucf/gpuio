# Bar-background data: stable batches and resolved brushes

This pure OCaml fixture supplies the gallery's **Charts & data → Bar backgrounds**
preview. It creates 24 categorical bars whose source-owned colors and optional
data-unit baselines follow batch identity through value updates and reversal.
Read this before the
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
the record: `phase`, `reordered`, `patterns` and `baselines`. `Baselines.t` is a
variant with three alternatives: `Zero`, `Shared` and `Individual`. `all` lists
the choices in button order and `label` supplies their visible text. `initial`
starts at phase zero, original order, solid fills and `Baselines.Zero`.
`advance` increments modulo 13; `reorder`, `toggle_patterns` and receiver-first
`with_baselines t choice` return updated records, preserving the other fields.
`baselines t` reads the choice. `[@@deriving equal]` generates typed
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
5. Baseline descriptors also use original `indices`. `List.filter_map` keeps
   only the `Some` results returned for those indices; `Option.map` converts
   each present number into a validated descriptor. `Zero` returns `None` for
   every index, supplying an empty list: the default zero baseline without
   explicitly stored entries.
   `Shared` supplies 40 for every batch. `Individual` uses
   `20 + 20 * (index % 3)`, cycling 20, 40 and 60 by original batch index.
   `D.Bar_baseline.create ~series ~datum base` validates each data-unit value;
   `D.with_bar_baselines data baselines` attaches the complete sidecar to the
   already colored source. An empty list clears baselines while keeping brushes.

An endpoint is still the formula in step 3: selecting a baseline does not subtract
it from the stored value or change any ID, label or ordering. At phase zero, Batch
01 keeps endpoint 30; its interval is 0→30 in Zero mode, 40→30 in Shared mode and
20→30 in Individual mode. Reversal preserves each batch's baseline because the
descriptor uses its original ID. Native preparation projects those intervals;
this fixture creates no pixel coordinates. See the
[baseline contract](../../docs/design/bar-baselines.md) for validation and
aggregation rules.

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
Baseline descriptors do too: Batch 01 retains 20 in Individual mode regardless
of its new screen position. Choosing **Shared baseline 40** calls
`with_baselines` through the same publication path, keeping values, brushes and
order while replacing the baseline sidecar.
The preview submits the resulting dataset, then stores the new model after local
admission succeeds. Native publication and presentation happen later; this pure
module cannot acknowledge either.

To add a batch, change the `List.init 24` bound and review the ID formula, labels
and fixture bounds. The demonstration's `240 - index * 7` becomes negative at
index 35, so replace that formula before growing beyond 35 batches.
Preserve unique typed IDs and derive backgrounds from original
identity rather than the current display position. To accept missing values, use
`None` deliberately and review selection descriptions and aggregation behavior.
Baselines for missing observations remain source metadata even though no bar is
drawn; defined observations with different baselines in one Mean bucket are
incompatible. Keep that error recoverable in an application using external data.
To change colors, change the tokens in `background` and provide a theme that
defines them. Colors resolve when constructing data; merely changing a chart's
view theme does not rewrite the stored brushes.
