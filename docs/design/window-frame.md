# Window-owned client frame

OCH-41 implementation contract. Native/platform acceptance is tracked separately.

`Window.Config.create ~chrome:Custom ?frame` owns immutable `Window_frame`
geometry for that native window: default shadow margin 20 and resize half-band 4
logical pixels. Shadow is finite 0..128, hit half-band 0.5..32. Standard/Hidden
continue to use their existing frame behavior. Custom requests client decorations
on Linux, but uses actual compositor decorations at render time. macOS and server
fallback use the native OS frame with no extra application margin or resize bands.

The frame wraps the complete retained application tree without changing node,
editor, callback or Bonsai identity. The root's resolved Border_color supplies the
frame stroke; absence uses neutral translucent gray. Shadow and border geometry
remain native; no pointer-motion/paint callback round trip to OCaml is introduced.

Client padding and borders disappear independently on tiled sides. Fullscreen
has no visual frame. The platform client inset remains the configured full shadow
margin through maximization/tiling/fullscreen, avoiding the pinned backend's
restore/resize size jump. Tiny viewports compress visual insets proportionally;
content stays within the drawable viewport. Snapshot content dimensions retain
their original drawable-viewport meaning.

Resize bands follow current frame geometry, with corners taking precedence.
No resizing starts on tiled edges, fullscreen/non-resizable/inactive windows or
during another capture. macOS has no GPUI custom-resize operation and remains
native-frame-owned. Cursor and gesture hit testing must agree. Frame decoration
does not introduce a keyboard focus or accessibility action owner.

Popup, sheet, palette, menu, choice, tooltip and toast placement must use current
frame content bounds, including asymmetric tiling. Merely padding the root is
insufficient. Placement Point coordinates stay drawable-window coordinates;
clamping/snap margins apply inside frame content. The new frame must not double
charge the existing popup client-inset margin.

The unpublished epoch-3 configured-open payload appends Frame geometry. Both
runtimes must be rebuilt together. Invalid geometry is rejected before opening a
native window. Required evidence includes paired bytes, geometry/tiling tests,
actual TestPlatform frame/overlay layout and existing full regressions; none of
those substitute for physical macOS or later Linux desktop qualification.
