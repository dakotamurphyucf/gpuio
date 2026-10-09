# Opaque block selection and frozen Copy — OCH-17

Local macOS arm64 checkpoint, 2026-10-07. OCH-17/OCH-41 and milestone 07 remain
open; this does not qualify rich rendered accessibility or the complete release.

## Behavior

Opaque/NonText custom Markdown blocks now have whole-object native selection
around their actual child layout. Empty Copy alternatives use ordered owner edges,
so selecting an object is distinct from a caret without adding synthetic text.
Native requests, background double/triple clicks and drags share that ownership;
Source Copy identifies the selected occurrence. Highlighting is painted after the
child content so opaque backgrounds cannot hide it.

Typed Text/Object ownership prevents a resource change from leaving a whole-object
flag over partial glyph selection. Compatible appends retain unchanged blocks;
changed occurrences with identical Copy alternatives are rejected. Weak projection
owners do not keep obsolete parsed blocks alive. Unpainted blocks can still be
selected and copied before virtualization realizes them.

Native child controls keep their semantics and interaction. Button, checkbox,
radio, toggle, switch, slider and link mouse-down handlers suppress parent text
selection while preserving activation/focus propagation. Native Input retains its
own selection and typing. Arbitrary custom controls must use the native suppression
or editor adapter described in the [contract](../design/rendered-document-selection.md#opaque-and-nontext-block-ownership);
a role alone does not establish pointer ownership. No new OCaml protocol or
synchronous OCaml paint/input callback is introduced.

The hosted streaming-document failure also reproduces locally: appending inside
a final paragraph dropped the newline from frozen Select All Copy. The old whole
Copy and old projection matched, but the rebound logical range excluded the old
terminal separator. Copy now retains its immutable representation when it differs
from the **new selected range**. Paint still excludes newly appended characters.
New selection/clear retires the snapshot. Window-level Copy keeps its existing
normalization policy.

## Validation

- Eight focused tests cover empty/nonempty Opaque/NonText blocks, repeated
  occurrences, backward ranges, Plain/Source, compatible streaming, changed-node
  cancellation, resource Text/Object transitions, background clicks/drags,
  resize, eight child-control families, 200 unpainted blocks, stale requests,
  replacement and weak-owner cleanup, plus terminal-separator Copy retention.
- **1,118 native library tests pass; 2 existing tests are ignored.** The complete
  Rust workspace, strict all-target native Clippy, Rust formatting and full Dune
  `@all @runtest @fmt` pass through the isolated toolchain with two jobs.
- Actual macOS `native_document` passes the exact assertion that failed in CI.
  `native_highlight_document` passes cyan selection pixel checks, keyboard Source
  Copy, streamed retention and clear for four custom-block variants, plus the
  full existing document checks. Marker: `GPUIO_BLOCK_ATOMIC_SELECTION_OK`.
- Freshly rebuilt gallery reading order, actual Copy and normal close pass, with
  child exit zero and typed clipboard restoration. The native drivers also restore
  typed clipboard contents after failures. No VoiceOver/system settings changed.
- Base reconstruction matches **238 files** exactly (excluding Cargo.lock), with
  patch SHA-256 `cd7440015345f61e8ecc0714290c5203662ddc73e4197a60f087325dacbf2fc0`.
  Example documentation audit: 429 sources / 266 reviewed groups / 0 pending
  (structural audit only). `git diff --check` passes.

The [archive](rendered-block-selection-och17/reports.tar.gz) and
[manifest](rendered-block-selection-och17/manifest.json) preserve exact final
commands, source hashes including the new block module, parent revision, platform,
gallery hash, reports and intermediate failures. Archive contents are verified
member by member. See the manifest for file counts and sizes.

Initial evidence is retained: an empty-block request failed as unmapped before
integration; native Button and the additional control families reproduced parent
selection before suppression. Slider fixture corrections supplied its semantic
Group and required Indicator geometry before reproducing the actual bug. The
bare Input fixture uses actual shaped cell geometry; it does not claim an AX
adapter. The raw-HTML test fixture used different parser semantics from the host
and was replaced with stable ordinary-paragraph cases while retaining the exact
hosted integration regression. Old unit/GPU oracles that expected a lost terminal
newline now separately assert frozen Copy and unchanged logical paint edges.
Production assertions about source identity, selected glyphs and stream bounds
were retained.

## Remaining acceptance

The tests qualify scoped native dispatch, GPU readback and Copy. They do not
establish physical IME/VoiceOver, rich AX TextRun selection publication/actions,
broader reflow/virtualization/cross-view interaction or measured selection hot-path
cost. All remaining macOS catalog, performance/resource, physical presentation,
notices, API and distribution gates remain open.

[Hosted run 37612466848](hosted-run-37612466848.md) covers the older `4162d450`:
Linux foundation and scoped extracted apps pass, while macOS streaming and both
Metal probes fail. Its newline regression is repaired locally here; current-source
hosted confirmation is separate. Both informational Linux GUI smokes fail; full
Linux desktop qualification remains deferred OCH-47. No GUI, clean-machine,
physical-presentation or release acceptance is inferred from compilation.
