# Selectable-list Core input API — OCH-41

2026-10-03, macOS arm64, milestone worktree based on `83eb87e`. The public Core
API and Bonsai-effect facade now connect the [native input layer](selectable-list-native-input-och41.md).
The state-owning managed component, Eio search and persistent-list gallery remain
unfinished. This checkpoint does not close the catalog or release ticket.

## Public contract

`List_input` defines typed relative and keyed navigation, selection, focus,
primary/secondary confirmation, context, desired selected setters and cancellation.
`View.with_list_input` attaches the callback/configuration to an existing virtual
list with ListBox semantics; row Option metadata remains explicit. Tree/table
input cannot share that owner. `Gpuio_bonsai.View` exposes the same operation for
effects. Native input remains asynchronous and selection remains application-owned.

`List_input.Config` refers to the logical cursor and sibling query by View keys,
never Rust node IDs. The query must be a direct single-line Input sibling and may
serve only one list. Either sibling order works. The reconciler resolves query
nodes after sibling mounting, including a reused list whose query changed. It
also validates late semantic edits and mounted cursor eligibility before submission.

The application supplies an epoch key representing source/query policy. The
reconciler owns the monotonic native generation. Cursor/busy echoes preserve it;
application epoch, query node/controller, disabled/navigation policy and ListBox
mode changes advance it. Clearing input preserves a generation watermark. A
rejected/discarded preparation cannot consume an accepted generation. Ordered
relative requests may use an older accepted revision; unknown row identities,
wrong window/handler, future revisions and stale generations are ignored.

The application reducer must also validate its captured application epoch. A
callback from the currently accepted native view may already be scheduled while
a newer candidate awaits acknowledgement. Updating the wire generation cannot
unschedule that callback. The driver regression exercises this boundary; the
managed adapter must retain the same protection for controllers and query work.

## Validation

All commands use the isolated toolchain, with two build jobs.

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j 2 test/view_api test/virtual_list test/runtime`:
  passed. Seven new Core tests cover all request forms, sibling order/reorder,
  physically reused owners, invalid/missing/duplicate queries, speculative rollback,
  cursor/busy versus policy epochs, clear/reinstall, removed/reintroduced targets,
  invalid metadata, a 100k logical order and window/handler/revision retirement.
- The production Bonsai window driver receives multiple native requests before a
  render and verifies ordered cursor movement, independent selection, secondary
  confirmation, cancellation preserving selection, source reset, idle and close.
- Six frozen public Core transaction fixtures replay through the actual Rust
  Session: mount with a late query, cursor/busy update, application epoch change,
  clear and remove query, reinstall without query, dispose. These complement the
  independent hand-authored codec fixtures from the bridge checkpoint.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-native --test list_input`:
  **eight passed**, including the exact public transaction replay and existing
  admission/registry/lifetime checks.

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest examples/gallery/main.exe`:
  passed all OCaml tests and gallery linking, including the additional regression
  that queues an old callback while reset awaits native acknowledgement.
- `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` and `git diff --check`: passed.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 -p gpuio-native --test list_input -- -D warnings`: passed.
- `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-selectable-core-gallery-20261003`:
  passed fresh staging and an independent gallery/backend build (`run=False`).
  An additional module in that consumer used installed `Gpuio.List_input.Config`
  and both Core/Bonsai `View.with_list_input` constructors; the copied main module
  referenced it. Rebuilding with the staged `OCAMLPATH` passed compilation/linking.
  This probe was not executed as a GUI application.

No OS windows were opened. These checks do not establish physical keyboard/IME,
VoiceOver or Linux desktop qualification. Previous native-library validation
remains applicable to unchanged production Rust; this chunk adds a native replay
test and changes the OCaml API/reconciler. The completed managed list still needs
its public gallery, installed-consumer and physical macOS evidence. Linux non-GUI
release gates remain required; OCH-47 desktop qualification stays deferred.
