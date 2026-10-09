# Deterministic table payload cache

Read [page_cache.ml](page_cache.ml) and its [interface](page_cache.mli), then the
[workload driver](main.md). This is a pure immutable helper: it has no Bonsai graph, native
owner, I/O task or timer. `D` aliases `Gpuio.Table_data`.

`Row.t` contains row number and 64 deterministic Unicode cell strings; `Row.create` materializes
them, and accessors expose number/cell. `cell ~column` directly indexes the array, so callers
must supply 0–63; the driver derives that from its validated columns. Cache constants are 128
rows per page, at most four pages and 64 columns.

`create ~rows` accepts 1–100,000 and creates all stable logical IDs with None payloads. Thus
metadata remains O(rows) even before payload loading. `t` owns immutable Table_data, logical
count, resident page set and counters for loaded/peak rows, loads/evictions. Holding old
snapshots can retain old payloads; a current-snapshot bound is not a process-memory bound.

`prepare ~target` validates target, computes a contiguous up-to-four-page window around its page
with edge clamping, evicts removed pages to None first, then materializes added pages.
`Table_data.set` preserves source lineage and row references. Loaded rows are asserted at most
512; counters count pages added/removed, not rows or native cells. Returning Ok new cache does
not publish it to a view.

Concrete driver trace: worker computes `prepare cache ~target:n`; its UI-domain effect sets the
source Var to `data next`; Bonsai derives managed cells, and native reconciliation replaces
Loading with payload text. Later disjoint preparation evicts payloads without removing logical
row membership/selection identity. There is no native event callback or fetch service inside
this helper.

The inline expect test reviews disjoint windows/partial final page, source/row-reference
preservation, counters, invalid targets and a one-row dataset. Its expected outputs include peak
512 and preserved 1,030 logical rows; it was read, not executed for this change. Use
[README commands](README.md) to run the repository tests.

Adapt with validated target/column bounds and domain-stable IDs. This deterministic cache is
neither remote I/O benchmarking nor the boundary-append Table_paging adapter. Put real fetch
cancellation/error policy outside it and measure process memory separately from these payload
counters.
