# OCH-39 acceptance audit

Local implementation audited at `1fbcd37` on 2026-09-26 against the live OCH-39
scope. The paired capability addition follows this audit. This is **local macOS
acceptance**, not ticket/milestone closure: required hosted macOS/Linux build/unit
validation and merge remain pending. Full Linux GUI validation remains OCH-17.
The [evidence ledger](data-tables-och39.md) records exact commands, measurements,
coverage limits and the development failures that led to the final implementation.

| Live requirement | Implementation and inspected evidence | Result |
| --- | --- | --- |
| Evaluate and compile the actual virtualized styled table against the GPUI pin | [Adapter provenance](../../rust/table/UPSTREAM.md), `rust/table/Cargo.toml`, `scripts/probe_table_adapter.py`; candidate compile/layout probe followed by the extracted native adapter. Existing GPUI pin unchanged; full styled dependency is not linked or claimed compatible. | Local pass; hosted Linux pending |
| Typed stable row/column identity and lifetime | `Table_data.Row_ref`, `Table_column.Id`, Core and Bonsai tests in `test/view_api/table_data_test.ml`, `test/view_api/table_column_test.ml`, `test/virtual_list/table_component_test.ml`; removal/reinsertion, fresh source/query and stale controller tests | Pass |
| Width/min/max, pinning, reorder, sorting, grouped headers | `Table_column.Collection`, Rust schema validation; 4,160 valid/invalid 64-column moves, header membership/partition checks, `rust/table/tests/support/events.rs` and `anchors.rs`; native resize/reorder/sort, pinned horizontal sweep and keyed anchors | Pass |
| Row/cell selection, keyboard, copy, context | `rust/native/src/table_host_test/input.rs`, `appkit_input.rs`, `examples/table/event_actions.ml`, `scripts/test_table_appkit.py`; exact Unicode cell/quoted TSV clipboard, incomplete-copy preservation, OS arrows/Return/Shift-F10, independent right-click target, guarded actions and focus restoration | Pass on macOS |
| Revisioned in-memory and paged sources, bounded cells | Core `Table_data`/`Table_paging`, Bonsai `Table`, Eio `Table_paging`; deterministic expect tests and Table Lab's real producer/renderer integration | Pass |
| No synchronous OCaml render/format/sort callbacks | `rust/native/src/table_view.rs` delegate reads retained native trees/copy text, queues versioned `TableInput`; native demand observation drives later Bonsai materialization. Rust owns entities/paint/input; no host-language callback stored in the delegate. | Verified implementation |
| Application sort/filter ownership and optimistic native selection | Native sort only emits a request; accepted source/config updates preserve stable targets. Table Lab sorts its fully loaded data and resets the Eio query. Remote partial data is explicitly not locally sorted as a complete result. | Pass |
| Obsolete results, bounded work and stable anchors | `test/runtime/table_paging_test.ml`: two reusable workers, 100 query resets, cancellation cleanup and inbox backpressure; public pending-page resize/reorder and held obsolete producer during sort. Native data arrivals during active column gestures preserve previews. | Pass |
| At least 100k logical rows, full history/cache and horizontal/vertical measurements | Native history visits all 100,000 rows twice, 128 active rows/512 cells; all four columns swept, pinned column fixed, zero retired payloads after batch/unmount/window checks. Peak accounting delta 442,368 bytes; peak RSS 183,451,648 bytes; 517 seconds. Separate Bonsai two-pass 100k test releases cell models. | Pass; debug workload, not release latency benchmark |
| Explicit column and other bounds | 64 columns, four header levels, 256 KiB schema, 1 MiB wire batches/clipboard, at most 16,384 active cells; every active row's columns count toward admission even when horizontal paint is culled. No full two-axis materialization claim. | Validated/documented |
| Removed selection, Unicode, focus, empty/error and teardown | Native adapter/host tests; public selected-row removal, empty source, controlled failure/retry, final cell deactivation and all five Eio producers finished; embedded editor composition/grapheme deletion and disabled/hidden ancestry | Pass |
| Native semantics and stale queued actions | Actual AppKit counts/indices/values/selection/focus and sort actions, bounded accessibility tree after 50k jump; hidden/disabled old objects inert. Context row/query/dialog identity guards reject old callbacks, including same-row reopen. | Pass; no VoiceOver speech claim |
| Paired protocol, malformed inputs and public example | Independent `.hex` fixtures and Rust/OCaml codecs in `rust/protocol/tests/table.rs` and `test/view_api/table_wire_test.ml`; truncation, wrong identities, oversized schemas and invalid batches; public Table Lab | Pass |
| Dependency changes isolated/reproducible | Exact-version AccessKit table semantics patch and Taffy context-retirement patch, archive/license hashes and reconstruction evidence; default and generated backend paths verified; independent staged extension consumer passed | Pass locally |
| Build/test/format and platform gates | Local full workspace/Dune/Clippy/shared native checkpoints plus final focused AppKit/context regression, 225 native unit tests and public checks; required `.github/workflows/foundation.yml` includes table adapter/host/history/public AppKit on macOS and compilation/unit/lint on Linux | Local pass; hosted M5 run pending |

The remaining milestone integration is the complete OCH-46 chat showcase, whose
[coverage plan](../design/agent-chat-m5-showcase.md) remains binding. Separate
component evidence does not establish that integrated application or its combined
latency/retention/visual acceptance. Keep OCH-39 In Progress until required hosted
gates and the accepted delivery workflow are complete.
