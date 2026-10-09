# OCH-41 text-area search: native prerequisites

2026-10-01, local macOS arm64; stock OCaml 5.3/Bonsai v0.17 and pinned native
dependencies. This is an uncommitted implementation checkpoint. Public search
commands, Eio/controller integration and the search bar remain to be implemented.

Three initial TestPlatform checks reproduced two failures: an oversized
replace-current reported success even though the text limit rejected the edit;
replacement during composition could commit the provisional text. The third
check passed Unicode replacement, single-step undo/redo and read-only/disabled
guards. These were real Base editor entities without physical OS windows.

The adaptation preflights resulting UTF-8 size with checked arithmetic, rejects
active composition/NUL/noneditable state, and checks the current exact-text
policy before mutation. It constructs bulk output in one pass over the source
and confirms the editor revision advanced before reporting a replacement count.
Rejected edits preserve matcher position; scrolling follows successful editing.
The whole-document edit now explicitly converts its byte range to UTF-16.
The previous oversized endpoint clamped to the document end; it was not a
reproduced Unicode corruption defect.

Search highlight layout now selects a contiguous slice of fully contained
matches using binary search and reserves paths only for that slice. It preserves
global occurrence indices and the existing treatment of partial matches. This
reduces per-layout work; it is not a measured frame-time or memory acceptance
result. The full match vector remains retained and needs resource measurement.

Five native regression checks cover rejection and preserved directed selection,
match index/history/revision, IME preservation, Unicode undo/redo, equal-text
replacement, editability and local highlight slices. The dense-document case
uses 262,144 matches and checks bulk deletion plus undo. The first repair passed
the full native suite (568 passed, two existing skips), strict Clippy, formatting
and full OCaml tests/gallery build. After the linear output construction refinement, all 568 native tests pass
again (two existing skips), as do strict Clippy, Rust formatting and the final
full OCaml tests/formatting/gallery rebuild.

`cargo test -p gpui-base --lib input::search::tests --offline` could not run:
Base is an excluded dependency with dev-dependencies, not a workspace member.
It is not counted as a passing check. The native tests above directly exercise
the retained Base engine through its public methods.

The final canonical Base patch SHA-256 is
`f30f1071937fa412bcbcc5ece9713b2f280be665a310a74150f190a8fd1568e3`.
Fresh hash-verified archive reconstruction matches all 233 vendor files exactly
(excluding generated Cargo.lock). No protocol tags changed
in this checkpoint; the preceding full protocol suite passed 309 tests.

Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec`, with `-j2` for builds:

- `cargo test -p gpuio-native --features native-image-tests --lib --offline`
- `cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --offline -- -D warnings`
- `cargo fmt --all --check`
- `dune build @runtest @fmt examples/gallery/main.exe`

Actual macOS keyboard/IME/accessibility, the public search UI, installed consumer
and performance/resource acceptance remain open. Linux GUI remains deferred
OCH-47; required Linux automated checks remain separate.

See the [search design and remaining delivery](../design/textarea-search.md).

## Native session revisions and public value types

The next checkpoint adds `Editor_search` / `Text_input.Search` value types and
paired bounded observation schemas. Two Core expect tests check literal-query
validation, exact window/node ownership, editor/search revision identity, invalid
metadata and an independently specified UTF-8 observation fixture. Rust checks
the same fixture and bounds. All **310 protocol tests** pass. This fixture is
standalone metadata; editor command/result envelope integration remains open.

A native monotonic session counter detects query/navigation round trips and
repeated open requests. Identical query echoes and rejected edits preserve it.
Read-only changes route replace-mode changes through the same counter; disabling
search closes the session. Native selection seeds and direct query updates now
share the 2,048-byte bridge bound. Oversized selection seeds retain the prior query.

The seven native search checks pass, and the full native suite passes **570 tests
with two existing skips**. Fresh reconstruction again matches all **233 files**.
The current Base patch SHA-256 is
`3592b40ae45c0ed8dd170b0c02afaf48426136930f5ea09f74636fb655068b8c`.
Strict Rust lint and formatting pass, as do full OCaml tests, OCaml formatting
and the gallery build for this checkpoint.
The first Core test run found only a Unicode sexp spelling mismatch; the expected
escaped byte spelling was reviewed and corrected manually before the passing run.

There is still no public search command implementation, native message routing,
search observation subscription, reusable search bar or search gallery example.
The session counter supplies the identity needed for stale-command rejection;
the command adapter must still compare the expected stamp before performing edits.
