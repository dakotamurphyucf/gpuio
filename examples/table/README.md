# Table Lab

A public OCaml/Bonsai/Eio example with 100,000 agent-event rows, a pinned identity
column, horizontal overflow, native resize/reorder/sort requests, and bounded
transient cell computations. The application owns accepted column descriptions
and sort order. The example sorts its fully loaded in-memory dataset; a remote
application should send the new sort as its pager query instead.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/table/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec dune exec -j2 examples/table/main.exe
```

Create the Eio pager in the window/application factory, before graph evaluation.
Inside the graph, bind it to the table:

```ocaml
Gpuio_bonsai.Table.paged
  (Gpuio_eio.Table_paging.value pager)
  ~paging:(B.return (Gpuio_eio.Table_paging.controls pager))
  ~config
  ~render_cell:(fun ~row:_ ~data ~column ~lifetime:_ _graph ->
    B.map2 data column ~f:(fun text column ->
      Gpuio_bonsai.Table.Cell.text ~column:(Gpuio.Table_column.id column) text))
  graph
```

The snippet uses string row payloads; the runnable example formats a typed event
record. A custom cell can provide an ordinary View plus independent copy text via
`Table.Cell.create`. Guard delayed cell effects with its `Managed_rows.Lifetime`,
and keep persistent preferences and I/O jobs outside cell computations.

Capture a row membership with `Table.Output.target`, then use the output's
controller to select/reveal/scroll. A batch is the latest pending command intent:
put selection and reveal in the same batch when both should execute. A later
native selection supersedes an older pending batch. Same-source query resets
retire old effects and cell models while preserving surviving native anchors;
a fresh `Table_data` lineage resets the widget.

The bounded native integration test opens and closes one window:

```sh
GPUIO_JOBS=2 python3 scripts/test_table_public.py
```

`--background` is available for environments where background frame delivery is
reliable. The default permits activation so an occluded window cannot prevent the
frame checks. The test requires a completion marker and terminates/reaps the
process group on timeout. It covers public retained cells, keyed commands and
anchors, query retirement, streaming, selected-row removal, paging failure/retry,
and window-scoped producer/cell cleanup. It does not establish physical keyboard,
clipboard, IME, accessibility, or Linux GUI acceptance.

See [table contracts](../../docs/design/data-tables.md) and
[validation evidence](../../docs/evidence/data-tables-och39.md). The remaining
native acceptance and polished chat showcase are tracked separately; OCH-39 is
still in progress and no table capability bit is advertised yet.
