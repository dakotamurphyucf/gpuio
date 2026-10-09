# Application document defaults — OCH-41

Local macOS arm64 checkpoint on `83eb87e865c86717a8bc51b9db6fe1f379d909a9`
plus the working tree. See the [contract and usage](../design/document-defaults.md).

## Implemented path

`Document.Setting` distinguishes Inherit, Builtin and Value. Existing Config.create
arguments remain source-compatible but retain whether they were supplied.
`Config.with_overrides` changes only supplied fields. Immutable application defaults
cover reader appearance/layout, line numbers, initial collapse, selected-copy format,
preview limit, internal style, parser options and code/table actions. Source-specific
resources and diff state remain explicit per document.

Defaults flow through `App.run` and `run_desktop`, the application record, every new
window driver and Core reconciliation. Both old and new documents resolve before
wire comparison, callback binding and epoch checks. There is no new opcode or Rust
mutable global. Unchanged view subtrees retain the existing physical-sharing shortcut.

Shared action/profile callbacks receive the resolved Config so they can identify
the source. Explicit local callbacks/profiles override inherited ones. The Core and
Bonsai `without_document_profile` modifier clears a profile and suppresses inheritance.
Returning to an ordinary document view restores inheritance with a fresh native epoch.

Inherited settings apply only where supported: Markdown parser options, rich-reader
styles/actions/profiles, and Flow preview limits. Explicit incompatible overrides
remain errors. Resolution preserves original intent, allowing the same immutable
config to resolve differently in separate applications without leaking earlier values.

## Behavior evidence

Five Core expect cases pass, covering:

- Omitted versus explicit built-in values, independent per-field reset and restored
  inheritance, modifier preservation and resolution without erasing original intent.
- Inapplicable inherited values across Markdown/HTML/Code/Diff/Viewport, invalid
  limits/layouts and rejection of explicit incompatible settings.
- Default profile callbacks retaining per-source Config and mode restrictions.
- Separate reconcilers with different defaults, unchanged-subtree silence, inherited
  profile events, failed-prepare rollback, explicit clear and restored inheritance.
- Shared action callbacks, local callback refresh without native changes, explicit
  action reset, and rejecting custom actions that have no usable handler.

An Eio inline test creates actual native bridge allocations but never runs the GUI
loop. It injects Opened replies, inspects actual window-driver transactions and
verifies inherited appearance, explicit override, a later-created window and a
separate application without defaults. Existing runtime tests pass. This is
application/driver evidence, not physical native rendering or OS-window acceptance.

The gallery has an opt-in `--document-defaults` launch mode using the real desktop
runner and a Documents comparison of Inherit, Built-in and Compact override. Its
normal startup retains previous defaults. The comparison uses a scoped registered
source and explicitly suppresses any inherited native profile.

## Commands and limits

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @test/view_api/runtest
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @lib/eio/runtest @test/runtime/runtest
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest
GPUIO_JOBS=2 ./scripts/gpuio check-fmt
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace /private/tmp/gpuio-document-defaults-gallery-20261004
python3 scripts/audit_component_catalog.py
git diff --check
```

All five Core cases, runtime tests, full `@all @runtest`, final formatting and the
fresh independent installed-gallery consumer pass. A strengthened final Core run
also checks inherited/reset line numbers, initial collapse and layout-dependent
preview limits. The consumer ran its copied package expect test and reported
`GALLERY_CATALOGS_PASS counter=1 document_profile=1` followed by
`INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`. Structural catalog
and whitespace checks pass. Formatting initially rejected adjacent App.run docstrings;
they were merged without changing the interface or behavior.
Initial implementation checks corrected a missing `Core` qualification and one
unmatched parenthesis in the runtime test. No dependency pins, switches or vendor
sources changed. No physical windows, native clipboard/VoiceOver or GPU validation
are claimed. Defaults are immutable for a run; a live setter is not part of this API.
Virtual offscreen focus/clipping, remaining catalog coverage and the OCH-17 physical
macOS, resource/performance, notices, distribution, review and required Linux gates
remain open. This does not complete the milestone.
