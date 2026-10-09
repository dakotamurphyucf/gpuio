# Codec fixture

`codec-v1.hex` is the independent OCaml/Rust research fixture, ported unchanged.
Both languages construct the expected typed value, decode these bytes with full
consumption, and encode the value back to the same bytes. It covers integer
boundaries, UTF-8, NUL, variants, lists, options, booleans and a float.

This is a compatibility fixture, not the production GPUIO protocol schema.

`style-v1.hex` exercises every OCH-8 Field tag, both Fill variants, shadows and
state styles. Independent constructors live in `test/view_api/style_fixture_test.ml`
and `rust/protocol/tests/common/style_fixture.rs`. Rust also tests all truncated
prefixes. Deliberate schema edits may regenerate it with `cargo run -p gpuio-protocol
--example emit_style_fixture`, followed by review of both independent tests.

`controls-v1.hex` covers checkbox/switch kinds, every initial control configuration
and all appended semantic style state tags. Independent constructors live in
`test/view_api/control_fixture_test.ml` and
`rust/protocol/tests/common/control_fixture.rs`. Regenerate deliberately with
`cargo run -p gpuio-protocol --example emit_control_fixture`; both language tests
must still agree. The earlier editor fixture keeps its original capability mask
15 to preserve those bytes as a compatibility check.

`choice-v1-request.hex` and `choice-v1-events.hex` cover stable choice configuration,
radio-group/Select creation, choice appearance and the selected-ID event, including UTF-8 IDs and a disabled
selected value. Both languages construct the values independently. Regenerate
using `emit_choice_fixture` (request) and `emit_choice_fixture -- --events`
(events), and review both language checks.

`combobox-v1-request.hex` and `combobox-v1-events.hex` cover the editable choice
kind, both filter tags, and an exact selection snapshot with multibyte UTF-8 and
reversed byte selection. Literal fixture bytes and independent OCaml/Rust value
constructors agree; the request is a codec fixture, not a complete valid UI tree.

`canvas-v1-path.hex` fixes the retained-canvas Move/Line/Quadratic/Cubic/Close
tags, control-point order and little-endian float encoding. The OCaml expect test
in `test/canvas/path_test.ml` and Rust `canvas::tests` construct the path
independently. It is a geometry codec fixture; it does not claim implemented scene
upload, resource ownership or native rendering.

`canvas-v1-scene.hex` covers scene version/description, path/text/image resources,
all shape/drawing kinds, paint options, transform/clip records and interactive
rectangle/ellipse/polygon hit regions. Rust constructs it in
`tests/common/canvas_fixture.rs`; OCaml independently constructs it in
`test/canvas/scene_codec_test.ml`. Both decode and compare the complete value.
Regenerate deliberately with `cargo run -p gpuio-protocol --example emit_canvas_fixture`.

The current `chart-view-v2-{labels,rich-labels,inspection}.hex` envelopes use
chart style schema -8 for native pattern brushes (view/options/data remain
-2/9/1). This revision changes only their nested style version byte from `fff9`
(-7) to `fff8` (-8); their default brush fields remain unchanged. Older named
chart fixtures remain historical rejection/compatibility evidence.

Area-baseline configuration appends a float option to appearance series and
advances current chart style from -8 to -9. `chart-appearance-v2.hex` adds the
omitted option byte after its series legend; `chart-appearance.hex` remains
historical. The three current `chart-view-v2-*` fixtures contain empty appearance
series, so only their style tag changes (`fff8` to `fff7`).
