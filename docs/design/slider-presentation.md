# Slider presentation — OCH-41

Locally implemented contract. Physical macOS and release acceptance remain open.

`Slider.Appearance` separates presentation from `Slider.Config` and native value
ownership. `View.slider` and the Eio controller's `view` accept optional appearance.
`Fill.Selected` is the default. `Fill.Remaining` fills from a single thumb to the
maximum; a range always fills between its thumbs. Neither changes values, input
direction, normalization or accessibility bounds.

Appearance controls rail thickness/radius, thumb size, target size and focus-ring
width, plus optional track/fill/thumb/focus-ring colors. Defaults preserve current
4px rail, 12px thumb and 20px target. Dimensions are finite and bounded; the target
must contain both rail and thumb, and the ring must fit within the target. Colors
resolve theme tokens when reconciling; absent colors use the existing foreground
accent (track at 25% opacity). The outer View style retains background/border/layout
semantics. Thumb paint remains circular. Per-corner rail radii and arbitrary
thumb content are not part of this interface.

Op84 carries optional presentation independently of the editing configuration.
Removing it restores defaults. Paint-only changes preserve value, native revision,
focus handles and single/range identity. A target-size change changes rail travel
and cancels capture against old geometry through the normal revisioned cancellation; paint-only changes preserve capture.
No synthetic editing event is generated merely for a color or fill update.

Native thumb hover/pressed rings are now implemented locally. They grow outside
visible thumb paint to 3px at 50% of the resolved ring color. Hover uses the current
thumb target; a captured thumb retains its ring while dragging away. Each thumb
has an independent, critically damped 180ms-response spring, preserving velocity
when interrupted. Existing focus rings remain separate: `ring_width` controls the
keyboard focus outline, while `ring_color` also colors interaction rings.

Springs use the existing native analytic solver and GPUI clock. One pending weak
frame callback serves both thumbs; settled controls request no frames. No timer,
OCaml callback or bridge event is introduced. Reduced motion adopts the target
immediately. Inactive, disabled, read-only, inert, pointer-disabled, hidden, clipped
or transparent controls retire motion. A render that never paints cannot keep a
previous spring alive. Capture loss retires the pressed state through ordinary
slider cancellation. Removal and explicit Close retire queued work even when an
external test/reference still holds the owner. Node admission reserves an extra
512 bytes for two interaction springs and a weak frame callback; this is a bounded
quota allowance, not measured allocator/RSS usage.

Paired fixture/reconciliation tests, atomic admission and mounted paint/geometry/
ownership tests cover static presentation. The interaction follow-up adds solver
and fake-clock TestPlatform lifecycle checks. Physical macOS behavior, GPU paint,
resource measurement and release qualification remain separate gates. Exact
checkpoint results are in the [evidence](../evidence/slider-presentation-och41.md).
