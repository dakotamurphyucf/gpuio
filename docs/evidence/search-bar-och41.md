# Reusable search bar — OCH-41

Local checkpoint, 2026-10-01, macOS, isolated repository toolchain; worktree based
on `83eb87e`. Automated behavior evidence only. No OS window was opened for this
checkpoint; physical keyboard, IME, VoiceOver and visual acceptance remain open.

`Gpuio_eio.Search_bar.create`, `open_` and `wrap` provide ordinary text-area
Find/Replace presentation using public APIs. The gallery's “Room to write” example
demonstrates it. Rust retains document ownership and matching; Bonsai owns the bar
controls. See the [contract](../design/textarea-search.md).

Each opening has its own query-control lifetime. The source editor remains under
a stable keyed wrapper, preserving draft, selection and history. Committed
replacement text survives closing and reopening. The bar owns the query draft
during an opening; direct source query commands do not replace that draft.
Reopening seeds it from native state. Labels are English and styles inherit from
the application with an optional bar-container override.

Commands append tags 8–11: activation-checked close/focus, query-only update,
case-only update and native case toggle. Existing tags remain unchanged. Close
rejects stale openings and composition; hidden/disabled sources close without
stealing focus. Eligible read-only sources can regain focus. Query echoes preserve
native match position. Replacement uses the displayed stamp and exact committed
fields, rejecting composition or a query that no longer matches that stamp.

Eight real Bonsai driver/reconciler tests with mocked transport cover:

- Initial seed selection, coalesced typing before the first observation,
  composition-safe queries and control retirement.
- Close/reopen composition guards and late reads from a retired opening.
- Exact replacement stamps, Unicode/newline replacement retention and bounded
  query validation.
- Returning to a previous query while another update is pending; old command
  errors cannot replace newer feedback.
- Scoped shortcut policy, including source/replacement Enter preservation.
- Duplicate query echoes cannot cancel an in-flight Next action.
- Late root-open errors cannot replace newer successful feedback.

The harness asserts that bar transitions never remove or set the source draft.
Its mocked source owner does not establish native end-to-end ownership: separate
Eio command tests and native tests cover owner rejection, close result validation,
stale activation, composition and focus eligibility. Paired independent fixtures
check the new command tags and malformed values on both language boundaries.

All build/test commands below use `GPUIO_JOBS=2 ./scripts/gpuio exec`:

- `cargo test -j2 -p gpuio-native --features native-image-tests --lib --offline`:
  **579 passed, two existing skips**.
- `cargo test -j2 -p gpuio-protocol --offline`: **311 passed**.
- `dune build -j2 @runtest @fmt examples/gallery/main.exe`: passed.
- After the final OCaml-only root feedback and replacement-retention changes,
  `dune build -j2 @lib/eio/runtest @fmt examples/gallery/main.exe`: passed,
  including all eight current driver tests.
- `cargo clippy -j2 -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --offline -- -D warnings`: passed.
- `cargo fmt --all --check`: passed.

`python3 scripts/audit_component_catalog.py` passes structural mapping, not
behavioral acceptance. `git diff --check` passes. No new vendor patch or dependency
change is required for this bar. These local changes are not yet a reviewed,
committed release; current Linux automated checks and macOS physical/resource
qualification still remain. No whole-editor-family or milestone completion is
claimed.
