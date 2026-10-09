# OCH-41 native input validation evidence

2026-10-01, local macOS arm64 worktree. Preparation, native single-line edit
filtering and a public gallery example are implemented locally. These checks do
not establish real desktop input, Linux acceptance or release completion. The
[design](../design/input-validation.md) records the exact boundary.

## Integrated edit filtering

`Text_input.Config ?edit_filter` now carries a prepared `Input_validation` rule.
Paired Op77, atomic native compilation/admission and a shared compiled-policy
owner connect it to the retained editor. Formatting precedes filtering; exact
commands require matching formatted text. Incompatible drafts and history are
retained, and submission remains an observation rather than form validation.

Verified commands use the repository environment and `GPUIO_JOBS=2`:

- `./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --test input_validation --offline`:
  both fixtures pass, including independent operation-77 bytes, clear/config
  variants, flags, truncation and invalid booleans.
- `./scripts/gpuio exec dune runtest -j2 test/input_validation`:
  all five expect tests pass. The additional test prepares rules through the
  real FFI, checks the independent operation bytes, rejects multiline config,
  preserves legacy editor-config encoding and verifies filter-only reconciliation
  without seed replacement.
- `./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --lib --test input_validation_admission --test choice_picker --offline`:
  **552 library tests pass, two existing skips; both admission tests and all
  eleven picker tests pass**. Checks cover retained draft/entity/history,
  interactive and exact rejection, formatting order, empty policy, IME provisional
  text/commit/rollback, installing/removing rules during existing composition,
  submission of a retained incompatible draft, compiled-owner
  reuse/release and representative cache growth through the 256 KiB text boundary.
  Admission rejects invalid syntax/expansion, rolls back prior operations on
  failure, charges/releases policy reservation and excludes picker query fields.
- `./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --lib input_format_accessibility_value_actions --offline`:
  the expanded rendered-host test passes. An AX replacement queued before a
  filter change uses the current rule; an accepted replacement matches reference
  caret geometry; removing the editor releases its compiled filter and cache.
  A retired target cannot edit a sibling. This is TestPlatform callback routing,
  not an external macOS AX client or VoiceOver test.
- `./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe`:
  full OCaml tests, formatting and the new gallery example build pass. Rules are
  prepared once during application Eio initialization and reused across windows.
  The gallery itself has not received a physical desktop walkthrough at this
  checkpoint.

The final full protocol suite passes **306 tests** with
`./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --offline`. Rust formatting,
strict all-target native/protocol Clippy (including native-image-tests),
`git diff --check` and the structural catalog source audit pass. No hosted CI or
current-revision Linux result is claimed.

The regex cache reservation is conservative admission accounting, not an allocator
quota or measured latency/RSS guarantee. Physical keyboard/IME/clipboard/AX,
visual gallery, installed-consumer runtime, aggregate resources/performance and
final required Linux checks remain open. OCH-41 and milestone 07 stay in progress.

## Earlier preparation checkpoint

All commands use the repository environment with `GPUIO_JOBS=2`.

- `./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --test input_validation --offline`:
  one independent fixture test passed, covering five source encodings, result
  encodings, truncation, invalid tags/booleans/UTF-8/NUL, trailing bytes and an
  oversized declared source length.
- `./scripts/gpuio exec dune runtest -j2 test/input_validation`:
  four expect tests pass through the actual Rust FFI. They check immutable source
  round trips, typed syntax/resource errors, bounded diagnostic decoding,
  concurrent calls, recovery after failure and cancellation before preparation.
  A 2,047-byte multibyte invalid source also returns a valid UTF-8 diagnostic
  within the 1,024-byte bound through the real FFI.
  The first run failed because the test used an Eio cwd capability to read Dune's
  out-of-directory symlink. Using the runner's filesystem capability, consistent
  with existing fixture tests, fixed that test-only failure.
- `./scripts/gpuio exec dune runtest -j2 lib/eio test/input_validation`:
  passed after adding two scheduling tests. A cancelled admitted operation keeps
  its slot until work finishes and does not deliver its result; a cancelled waiter
  never enters, and subsequent callers can still use the slot. These scheduling
  tests use controlled Eio promises; the separate preparation tests exercise the
  real system-thread/FFI path.
- `./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --lib --offline`:
  **548 passed, two existing skips**. Three new compiler tests cover whole-value
  alternatives, trailing free-spacing comments, multiline flags, Unicode classes,
  empty expressions, substring/case behavior, unsupported syntax, nesting and
  enormous repetition counts, and capture-table memory. The full suite includes rendered formatting and
  queued accessibility-action checks described in
  [formatting evidence](input-formatting-och41.md).

The capture-table test first reproduced **5,250,111 bytes** of populated cache
for a short source with a Unicode word boundary, 200 optional capture groups and
an 8,394-byte value. The value is long enough to require the PikeVM fallback
instead of bounded backtracking. Merely constructing an unused cache or matching
a short input did not reproduce the allocation. Compilation now omits user
capture slots, preserving the implicit whole-match slots. The same real search
stays below 1 MiB and Boolean match/rejection semantics pass. This is a regression
workload, not a universal total-memory bound. No shipped editor policy used these
new preparation values yet.

These checks open no OS windows. Native edit-policy attachment, actual IME/
clipboard/VoiceOver behavior, aggregate compiled-policy/cache resource accounting,
input latency, final consumer builds and required Linux checks remain outstanding.

The full OCaml test suite, formatting and gallery rebuild also pass:
`./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe`.
The full protocol suite passes **305 tests** with
`./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --offline`.
Strict Rust lint passes with
`./scripts/gpuio exec cargo clippy -j2 -p gpuio-native -p gpuio-protocol --all-targets --features gpuio-native/native-image-tests --offline -- -D warnings`.
Rust formatting, `git diff --check` and `scripts/audit_component_catalog.py` pass.
The source audit proves inventory structure, not implementation acceptance.
