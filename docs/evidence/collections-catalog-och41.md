# Collections catalog and transcript semantics — OCH-41

2026-10-03, local macOS arm64, working tree based on `83eb87e`.
This checkpoint extends the uncommitted milestone-07 work. It is not a reviewed
commit, hosted result, physical desktop acceptance or ticket closure.

[The pinned review](../catalog/collections-review.md) now maps the exact
Base/component managed list, scroll, tree, managed/structural table and message
scroller sources. Snapshot hashes and revision are in the catalog manifest.
Functional equivalents, interaction differences and remaining public gaps are
explicit; the three family rows remain open.

The Collections message preview now uses public `List_collection` and managed
`Virtual_list` APIs to demonstrate earlier history, appending a new message,
growing/resetting the latest response, reading history and resuming tail following.
The application model permits 1,000 initial messages plus 32 at each end and at
most eight extra lines in the latest response. It has no background producer;
the controls advance the example manually. Only structural edits splice the
collection; fragments use point replacement with a shared order snapshot.
The active-row budget remains 24. Two new model expect tests check preservation
of old streamed values, value invalidation, order sharing and saturation under
repeated queued actions.

`Accessibility.Role.Log` now supplies transcript semantics on an ordinary
container or managed list, with Polite as the default and explicit Off/Assertive
overrides. Appended role tag16 preserves old tags and field layouts. The gallery
uses a named Log with Live.Off so fragments do not request repeated live
announcements. Applications may announce completed responses in a separate
Status. This API adds no timer, callback, data source or offscreen accessibility
history. The independent fixture is checked by both runtimes; the native wrapper
preserves hidden state and supplies no new Focus/Click actions. Session tests
cover incompatible child rejection with rollback, metadata-only replacement,
reset and close reclamation; the tree-input test rejects a Log root while native
tree interaction is enabled.

The pinned Cocoa adapter already maps AccessKit Log to AXGroup with subrole
AXApplicationLog (`vendor/accesskit-macos/src/node.rs`). That source mapping and
the native semantic-wrapper tests do not prove external AX notification delivery
or VoiceOver speech. No platform adapter patch was added.

## Validation

Use the repository's isolated environment; no unrelated switch/default changed.

| Command | Result |
| --- | --- |
| `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-protocol` | Pass, including nine accessibility tests and the independent Log fixture |
| `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib --test accessibility` | 751 library tests passed; two existing private-D-Bus tests skipped outside an isolated bus; three admission tests passed |
| `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --test tree_input` | Four tests passed, including incompatible Log/tree-input rollback |
| `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe` | Pass: full OCaml tests/format and gallery build |
| `GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests,native-canvas-tests -- -D warnings` | Pass |
| `GPUIO_JOBS=2 ./scripts/gpuio exec cargo fmt --all -- --check` | Pass |
| `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace scratch/agents/root-20260929-m7-resumed/collections-installed-gallery` | Pass: fresh staged libraries and independently rebuilt backend/gallery; `run=False`, no desktop launch |
| `python3 scripts/audit_component_catalog.py` | Pass: hashes, 146 root entries / 43 families, style/event inventory; structural verification only |
| `python3 -m py_compile scripts/test_gallery.py` | Pass; updated Collections walkthrough authored, not run |
| `git diff --check` | Pass after final documentation updates |

Native tests here use pure session models and TestPlatform/semantic builders.
No OS application window was opened. Desktop automation remains unavailable in
this environment; the previously failing preflight was not retried unchanged.
The updated public walkthrough checks native follow/pause state through prepend,
append, growth, reset and tab changes when it can be run. It is not evidence of
pixel-stable scrolling until that actual run succeeds.

The initial gallery test build caught a mislabeled fold callback; it was fixed.
The first protocol test referred to a nonexistent `Kind::Table`; the test was
corrected because a managed table specializes the existing VirtualList kind.
The full OCaml build also caught an exhaustive role match in a tree test; the
new Log case was added explicitly. These development fixes did not change
production collection ownership or protocol kind layout. All commands above
finished successfully after these fixes. Remaining catalog, physical and release
work stays open.
