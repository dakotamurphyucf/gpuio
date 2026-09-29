# OCH-41 gradient interpolation

`Background.linear_gradient` retains its existing constructor signature, sRGB
behavior and wire bytes. `Background.linear_gradient_in` adds an explicit
`Background.Color_space.Srgb | Oklab` choice. For example:

```ocaml
Background.linear_gradient_in
  Background.Color_space.Oklab
  ~angle:90.
  ~from:(Color.rgb_exn 0xff0000, 0.)
  ~to_:(Color.rgb_exn 0x0000ff, 1.)
```

The constructor returns `Or_error.t`; angle is clockwise degrees from the top in
0..360, stops are ordered fractions in0..1, and equal stops are allowed. Ordinary
RGBA colors and theme tokens still resolve before submission. Missing theme
references remain recoverable errors. `Background.Expert.description` now carries
the interpolation space in its gradient payload; direct Expert consumers must
include that field when matching.

This matches the pinned GPUIX `LinearGradientBackground.colorSpace` vocabulary and
its sRGB default (`packages/native/src/style.rs` at
`18e695ed0ee8121a7793413ca795e08eda2a13df`, lines261–278). The native implementation
uses the pinned GPUI background interpolation support directly, with no new fork,
CPU gradient rasterizer or dependency.

## Wire and validation

The existing Fill tags0(solid) and1(sRGB linear gradient) retain their bytes.
Tag2 adds `(space:int64, angle:float, from:Color, start:float, to:Color, end:float)`;
space0 is sRGB and1 is Oklab. The public sRGB constructor continues emitting tag1.
Native admission rejects unknown spaces, invalid/nonfinite geometry and invalid
colors before style refinement. The new tag requires the matching GPUIO backend;
it does not make an older decoder understand Oklab.

Two OCaml expect tests independently check legacy/explicit-space encoding, default
equality, theme-token failure, nine invalid geometry cases in both spaces and
valid angle/equal-stop boundaries. A Rust fixture checks the same exact fill bytes
and live request decoding, every truncated request prefix and trailing data.
Native refinement tests verify both GPUI color spaces and malformed admission;
retained-tree tests verify unknown spaces/NaN stops reject the whole transaction
without changing its revision or retained allocation count.

## Native and public gallery evidence

The production declarative View is tested in the `native_image_views` executable.
GPU readback verifies:

- Legacy and explicit sRGB yield identical red/blue gradient pixels.
- Oklab has the independently expected midpoint nearRGBA(140,83,162,255), distinct
  from sRGB's midpoint near(128,0,128,255); tolerance5 covers pixel-center sampling.
- Stops at0.25/0.75 clamp outer samples to the endpoint colors.
- Restyling the same retained node back to legacy sRGB restores identical pixels.

Existing SVG, button-icon, avatar/image lifetime and GPU regressions pass in that
same executable. Its first gradient development run used an invalid noncontiguous
node slot in the test fixture; using the next available slot corrected the fixture.
No rendering defect was found in the interpolation implementation.

The public Styling details gallery adds a labeled swatch and interpolation toggle.
Its macOS driver verifies both choices, return to sRGB, and Oklab retention through
theme/size changes and three page departures/reentries, alongside existing cursor
and truncation checks. The gallery screenshots show the upper cards; the gradient
card lies below the fold, so those screenshots are not pixel evidence for it.
The native readback above supplies the interpolation pixel evidence.

Local macOS14.5 arm64 commands:

```sh
./scripts/gpuio exec dune build @test/view_api/runtest @fmt
./scripts/gpuio exec dune build examples/gallery/main.exe -j2
./scripts/gpuio exec cargo test -p gpuio-protocol --test gradient --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --lib style::tests --features native-image-tests --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --test tree --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --test native_image_views --features native-image-tests --locked -j2 --no-run
python3 scripts/test_gallery.py --section styles
./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --locked -j2 -- -D warnings
python3 scripts/audit_component_catalog.py
```

The Cargo-reported native executable ran under a90-second process-group watchdog.
All listed checks pass. This is focused local macOS evidence, not consolidated
hosted, complete installed-consumer, all-style-value or Linux desktop acceptance.
The ledger now covers three reviewed value sets: cursor, text overflow and nested
gradient color space. It does not imply acceptance of all73 style fields. Remaining
selection/inheritance, nested APIs, combined gallery/consumer and application
performance/resource/release gates remain required milestone07 work.
