# Document selection format — OCH-41

Local macOS arm64, 2026-10-03, working tree based on `83eb87e`. Public
`Document.Selection_format` and `Document.Config.create ?selection_format` now
expose Plain_text (default) and Markdown copy behavior. The Documents gallery adds
**Copy selection as Markdown**. See the
[contract](../design/document-selection-format.md).

Append-only Op116 updates a retained DocumentView setting. The Core reconciler
compares the existing document wire payload separately, so a format-only update
does not republish content or restart parsing. Rust updates an existing Markdown
state immediately, supplies the mode when a prepared snapshot arrives, and
preserves it through rendering. Source fallback/code/diff and dedicated Copy
source/code/table actions retain their behavior.

## Local validation

- `cargo check --offline --locked -j 2 -p gpuio-native` passes in the isolated
  repository environment.
- `dune build -j 2 @all @runtest @fmt` passes through `./scripts/gpuio exec`,
  including all OCaml examples and the gallery. The new expect test verifies
  independent opcode bytes, one retained node, no Set_document on format-only
  changes, explicit reset to plain and silence for unchanged config.
- The focused native library test `document_copy_format` passes with
  `--features native-image-tests`. It mounts the production document through
  TestPlatform, publishes Unicode Markdown, waits for the actual worker result,
  selects the body, switches formats repeatedly and checks returned selected
  text, retained Markdown entity/source revision and unmount release.

The native test checks both selected-content extraction and the window copy
provider, not a physical keyboard or system clipboard. It does not claim actual desktop focus, IME, VoiceOver or GPU
acceptance. General native selection behavior remains covered by the existing
document tests; new physical gallery validation remains in OCH-17.

## Window-copy regression and fork reconstruction

Extending the test beyond `TextViewState.selected_text` exposed a real mismatch:
the window participant callback always trimmed paragraph-edge whitespace. The
selected source contained a trailing newline, but the window copy result dropped
it. The stronger test failed with that exact difference before the fix.

The existing GPUI Base adaptation now trims only rendered/plain copy. Source copy
preserves its own bytes, including select-all's original source. The callback uses
the state's effective format, so a format that cannot produce source (such as
upstream HTML) retains its existing plain behavior. No window-global trimming or
neighboring participant policy is changed. The stronger native test passes after
the fix and still checks selection identity/revision and teardown.

The two-file vendor change is retained in `third_party/patches/gpui-base.patch`,
with the corresponding `third_party/sources.json` hash updated. Reconstruction:

```sh
python3 scripts/vendor_gpui_base.py \
  --archive scratch/agents/root-20260912-milestones/gpui-kit-84f57fd.tar.gz \
  --output scratch/agents/root-20261003-release-notices/base-copy-format-reconstruction
```

All **233 reconstructed files** match the vendored tree byte-for-byte. The local
ignored `vendor/gpui-base/Cargo.lock` is excluded as in the prior reconstruction
audit; no other file difference is ignored. The upstream revision is unchanged.

## Consolidated checks

The full protocol suite passes **372 tests**. The full native library suite with
`native-image-tests,native-canvas-tests` passes **816 tests**, with two existing
private-D-Bus tests ignored on macOS. The separate native admission test passes
wrong-kind atomic rejection, stale-generation rollback, explicit clearing and
plain default on a recycled node slot. Native tests were repeated after the
window-copy fix; earlier protocol/admission results remain applicable because
that fix changes no wire/admission code.

Strict `cargo clippy --offline --locked -j 2 -p gpuio-protocol -p gpuio-native
--all-targets --features native-image-tests,native-canvas-tests -- -D warnings`
passes. Final `dune build -j 2 @runtest @fmt examples/gallery/main.exe` passes after
the vendor fix. A fresh independent consumer also passes:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace /private/tmp/gpuio-document-format-gallery-final-20261003
```

The result is `INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.
The catalog structural audit and `git diff --check` pass. Detailed logs remain
under the agent's OCH-41 scratch notepad, `document-format-*.log`; these local
paths are evidence artifacts, never build inputs. Hosted/Linux execution of this
revision is not claimed.

## Release limits

This addresses the selection-format row only. Line previews were added in the
[subsequent preview slice](document-preview-api-och41.md). Richer internal
presentation, standalone reader HTML, parser options and static document
renderers/plugins remain explicit [catalog gaps](../catalog/documents-review.md).
Required Linux/hosted checks, physical macOS and full release qualification stay
open. Historical packaged reference executables predate this API addition and
are not relabeled as containing it.
