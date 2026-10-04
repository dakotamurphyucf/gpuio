# OCH-41 input formatting — native prerequisite checkpoint

2026-10-01, local macOS arm64 worktree. **Core configuration, paired protocol,
retained native formatting and the public gallery example are now implemented
locally.** The latest integration checkpoint is below; earlier sections preserve
the prerequisite evidence. Native regex filtering now has separate
[integration evidence](input-validation-och41.md); final physical macOS acceptance
remains open. This does not complete OCH-41 or milestone 07.

The integration fixture is `rust/native/tests/input_format.rs`. It uses the
actual vendored `gpui_base::input::MaskPattern` and, with `native-image-tests`,
GPUI TestPlatform plus `InputState`/`EntityInputHandler`. It opens no OS window and
does not exercise a physical IME candidate panel.

## Rendered formatting and queued accessibility actions

The later `rust/native/src/editor_format_view_test.rs` fixture mounts the actual
host `View` on GPUI TestPlatform. It draws formatted and plain reference inputs,
then compares caret geometry at every UTF-16 character boundary after paste,
middle edits, Backspace, Copy/Cut, Undo, policy replacement and policy removal.
It also checks the accessibility value, placeholder and native-owner disposal.

A second test sends AccessKit SetValue through GPUI's test-platform window
callback and asynchronous host routing. Exact malformed replacements are rejected;
requests queued before a policy change use the current policy, and requests for a
removed editor cannot edit its sibling. This is native routing evidence, not
macOS accessibility-client or VoiceOver acceptance.

Both tests pass. The subsequent full native checkpoint passes **548 tests with two
existing skips**, including the three regex preparation tests added afterward:

`./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --lib --offline`

Test-only GPUI helpers expose `simulate_a11y_action` on TestWindow and
VisualTestContext. The callback is invoked outside the test-window mutex; there
is no production AX routing change. The canonical `third_party/patches/gpui.patch`
SHA-256 is `5e486cc4ad273aee8479c8bf634c5d58f11d3cb9d21439ba217a4eaf8f606c5d`.
Reconstructing with `scripts/vendor_gpui.py` from the pinned Zed archive matches
all **155 files**, excluding generated Cargo.lock/target. The GPUI Base patch is
unchanged by these rendering tests.

## Reproduced failures and fixes

- A wildcard rejected `界`: validation counted consumed Unicode scalars but
  compared them with UTF-8 byte length. The comparison now uses scalar length.
- Pattern `9A` accepted `x` by skipping the required digit slot. Formatting would
  then discard that input. Validation now skips only omitted literal separators.
- Marking `界12`, committing it as `界–12`, then undoing produced `––12` instead of
  the empty draft. Undo reapplied the mask to a recorded intermediate composition
  value, invalidating the next change's byte ranges. A private history-replay flag
  now bypasses normalization, validation and masking during Undo/Redo only.
  Suppressing history during an ordinary `set_value` does not bypass formatting.

Each failure was observed before its repair. The seven passing focused checks
cover UTF-8 wildcard/literal handling, ASCII class bounds, raw/formatted separation,
formatted caret byte offsets, rejected-edit text/selection/revision retention,
undo/redo, composition transactions and changed policy during history replay.

## Verification

Commands use `GPUIO_JOBS=2` and the repository environment:

- `./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --lib --test input_format`:
  **535 library tests passed, two existing skips; seven formatting tests passed.**
- `./scripts/gpuio exec cargo clippy -j2 -p gpuio-native -p gpuio-protocol --all-targets --features gpuio-native/native-image-tests -- -D warnings`:
  passed.
- `python3 scripts/vendor_gpui_base.py --archive <verified-local-archive> --output <fresh-scratch-directory>`:
  final reconstruction matches all **233 files**, excluding only generated
  `Cargo.lock`; archive and patch hashes are verified by the script.

The earlier checkpoint GPUI Base patch SHA-256 was
`ec4cf42aa27bca47e729823f7dfa11e4e77f038f6fee5f114a0bfc048c061de0`.
The upstream archive and GPUI/source pins are unchanged. Normal builds use the
versioned vendor sources and patch; scratch output is not a build dependency.

The full OCaml tests, formatting and gallery rebuild against the changed native
dependency pass with
`./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe`.
Rust formatting, `git diff --check` and the catalog source audit also pass. Final platform/consumer/desktop evidence remains open. See the
[formatting design](../design/input-formatting.md) for replacement and policy
contracts still to implement, including regex/native validation and resource limits.

## Pure Core format values and conversion helpers

`lib/core/input_format.mli` was drafted before implementation. Pattern and Number
constructors validate bounded values; `format_raw` and `raw_of_formatted` distinguish
literal positions from slot content and preserve decimal precision. They do not
configure a native editor. The exact rules are in the [design](../design/input-formatting.md).

Six expect tests in `test/view_api/input_format_test.ml` pass with
`./scripts/gpuio exec dune runtest -j2 test/view_api`. They cover Unicode/literal
collisions, required classes, exact formatted prefixes, decimal grouping and
full-width normalization, incomplete drafts, fraction limits without grouping,
malformed text/configuration, long decimal round trips and the exact 256 KiB
expanded-output boundary. The first run differed only in S-expression quoting and
UTF-8 escaping; the expected output was reviewed and corrected manually.

The implementation scans large text by UTF-8 byte cursor, avoiding per-character
list allocation; decimal splitting stops at the first point. Full OCaml tests,
formatting and the gallery build pass for this Core addition with
`./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe`.
The catalog source audit and `git diff --check` also pass. Native policy, runtime editing/validation and the format gallery remain
required work. Paired standalone format data was added at the following checkpoint.

## Paired format data and retained-draft recovery

The OCaml wire representation and Rust format values now share 23 independently
specified fixtures for binary encoding and raw/formatted conversion. The bounded
standalone decoder rejects malformed configurations, oversized lengths and trailing
bytes. Unicode digit classification uses Unicode 17 in both languages. Three Rust
tests and the seventh Core expect test pass with:

- `./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --test input_format --offline`
- `./scripts/gpuio exec dune runtest -j2 test/view_api`

These are standalone data and conversion APIs; the editor configuration field,
message operation, native policy admission and gallery integration are still absent.

A further native regression reproduced text loss when installing a digit mask over
an existing `abcd` draft, then editing it to `abc`: the mask discarded the invalid
candidate and produced empty text. The repair preserves invalid drafts during
recovery and applies formatting only when the candidate is valid. Edits from valid
to invalid remain rejected; undo restores both intermediate and initial drafts.

`./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --test input_format`
passes all eight focused tests after the repair. Exact archive reconstruction
matches all 233 files, excluding generated `Cargo.lock`; the current Base patch
SHA-256 is `8335bd5168ff3287550caa439763cd879b4158ec2a99f5a77ab1144c387689a5`.
No OS window or physical IME test was run for this checkpoint. The full-suite and
Clippy results above predate this recovery change unless updated below.

The full native library suite also passes after the recovery repair:
`./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --lib`
reports **535 passed, two existing skips**. `git diff --check` passes.

## Integrated ordinary-input formatting checkpoint

The public interface now includes `Text_input.Config.create ?format`; multiline
configuration is rejected. Paired operation 76 carries optional format data without
changing legacy editor-config bytes. Core reconciliation tests verify that changing
or clearing a format emits only that operation, preserves an incompatible mount
seed and never replaces the editing owner. Nine Core expect tests pass with
`./scripts/gpuio exec dune runtest -j2 test/view_api`.

Native admission rejects wrong owner kinds, malformed configurations and picker
query policies, including descendant-only updates. Budget exhaustion rolls the
whole transaction back; clearing/removing policies releases their retained charge.
The native allowance includes bounded compiled pattern storage and policy overhead.
Independent operation bytes are in `test/fixtures/input-format-operation.hex`.

The Rust policy keeps canonical draft acceptance separate from interactive
formatting. Exact replacements (including the existing accessibility SetValue path)
are checked before editing or applying selection. Interactive numeric edits retain
full decimal precision, reject fraction overflow with or without grouping, and map
UTF-8 carets through grouping of the whole value. Policy changes retain native
identity, text, directed selection, revision, composition, placeholder and undo.

TestPlatform `editor_format_test.rs` exercises actual native Instances and input
handlers. Marked IME text may temporarily exceed the pattern's slot count: `jie`
can commit as `界`. Successful commit is one undoable change; rejected commit and
cancellation restore the previous draft/selection without adding an undo entry.
Readonly unmark and format changes/removal during composition are covered.
A separate before/after regression reproduced enabling formatting during a plain
composition: rejection left `12` marked instead of restoring the empty draft.
Capturing the baseline before format activation fixes this case.

Current commands (`GPUIO_JOBS=2`, repository environment):

- `./scripts/gpuio exec cargo test -j2 -p gpuio-native --features native-image-tests --lib --test input_format --test input_format_admission --test choice_picker`:
  **543 library tests passed, two existing skips; eight Base format, two admission
  and ten picker tests passed.**
- `./scripts/gpuio exec cargo clippy -j2 -p gpuio-native -p gpuio-protocol --all-targets --features gpuio-native/native-image-tests -- -D warnings`: passed.
- Archive reconstruction: all **233 files** match, excluding generated Cargo.lock.
  Current Base patch SHA-256:
  `13fd104887fdf9b09e215d31399efef281da44f804efd63121bd1d57ce37a1b7`.
- `git diff --check` and the catalog structural source audit: passed.

`examples/gallery/format_preview.ml` uses public Core/Bonsai/Eio APIs. It switches
between reference masks, grouped decimals and free text while retaining the draft,
shows observed raw extraction, and prepares explicit sample replacements through
`format_raw` plus `replace_if_unchanged`. No native observation triggers an implicit
Bonsai text replacement. The first full gallery build found a missing dependency
edge in the independent backend lockfile; both example lockfiles now include the
already-pinned Unicode 17 dependency. Full OCaml tests, formatting and the gallery
build pass with `./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe`.

No OS window was opened for these tests. Physical candidate-panel behavior,
clipboard, external AX/VoiceOver, visual layout, final platform/consumer runtime
and measured resource acceptance remain open. Regex/declarative validation and
other required catalog controls are separate unfinished scope, not deferred here.

Final local checks for this integration checkpoint also pass:
`./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --offline` reports **304
passing tests** across the protocol suite; `cargo fmt --all -- --check` passes.
The full Dune command above completed successfully after the lockfile repair.
These commands do not establish rendered-gallery or physical platform acceptance.
