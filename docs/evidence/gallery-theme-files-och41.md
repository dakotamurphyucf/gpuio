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

## Remaining acceptance at the initial checkpoint

Physically load/edit/reload the sample, type/select/undo in the draft, try an invalid
file, switch built-in/system appearance while work is pending, leave/revisit Styles,
and compare independent windows. Verify the actual visual palette and native
picker/focus behavior. Full gallery/accessibility, performance, distribution and
current required Linux checks remain milestone07 gates; Linux desktop qualification
remains OCH-47. No release ticket is completed by this checkpoint.


## Physical macOS walkthrough and cancellation repair — 2026-10-05

Final clean test checkpoint `197ea56292baea763bf4b13c889ae71118a00fa2`; gallery
behavior repair `52d796e`. macOS 14.5 arm64, real foreground AppKit window and file
picker. [Passing report and binary hash](gallery-theme-files-och41/report.json),
[native walkthrough log](gallery-theme-files-och41/native-walkthrough.log),
[Aurora capture](gallery-theme-files-och41/aurora.png),
[same-mode Amber reload](gallery-theme-files-och41/amber.png),
[invalid file retains Amber](gallery-theme-files-och41/invalid-kept.png).

The actual public Core/Bonsai/Eio example now passes twelve checks:

- Choose a Unicode/spaced profile path through NSOpenPanel, then load Aurora.
  The native editor retains its Unicode selection and draft.
- Edit the profile and explicitly reload changed colors with the same Dark
  appearance. Screenshot samples verify the requested surface RGB changed from
  `(28,32,51)` to `(48,48,28)`; over 168,000 sampled pixels match each expected
  surface within two levels. The actual captures were also inspected.
- Native Backspace followed by Command+Z after the reload restores the draft;
  changing logical scale preserves the selection and draft.
- Invalid profile data preserves the last good palette. Cancelling the native
  picker also keeps the draft and theme. A second window starts with its own
  built-in appearance and original draft.
- Explicit appearance and Follow system choices made during a pending Eio read
  each win over the delayed result. These checks wait for the file profile to
  disappear before delivering data, acknowledging the actual Bonsai choice.
- Page departure cancels a pending read. Returning acquires a fresh native draft,
  shows cancellation rather than stale loading, and keeps the path available for
  explicit retry. A subsequent successful retry and applied theme survive another
  page departure.
- Closing the final window while a read is pending returns from the app with
  status zero **before** the fixture writer closes. EOF therefore cannot substitute
  for scoped cancellation in that check. The child is reaped on all paths.

Pending reads use a disposable named FIFO in place of the already selected file.
The harness holds its writer, observes the owned app's open descriptor through
`lsof`, and only then makes the competing UI choice or closes the window. This
exercises the real Eio loader without application hooks or a mocked scheduler.
No directory watcher, OS appearance change, general clipboard write, input-source
change or VoiceOver operation is performed. Choosing the application's Follow
system option is distinct from physically changing the OS appearance.

The first native run found a real example bug: after cancelling the read on page
departure, the return page still displayed `Loading theme…` even though its busy
flag had cleared. The [bounded AX diagnostic](gallery-theme-files-och41/before-cancellation-status.log)
records it. The scope cancellation callback now updates the status only when work
was pending, retaining settled status and the selected path. A later harness run
exposed a separate synchronization mistake: AXPress queues its action and does
not prove the user choice has been applied. The final test waits for the resulting
profile change. macOS lsof also requires querying the owned PID rather than its
filename filter for FIFO observation; non-ASCII paths are matched in its C-locale
escaped representation. The completed walkthrough passes after these corrections.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe @fmt
python3 -m py_compile scripts/test_macos_theme_files.py scripts/test_gallery_desktop_macos.py
python3 scripts/test_macos_theme_files.py --output scratch/theme-files-native-001
```

Build/format, Python syntax, Actionlint 1.7.12 and `git diff --check` pass. The shared
picker helper keeps the represented-file test's original defaults and accepts the
new theme labels as explicit arguments. The new walkthrough is wired into macOS
CI; its hosted run is pending. Parser/schema/IO bounds are unchanged, with the
previous unit evidence above retained. OCH-41 and OCH-17 remain open for the broader
catalog, physical OS appearance, accessibility, GPU and distribution gates.
