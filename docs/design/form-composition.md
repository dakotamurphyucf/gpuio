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

## Placement contract to implement first

The pinned GPUI represents a complete grid location as two ranges of
`Auto | Line(i16) | Span(u16)`, and its style refinement replaces that whole
location. `col_span` writes both column endpoints; full span is `Line(1)` to
`Line(-1)`. The row helpers mirror the column helpers. See the pinned
[geometry](../../vendor/gpui/src/geometry.rs),
[styled helpers](../../vendor/gpui/src/styled.rs) and
[Taffy conversion](../../vendor/gpui/src/taffy.rs).

Use one atomic `Style.Property.Grid_location` value containing both axes. This
matches native refinement and can express all endpoint combinations without
silently resetting an unrelated axis through independently exposed row/column
properties. A state-specific location replaces the whole base location; callers
include both axes when both must be preserved. Removing a state declaration
reveals the base location. Do not imply CSS-style independent endpoint cascading.

Proposed types, still unimplemented:

```ocaml
module Grid_location : sig
  module Line : sig
    type t [@@deriving equal, sexp_of]
    val of_int : int -> t Or_error.t
    val to_int : t -> int
  end

  module Span : sig
    type t [@@deriving equal, sexp_of]
    val of_int : int -> t Or_error.t
    val to_int : t -> int
  end

  module Edge : sig
    type t = Auto | Line of Line.t | Span of Span.t
    [@@deriving equal, sexp_of]
  end

  module Axis : sig
    type t [@@deriving equal, sexp_of]
    val create : start:Edge.t -> end_:Edge.t -> t
    val auto : t
    val full : t
    val span : Span.t -> t
    val start : t -> Edge.t
    val end_ : t -> Edge.t
  end

  type t [@@deriving equal, sexp_of]
  val create : ?column:Axis.t -> ?row:Axis.t -> unit -> t
  val column : t -> Axis.t
  val row : t -> Axis.t
end
```

Lines are signed and nonzero: positive indices count from the first explicit
grid line, negative indices from the last. Proposed limits are absolute line
index 1,025 and span 1..1,024, matching the existing 1,024-track limit while
allowing the final line. The general style follows native placement/normalization
semantics, including both-span endpoints; it does not invent a second layout
engine. Capture independent wire bytes and reject zero, out-of-bound integers,
unknown tags and malformed replacement transactions on both runtimes.

The form collection applies stricter column bounds: resolve negative lines
against its current explicit column count, reject placements extending outside
those columns and reject spans larger than the count. Changing columns therefore
returns an explicit construction error if an item no longer fits. Row placement
is automatic and follows supplied item order; arbitrary two-axis grid composition
remains available through the general style API. The pinned [Taffy resolver](../../vendor/taffy/src/style/grid.rs) establishes
these rules for in-flow items; native geometry tests must verify the adapter
preserves them before acceptance:

| Endpoint combination | Normalized placement |
| --- | --- |
| Two different lines | Earlier resolved line to later resolved line, even if supplied in reverse order. |
| Two equal lines | One track starting at that line; the final explicit line therefore extends outside a form's columns and is rejected by the form constructor. |
| Start line and end span/Auto | Forward by the span, or one track for Auto. |
| Start span/Auto and end line | Backward from the end line by the span, or one track for Auto. |
| Two spans | Automatic placement using the start span; the end span is ignored by the pinned native resolver. |
| Auto and one span | Automatic placement with that span. |
| Auto and Auto | Automatic placement with one track. |

For `columns = n`, positive line `k` maps to zero-based line `k - 1` and
negative line `k` to `k + n + 1`. A definite form range must satisfy
`0 <= start < end <= n`; an automatic span must be at most `n`. For example,
in four columns `1..-1` covers all four tracks, `3..1` covers the first two,
`2..2` covers the second, and `Auto..1` lies outside the form. These are
source-derived normalization cases, not yet passing GPUIO behavior evidence.
Absolutely positioned grid items have different native Auto-edge behavior;
cover that in the general style primitive and do not apply form validation to it.

## Item and collection ownership

Add a separate `Form.Item` type; retain `Form.Field = Accessibility.Field` and
`Form.field` unchanged. Each item has a caller-supplied stable key, column-axis
placement, arbitrary rich content, optional rich label/description/error slots,
required-marker display and optional overrides for layout, size, label width,
label style and item alignment. Collection defaults are resolved into each item;
changing those values changes styles and slots, not the keyed item parent.

Keep item roots as direct keyed children of one native grid. Do not partition
items into newly created row containers when columns or spans change. Within an
item, keep content under a fixed key while changing label orientation through
style. Optional labels/annotations use their own stable keys, so adding an error
cannot shift or replace the control. A full-width trailing footer can be a stable
sibling after the grid, with caller-owned actions and state.

Offer a semantic-control convenience constructor which applies
`Accessibility.Field` directly to a supported native control and supplies default
label/help/error display from that metadata. Rich display overrides must leave
those native semantic associations intact. Arbitrary-content items do not
silently label every descendant: the caller supplies semantics to interactive
children, using the same existing validated accessibility operation.

An absent label may reserve the shared label width in horizontal layout when
`label_indent` is enabled. A present label stays visible regardless of that
absent-label policy. This intentionally avoids the pinned implementation's
`label_indent = false` branch suppressing even a supplied label. Visibility uses
existing explicit caller removal or retained View visibility/inert policy, with
separate lifecycle tests; no ineffective item `visible` flag is copied.

Implementation sequence: paired grid-location primitive and native/state-style
geometry tests; typed item/collection composition and deterministic identity
checks; public gallery and fresh installed-consumer keyboard/metadata/paint
acceptance. None of these proposed types is currently exported.

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
