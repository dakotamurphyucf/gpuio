# Managed selectable-list component — OCH-41

2026-10-03, macOS arm64, milestone worktree based on `83eb87e`. The public
`Gpuio_bonsai.Selectable_list` now owns selection and controller lifetimes over
the native selectable-list bridge. Eio search, the persistent-list gallery and
physical macOS qualification remain unfinished. This does not close OCH-41 or
the milestone release gates.

## Ownership and behavior

`List_collection.Source_id` exposes metadata-only source identity, including empty
sources. Value edits, reorder and splice preserve it; a separately constructed
collection has a distinct identity. `List_rows` projects visible payloads while
retaining original membership references. Filtering preserves hidden application
records and selected preferences. Removed/reinserted keys get new row identities,
even if the intermediate deletion never rendered.

`List_rows.Layout` computes logical option positions/counts, excluding explicitly
nonselectable section decorations. Reuse this metadata across streamed payload
updates. The incremental projection visits changed values through persistent-map
sharing; a hidden value update leaves the projected collection unchanged.
Selection/cursor metadata is computed only for mounted rows.

`Virtual_list.Input` adds checked query/header/footer slots around the existing
native list owner, inside its existing parent. Moving the query between slots
preserves the owner, scroll handles and surviving row computations. All four
ordinary/reactive/paged constructors expose this optional input adapter.

`Selectable_list` scopes its state to a source lifetime. Initial selection seeds
once, subsequent filtering reconciles preferences, and ordered native requests
reduce against the latest catalog. Controllers retain no payload snapshot and
reject obsolete membership targets. Replacing/unmounting a source permanently
retires its controller, including when the original source returns later.
Confirmation, context and cancellation are distinct application intents;
cancellation preserves committed selection.

Native callbacks additionally carry query/policy metadata. The reducer rejects
obsolete callbacks while a new native frame is still awaiting acknowledgement.
An accepted policy checkpoint advances the native epoch for query, editor,
mode, boundary and navigation/disabled changes. Cursor/busy changes preserve
ordered input. Programmatic commands use the current catalog and policy.

The component provides ListBox/Option semantics and themed row presentation,
with custom row content/style and ordinary before/after slots. A fixed border
reservation avoids changing row dimensions when the cursor moves. Both axes
share the existing active-row budget. The cursor does not independently pin a row.

Native section rows now prevent bare mouse-downs from moving focus away from
the list owner, just as option rows do. Independently focusable controls in a
section retain their own input and native row-retention behavior. The regression
failed before this correction and passes after it.

## Validation

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest -j 2 test/virtual_list`:
  passed, including seven projection/source tests, two low-level managed-input
  tests and five state-owning component tests. Covers 100k loaded metadata,
  point/hidden updates, decorations, filtering, queued requests, pending native
  acknowledgement, policy changes, one-row budgets, both axes, source return,
  unmount, initial-seed errors and stale layout errors.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib --test list_input --test lists --test tables --test tree_input --test accessibility --test session --test tree --test accessibility_busy --test accessibility_list_selection`:
  **806 library tests passed**, two existing private-D-Bus tests ignored on this
  platform, **44 integration tests passed**, both AppKit no-window fixtures passed.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 -p gpuio-native --all-targets --features native-image-tests,native-canvas-tests -- -D warnings`:
  passed after the section-focus correction.
- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest examples/gallery/main.exe`:
  passed with the completed managed component.
- `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` and `git diff --check`: passed.
- `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-selectable-managed-gallery-20261003`:
  passed fresh package staging and an independent gallery/backend build
  (`run=False`), without installing into or changing an opam switch.
  An additional module in that consumer uses installed `List_rows.Layout`,
  `Selectable_list.component`, a real query Input, section/disabled metadata,
  initial selection, custom row content and guarded controller confirmation.
  The copied main module references the probe. Compilation/linking passed with
  `OCAMLPATH=/private/tmp/gpuio-selectable-managed-gallery-20261003/installed/lib GPUIO_JOBS=2 ./scripts/gpuio exec dune build --root /private/tmp/gpuio-selectable-managed-gallery-20261003/consumer -j 2 main.exe`.
  The probe was not executed as a GUI application.

No OS windows were opened. Bonsai-driver, native TestPlatform and AppKit selector
checks do not establish physical keyboard/IME, VoiceOver or Linux desktop
qualification. Required Linux non-GUI release checks remain separate; OCH-47
desktop qualification remains deferred.
