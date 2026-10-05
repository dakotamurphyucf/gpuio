# Numeric input presentation

Implemented local contract for OCH-41; [verification](../evidence/number-presentation-och41.md)
covers Core, protocol, admission and TestPlatform behavior. This does not claim
catalog or desktop acceptance.

[Fresh installed macOS evidence](../evidence/installed-numeric-otp-och41.md) now
passes scoped desktop checks after fixing duplicate generic rendering of the
component-owned slots. Actual native paint counts cover Side/Stacked/Hidden,
and public input/geometry/history checks cover 24 presentation combinations.
Broader timing, resource and release acceptance remain separate.

`View.number_frame` decorates a direct native `number_input` view and preserves
its controller, draft, committed value, selection, composition, history and native
step-repeat behavior. The Bonsai view module exposes the same helper; Eio users
apply it to `Number_input.view`. It returns a checked view, just like
`View.input_frame`, and does not add another editor or an ancestor owner.

Four stable structural slots hold optional leading/trailing content and custom
decrement/increment content. Leading/trailing slots are ordinary interactive
views. They keep their own actions/focus and do not implicitly commit or replace
the numeric draft. Their placement is inside the editable middle region, between
the outer step controls. Disabling the numeric owner disables descendants;
read-only disables native editing/stepping, leaving auxiliary application controls
available. Custom step-button content is passive: no callbacks, focus targets,
text selection, scrolling or pointer shielding. Native button labels, actions
and hold-repeat stay on the existing numeric owner. Hidden step controls do not
render their decorative slots.

`Number_input.Appearance` contains bounded geometry and familiar `Style.t`
declarations for the whole frame, editor region and each step button. Geometry
is explicit: gap, button width, button minimum height, stacked button minimum
height, editor horizontal padding and border width. Colors, gradients, typography,
shadows, opacity, corner radii and borders use the existing style vocabulary and
theme tokens. Frame/editor styles admit Base, Focused and Disabled; buttons admit
Base, Hovered, Pressed and Disabled. Read-only buttons use Disabled styling.
Unsupported layout/interaction fields and more than 128 declarations are errors.
Root view style remains available for outer layout and supplies the initial frame
styling; frame appearance refines it, then current focus/disabled appearance.
Focused appearance surrounds the complete control using the actual editor's
focus handle. Interactive slot focus does not imply numeric editor focus.

Removing the helper removes slots/presentation while retaining the editor.
Changing appearance invalidates native rendering, never editing state. Geometry
changes cancel a held step gesture if its painted target moves. Stepping policy
and custom application step requests remain a separate contract.

The paired unpublished epoch-3 operation appends tag 82, optional numeric
presentation. A present presentation requires exactly four structural slots;
absence requires no numeric children. Both sides validate the shape, passive
button subtrees and bounded appearance; native admission charges retained style
buffers and rejects invalid batches atomically. No synchronous OCaml callback
from native input/layout/paint is introduced.
