# Pie leader pixel checks across density — OCH-41 / OCH-17

2026-10-06, macOS 14.5 arm64, source base `2b91fdb` plus this test change.
[Hosted run 37523471664](hosted-presentation-calibration-och17.md#hosted-run-37523471664)
failed the mounted pie's independently colored leader assertion. The same failure
is now reproduced locally by setting the test window to synthetic scale 1.

The rendered blue leader is present. In the isolated sample between its gray
wedge and green caption, the framebuffer contains `[8, 8, 136, 255]` over a
`[16, 16, 16, 255]` background. The original assertion required a channel value
above 150 in addition to color dominance, so this thin partially covered pixel
failed despite its correct blue hue. The captured image was visually inspected.
This reproduction identifies an invalid minimum-intensity assumption in the probe;
no production rendering, geometry, color or protocol code changed.

The revised leader assertion requires the expected hue to dominate both other
channels by more than 80 across at least `ceil(2 * scale)` distinct physical
columns within the same bounded sample region. Both unexpected hues must be
absent. Actual gray leader variants are negative controls: neither gray geometry/
background nor nearby green text may satisfy red or blue detection. Caption text
retains its original stronger channel/count checks. These requirements still
reject absent or incorrectly colored leaders and no longer demand a high-intensity
pixel from a one-logical-pixel stroke.

The complete production-mounted chart suite passes with six pie configurations
at synthetic scales **1, 1.25, 1.5 and 2**: ordinary/bold captions, a narrow chart
with one hidden caption, inside labels, globally hidden labels and neutral leaders.
All 20 colored leader samples pass; all eight neutral samples report zero detected
hue columns. Original data equality is checked throughout. The test restores its
original scale, and the full existing axis/radar/source/lifetime/stream checks
pass afterward. The hidden test window closes and is reaped.

Native-feature strict Clippy and whitespace checks pass. CI now supplies an
absolute optional failure-image path under its existing artifact directory.
Diagnostic image-write errors are reported without masking the original assertion.
The initial local diagnostic used a relative path and failed to save its image;
the subsequent absolute-path reproduction retains both the actual assertion
failure and image. Both attempts remain in the archive.

[Before/after logs, image and commands](pie-leader-density-och41-logs.tar.gz) and
the [verified manifest](pie-leader-density-och41-manifest.json) preserve the result.
This does not retroactively turn the hosted run green. Current-source CI must
confirm the repair. The two Metal presentation failures, broader catalog/native
accessibility and macOS release qualification remain open. Synthetic scale checks
are not physical monitor transitions, OS IME, VoiceOver or presentation timing.
No full OCaml rebuild or public-app rerun was needed for this test-only repair;
production code retains its separately scoped [guide-span validation](chart-guide-spans-och41.md).

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --features native-canvas-tests --test native_chart_view --locked -j2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --features native-canvas-tests,native-image-tests --lib --tests --locked -j2 -- -D warnings
git diff --check
```
