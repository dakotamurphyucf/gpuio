# OCH-41 clear on Escape — local checkpoint

2026-10-01, local macOS arm64, uncommitted worktree, isolated stock OCaml 5.3 /
Bonsai v0.17 and pinned Rust/GPUI dependencies. This is implementation evidence,
not published or physical desktop acceptance.

All commands use `GPUIO_JOBS=2 ./scripts/gpuio exec` and two build jobs.

- `dune runtest -j2 test/editor_escape`: two expect tests pass for both modes,
  default-off behavior, unchanged legacy config bytes, independent Op79 encoding
  and changed-only reconciliation without reseeding.
- `cargo test -j2 -p gpuio-native --features native-image-tests --lib --test
  editor --test choice_picker --offline`: **558 library tests passed, two existing
  ignored**, plus **four editor and twelve picker checks**. The two new rendered
  tests exercise both input modes, opt-in toggling without replacement, focus,
  undo/redo, read-only/disabled and empty propagation, composition precedence, and
  rejected-filter selection/revision preservation. A global native Escape
  listener observes actual action propagation. Native admission covers atomic
  rollback and picker-query ownership on descendant-only updates.
- `dune build -j2 @runtest @fmt examples/gallery/main.exe`: full OCaml suite,
  formatting and public gallery pass, including independent native example builds.
- Full protocol suite: **308 passed**, including the independent Op79 fixture and
  truncated/trailing/noncanonical Boolean rejection.
- Strict Rust lint (`cargo clippy` for native/protocol all targets with
  `native-image-tests`, `-D warnings`) passes after the related frame-clear repair below.

No OS windows are created by these TestPlatform checks. Real macOS keyboard,
IME/candidate behavior, clipboard, accessibility/VoiceOver, visual walkthrough,
installed consumer and performance/resource acceptance remain open. Linux desktop
qualification remains deferred OCH-47; required build/unit/consumer checks remain.

Contract: [clear on Escape](../design/editor-escape.md).

## Related input-frame regression

The clear-button integration test reproduced a rejected-edit defect: with a
nonempty-only filter and directed selection 4→1, the old Base `clean` path kept
`edited λ` and revision 1 but collapsed selection to 0→0. The test exercises a
real mounted frame and native `clear_input` routing. The adapter now submits an
exact, revision-guarded, undoable empty replacement and focuses only on success.
The regression also verifies prior undo/redo history and a successful clear after
removing the filter. The repaired full native suite passes 558 library tests (two existing skips),
four editor checks and twelve picker checks. Full OCaml tests, formatting and
gallery rebuild also pass against the repaired native backend. Final strict Rust
lint passes; the already passing 308-test protocol suite is unchanged.
