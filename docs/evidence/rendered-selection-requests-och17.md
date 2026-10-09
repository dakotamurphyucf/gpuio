# Native rendered-selection requests — OCH-17 / OCH-41

The prepared-text foundation now has a native request primitive that retains
anchor/head direction and updates existing selection owners together. This is
part of the [rendered-document selection plan](../design/rendered-document-selection.md).
It does **not** yet publish accessible TextRuns or authorize OS selection actions.

`prepare_rendered_selection` captures checked positions and a view-specific
interaction epoch. Applying the request validates the current projection,
selection policy, atomic boundaries and all native owners before mutation.
Foreign/stale requests and mapping failures preserve the old selection. Plain
Copy uses the accepted logical range; Markdown source Copy uses the same native
owners and existing markup reconstruction. Collapsed ranges clear highlighting
and do not activate Copy.

Unpainted virtual blocks receive their prepared text without layout, so selection
does not depend on having visited every row. Compatible streamed updates can
rebind a retained directed range when the native transfer succeeds and the text
prefix through the endpoints is unchanged. Queued requests still expire across
that installation. Clear, native selection gestures, Select All and disabling
selection also invalidate pending requests. Copy-format changes preserve them.

Atomic alternatives require boundary endpoints. Declared custom text is identified
separately from an unpainted ordinary run, including empty declared glyphs. When
its copy alternative differs, the request currently fails with `UnmappedOwner`;
general mapping of that case remains required work. Empty owner intervals remain
in the projection so a new request can clear their selection without inventing
copy text.

## Local checks

On macOS 14.5 / arm64 / Apple M1 Max, the native suite passes **1,090 tests**, with
two existing ignored. Tests cover backward Unicode ranges, repeated text with
different Markdown formatting, source/plain Copy, view identity, interaction
epochs, equal-text replacement, disabled selection, atomic alternatives, empty
and collapsed ranges, unpainted selection across 197 of 200 paragraphs, compatible
streamed retention and failure without partial mutation. The mounted-document
tests also draw the production document and query the window Copy provider.
Strict native all-target Clippy and Base patch reconstruction pass.
Full Dune `@all @runtest @fmt` and Rust formatting also pass after the final
renderer-resource guard change.

The actual native document suite adds a backward request case alongside its
existing Code and Markdown Select All cases. It verifies selection pixels,
window Copy, style precedence and retention across streamed publication, emitting
`GPUIO_RENDERED_SELECTION_REQUEST_GPU_OK`. The rest of the document suite passes.
The rebuilt gallery also passes the permanent selected-document shutdown
regression with both Rust backtrace variables enabled. Test children exit normally
and the captured clipboard representations are restored. No VoiceOver or OS
settings changed. Native dispatch/GPU checks are distinct from OS AX selection
actions and screen-reader acceptance.

A before/after regression also covers renderer-resource refresh without AST
replacement. Before the fix, a pending request remained valid on that path.
The refresh now expires its epoch and rebuilds the selection projection alongside
the existing displayed-text projection. Compatible backward selection survives
with new positions; changed declared glyphs clear an incompatible selection and
reject a new unmapped request, including before layout.

The first focused run had five passing tests and one fixture failure: an HTML
image lowered to an atomic inline node in its own paragraph, so it did not test
the declared custom block mismatch. The corrected fixture uses an actual block
plugin. A later actual-window fixture initially failed compilation because it
called the two-argument window Copy helper through the one-argument App updater;
the corrected fixture uses `WindowHandle::update`. Both failures are retained.

The [archive](rendered-selection-requests-och17/reports.tar.gz) and
[verified manifest](rendered-selection-requests-och17/manifest.json) retain exact
commands, before/after resource-refresh regression output, initial failures,
source patch/hashes, native logs and the gallery binary hash. Reproduce with
`GPUIO_JOBS=2` and the isolated wrapper:

```sh
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native --lib --features native-canvas-tests,native-image-tests
./scripts/gpuio exec cargo clippy --offline --locked -j2 -p gpuio-native --all-targets --features native-canvas-tests,native-image-tests,presentation-diagnostics -- -D warnings
./scripts/gpuio exec dune build -j2 @all @runtest @fmt
./scripts/gpuio exec cargo fmt --all --check
python3 scripts/test_macos_document_shutdown.py --log scratch/document-shutdown.log --report scratch/document-shutdown.json
```

The actual-window Rust target is `native_highlight_document` with
`native-image-tests`; the archived driver bounds the child lifetime and restores
the captured clipboard after it exits. All local build/test children have exited.

## Remaining integration

The request accessor covers accepted requests, not all native pointer/multi-click
selection or Select All overrides. Complete endpoint capture and remapping,
declared-object mapping, window selection coordination, focus/reveal, current
visibility/modality checks and semantic-node identity validation remain required
before wiring OS actions. The full bounded rich semantic hierarchy needs TextRuns
and range publication, including offscreen content and unique interactive links.
Root/fresh-consumer AX, real VoiceOver and broader release acceptance remain open.
No Linux GUI or physical-presentation claim follows from these checks.
