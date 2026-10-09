# Geometry and element helpers — OCH-41

Reviewed against GPUI Kit `84f57fdfcb4910623fb0bb7f795b077e249f9271` on
2026-10-04. The original Base geometry/element-extension and Component
element-extension sources are retained in [the manifest](sources/manifest.json).
This maps infrastructure to its owning APIs; it does not certify desktop behavior.

| Pinned helper | Public mapping and limits |
| --- | --- |
| `Placement` Top/Bottom/Left/Right and axis predicates | `Placement.Side` has the same four alternatives; `Placement.Align` and `Placement.at_point` additionally specify alignment/corners. Native popup fitting owns measured anchor/viewport conversion. A caller can pattern-match the closed side type to derive an axis. [Placement contract](../design/placement-geometry.md). |
| `Side` Left/Right and `AxisExt` predicates | Ordinary closed orientation/side variants on split, list, slider and placement APIs. These are value helpers, not independently mounted components or native resources. |
| `LengthExt::to_pixels` | `Length` describes logical pixels, percentages or Auto; GPUI resolves layout-relative units. The public API deliberately does not claim that a percentage can be resolved without its property's native context. There is no general OCaml `rem` length or native `AbsoluteLength` object. Applications can derive explicit metric values in their style model. [Length interface](../../lib/core/length.mli). |
| Generic `Edges<T>` and `all` | Per-edge padding, margin, border, radius and offset declarations, plus checked shorthands in `Style`. Last declaration wins per expanded field. Widget-owned insets have dedicated contracts; they do not imply platform safe areas. [Style interface](../../lib/core/style.mli), [sheet insets](../design/sheet-insets.md). |
| Base `text_selection_scope` | Native focus/overlay owners assign stable selection scopes; the host activates the current trap's scope before rendering `TextSelectionLayer`. `View.focus_scope` and modal compositions provide application-facing ownership. Arbitrary scope IDs do not cross the FFI. [Window selection](../design/window-selection.md). |
| Base `on_prepaint` | A synchronous Rust callback over resolved bounds. Custom native components can use GPUI layout/prepaint through the [extension SDK](../design/extensions.md). It is not exposed as a synchronous OCaml hook. Native responsive `Container_query` rules handle size-dependent branches without a round trip; their observations report dimensions only when selection changes. Specialized split/list/table/editor/window APIs expose their documented measurements. There is no generic per-element bounds subscription in the current public OCaml API. |
| Component `ChildElement` / `AnyChildElement` | Rust type erasure plus child index and size injection. OCaml `View.t` composition, explicit component size/style and sibling keys supply the corresponding application construction behavior. A builder's positional index is not a stable identity for changing records; use keys and generation-checked controllers. No Rust closure is serialized. |

## Evidence and boundary

`test/view_api/placement_geometry_test.ml`, `placement_test.ml` and
`container_query_test.ml` check public validation, style/placement serialization,
responsive predicates, branch identity and stale observations. The current full
OCaml suite passed at the [selection checkpoint](../evidence/window-selection-och41.md).
Native popup fitting, content-aware client frames, responsive layout and selection
scopes have their own [placement](../evidence/placement-geometry-och41.md),
[frame](../evidence/window-frame-och41.md),
[responsive](../evidence/container-queries-och26.md) and
[selection](../evidence/window-selection-och41.md) evidence.

The Runtime gallery demonstrates native window geometry; Overlays demonstrates
placement and insets; responsive examples use `View.container_query`. They do not
demonstrate a general OCaml prepaint callback. A native extension may measure its
own component and enqueue a bounded schema-defined observation; the SDK does not
grant it arbitrary access to another OCaml view's layout. General editor range
geometry remains in [the editor review](editor-review.md).

This review adds no new wire protocol, wrapper layout nodes or paint polling.
Current physical gallery, fractional display-scale/multi-monitor and release
qualification remain open; earlier targeted desktop results do not cover the
entire expanded gallery.
