# Icon test drawable capacity — OCH-17 / OCH-41

[Hosted run 37579057201](hosted-run-37579057201.md) failed when the icon-transform
probe sampled `(142,114)` from a `144×96` native snapshot at synthetic density 2.
GPUI's test-only `Window::set_scale_factor` changes scene coordinates and requests
a refresh; it does not resize the native CAMetalLayer. Metal readback uses that
layer's drawable size. The fixture inherited a small window from preceding SVG
checks and did not establish enough backing space for its largest test scale.

A local diagnostic on macOS 14.5 arm64 / M1 Max reproduces the exact hosted
out-of-bounds failure by requesting the same 144×96 physical drawable on Retina
and isolating the density-2 pass. The first diagnostic retained the normal loop
and instead failed earlier: its smaller logical viewport clipped a sample at
density 1. Both failures are retained; together they distinguish logical canvas
size from physical drawable size.

The final fixture starts from the small drawable, then waits for an actual
256×192 logical viewport and verifies sufficient native readback dimensions.
It restores all density 1/1.5/2 passes. Each pixel access also checks bounds with
the scale, viewport and drawable in its failure message. All original color
tolerances, negative controls, fit modes, transform/cache/layout assertions,
clipping and cleanup checks remain. Production rendering is unchanged.

The complete local `native_image_views` target passes after this correction,
including icon transforms and slots, avatars, patterns/gradients, spinner,
progress and control appearance. The owned test windows close and the driver
restores captured clipboard representations, including after the deliberately
failing diagnostics. No VoiceOver or OS settings changed. Strict native
all-target Clippy and Rust formatting also pass. Because this changes only a
native test fixture, the preceding endpoint checkpoint's full Dune/native-unit
results remain scoped to that implementation; they are not claimed as new runs.

The [archive](icon-readback-capacity-och17/reports.tar.gz) and
[verified manifest](icon-readback-capacity-och17/manifest.json) retain the exact
diagnostic source, commands, failures, passing native log, final patch and lint.
Current-source hosted confirmation remains required. Synthetic density checks
do not establish monitor migration or physical presentation/FPS.
