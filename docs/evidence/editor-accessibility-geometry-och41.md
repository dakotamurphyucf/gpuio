# Editor accessibility range geometry — OCH-41

Local implementation checkpoint, 2026-10-07, based on `e5742947`, macOS 14.5
arm64 / Apple M1 Max. Scoped local integration checks pass. OCH-41 and
OCH-17 remain open; this is scoped geometry evidence.

## Reproduction and repair

The public gallery's **Text editing → Multiline & search → Working notes**
inspector returned the correct three-line rectangle for
`a\nMMMMMMMMMMMM\nz`: `(289, 450.5, 164.1, 78)` in logical window-content pixels.
macOS `AXBoundsForRange` simultaneously returned success with an empty rectangle
`(0, 1117, 0, 0)`. The adapter published source text and selection offsets but
omitted the character geometry required by AccessKit's range consumer.

The [repair contract](../design/editor-accessibility-geometry.md) uses an
immutable Rust-only Base layout snapshot published during prepaint. The enclosing
synthetic accessibility callback runs after child prepaint and uses that frame's
source identity, cluster extents and scale. It does not use the previous paint,
shape text again, call OCaml, poll or schedule another redraw.

Text runs split at wraps, direction changes and discontinuous glyph extents.
Members of a shaped cluster share its extent; line links join directional runs
belonging to the same visual row. Source byte intervals and revisions identify
runs so a reflow cannot reinterpret a stale character index. UTF-8/CRLF offsets,
directed selection and existing ownership/composition/disabled guards remain.
Empty source produces an empty caret, never placeholder text. Off-layout text
remains readable/selectable without fabricated bounds. Private editor adapters
remain excluded and masked layouts clear their snapshot.

## Local checks

All build commands use the isolated repository toolchain and two jobs.

- Final native full suite: **1,072 passed / 2 existing ignored**.
  Tests cover a source edit published in exactly one draw, wraps, horizontal
  scrolling, synthetic 1×/1.5×/2× scale, empty source, mixed-direction line links,
  revision/reflow selection routing and releasing geometry when AX deactivates.
- Strict Clippy for native/protocol, all targets, with
  `native-canvas-tests,native-image-tests,presentation-diagnostics`: passed.
- Hash-verified Base reconstruction matches the source tree. The comparison
  excludes only an ignored, locally generated `Cargo.lock`; the upstream
  archive, patch and pinned dependency versions are otherwise unchanged.
- Root and freshly installed gallery drivers each pass **eight cases**:
  intermediate wide line, soft wrap, empty source, blank lines, CRLF,
  combining marks/joined emoji/Japanese, RTL glyph bounds, and mixed-direction
  text. Every scalar of the joined family emoji shares the cluster's rectangle;
  English/Hebrew/English text remains one AX line with correct RTL character boxes.
- Full Dune `@all @runtest @fmt`, Rust formatting, documentation and catalog
  audits pass. Coverage remains **429 sources / 266 reviewed groups / 0 pending**.
- Fresh staged installation and an independent composed gallery backend build
  pass, including both extension/document-profile catalog handshakes. Installed
  application checks use the same machine/toolchain; this is not clean-machine
  or signed-distribution acceptance.

The new `scripts/test_gallery_editor_geometry.py` uses actual AX value/selection
actions and parameterized range queries, then the public OCaml/Eio inspector.
It converts the OCaml result through the independently queried AX window-content
origin. It checks text/selection nonmutation and always closes/reaps its child.
These are AX injection/actions, not physical keyboard, IME or VoiceOver tests.

An initial driver run used the prior binary while the new build was still
linking; its recorded hash and failing empty AX result preserve the reproduction.
The first new-binary run exposed an assertion error for a trailing newline:
the public query includes its ending caret row, while AccessKit's nonempty text
range excludes that empty line. The corrected comparison unions the separately
queried AX end-caret box with the text box. No runtime behavior was changed to
make that assertion pass.

The existing `test_macos_text_selection.py` walkthrough initially could not find
its startup window and reached no input assertion. Its untraced application log
was empty; the cause remains unknown. After adding close tracing and durable
failure reports, the same binary passed source/editor Unicode selection, actual
keyboard select-all/backward selection, read-only copying/deletion rejection,
clipboard restoration and normal shutdown. The traced repeat records only the
expected final close request; no production change was made for this failure.

## Reproduction commands and artifacts

Build and native checks use `GPUIO_JOBS=2 ./scripts/gpuio exec`:

```sh
cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j2
cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-canvas-tests,native-image-tests,presentation-diagnostics --offline --locked -j2 -- -D warnings
dune build -j2 @all @runtest @fmt
cargo fmt --all --check
```

Run the physical geometry driver after building the gallery:

```sh
python3 scripts/test_gallery_editor_geometry.py --report <new-report.json>
python3 scripts/test_macos_text_selection.py --output <new-directory>
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace <new-workspace>
python3 scripts/test_gallery_editor_geometry.py --executable <new-workspace>/consumer/_build/default/main.exe --report <new-installed-report.json>
```

The macOS CI workflow now runs the geometry driver and preserves its report/logs.
The [archive](editor-accessibility-geometry-och41-logs.tar.gz) and
[manifest](editor-accessibility-geometry-och41-manifest.json) retain exact local
commands/results, initial failures, app traces, binary/source hashes and the
qualified patch. The Base reconstruction comparison excludes only its ignored
local Cargo lockfile. Scratch files are not build dependencies.

Current-source hosted/Linux checks,
physical IME/VoiceOver, whole-editor-family behavior, performance and release
acceptance remain separate. No VoiceOver or desktop settings changed.
