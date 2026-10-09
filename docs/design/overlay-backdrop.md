# Modal backdrop color

OCH-41, 2026-10-02. Local codec, admission, native-host, OCaml and lint checks pass.
Physical qualification remains open; see [evidence](../evidence/overlay-backdrop-och41.md).

`View.dialog`, `View.sheet` and `View.alert_dialog` (including Bonsai helpers)
accept `?backdrop:Color.t`. It resolves theme tokens and opacity before submission.
Omission preserves `0x00000080`, the existing half-opacity black backdrop.
Transparent color removes artwork only: the full viewport still blocks background
pointer/scroll input, retains focus trapping and rejects background AX activation.
Dismissal still follows the configuration and stays a request until content is
removed. Alert dialogs still reject backdrop dismissal.

Backdrop color is separate from panel `Style`. Updates retain the mounted focus
scope, panel, child editors and callbacks. The reconciler caches the resolved
color so a theme-only update of an unchanged View submits the correct new value.
Missing tokens reject preparation rather than silently choosing a color.

Append-only Op90 `Set_overlay_backdrop (NodeId, int64 option)` uses concrete
RGBA 0..0xffffffff; `None` restores default paint. Nonempty backdrop metadata is
valid only on a modal FocusScope in the final tree. Clearing an overlay while
retaining its backdrop is rejected atomically, as are invalid RGBA values or
nonmodal targets. Clearing both in one transaction is valid. Storage is fixed
size in the native node; there is no timer or callback owner for backdrop paint.

The native modal flag must be applied to the concrete panel before type erasure.
Wrapping `SurfaceBounds`/`AnyElement` afterwards does not export the panel's
identity and loses the accessibility flag. The new renderer path attaches it to
the existing named dialog/alert-dialog target and leaves popovers nonmodal.

The gallery demonstrates tinted/transparent backdrops and live changes while a
dialog stays open. Opt-in outer-surface entry motion is documented separately in
[modal motion](overlay-motion.md). Arbitrary backdrop content and nonmodal sheet
behavior are not implied. See the
[pinned feature review](../catalog/overlay-review.md) for the remaining gaps.
