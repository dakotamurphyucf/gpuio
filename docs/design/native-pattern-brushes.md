# Native pattern brushes

OCH-41, 2026-10-06. [Local rendering and public consumer qualification](../evidence/native-pattern-brushes-och41.md)
passes for the recorded scope; remaining catalog/release acceptance stays open.

The pinned GPUI `Background` includes `pattern_slash` and `checkerboard` in
addition to solid colors and two-stop gradients. Its `color.rs` constructor and
Apple `shaders.metal` implementation at revision `a57ba9b` establish the semantics:
slash width/interval are packed and quantized, both patterns use physical pixel
coordinates relative to painted bounds, and gaps are transparent. Window quad
painting scales geometry but passes the background through without scaling these
dimensions. These are native brushes, not extra per-frame geometry or callbacks.

The common `Background.pattern_slash color ~width ~interval` and
`Background.checkerboard color ~size` constructors return `Or_error.t`. Dimensions
must be finite and in `[0.5,64]`. This excludes division by zero, negligible
packed widths and overflow in GPUI's integer packing. Quantization still applies;
these are not precise vector hatches. Colors may be theme tokens and resolve
before submission. Pattern angle is GPUI's fixed diagonal; orientation switches
do not rotate it. Application logical-size controls do not implicitly change
physical brush dimensions. To change density, submit different validated values.

Ordinary style backgrounds and chart path/bar brushes use the same OCaml type.
The ordinary Fill wire adds tags 3/4 after existing tags 0/1/2. Chart Brush adds
tags 2/3 after Solid/Linear. Existing variant tags and field ordering stay fixed.
Chart style advances from -7 to **-8** so a mismatched native host fails before
interpreting a newer style. Current view/options/style/data schemas are
**-2/9/-8/1**. Old unsupported tags are rejected; applications must use matching
GPUIO packages. Both native style admission and chart decoding validate pattern
dimensions before calling native constructors. Scrollbar thumb backgrounds use
the shared Fill validator as well.

Chart preparation retains a constant-sized brush with its existing mesh or quad.
There are no new data values, native registrations, source selections or geometry
sampling rules. `Chart_appearance` still permits 128 series and 1,024 sparse datum
overrides; this work does not create an arbitrary dense per-datum style resource.
Series fills can pattern every mark in that series without a per-datum override.

The gallery's pure `Chart_marks` adds Slash and Checkerboard presets using series
path/bar appearance. `Charts_page` owns the Bonsai preset state and source.
Solid strokes and legend colors remain independent of patterned interiors.
Native GPU checks must distinguish patterned pixels from solid and empty negative
controls, including transparent gaps; chart evidence must include path and quad
families, orientations, themes, source selection and cleanup in root/installed
consumers. Decoder or compilation success alone does not establish that evidence.

Scalar area and per-bar baselines remain separate work. The pinned low-level
area shape accepts one `y0` coordinate and the bar shape a per-datum base accessor.
Their GPUIO contract must define coordinate units, domain inclusion, stacking,
aggregation, source provenance and bounded storage without synchronous OCaml
layout callbacks. Adding pattern brushes does not resolve or defer that scope.
