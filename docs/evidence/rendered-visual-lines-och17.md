# Rendered document line geometry — OCH-17

Local macOS 14.5 arm64, following `7a4138e3`. This checkpoint repairs visual-line
relationships in rendered Markdown and qualifies a visible paragraph through the
actual macOS accessibility APIs. It does not establish complete VoiceOver reading,
all document geometry, Linux desktop behavior or milestone 07 acceptance.

## Reproduction and implementation

The gallery paragraph containing `世界` and `👨‍👩‍👧‍👦` paints both on the same line,
with y = 725.5 and height = 26 in the captured window position. Before this repair,
`AXLineForIndex` reported lines 10 and 12 respectively. `AXRangeForLine(10)` returned
UTF-16 `[51,41]`, stopping after `世` rather than covering the paragraph. The existing
TextRuns had character rectangles but lacked `previous_on_line`/`next_on_line` links;
the pinned AccessKit consumer therefore treated fragment boundaries as line breaks.
The native regressions independently reproduced fragmentation across a styled
paragraph/link and within table-cell text.

Base's final Document prepaint now relates its prepared runs in source order.
Adjacent runs with matching current vertical extents join within the same semantic
flow. Paragraphs, headings and table cells have distinct flow identities. A logical
hard newline can terminate the preceding line without receiving invented glyph
geometry. Hard breaks, actual soft wraps and unavailable geometry remain boundaries.
The code reuses existing shaped rectangles and prepared coordinate mappings; it
neither shapes another copy of the text nor calls OCaml during layout.

GPUI's scoped `A11ySubtreeBuilder::set_text_run_lines` validates a complete proposed
edit before mutating the selected TextRuns. Repeated, foreign, non-text and empty
line entries are rejected, as are partial edits whose old links escape the supplied
set. Singleton lines clear earlier links. It leaves other native text owners,
node identities, actions, values and rectangles intact. Native tests verify atomic
rejection and resets against actual completed sibling subtrees.

The remaining native cases verify exact paragraph text across styling/links/Unicode,
hard-break and table-cell separation, and wide → narrow → wide reflow. Reflow changes
line counts without losing text, retaining links to removed IDs or creating cycles.
Missing off-layout geometry does not become an estimated rectangle or invented
soft-wrap layout. This is a native metadata change, not a new OCaml protocol feature.

## Actual macOS results

`python3 scripts/test_macos_text_selection.py --output <fresh-directory>` passed
against the rebuilt public gallery. Both Unicode terms now report **line 2** and
UTF-16 range **`[51,58]`**, containing the entire expected paragraph. Existing shaped
rectangles are unchanged. A point inside the first Chinese character resolves to
**`[91,1]`**, the range for `世`, through `AXRangeForPosition`.

The same run also passed rendered/source/editor mutability, Unicode ranges,
backward selection, read-only input rejection, native Copy and caret checks.
Child exit 0; clipboard restored and verified. VoiceOver was off during these
geometry checks. Gallery SHA-256:
`ac098d4560365756d15a647a72231337f8f048c8a75f7ff7e11114a14e0a3449`.
The before-fix run used the previously delivered gallery and failed at the expected
partial paragraph range; it is retained alongside the successful result.

## Checks and provenance

Through the isolated toolchain with `GPUIO_JOBS=2`:

- `cargo test --offline --locked -j2 -p gpuio-native --lib --features native-canvas-tests,native-image-tests`:
  **1,179 passed, 2 ignored**. An initial featureless invocation selected zero
  relevant tests and is not counted as validation.
- `dune build -j2 examples/gallery/main.exe` passed before the OS regression.
- Strict workspace/all-targets Clippy with canvas/image/presentation features
  (`-- -D warnings`), workspace formatting and explicit changed-vendor formatting
  passed.
- `cargo test --offline --locked -j2 -p gpuio-native --features native-tests --test native_document`
  passed against an actual macOS window, including document/code/diff interactions,
  streaming selection, native Copy and cleanup. The clipboard was restored and
  verified. Its diagnostic timings are not physical-presentation qualification.
- All **159 GPUI** and **244 Base** files reconstruct exactly from pinned archives
  plus the recorded patches, excluding generated Cargo.lock.

GPUI patch SHA-256: `8666a27801ffecc7e52528216acf5bb0e6ac36f81263c037a3e36e55f6c8a097`.
Base patch SHA-256: `a390c8a2026e7bfd4e9bb3bdb4a9df9ab1311e8cd9052da871a831c322d5baee`.
Exact commands, failed and passing reports, source snapshots and reconstruction
hashes are in [reports.tar.gz](rendered-visual-lines-och17/reports.tar.gz) and the
[SHA-256 manifest](rendered-visual-lines-och17/manifest.json). Source provenance is
the dirty tree based on `7a4138e3`, before the delivery commit.

## Separate VoiceOver setup finding

Further probes confirmed the actual VoiceOver process starts, but neither the
last-phrase clipboard command nor caption-window discovery yielded speech text.
Reading `last phrase` through AppleScript timed out. These are not reading passes.
VoiceOver Utility's scripting checkbox was off; attempting to enable it waited
inside `SFAuthorization`/`AuthorizationCopyRights`, as shown by the process sample.
The setting remained false. The owner was asked to complete that macOS setup
manually; no authorization bypass was attempted. Only the test-owned utility and
script were closed. VoiceOver was restored off. Filesystem, Linear and GitHub
access remain working; this is a distinct macOS authorization prerequisite for
that automation route, not a Codex session-permission failure.
