# OCH-41 semantic input hints — partial checkpoint

Local dirty worktree, macOS arm64, 2026-10-01. This feature is **in progress**:
the status-query and gallery additions pass local checks; actual macOS
input/autofill qualification remains open. No release acceptance is claimed.
See the [implementation contract](../design/input-content-hints.md).

Implemented locally:

- Forty-five typed content hints, `Text_input.Config.content_hint`, separate paired
  operation 75 and native privacy/kind validation. Password/NewPassword hints
  require explicit password privacy. Legacy editor-config bytes remain unchanged.
- Retained editor AX role updates and focused-owner selection. Ordinary edit
  snapshots/revisions, composition and native identity survive hint changes.
  Focus on a non-input, read-only state and editor removal clear the selected hint.
- A macOS NSTextContent adapter with view-associated NSString and owner metadata.
  It refuses incompatible property methods and does not replace a foreign method.
  Lease teardown clears only its own property and releases the retained NSView;
  old leases cannot clear newer or foreign setter values. There is no global
  pointer-keyed value map or native fork change. The host has one lazy binding per
  window and avoids repeated setter calls while its desired owner/value is current.

Passing focused commands (run through `GPUIO_JOBS=2`):

- `./scripts/gpuio exec dune build -j2 lib/core lib/bonsai`
- `./scripts/gpuio exec dune runtest -j2 test/view_api`: three new expectations for
  validation/legacy bytes, independent paired bytes and hint-only reconciliation.
- `./scripts/gpuio exec cargo test -j2 -p gpuio-native -p gpuio-protocol --features gpuio-native/native-image-tests --test input_content_hint`:
  paired fixture, all 45 tags/invalid tags, malformed byte boundaries and atomic
  password-privacy/owner-kind admission.
- `./scripts/gpuio exec cargo test -j2 -p gpuio-native --test input_content_macos`:
  real headless AppKit getters/setters on isolated NSView subclasses, per-view
  isolation, owner handoff, foreign setter, invalid-value atomicity, view release
  and method collision. It creates no NSApplication or OS window. This proves
  the property/storage boundary, not that an autofill provider offers values.
- `./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --lib input_content_view`:
  production TestPlatform focus routing via Tab/Shift-Tab, semantic AX roles,
  composition/identity preservation, read-only and removal. This does not exercise
  a real window's raw AppKit handle or an OS autofill/IME provider.

The full native library suite passes **535 tests**, with two existing skips
(`--features native-image-tests --lib`). Strict all-target Clippy passes for
native/protocol with native-image-tests and `-D warnings`. Catalog source audit
and `git diff --check` pass; source coverage is not functional acceptance.
Full OCaml tests, formatting and the existing gallery build pass with
`./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe`.
The final AppKit property fixture, including current-owner checks, and Rust format
check also pass. These checks validate this partial implementation; they do not
establish physical acceptance; the later status-query/gallery work is recorded below.

Implementation note: the earliest native build required corrected FFI pointer
mutability and a qualified raw-window-handle call; the first AppKit fixture build
required ClassBuilder function-pointer lifetime inference. Both compile failures
were fixed before the passing checks above.

## Status-query and gallery continuation

`Gpuio_eio.Text_input.content_hint_status` now uses private wire command 7 and
result 2. It returns `Inactive`, `Exposed` or `Unavailable` with the hint at query
execution. Ordinary public editing commands still return snapshots. The host
intercepts metadata requests before the editing path and does not publish a text
observation or request a redraw merely to answer one.

Three Eio transport expectations pass for exact correlation/window/node checks,
wrong result kinds, duplicate and late replies, the shared 64-request budget,
close rejection and exception-safe cleanup. These tests use native transport
allocation and injected events, without an OS window.
Command: `./scripts/gpuio exec dune runtest -j2 lib/eio`.

Independent request and six-status event fixtures pass on the Rust side:
`./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --test input_content_hint`.
The expanded native focus test passes for inactive other fields, unavailable
native view/mapping, status during composition without text/focus changes,
read-only, absent hint and stale/non-editor rejection. It uses TestPlatform;
it cannot prove a real macOS view reports `Exposed` through the host.

The Text inputs gallery now includes email/URL/IMEI/no-hint configuration,
read-only state, explicit focus and a focus-preserving status check. Its last
observation names the returned hint rather than the current config. The password
example also configures a password hint with explicit password privacy.

A direct Bonsai/controller expectation passes for `Not_mounted` and a delayed
hint reply after new typing and a config change, preserving the latest snapshot.
Full OCaml tests, formatting and the gallery build pass with
`./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe`.
This also checks OCaml fixture equality, truncated/trailing data and invalid status,
hint, option and unavailability tags. The first full build found an incorrect
inline-test module declaration; it was corrected before the passing run.
Final strict all-target Clippy passes for native/protocol with native-image-tests
and `-D warnings`. The complete native library suite passes again: **535 passed,
two existing skips**. Catalog source audit and `git diff --check` also pass. These
checks cover the current local query/gallery implementation, not OS acceptance.
The full Rust protocol suite also passes with
`./scripts/gpuio exec cargo test -j2 -p gpuio-protocol`.

Required continuation: final-revision desktop, consumer and Linux evidence. Do not equate an accepted config or NSString property
with autofill capability. The full catalog and milestone 07 release scope remain
unchanged.
