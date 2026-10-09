# Per-observation bar baselines

OCH-41, 2026-10-06. Interface and semantics drafted before implementation.
The source API and native implementation have [local qualification](../evidence/bar-baselines-och41.md),
including paired codecs, native unit/GPU checks and root/fresh-installed gallery
walkthroughs. This is not whole-catalog or release acceptance.

The pinned low-level Bar shape has independent pixel-space `base` and `value`
accessors. GPUIO exposes their data-space equivalent: existing numeric or
categorical bar values remain endpoints, with optional immutable source-owned
baselines keyed by series/datum IDs. Native preparation owns projection. No
layout callback enters OCaml, and an appearance change cannot silently change
the meaning of an interval.

## Public source interface

```ocaml
module Bar_baseline : sig
  type t [@@deriving equal, sexp_of]
  val create
    : series:Series_id.t
    -> datum:Datum_id.t
    -> float
    -> t Or_error.t
end

val with_bar_baselines : t -> Bar_baseline.t list -> t Or_error.t
val bar_baseline : t -> series:Series_id.t -> datum:Datum_id.t -> float option
```

Baselines are finite data-unit values within ±1e100. At most 100,000 unique live
bar pairs are accepted, including missing categorical observations. Constructors
canonicalize by identity; decoders reject unordered/duplicate/unknown/nonbar
references. `with_bar_baselines` replaces the entire baseline sidecar; an empty
list clears it. Omission means zero for rendering. `bar_baseline` returns an
explicitly stored baseline, or None when absent; it does not validate a caller's
pair or distinguish an omitted live pair from an unknown pair. The OCaml lookup
is linear in sidecar length and is intended for occasional original-data
inspection, not a render loop. Native workers use binary search.

Source endpoints, labels, order and selection IDs remain unchanged. Baselines
and backgrounds are independent sidecars and coexist in the same publication.
Both count toward the unchanged 16 MiB encoded-source limit and retained-memory
budgets. Missing endpoints produce no mark; their baseline remains available in
the original-data browser. Clearing either sidecar preserves the other.

## Reduction and stacking

Exact bars run from the effective baseline to the original endpoint, in any of
the four orientations. Endpoints above, below or equal to the baseline are valid.
An exact endpoint is never reconstructed through subtraction/addition, which
could destroy precision at very different magnitudes.

Sum/Mean remain explicit sampling policies. Within each bucket, all defined
observations must share an effective baseline (including omitted zero). Missing
observations do not participate. Sum combines contributions relative to that
baseline and adds it once; Mean averages original endpoints. For baseline 20
and endpoints 25 and 27, Sum ends at 32 and Mean at 26. A single defined value
retains its original endpoint even under an aggregate policy; it still carries
aggregate provenance. Differing baselines within a bucket fail explicitly with
Invalid_config; no origins are summed and no arbitrary first baseline is chosen.

Stacking keeps existing aligned positions and natural series order. At each
populated position/bucket, participating series must share a baseline. Each bar
contributes endpoint minus baseline, starting the cumulative stack at the common
baseline once. Thus 20→25 and 20→27 occupy 20→25 and 25→32. Signed contributions
retain natural-order cumulative behavior. Missing series do not contribute or
constrain the baseline. A baseline may differ between distinct exact positions.
Stacking after aggregation applies the same rule to reduced endpoints; incompatible
bases fail explicitly. Line and area behavior is unchanged.

Geometry/domain, hit testing and base-to-tip gradients must use actual interval
bounds. Exact inspection reports source baseline and endpoint; aggregate
inspection reports the common baseline, reduced endpoint and source span;
stacked inspection additionally reports cumulative bounds. The original-data
browser always exposes stored baseline and original endpoint, including missing
values. Source selection remains publication-bound and ID-based.

## Encoding, ownership and qualification

The paired codecs append bounded `(series, datum, baseline)` records to data schema 3; retain old
fixtures and explicitly reject data schema 2. Matching native/OCaml packages are
required. Decode count bounds apply before allocation, scalar/reference checks
before publication, and combined size limits to the entire source. Recompute
decode-workspace and OCaml/native retained-snapshot charges without relaxing
existing limits. No new asynchronous owner or lifetime is introduced.

Required evidence: independent paired bytes and malformed/truncated inputs;
canonical/invalid/clear/coexistence cases; numeric/categorical/gaps; 100k bounded
roundtrip and retained-source replacement accounting; precise exact values;
Sum/Mean/shared/mismatched and signed stacks; domain/provenance/inspection/browser;
all four orientations and native GPU filled/empty pixel controls; public gallery
and fresh installed-consumer updates/reorder/original-data/cleanup. Existing
zero-baseline behavior must remain covered. Linux compilation/unit/consumer checks
remain required; actual Linux desktop qualification stays deferred under OCH-47.
