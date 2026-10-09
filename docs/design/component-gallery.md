# Component gallery contract (OCH-41)

The gallery is a native OCaml application using installed public `Gpuio`,
`Gpuio_bonsai` and `Gpuio_eio` APIs. It is a browsable component workbench, separate
from the agent-chat and Signal Studio application showcases. Existing examples
and their behavior tests remain useful; a screenshot is not parity acceptance.

## Catalog and provenance

`docs/catalog/sources/manifest.json` pins the unmodified upstream entry points,
GPUIX host/native contracts and licenses by revision and SHA-256. Snapshots come
from exact Git objects, not an author's modified checkout. The coverage ledger
must account for every public family, including grouped helper/infrastructure
modules, deferred editor/docking/grid capabilities and unsupported platforms.
GPUIX style fields and event names are audited from source, not just the older
research table: that table omitted `onVisibleRange` and `onHighlight`.

Each reviewed row records upstream surface, GPUIO public API, configuration and
commands/events, styling/accessibility behavior, owning ticket, runnable example,
behavior evidence, scope/status and actual platform coverage. Shared runtime
helpers map to the owning public contract. An existing source file is not proof
of full behavioral equivalence. Missing or partial semantics must remain visible.

## Application structure

A pure page/model library defines stable page identity, appearance and preview
scale. Per-window Bonsai state owns navigation and sample values; Rust retains
native input, popups, focus, layout and animation. Preview controllers are
constructed in lazy page branches so leaving a page unmounts native leases and
cancels its scoped resources. State persistence is explicit; there is no global
editor/controller registry or synchronous callback from Rust into OCaml.

The shell provides keyboard-reachable navigation, light/dark appearances,
compact/comfortable/large preview sizing and a separate-window action. Page
controls demonstrate their meaningful states and report semantic results. Group
related families: foundations/presentation, controls, text/numeric inputs,
date/color, overlays/commands, navigation/workspaces, collections/documents,
graphics/motion and desktop services. A page uses ordinary public views and
controllers; it does not embed the test-only native bridge or spawn external
example processes to stand in for interactive component coverage.

## Acceptance

- Full pinned catalog and style/event mappings, with no unreviewed v1 rows.
- Public gallery examples for every committed v1 family; meaningful state changes,
  keyboard behavior, theme/scale and repeated page/window teardown coverage.
- Pure expect tests for gallery navigation/state invariants, plus native macOS
  semantic queries and interaction tests. Bounds/screenshots supplement behavior.
- Clean checkout/installed-consumer builds and runnable commands; no scratch,
  private runtime APIs or upstream checkout required at runtime.
- Adapter-author, upgrade/reconstruction and license guidance linked from the
  catalog, including styled-layer compatibility limitations.
- macOS functional acceptance and required Linux build/unit/consumer checks;
  Linux desktop qualification remains OCH-47 under the platform release policy.

This document defines the implementation contract, not completed acceptance.
