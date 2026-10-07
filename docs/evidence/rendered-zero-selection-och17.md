# Empty inline-object selection — OCH-17

Local macOS checkpoint, 2026-10-07, following
[declared custom block glyph selection](rendered-custom-selection-och17.md).
OCH-17/OCH-41 and milestone 07 remain open. This integrates empty-copy inline
objects into the common native selection model; it does not finish opaque block
widgets, rich AX selection or release qualification.

## Contract and implementation

A visible inline object can have an empty plain Copy alternative. Its selection
must differ from a caret without adding an invisible placeholder character to
Copy. Logical endpoints now carry `(preparation identity, UTF-8 byte offset,
object boundary)`. Consecutive empty atoms at the same byte own ordered edges.
Declared empty block Text remains ordinary text and consumes no object edge.

`position(byte)` is the canonical position before empty objects at that byte.
`selection_for_part(index)` addresses checked owner edges, and `full_selection()`
includes trailing/only empty objects. `is_collapsed()` and `is_backward()` use
logical edge order; `bytes()` still returns only real Copy bytes. An empty byte
range alone cannot answer whether an object is selected. Captured native endpoints
validate both the exact immutable preparation and the boundary slot.

Native requests stage owner locks before changing selection. The existing inline
object wrapper paints its selected state; plain Copy stays empty and Source Copy
can emit that specific object's declared Markdown. Double/triple click and pointer
drags use the same owner edges. The passive glyph child no longer registers a
second unmapped text area inside its atomic parent. Zero-byte structural boundary
parts prevent paragraph selection from absorbing an adjacent empty paragraph.

Compatible append/resource rebinding checks ordered empty-object occurrence
metadata, in addition to the existing source/AST-transfer and text checks. Weak
projection identities and shared name/Markdown/source-span metadata retain no
parent AST or native view. An old byte offset cannot silently select a different
empty object merely because both copy as the empty string. Part/fragment storage
and retained metadata are included in conservative preparation accounting;
existing source/generated-string/part count limits remain in force.

## Evidence

Five targeted tests cover independently addressing adjacent empty objects,
backward versus collapsed requests, plain/Source Copy, All across a compatible
append without selecting new atoms, resource refresh/stale requests, replacement,
native double/triple click, both drag directions, and adjacent paragraph boundaries.

The macOS window suite checks actual cyan GPU selection pixels, keyboard Source
Copy of the second object, compatible streaming retention, and clearing. It emits
`GPUIO_EMPTY_ATOMIC_SELECTION_OK` and completes the existing full native document
suite. These dispatched events are not physical keyboard/IME or VoiceOver acceptance.

Local macOS arm64 qualification passed:

- **1,110 native library tests passed, 2 ignored** (native-008), followed by the
  complete real-window document suite (gpu-013). The latter uses the corrected
  node Copy-mode fixture; production selection code is unchanged between them.
- Full Rust workspace tests, strict Clippy with native features, Rust formatting,
  and `dune build -j2 @all @runtest @fmt` passed through the isolated toolchain.
- Freshly rebuilt gallery shutdown-014 passed rendered reading order, actual Copy,
  and clean child exit. Typed clipboard contents were restored by the drivers.
- Vendored Base reconstruction matched **237 files** exactly, excluding
  `Cargo.lock`; patch SHA-256
  `3e633f512159c0980ca700c15ab59ae2f61542d27835acfb8441e73dbcba98e1`.
- Example documentation structural audit remains **429 sources / 266 reviewed
  groups / 0 pending**; `git diff --check` passed.

The [archive](rendered-zero-selection-och17/reports.tar.gz) and
[manifest](rendered-zero-selection-och17/manifest.json) contain **27 files /
523,738 compressed bytes**, verified member by member. Metadata records exact
final commands, source hashes, parent commit and gallery binary hash. Intermediate
failures and their commands remain included. No VoiceOver/system settings were
changed. Hosted run `37612466848` covers the older `4162d450` checkpoint, with
Linux foundation passing and macOS still running when this evidence was recorded;
it does not validate this local change.

Intermediate evidence is retained honestly: `rendered-zero-first.log` compiled with the test module
feature disabled and ran zero relevant tests. Targeted-003 used a nonexistent test
fixture API. Targeted-004/005 reproduced the duplicate passive-child barrier and
corrected a resource fixture that had changed parser configuration. GPU-009 painted
the selection but failed Source Copy because its presenter-only flag was overridden
by the node property; GPU-013 uses `Op::SetDocumentSelectionFormat`, the actual
application API. No production Copy assertion was relaxed.

## Remaining work

Opaque/NonText **block** widgets need their own native whole-object selection and
interaction wrapper; zero-byte block requests are rejected as unmapped instead of
acknowledging an unpaintable selection. Broader reflow/virtualization/cross-view/stream behavior, measured selection hot-path cost,
rich rendered AX publication/actions and all remaining macOS release gates still
need completion. No Linux GUI, physical frame-presentation or distribution
acceptance is inferred.
