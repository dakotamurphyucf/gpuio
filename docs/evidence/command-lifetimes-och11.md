# OCH-11 shared command entry lifetimes

Native command routes now share each immutable retained registry entry. Previously
each Route construction allocated another CommandConfig and copied its label and
shortcut buffers, including during palette refresh and native menu construction.
The retained registry now owns an Arc per entry, and routes clone that Arc.
The retained-tree budget includes the added per-entry pointer and reference-count
metadata. This reduces route payload duplication; it is not a process RSS bound
or a benchmark of total frame allocation.

The wire and shared native snapshots use the same registry validator, preserving
entry count, text size, generation, unique ID and shortcut validation. There is no
wire format or public OCaml API change. Tree lookup still selects the nearest
registry, and invocation checks current state before delivering an action.

Local macOS evidence:

- The deterministic `routes_share_entries_and_release_obsolete_generations` test
  creates 1,024 routes referencing one long-label entry. They share the retained
  payload. After registry replacement, the old route cannot invoke the command;
  the new route can. The old entry survives only while an old route holds it.
  Unmount invalidates the new route, and dropping it releases the last entry.
- Full Rust workspace tests pass, including registry shadowing, atomic validation,
  invocation guards and retained-payload budget tests.
- The `native_controls` production-window suite passes command shortcuts, native
  menu/palette behavior, current command updates, unmount, focus and accessibility
  checks. The `native_image_views` suite also passes icon-button command labels,
  activation and disposal. Input is synthetic GPUI dispatch in real windows.
- Feature-enabled all-target native Clippy passes with warnings denied.
- Full Dune build, OCaml expect tests and formatting checks pass after relinking
  the updated native library.

Commands:

```sh
./scripts/gpuio exec cargo test --locked --workspace
./scripts/gpuio exec cargo test --locked -p gpuio-native --features native-image-tests --test native_controls --test native_image_views --no-run
./scripts/gpuio exec cargo clippy --locked -p gpuio-native --features native-image-tests --all-targets -- -D warnings
./scripts/gpuio exec dune build @all @runtest @fmt
```

The reported native binaries run sequentially under 90-second subprocess timeouts.
Logs are `command-lifetime-*.log` in the implementing agent's ignored scratch
directory. This is local evidence; consolidated hosted CI and merge remain pending.
