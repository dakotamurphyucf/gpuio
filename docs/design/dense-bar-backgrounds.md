# Dense bar backgrounds

OCH-41, 2026-10-06. The typed source API, paired codecs and native rendering are
implemented. The [public gallery and installed consumer](../evidence/dense-background-gallery-och41.md)
now pass local interaction and source-color checks; this does not establish catalog
or release completion. Chart data now uses
schema **2**, with matching OCaml/native packages required.
[Local foundation evidence](../evidence/dense-background-foundation-och41.md)
records codec, resource, native GPU and integration checks.

## Why this belongs to the source

The pinned [BarChart fill accessor](../catalog/sources/component-chart-bar_chart.rs.txt)
can choose a background for every observation. GPUIO's sparse
`Chart_appearance.Datum` overrides intentionally stop at 1,024 pairs; a dataset
can contain 100,000 observations. Increasing the style envelope would make large
data-dependent styling travel with view updates and would blur resource ownership.

Store a bounded, immutable background sidecar in `Chart_data.t` instead. Its
stable `(series, datum)` keys refer to that exact dataset. Publishing replacement
data replaces the sidecar atomically with the observations. Existing source
registration, cancellation, stale-publication rejection and snapshot leases own
both. No second native resource or synchronous callback is needed.

Keep existing point records unchanged. A brush field on every point would charge
line/area sources for a capability they do not use. An empty sidecar adds only
dataset-level storage. Dense styling still consumes memory and transfer time;
it is not free merely because it is separate from view metadata.

## OCaml interface

The interface was drafted before implementation and is exposed by
[Chart_data](../../lib/core/chart_data.mli).

```ocaml
module Bar_background : sig
  type t [@@deriving equal, sexp_of]

  val create
    :  series:Series_id.t
    -> datum:Datum_id.t
    -> Background.t
    -> t
end

val with_bar_backgrounds
  :  t
  -> ?theme:Theme.t
  -> Bar_background.t list
  -> t Or_error.t
```

The operation replaces the entire sidecar; `[]` removes it. Ordinary dataset
constructors produce an empty sidecar. Validate before returning the new value:

- At most 100,000 entries and one entry per `(series, datum)` pair.
- Every pair exists in a numeric or categorical **bar** layer in this dataset.
  A categorical missing observation may carry a brush for identity consistency,
  but does not paint or participate in aggregate appearance comparison.
- Every color resolves against the supplied theme (default `Theme.default`),
  including currently unused colors. Unknown tokens return an error.
- The full encoded dataset, including backgrounds, fits the existing 16 MiB
  envelope. The independent point, series and 8 MiB text limits remain.

Use a deterministic key order and reject duplicates before admission. User list
order is not visual order; source order remains untouched. Original values,
labels, IDs and selection spans remain unchanged. The source browser continues
to expose original observations, not brush metadata as fabricated data values.

Resolve colors once when constructing the decorated value. It does not retain a
live theme. To change resolved colors, call the operation again with the original
background descriptions and new theme, then publish the resulting dataset.
Applying an unrelated view theme does not secretly rewrite an immutable source.

`Chart_appearance` already imports `Chart_data` identities, so `Chart_data` must
not import that Core module. An internal `Chart_brush` resolver depends only on
`Background`, `Theme` and the protocol brush representation. The current
appearance and source implementations share exactly that resolution without introducing a module cycle or a new public low-level API.

## Appearance and aggregation

For each defined source observation, resolve fields in this order:

1. Existing chart defaults, then the series appearance.
2. Data-owned background, replacing only the fill.
3. Sparse datum appearance, overriding only its explicitly supplied fields.

For example, a sparse corner override keeps the dense fill; a sparse fill wins
over the dense fill. No background means inheritance, not transparent paint.
A transparent background is an explicit brush and remains distinct from absence.

Preserve the current `Chart_appearance.Aggregates` contract:

- A bar representing one defined observation uses that observation's effective
  appearance. This also applies to a one-observation Sum/Mean aggregate; styling
  does not turn its selection identity into Exact.
- For multiple defined observations, `Inherit_series` uses the series appearance
  and ignores both dense backgrounds and sparse datum overrides.
- `Uniform` uses the effective appearance only if every defined participant has
  the same complete bar appearance; otherwise it uses the series appearance.
  Gaps do not participate. Compare resolved brush values and corners, not object
  addresses, raw override records, or whether a sidecar entry happens to exist.

Use the source span already carried by the prepared mark. Sum/Mean do not blend
colors, arbitrarily choose the first brush, or rewrite source values. A stack's
bar contribution follows the same source-participant rule for its own series.

Native `Index::bar` only takes its inherited-style shortcut when both the sparse
datum map and dense sidecar are empty. Sources using only the new API therefore
receive their data-owned fills.

## Wire, worker and memory requirements

The owned data record appends a bounded list of `{ series; datum; brush }`,
advancing the chart **data** schema to 2. View/options/style remain -2/9/-9.
Historical schema-1 fixtures are preserved; schema-2 data, categorical and
background fixtures are checked independently from OCaml and Rust. Old payloads
fail explicitly, rather than defaulting the absent sidecar to empty.

The sidecar is a sorted vector on Rust's side. Binary search resolves a stable
pair without a second persistent 100,000-entry tree. Validation must establish
strict pair ordering, valid brushes and membership in bar layers. Avoid scanning
an entire source separately for every sidecar entry: validation must remain
bounded by one source traversal plus sorting/index lookup, not quadratic work.
Native appearance preparation checks cancellation during source-span traversal.
Source decoding retains the existing bounded-worker contract: cancellation is
checked before and after decoding/validation, not between individual decoded
records. A cancelled completion cannot publish; its memory remains charged until
it finishes or drops. Malformed counts fail before allocating the advertised list.

All limits apply together; a dataset with maximum labels and maximum-size
brushes need not fit even if each separate count is legal. Compute encoded size
before creating the upload buffer. Charge the retained sidecar in OCaml's
`Chart_data.Expert.retained_bytes` and the 128 MiB OCaml registry budget, native
`chart_store` snapshot accounting and the conservative decode-workspace reservation. Verify temporary membership
indexes and sorting storage against the existing 64 MiB decode workspace and
256 MiB total source reservation. Do not increase either bound without evidence.

Standalone validity does not reserve room for multiple retained revisions. In
particular, a maximum-size dense source may fit initial registration while its
replacement plus the retained old source exceeds the OCaml registry budget.
`set` then returns `Resource_limit` locally and preserves the prior desired data.
Applications must handle that result; they cannot assume every individually valid
100,000-value dataset can coexist with another. The conservative registration
charge includes 512 bytes per background, in addition to existing source charges.

Worker paint uses brushes from the exact retained source publication. Prepared
quads already retain their resolved fills; avoid cloning all sidecar data into
each plan or rebuilding an index on every frame. Old snapshots remain charged
until their last reader drops them, including during replacement and window
close. The new field must not create an uncharged parallel ownership path.

## Qualification before catalog acceptance

- OCaml expects and paired Rust codecs: empty/solid/gradient/slash/checker,
  canonical order, duplicate/unknown/nonbar keys, theme errors, invalid brushes,
  malformed/truncated data, old schema and combined byte limit rejection.
- A genuine 100,000-observation dense resource, including IDs that differ from
  source order, verifies construction, transfer, admitted memory and teardown.
  Empty-sidecar sources must retain their prior point representation.
- Native preparation checks the precedence rules, all aggregate policies,
  single-defined-observation aggregates, gaps, stacks and cancellation. Raw
  values, source spans and publication identities remain unchanged.
- Actual GPU pixels verify distinct brushes across bars, sparse overrides,
  all four value directions and relevant scales. Unit output alone does not
  establish rendered appearance.
- A modular public OCaml gallery example and adjacent beginner walkthrough run
  from both the root and a fresh installed consumer, exercise update/reorder,
  theme republishing, original-data inspection and source cleanup.
- Linux build/unit/consumer checks remain required; full Linux desktop
  qualification remains OCH-47. This design adds no platform waiver.

## Scope still separate

The sidecar supplies precomputed backgrounds, not arbitrary callbacks
with access to current native pixel bounds. Existing declarative signed/domain/
value gradients cover some geometry-aware cases; the remaining difference from
the pinned fill closure must stay explicit in the catalog.

Per-bar baselines are separate work. The pinned low-level Bar has independent
base and endpoint accessors. An interval API must preserve both values in the
source browser and define Sum/Mean/stacking semantics; adding origins together
or dropping differing bases silently would be incorrect. The area scalar
baseline and this background sidecar do not close that requirement.
