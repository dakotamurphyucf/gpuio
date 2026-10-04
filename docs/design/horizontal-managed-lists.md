# Horizontal managed lists — OCH-41

Status: native engine, public Core/paired bridge/Host and reactive Bonsai
configuration are implemented. Gallery/consumer and physical acceptance are
recorded in [integration evidence](../evidence/horizontal-list-integration-och41.md).
The pinned catalog requires the functionality of `h_virtual_list`, including
bounded materialization. A carousel or a horizontally scrolling ordinary View
is not a virtualized collection.

## Public contract

Retain the existing vertical `Virtual_list.Config.create ~height` API. Add a
checked `Extent` (Estimated/Fixed logical pixels) and a horizontal constructor
`Config.horizontal ~width:Extent.t`, with the same overscan, max_active, scroll
policy and scrollbar options. Expose `Config.axis` and `Config.extent`; keep the
existing Height spelling for vertical source compatibility. Heights/widths remain
in 1..1_000_000 and overscan/pin/active bounds remain unchanged. The axis is a
configuration property, not inferred from child styling.

The Bonsai component, managed row lifetime, collection identity/order, paging,
controller and viewport observation contracts apply along the chosen axis.
`anchor` is the stable item key plus a logical-pixel distance from its leading
edge; `at_start`, `at_end`, following and overscan are axis-relative. Fixed extent
clips along that axis; estimated extent is replaced by native measurement. The
viewport needs a bounded main-axis extent and constrains the cross axis.
Horizontal lists advance left to right; RTL/reverse ordering is not implied.
Existing row-named operations remain source-compatible aliases for logical items.

The existing Bonsai constructors retain static config arguments for source
compatibility. `component_with_config` and `paged_with_config` accept a reactive
checked config for changing axes or sizing within the same collection lifetime.

Axis changes preserve a surviving logical anchor (subject to the new content
range) and application data, retire
old measurement/capture state and remeasure in the new cross-axis constraint.
They must not resurrect obsolete controllers, reset Bonsai row models or keep
an old scrollbar handle connected to a new list. Tree input and managed tables
remain vertical; reject incompatible horizontal metadata atomically rather than
silently reinterpret tree/table navigation.

## Native ownership and transport

Generalize the existing measured GPUI ListState engine. Use a small explicit
axis mapping at physical layout/input boundaries: the measured sum tree uses
cross-axis width and main-axis height internally, and children are laid out and
painted in their ordinary physical coordinates. This is coordinate bookkeeping,
not rotation of text, hitboxes or accessibility. All native public bounds and
scrollbar offsets remain physical x/y values. Existing vertical constructors and
behavior remain unchanged. Do not duplicate the retention/anchor algorithm.

Append an independently decoded axis operation to the bridge; keep existing
Config/Viewport/Scroll_request record layouts and numeric tags stable. Existing
`estimated_height` transport storage carries the main-axis estimate when the new
axis metadata is attached. Core hides that legacy spelling behind its checked
configuration. Admission rejects unsupported combinations and bad enum tags.
Default/reset is vertical. Pair independent OCaml/Rust fixtures before exposing
new public behavior. Demand/reconciliation remain asynchronous and bounded.

Main-axis wheel input consumes only that axis and preserves existing child and
nested routing. Do not silently convert every vertical wheel into horizontal
scrolling. Pointer, keyboard and accessibility scrollbar actions use the same
native handle. Cross-axis resize invalidates measurement; main-axis resize
updates the viewport without losing the logical anchor. Tail-following and
streamed item updates use main-axis extents, including prepend and reordering.

## Validation and completion

Start with native horizontal/vertical equivalence tests over unequal item sizes,
physical child bounds, padding, clipping, wheel ownership, scrollbar offsets,
reveals, resize/remeasure, prepend/reorder, streamed growth and tail state.
Verify no text/hitbox/accessibility rotation and bounded rendering at 100k items.
Then test checked Core values, independent wire fixtures and atomic admission,
Bonsai retention/controller fencing/paging and production Host integration.
Add a public horizontal collection gallery with variable widths, growth/reorder,
scrollbar and controller controls, and build it as an installed consumer.
Physical macOS input/accessibility/GPU and required Linux non-GUI checks retain
the milestone release policy; none is proven by a pure geometry test.

## Native foundation checkpoint

The axis-aware engine and accessibility retry rollback are implemented in the
pinned GPUI adaptation. Native TestPlatform tests exercise both axes, 100k items,
variable extents, physical accessibility bounds, anchors/resize/streaming/tail,
wheel precedence, padding and inferred sizing, focused-child autoscroll retries
and prepend. The existing Host now retires custom scrollbars before list config
replaces their native handle; its list/tree tests check capture, range focus,
new numeric extent and commands against the replacement. Core/transport/Host
axis configuration, reactive Bonsai retention/paging and the horizontal gallery
are covered by the subsequent integration evidence.

## Public use

```ocaml
let config =
  Gpuio.Virtual_list.Config.horizontal
    ~width:(Estimated 220.) ~overscan:220. ~max_active:16 ()
  |> Core.Or_error.ok_exn
```

Pass this to the existing `Gpuio_bonsai.Virtual_list.component`/`paged` `~config`
argument. Give the viewport a bounded width and height. Variable-width item views
provide their own preferred width; fixed extents clip at the configured width.
Use `component_with_config`/`paged_with_config` when `config` is a Bonsai value.
These variants share the implementation and controller lifetime with the existing
static-config wrappers. A configuration change invalidates the exposed viewport
until a fresh geometry report, preventing stale boundary flags from triggering
paging. Previously requested/pinned rows remain eligible within the active budget;
there is no whole-collection model reset. Tail prefetch uses only an observation
of the current config.

`Set_list_axis`/`SetListAxis` is Op109, with Vertical=0 and Horizontal=1. It defaults
to Vertical on creation, and setting Vertical resets it. Old list records and
operation tags remain unchanged. Native validation rejects malformed enum tags,
non-list targets and horizontal lists with native tree input or a managed table.
The list root exposes its orientation through native accessibility metadata.
