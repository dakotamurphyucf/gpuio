# Sidebar activation — OCH-41

2026-10-02, macOS arm64 checkout `milestone-07-gallery-release`, base `83eb87e`
plus local milestone changes. Stock OCaml 5.3/Bonsai v0.17/Core/Eio toolchain;
no dependency or Rust/wire format changes.

The exact pinned `SidebarMenuItem` supports click-to-open and click-to-toggle,
then invokes its destination handler. `Sidebar.Item.Activation` now expresses
that choice as `Select_only` (default), `Expand` or `Toggle`. Eligible user Select
requests reduce against the latest item policy. Expand is idempotent, Toggle
preserves each accepted request, leaves only select, and disabled/hidden/stale
items do nothing. Compact mode updates the retained expansion preference while
children remain hidden. Programmatic `Sidebar.select` still changes selection
only; the independent caret still changes expansion without navigating.

The Journeys gallery has **Branch selection: navigate/expand/toggle**. Cycling
policy replaces the immutable collection while preserving native links, branch
owners, selection and expansion. The existing sidebar remains the renderer;
there is no extra timer, native state owner or serialized policy copy.

Two added expect tests cover:

- All three policies, repeated requests, child expansion retention, leaf behavior,
  disabled items and sidebar, hidden offcanvas, compact preferences, independent
  caret and programmatic selection, plus a request captured before policy changes.
- Actual reconciler-produced native link/handler IDs, selection request dispatch,
  model-only policy replacement, retained disclosure descendants and repeated
  selection events without Create/Remove/Bind operations.

Full OCaml tests, formatting and the public gallery build pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest @fmt examples/gallery/main.exe
```

The fresh installed-library gallery consumer also passes:
`INDEPENDENT_EXTENSION_CONSUMER_PASS example=gallery run=False`.

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace scratch/agents/root-20260929-m7-resumed/sidebar-activation-installed-gallery
python3 scripts/audit_component_catalog.py
git diff --check
```

The catalog hash/structural audit and whitespace checks pass. No real OS windows
were opened. This is pure model and
bridge reconciliation evidence, not physical keyboard, AX/VoiceOver or GUI
acceptance. The [source review](../catalog/journey-workspace-review.md) explicitly
retains the separate per-label styling and other family gaps. OCH-41/milestone 07
remain open.
