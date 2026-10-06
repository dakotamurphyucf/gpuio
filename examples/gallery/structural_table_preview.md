# Fully mounted structural table walkthrough

Read [structural_table_preview.ml](structural_table_preview.ml) and its [interface](structural_table_preview.mli). [collections_page.ml](collections_page.ml) mounts it on **Collections**. `B` is Bonsai.Cont, `V` Gpuio_bonsai.View and `T` Table_view. Graph hosts reactive reversed/inspected state, initially false/“No result opened”; `let%arr` reads current values and palette to derive the table/UI.

Local cell/row/section helpers wrap validated [`Table_view`](../../lib/core/table_view.mli) constructors with stable keys/styles. Header has grouped Research workspace span2 plus Review, then three column headers. Body has Memory/Native interface/Streaming rows with Row_header name, status and an ordinary Open button. Footer spans summary across two columns plus count; caption is visible explanatory text, separate from explicit accessible name “Workspace review summary”.

Native Open on Memory executes set_inspected("Opened Memory"), updates the reactive model, and makes `let%arr` derive readout text for native reconciliation. Effect construction while mapping rows opens nothing. Native Reverse rows executes the captured boolean setter, reverses the row list on derivation, and updates native order without changing each row/cell key. It is a display-order demo, not sorting external data or navigating a real project.

T.create declares three equal-fraction tracks; every row’s spans must total exactly three. Cells accept spans1–64, and construction validates sibling keys/column count. Header/body/footer share semantic column indices; row indices include headers/footer. Stable section/row/cell keys preserve embedded native controls. This builder adds no table selection, keyboard controller, resource or extra Tab stop; Open buttons retain ordinary behavior.

Reverse rows and activate multiple Open controls to compare layout with application readout. All rows are mounted: public limits are 4,096 rows/16,384 cells plus normal node/byte budgets. Use managed Table/Bonsai.Table for large/remote data, not this composition. GPUIO owns native layout/focus, Bonsai owns the two demo values; there is no controller, asset scope or Eio worker. Adapt with domain-stable keys, explicit semantic header kinds and span validation, and replace mock inspection effects with real application actions.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not validation performed for this documentation change. There is no standalone executable or self-test for this component. Compilation does not establish native focus, keyboard, animation or platform acceptance. See [gallery instructions](README.md) and [development](../../docs/development.md).
