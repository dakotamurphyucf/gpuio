# Custom chrome and native window regions

OCH-41 implementation contract. This document describes the intended complete
integration; implementation/validation status stays in docs/status.md and evidence.

`Window.Chrome.Custom` creates a transparent title bar with application-owned
macOS dragging and native traffic lights at (9,9) logical pixels, matching the
pinned toolkit default. Standard/Hidden remain unchanged. Platform decorations
remain backend-owned; Linux GUI qualification is still deferred, not inferred.

`Window_region.t` is Title_bar, Exclude, or Resize of a named edge/corner.
`View.with_window_region view option` attaches it to an ordinary Container, without
adding an OCaml callback or focus owner. None clears the policy. Composition and
part styling use normal Views; this is not a synchronous window-method FFI API.

Title_bar arms on primary down and starts native movement on the first matching
pointer motion. Double click invokes platform title-bar behavior; on Linux it
zooms. Right click offers the native window menu where supported. Exclude consumes
ancestor window-region hit testing while leaving its children interactive. Native
controls inside title bars must not arm the surrounding drag, including disabled
controls. Use explicit exclusion for application-specific interactive content.
Resize starts an OS edge/corner gesture from primary down on resizable Linux
windows. Pinned GPUI macOS has no implementation of that platform method; explicit
Resize regions there are inert, including their cursor/hit shield. AppKit's native
window border still resizes. Compositor-owned tiling/decoration constraints remain in
force; regions do not synthesize a software-resized window or claim an OS safe area.

Every callback checks the current native node, its region identity and input gate.
Removal, policy replacement, hiding/inertness, pointer disable, window inactivity
and gesture release cancel pending movement. No recurring timer, retained event
queue, synchronous OCaml input call or foreign ownership transfer is introduced.
Changing an unrelated child/style must preserve the native window and editor state.

`View.title_bar ~backend ~fullscreen children` supplies the ordinary composed
container: a row, centered items, no shrinking, minimum height of 34 px, and left padding
of 80 px on windowed macOS or 12 px elsewhere. Explicit style overrides those defaults.
Feed the observed fullscreen state through `App.Window.on_change`; this is a
layout policy, not a claim to query arbitrary OS safe areas. The gallery's
`--custom-chrome` mode demonstrates the helper and handles the pre-handshake period
when native capabilities are not yet available.

Custom title-bar controls compose ordinary accessible buttons with existing
Minimize/Zoom commands and App.Window.request_close. They retain Tab/Enter/Space
and application close decisions. A close button must not call force-close. The
macOS traffic-light placement/reservation and fullscreen layout now have a public
helper/example; physical coverage remains required. Backend-supported controls
now compose through `View.window_controls` and observed `Snapshot.presentation`;
see [the presentation contract](window-presentation.md). Client-frame insets/tiling
remain open parts of the same window-family scope, not implicit deferrals.
Pinned source control-area tags for
min/max/close target Windows only; that platform is outside this project's scope.

The unpublished epoch-3 wire appends Chrome tag2 and Op122 Set_window_region.
Region tags0/1/2 are Title_bar/Exclude/Resize; resize edges are Top, Bottom, Left,
Right, Top_left, Top_right, Bottom_left, Bottom_right (tags0..7). Admission rejects
non-container targets and malformed enums atomically. No existing tags move.

Validation must cover independent paired bytes, Core/Bonsai reconciliation and
atomic native admission, actual host pointer routing/child exclusion, stale
callbacks and resource retirement. A fake platform request recorder can prove
native routing but cannot establish OS movement/minimize/resize behavior. Physical
macOS and current Linux nongraphical checks remain required separately.
