# File-backed gallery themes — OCH-41

Local checkpoint, 2026-10-04, macOS arm64, uncommitted worktree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`. This change uses public OCaml
Core/Bonsai/Eio APIs. Rust, protocol, dependency pins and vendor patches are
unchanged from the [font checkpoint](default-fonts-och41.md).

## Implementation

Styles → **A theme from your workspace** provides Choose theme, Reload file,
current-profile/error status and a native draft editor. The checked-in
[Aurora profile](../../examples/gallery/themes/aurora.sexp) exercises the format.
Profiles update the window's common palette, presentation colors and document
appearance. Other windows and the logical-size control retain their own state.

The pure profile decoder enforces a 16 KiB input bound, UTF-8/no NUL, version1,
nonblank name of at most 128 bytes, exact record fields, checked concrete hex
colors and a maximum parsed list depth of16 before typed record decoding.
It reuses `Color_value.Rgba.of_hex`; it introduces no duplicate color parser.

The file loader uses bounded Eio reads and `with_open_in`. Expected I/O/parse
errors are values; cancellation propagates. The preview admits one load at a time
and owns work in its page/window child scope. It checks scope liveness inside the
completion effect, and checks an opaque selection identity before starting a read
after the picker and again before applying a result. Explicit Light/Dark or Follow
system creates a new identity even when its colors equal the previous choice.
Page departure cancels pending work and resets busy state; the applied palette
lives in the window model and survives departure.

The gallery's Bonsai theme effect now observes the resolved `Theme.t`, rather
than only its light/dark discriminator. This matters when a file reload changes
colors without changing appearance. Invalid files retain the last good selection.
Diagnostic text is limited to a valid UTF-8 prefix of at most512 bytes.

See [the contract](../design/gallery-theme-files.md). This example uses explicit
reload. It is not an implemented directory watcher, an upstream JSON importer,
or a new global native theme registry.

## Validation

`GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest examples/gallery/main.exe`
**passed**, log `theme-file-full-ocaml-003.log` under ignored
`scratch/agents/root-20261003-release-notices/`.

- Profile expectations cover a valid concrete palette, exact byte boundaries,
  duplicate/unknown/missing fields, malformed/trailing expressions, unsupported
  versions/modes, blank/oversized/invalid names, literal and escaped NUL, invalid
  UTF-8, invalid hex/token-like colors, and a deeply nested expression.
- Selection expectations cover last-good retention, invalidation by equal-valued
  explicit choice or System, cross-window token rejection, and a successful
  same-mode color reload.
- Reconciliation expectations require real Set_style operations while forbidding
  editor Create/Remove/Set_text on successive profile color changes. This proves
  transaction identity/value policy, not physical IME or selection restoration.
- Eio tests read the checked-in sample, accept 16,383/16,384-byte profiles, reject
  16,385/32,768-byte files and malformed/missing files, and verify that cancellation
  before reading propagates rather than returning an ordinary error.

`GPUIO_JOBS=2 ./scripts/gpuio check-fmt` **passed**, log
`theme-file-format-check-001.log`. Catalog structural checks, edited-document
links and `git diff --check` also pass; they are not desktop behavior evidence.

The first two runs exposed test-fixture capability issues: the parent-relative
fixture path and then Dune's symlinked input escaped Eio's confined cwd capability.
The test now declares/copies the input through Dune and uses the provided filesystem
capability for that read-only fixture; generated files remain under the test cwd.
No expectation was automatically promoted and no OS permission was changed.

No native production code changed, so the prior904 native tests and two existing
skips remain the latest native checkpoint; they were not rerun for this OCaml
example. Source presence, build success and these tests do not establish native
picker or physical gallery acceptance.

## Remaining acceptance

Physically load/edit/reload the sample, type/select/undo in the draft, try an invalid
file, switch built-in/system appearance while work is pending, leave/revisit Styles,
and compare independent windows. Verify the actual visual palette and native
picker/focus behavior. Full gallery/accessibility, performance, distribution and
current required Linux checks remain milestone07 gates; Linux desktop qualification
remains OCH-47. No release ticket is completed by this checkpoint.
