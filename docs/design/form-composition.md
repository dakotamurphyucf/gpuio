# Rich form composition — OCH-41 design draft

Status: source review and proposed API; **not implemented or accepted**. The
current `Form.field` helper remains available with its existing contract.

## Pinned source and gap

The reviewed sources are Longbridge GPUI Kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`:
[exports](../catalog/sources/component-form-mod.rs.txt),
[Form](../catalog/sources/component-form-form.rs.txt) and
[Field](../catalog/sources/component-form-field.rs.txt). Snapshots match the
SHA-256-verified dependency archive; the catalog manifest records each blob.

| Source behavior | Current GPUIO mapping and required addition |
| --- | --- |
| Vertical/horizontal labels, independently of form columns | `Form.field` supports both orientations. Add a collection composition; changing label orientation must not redefine the field grid. |
| Multiple columns and per-field span/start/end | Not exposed by Form. Native Style currently has column counts but no field grid placement. Choose a coherent validated placement primitive before implementing collection layout; regrouping controls into fresh keyed rows must not destroy native editor identity. |
| Rich optional labels/descriptions and arbitrary content | Current helper uses semantic strings and one native field-compatible control. Add rich slots and arbitrary view content without weakening its existing direct control/metadata association. Rust render closures become Bonsai-produced views, never synchronous FFI callbacks. |
| Label indentation, width, text size and item alignment | Current label width/style can be refined; richer composition needs explicit shared defaults, per-field override rules and behavior for absent labels. |
| Size-dependent form/field spacing | Define typed size policy, with ordinary style refinements. Exact source pixels are not required; control-specific configuration remains caller-owned. |
| Full-width trailing footer | Add an optional rich footer after every field, spanning all columns. Footer controls/state/tasks stay application-owned. |
| Required marker and semantic metadata | Existing `Accessibility.Field` and `Form.field` supply this. Rich display slots must not replace an actual input's semantic name/help/error with an unlabeled group. |
| Visibility | Source stores `visible`, but its pinned render method never reads it. Define actual caller-controlled removal or retained hiding explicitly; do not copy an ineffective flag. |
| Style refinement | Pinned Field applies its style, while Form's render method omits its stored style. GPUIO should honor documented root/field/slot refinements. |

Unchecked source column casts, zero/oversized spans and implicit positional IDs
are not invariants to copy. Stable caller field keys, validated bounds and explicit
placement semantics are needed. Existing `Form.Field` aliases semantic metadata;
do not repurpose that module incompatibly as a new rich field object.

## Proposed public shape

Keep `Form.field` unchanged. Draft a separate `Form.Item` domain with a required
stable key, content and optional display slots. A form constructor accepts items,
column/layout/size policy, optional footer and style refinements. Use distinct
validated placement types rather than unqualified integers for spans versus grid
lines. Review whether explicit signed grid lines are required or can be expressed
through the existing general layout API before freezing the interface.

For a normal input, attach `Accessibility.Field` to the native control through
the existing validated operation; the rich item places that decorated control.
For arbitrary content, the application supplies meaningful semantics to each
interactive child. Label/help/error display is separate from native metadata, so
a decorative label cannot silently erase an input's accessible name.

This is a draft, not a promised final signature. Review call sites and native
placement requirements before implementation. Add no editor owner, persistence
service, validation scheduler or native callback registry to Form.

## Acceptance before changing the catalog status

- Core validation and reconciliation: duplicate keys, placement bounds, missing
  slots, stable native controls when columns/orientation/annotations change.
- Public OCaml gallery: rich label/description, unlabeled and required fields,
  multiple controls, mixed spans and full-width footer with real actions.
- Native geometry/paint: columns independent of label orientation, shared and
  overridden label widths, sizes, clipping and trailing footer in both themes.
- Native keyboard/metadata and lifecycle: text draft/focus retention, required
  help/error associations, child actions, reorder and hide/remount semantics.
- Fresh installed consumer using public APIs, with scoped owner/resource cleanup.

Keep the Forms row pending until those requirements pass. Settings source-row
acceptance does not imply this separate collection API has shipped. Required
Linux non-GUI and OCH-17 release qualification remain separate gates.
