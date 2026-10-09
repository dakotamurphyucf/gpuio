# Rich form composition — OCH-41

Status: the rich `Form.Item`/`Form.create` API and public gallery example are
implemented locally. Core construction/reconciliation tests and the full local
OCaml build/test/format checks pass. **Native collection acceptance is pending**:
the first gallery driver attempt stopped at the macOS Accessibility preflight,
before exercising any Form behavior. The [grid-location primitive](native-grid-location.md)
has separate passing native geometry/GPU evidence. The existing `Form.field`
helper retains its contract.

## Pinned source and mapping

The reviewed sources are Longbridge GPUI Kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`:
[exports](../catalog/sources/component-form-mod.rs.txt),
[Form](../catalog/sources/component-form-form.rs.txt) and
[Field](../catalog/sources/component-form-field.rs.txt). Snapshots match the
SHA-256-verified dependency archive; the catalog manifest records each blob.

| Source behavior | Current GPUIO mapping (native collection acceptance pending) |
| --- | --- |
| Vertical/horizontal labels, independently of form columns | `Form.create` separates label layout from its explicit column count. |
| Multiple columns and per-field span/start/end | `Item.column` uses a typed grid axis with spans/signed endpoints and strict explicit-column validation. Items remain direct keyed grid children. |
| Rich optional labels/descriptions and arbitrary content | `Item.create` accepts arbitrary views and rich slots; `Item.of_field` associates metadata directly with a native control. Rust render closures become Bonsai-produced views, never synchronous FFI callbacks. |
| Label indentation, width, text size and item alignment | Shared defaults and item overrides include width, indentation, layout, alignment and label style. An absent horizontal label can reserve width; a present label is never suppressed by indentation policy. |
| Size-dependent form/field spacing | `Size.XSmall | Small | Medium | Large` defines composition spacing and label typography; control sizing remains caller-owned. |
| Full-width trailing footer | An optional footer follows the grid at full width, with trailing alignment. Footer controls/state/tasks stay application-owned. |
| Required marker and semantic metadata | Existing `Accessibility.Field` and `Form.field` supply this. Rich display slots must not replace an actual input's semantic name/help/error with an unlabeled group. |
| Visibility | Source stores `visible`, but its pinned render method never reads it. Define actual caller-controlled removal or retained hiding explicitly; do not copy an ineffective flag. |
| Style refinement | Pinned Field applies its style, while Form's render method omits its stored style. GPUIO should honor documented root/field/slot refinements. |

Unchecked source column casts, zero/oversized spans and implicit positional IDs
are not invariants to copy. Stable caller field keys, validated bounds and explicit
placement semantics are needed. Existing `Form.Field` aliases semantic metadata;
do not repurpose that module incompatibly as a new rich field object.

## Public shape

`Form.field` remains unchanged. `Form.Item.create` accepts a required stable key,
arbitrary content, optional rich label/description/error and per-item overrides.
`Form.Item.of_field` attaches `Accessibility.Field` directly to a native control,
then constructs its display slots. `Form.create` accepts items, explicit columns,
shared layout/size/label policy, slot styles and an optional footer.
`Style.Grid_location.Axis` distinguishes signed lines from validated spans.

For a normal input, attach `Accessibility.Field` to the native control through
the existing validated operation; the rich item places that decorated control.
For arbitrary content, the application supplies meaningful semantics to each
interactive child. Label/help/error display is separate from native metadata, so
a decorative label cannot silently erase an input's accessible name.

The signatures live in [form.mli](../../lib/core/form.mli). Form introduces no
editor owner, persistence service, validation scheduler or native callback registry.
Default label width is 160px. Size changes composition spacing and label typography;
control sizing stays with its caller. Root/grid/item/slot styles refine defaults.
Base structural properties are applied last; callers must not override structure
with interaction-state styles. Absent labels reserve width only in horizontal
layout with indentation enabled. Item removal unmounts; normal retained hiding
preserves identity and follows existing visibility semantics.

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

The placement types below now have an implementation; validation status is in
[the primitive contract](native-grid-location.md). `Line` and `Span` additionally
provide their conventional `of_int_exn` convenience constructors:

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
grid line, negative indices from the last. Limits are absolute line
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
`2..2` covers the second, and `Auto..1` lies outside the form. These cases pass
the Core collection validator and the separate native grid primitive matrix. Actual Form collection geometry still requires its gallery run.
Absolutely positioned grid items have different native Auto-edge behavior;
cover that in the general style primitive and do not apply form validation to it.

## Item and collection ownership

The separate `Form.Item` type retains `Form.Field = Accessibility.Field` and
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
acceptance. The placement types and rich Form constructors are exported. Native collection
and installed-consumer behavior acceptance remain pending.

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

## Current local evidence

On macOS 14.5 arm64, the repository's isolated OCaml 5.3/Bonsai v0.17 environment:

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @test/view_api/runtest`
  passed the new collection tests: 20 signed-line/span combinations, invalid
  columns/widths/duplicate keys, rich semantic metadata and editor identity across
  column/orientation/reorder/error/footer/retained-hiding changes. Explicit removal
  and remount obtain a new native identity. These are reconciliation tests, not
  assertions about physical focus or rendered geometry. A further lifecycle test
  covers 128 combinations/transitions of rich label/description/error/footer slots,
  both orientations and one/three columns: removed-slot actions are fenced,
  surviving controls use current callbacks, repeated views are idle and complete
  unmount fences all recorded actions.
- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @all @runtest @fmt`
  passed after adding the public gallery preview.
- `GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery
  --workspace <fresh-local-workspace>` passed with `run=False`. Installed public
  libraries and copied gallery sources built successfully without modifying the
  switch. This proves consumer compilation, not native behavior.
- `python3 scripts/test_gallery.py --section forms --images <local-directory>`
  stopped at `macOS accessibility access is unavailable to this test process`.
  The bounded wrapper reaped the application. No native Form assertions executed;
  the new `scripts/gallery_forms.py` scenario is not yet validated. The gallery
  driver now checks Accessibility before launching its app; a subsequent attempt
  failed at that preflight without opening a window.

The gallery's Text editing page demonstrates rich labels, help/error metadata,
required display, multi-control content, absent-label indentation, column and size
changes, signed full-span summary, mixed spans, reordering, hiding and a full-width
footer. It uses only public APIs. Its acceptance driver must pass on the repository
application and a fresh installed consumer before the catalog row is accepted.

```ocaml
let metadata =
  Form.Field.create ~label:"Workspace" ~help:"Visible to your team." ~required:true ()
  |> Or_error.ok_exn
in
let item =
  Form.Item.of_field metadata
    ~key:(Key.of_string_exn "workspace")
    ~column:Style.Grid_location.Axis.full
    ~label:(View.text "Your workspace")
    ~control ()
  |> Or_error.ok_exn
in
Form.create ~columns:2 ~layout:Horizontal ~footer:save_button [ item ]
```

`control` and `save_button` above are caller-created views with caller-owned
state/actions. Overriding the visible label preserves the semantic name
`Workspace` and its required/help metadata on the control.
