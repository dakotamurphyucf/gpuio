# Popup points, corners and viewport margins

OCH-41. `Placement` is shared by popovers, tooltips, hover cards and menu buttons.
The public constructors validate finite values; all lengths are logical pixels
in the current window's content coordinates, not screen or device coordinates.

`Placement.create ?side ?align ?offset ?viewport_margin ()` keeps the existing
Bottom/Start/zero-gap defaults and adds an eight-pixel default viewport margin.
Side placement measures the trigger in the current frame, flips to the opposite
side when appropriate, then clamps the resulting origin. Offsets remain signed,
bounded to ±16384, and can overlap a trigger.

`Placement.at_point ?corner ?viewport_margin ~x ~y ()` places one of four popup
corners at a window point. The default corner is `Top_left`. Coordinates must be
finite and within ±1,000,000. It clamps without flipping: a bottom-edge point
does not unexpectedly change which corner was requested. The trigger still owns
activation, accessibility relationships and focus return. It is not repositioned.

```ocaml
let placement =
  Placement.at_point
    ~corner:Bottom_right
    ~viewport_margin:24.
    ~x:560.
    ~y:400.
    ()
  |> Or_error.ok_exn
in
let config = Overlay.Config.create ~label:"Inspector" ~placement () |> Or_error.ok_exn in
View.popover ~config ~on_dismiss ~anchor content
```

Margins are finite and in 0..16384. Native positioning adds the platform client
inset, then caps the total per axis at half the current viewport dimension.
Content is measured before positioning. Oversized content keeps the leading
margin; placement does not shrink it or add scrolling. Applications still own
content dimensions and scroll views. Window resize recomputes placement natively
without an OCaml transaction. Geometry changes preserve the native surface,
descendant owners and focus; they do not reopen it or emit visibility changes.

Menu buttons apply the point to their root popup only. Nested menus retain their
row anchor and side/flip behavior, inheriting the same viewport margin. Tooltip
motion remains defined by its trigger history; point placement changes the
settled surface destination, not the entry/switch timing or ownership contract.

Dialogs and sheets do not use this placement setting. Dialogs center by default;
ordinary panel `Style.Position Absolute` and `Top`/`Left` insets can place them
explicitly while preserving modal focus and backdrop behavior. Such styles own
their geometry and do not automatically receive popup clamping. Sheets retain
their constrained edge/extent geometry. [Sheet.Insets](sheet-insets.md) reserves explicit application chrome space
without importing the pinned toolkit's custom-window wrapper offsets.

The pinned upstream positioner exposes both side and corner strategies. A
zero-size anchor with a side strategy is not equivalent to corner placement near
an edge because side placement can flip. GPUIO retains its existing eight-pixel
default rather than changing it to upstream's four-pixel default.

Op93 `Set_placement_geometry (NodeId, config option)` adds fixed-size optional
metadata: viewport margin and an optional corner/point. It does not change the
legacy side record. Default side placement omits the metadata, preserving old
fixture bytes; resetting emits `None`. Native decoding and tree admission check
bounds and reject invalid targets atomically. Valid final targets are popover
focus scopes, Tooltip/HoverCard and button menus. Changing to a modal or removing
the owner configuration must clear the metadata in the same transaction.

No new native resource, timer, worker, permanent polling or per-frame bridge
callback is introduced. The metadata lives inline in the accounted native node.
See [local evidence](../evidence/placement-geometry-och41.md). TestPlatform checks
are separate from physical macOS visual/input/accessibility qualification.
