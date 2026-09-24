# OCH-11 theme, scale and native-state acceptance

Local macOS arm64; pinned GPUI a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b.
This evidence distinguishes production paths, controlled test overrides and physical
platform validation. Hosted Linux GUI validation remains deferred to OCH-17.

## Per-window theme updates

`App.Window.set_theme` updates the window driver and wakes the Eio UI loop.
Reconciliation resolves theme tokens again even for physically shared view values.
The expect tests in `test/view_api/view_api_test.ml` and `appearance_test.ml` verify
changed colors, retained node/handler identity, updated Select part colors and an
unchanged-theme no-op. The public Bonsai animation example exercises a live theme
change through the actual runtime/FFI while the retained animation wrapper survives.

Native state precedence is documented in `docs/design/typed-ui.md`: base, value,
focus, hover, pressed, with explicit disabled behavior. The full `native_controls`
suite passes configurable Select geometry/colors while open, focus/highlight
retention, checked/indeterminate/disabled roles and styling, tooltip/menu/palette,
progress and overlay lifecycle checks. These use GPUI input injection and AppKit
semantic queries; they do not claim a physical screen-reader certification.
Native image tests additionally verify inherited icon foreground and hover color
in actual GPU pixels, including after its registered source is retired.

## Scale changes

UI geometry and animation targets remain in GPUI logical pixels. GPUI owns display
scale conversion, text shaping and renderer resize. At the pinned revision,
`gpui_macos/src/window.rs::update_window_scale_factor` updates drawable size and
passes content size plus scale to its resize callback; screen and backing-property
changes call this path. This source audit is not a physical monitor-change test.

The production SVG canvas reads `window.scale_factor()` on paint and keys its
bounded raster request by physical output size, density, fit and tint. The native
image suite now uses GPUI's test scale override to step through 1, 1.5, 2 and the
original scale. It verifies a new matching raster/density while logical viewport
size and tree revision remain unchanged, including after source retirement.
Scale is restored before subsequent GPU pixel assertions. This suite passes
locally (`native-final-image-scale.log` in the implementing agent's scratch).
Pure decoder tests also verify fit/density and bounded cover output.

## Motion and ownership

`docs/design/animations.md` records typed targets, native frames, interruptions,
endpoints, live System/Reduce/Full policy and cleanup. Native tests cover logical
width/height and top/left interpolation, right/bottom placement, zero-area startup,
and closing a window with a delayed run pending. The full native controls and
focused progress suites pass shared reduced-motion behavior and whole-window idle.
Asset, scroll, overlay and command lifetimes have their own linked evidence files.

Remaining project acceptance is the consolidated macOS/Linux build-and-test gate,
resolution of any hosted failures, and merge. Physical multi-monitor behavior and
full Linux desktop behavior are not implied by these controlled tests.
