# Native logical pointer selection — OCH-17

Local macOS checkpoint, 2026-10-07, based on `249d65fe`. This extends
[endpoint capture](rendered-pointer-endpoints-och17.md) into native selection
ownership. OCH-17/OCH-41 and milestone 07 remain open.

## Behavior

Mapped same-document pointer drags now apply their stamped anchor/head to the
existing native text owners. The state retains one directed range for painting
and exact plain Copy, distinguishing pointer origin from adapter requests.
Window selection continues to own the gesture and autoscroll. No OCaml paint or
input callback, parser job or second native selection controller is introduced.

The common range validates against the installed preparation. Reflow preserves
valid ranges; compatible append/resource changes retain direction and origin
with new preparation identity. Full replacement and explicit clear still retire
them. Exact partial plain Copy excludes unselected structural separators; native
source-format reconstruction remains available.

## Qualification

Commands run through the isolated repository environment with `GPUIO_JOBS=2`:

- `cargo test --offline --locked -j2 -p gpuio-native --lib --features
  native-canvas-tests,native-image-tests`: **1,092 passed, 2 ignored**.
- `cargo clippy --offline --locked -j2 -p gpuio-native --all-targets --features
  native-canvas-tests,native-image-tests,presentation-diagnostics -- -D warnings`:
  **passed**.
- `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests
  --test native_highlight_document`: **passed** on the actual macOS renderer.
  The added backward pointer case checks visible selection pixels, window Copy,
  inherited/local selection colors and retention through streamed publication.
  Existing request, disabled-selection, mixed-participant and lifecycle checks
  also pass. Input is dispatched through native event routing; this is not
  physical mouse or OS accessibility-selection acceptance.
- Base reconstruction from pinned upstream `84f57fdfcb4910623fb0bb7f795b077e249f9271`
  matches **237 files**, excluding the ignored generated `Cargo.lock`. Patch
  SHA-256: `0b0fa110c9400d5c8f48f43cf47338fc239d135eae28e692ed81b7b8ea6a7ff9`.
- `dune build -j2 @all @runtest @fmt`: **passed**, including rebuilt example
  applications. `cargo fmt --all` and the example documentation audit also pass;
  coverage remains 429 sources / 266 reviewed groups / 0 pending.
- `python3 scripts/test_macos_document_shutdown.py --log … --report …`:
  **passed** using the rebuilt gallery, with both Rust backtrace variables set
  to `1`. Actual native Select All/Copy followed by window close exits normally.
  Full typed clipboard contents are restored and owned children are reaped.
  No VoiceOver or desktop settings were changed.

The endpoint tests cover duplicate text, forward/backward identity, offscreen
anchor retirement from the realized semantic tree, resize, compatible append,
old-position rejection and equal-source replacement. Frontmatter verifies that
selecting its value copies the exact value without an unselected block newline.

The archive records exact commands, source hashes and the rebuilt gallery hash.
Shutdown-015 qualifies that binary. The earlier shutdown-012 pass used the
preceding gallery binary and is retained only as a historical control.

## Failures retained

The resize regression first failed because the legacy prepaint path cleared a
valid logical range after bounds changed. The fix keeps that invalidation for
geometric selections while preserving current logical ranges.

The next append test failed after successfully transferring the range. A temporary
backtrace identified `TextView::request_layout -> set_markdown_extensions ->
increment_update`: the fixture installed default parser settings into a view
with application-specific settings. The fixture now prepares with the mounted
presentation's settings. Production correctly retains its parser-change
invalidation. Diagnostic instrumentation was removed.

The full suite also exposed the old frontmatter expectation of an added trailing
newline. The new exact-range contract deliberately excludes that unselected
separator; the test asserts both exact state Copy and the window Copy provider.

## Remaining scope

Multi-click and cross-participant logical ranges, preserved Select All overrides,
custom declared glyph/copy-alternative mapping and bidirectional highlight
geometry still need integration and qualification. Raw capture can exist where
native adoption is unavailable; an absent common range is not proof of no native
selection. Staged pointer updates currently scan the bounded projection, so
hot-path performance still needs measurement.

Rich accessibility TextRun publication, guarded OS actions, actual AX/VoiceOver,
Linux graphical behavior, physical presentation and broader release acceptance
are not established by this checkpoint. Current hosted checks run at `249d65fe`,
which predates these changes.

Logs, initial failures, source patch, command metadata and reports are in the
[evidence archive](rendered-pointer-selection-och17/reports.tar.gz), with a
[verified manifest](rendered-pointer-selection-och17/manifest.json).
